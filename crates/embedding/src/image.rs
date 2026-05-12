use candle_core::{DType, Device, IndexOp, Tensor};
use candle_nn::{Conv2dConfig, Module, VarBuilder};
use skymemory_core::{SkyError, SkyResult};
use std::path::Path;
use tracing::info;

use crate::model_manager;

struct ClipVisionConfig {
    image_size: usize,
    patch_size: usize,
    hidden_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    intermediate_size: usize,
}

impl ClipVisionConfig {
    fn from_json(json: &serde_json::Value) -> Self {
        let vc = json.get("vision_config").unwrap_or(json);
        ClipVisionConfig {
            image_size: vc["image_size"].as_u64().unwrap_or(224) as usize,
            patch_size: vc["patch_size"].as_u64().unwrap_or(32) as usize,
            hidden_size: vc["hidden_size"].as_u64().unwrap_or(768) as usize,
            num_hidden_layers: vc["num_hidden_layers"].as_u64().unwrap_or(12) as usize,
            num_attention_heads: vc["num_attention_heads"].as_u64().unwrap_or(12) as usize,
            intermediate_size: vc["intermediate_size"].as_u64().unwrap_or(3072) as usize,
        }
    }
}

struct ClipAttention {
    q_proj: candle_nn::Linear,
    k_proj: candle_nn::Linear,
    v_proj: candle_nn::Linear,
    out_proj: candle_nn::Linear,
    num_heads: usize,
    head_dim: usize,
}

impl ClipAttention {
    fn load(vb: VarBuilder, cfg: &ClipVisionConfig) -> SkyResult<Self> {
        let q = candle_nn::linear(cfg.hidden_size, cfg.hidden_size, vb.pp("q_proj")).map_err(candle_err)?;
        let k = candle_nn::linear(cfg.hidden_size, cfg.hidden_size, vb.pp("k_proj")).map_err(candle_err)?;
        let v = candle_nn::linear(cfg.hidden_size, cfg.hidden_size, vb.pp("v_proj")).map_err(candle_err)?;
        let o = candle_nn::linear(cfg.hidden_size, cfg.hidden_size, vb.pp("out_proj")).map_err(candle_err)?;
        let head_dim = cfg.hidden_size / cfg.num_attention_heads;
        Ok(Self { q_proj: q, k_proj: k, v_proj: v, out_proj: o, num_heads: cfg.num_attention_heads, head_dim })
    }

    fn forward(&self, x: &Tensor) -> SkyResult<Tensor> {
        let (b, s, h) = x.dims3().map_err(candle_err)?;
        let q = self.q_proj.forward(x).map_err(candle_err)?;
        let k = self.k_proj.forward(x).map_err(candle_err)?;
        let v = self.v_proj.forward(x).map_err(candle_err)?;

        let q = q.reshape((b, s, self.num_heads, self.head_dim)).map_err(candle_err)?
            .transpose(1, 2).map_err(candle_err)?;
        let k = k.reshape((b, s, self.num_heads, self.head_dim)).map_err(candle_err)?
            .transpose(1, 2).map_err(candle_err)?;
        let v = v.reshape((b, s, self.num_heads, self.head_dim)).map_err(candle_err)?
            .transpose(1, 2).map_err(candle_err)?;

        let scale = (self.head_dim as f64).sqrt();
        let k_t = k.transpose(candle_core::D::Minus2, candle_core::D::Minus1).map_err(candle_err)?;
        let attn = (q.matmul(&k_t).map_err(candle_err)? / scale).map_err(candle_err)?;
        let attn = candle_nn::ops::softmax_last_dim(&attn).map_err(candle_err)?;
        let out = attn.matmul(&v).map_err(candle_err)?;
        let out = out.transpose(1, 2).map_err(candle_err)?
            .reshape((b, s, h)).map_err(candle_err)?;
        let out = self.out_proj.forward(&out).map_err(candle_err)?;
        Ok(out)
    }
}

struct ClipMlp {
    fc1: candle_nn::Linear,
    fc2: candle_nn::Linear,
}

impl ClipMlp {
    fn load(vb: VarBuilder, cfg: &ClipVisionConfig) -> SkyResult<Self> {
        let fc1 = candle_nn::linear(cfg.hidden_size, cfg.intermediate_size, vb.pp("fc1")).map_err(candle_err)?;
        let fc2 = candle_nn::linear(cfg.intermediate_size, cfg.hidden_size, vb.pp("fc2")).map_err(candle_err)?;
        Ok(Self { fc1, fc2 })
    }

    fn forward(&self, x: &Tensor) -> SkyResult<Tensor> {
        let x = self.fc1.forward(x).map_err(candle_err)?;
        let x = x.gelu().map_err(candle_err)?;
        let x = self.fc2.forward(&x).map_err(candle_err)?;
        Ok(x)
    }
}

struct ClipEncoderLayer {
    self_attn: ClipAttention,
    mlp: ClipMlp,
    layer_norm1: candle_nn::LayerNorm,
    layer_norm2: candle_nn::LayerNorm,
}

impl ClipEncoderLayer {
    fn load(vb: VarBuilder, cfg: &ClipVisionConfig) -> SkyResult<Self> {
        let self_attn = ClipAttention::load(vb.pp("self_attn"), cfg)?;
        let mlp = ClipMlp::load(vb.pp("mlp"), cfg)?;
        let ln1 = candle_nn::layer_norm(cfg.hidden_size, 1e-5, vb.pp("layer_norm1")).map_err(candle_err)?;
        let ln2 = candle_nn::layer_norm(cfg.hidden_size, 1e-5, vb.pp("layer_norm2")).map_err(candle_err)?;
        Ok(Self { self_attn, mlp, layer_norm1: ln1, layer_norm2: ln2 })
    }

    fn forward(&self, x: &Tensor) -> SkyResult<Tensor> {
        let r = x;
        let x = self.layer_norm1.forward(x).map_err(candle_err)?;
        let x = self.self_attn.forward(&x)?;
        let x = (r + x).map_err(candle_err)?;
        let r = &x;
        let y = self.layer_norm2.forward(&x).map_err(candle_err)?;
        let y = self.mlp.forward(&y)?;
        let x = (r + y).map_err(candle_err)?;
        Ok(x)
    }
}

struct ClipVisionTransformer {
    patch_embedding: candle_nn::Conv2d,
    position_embedding: candle_nn::Embedding,
    layers: Vec<ClipEncoderLayer>,
    final_layer_norm: candle_nn::LayerNorm,
    pre_layrnorm: candle_nn::LayerNorm,
    post_layernorm: candle_nn::LayerNorm,
    num_positions: usize,
    image_size: usize,
    patch_size: usize,
}

impl ClipVisionTransformer {
    fn load(vb: VarBuilder, cfg: &ClipVisionConfig) -> SkyResult<Self> {
        let num_patches = (cfg.image_size / cfg.patch_size).pow(2);
        let num_positions = num_patches + 1;

        let conv2d_cfg = Conv2dConfig {
            stride: cfg.patch_size,
            ..Default::default()
        };
        let patch_emb = candle_nn::conv2d(
            3, cfg.hidden_size, cfg.patch_size, conv2d_cfg,
            vb.pp("patch_embedding"),
        ).map_err(candle_err)?;

        let pos_emb = candle_nn::embedding(num_positions, cfg.hidden_size, vb.pp("position_embedding"))
            .map_err(candle_err)?;

        let mut layers = Vec::with_capacity(cfg.num_hidden_layers);
        for i in 0..cfg.num_hidden_layers {
            let layer = ClipEncoderLayer::load(vb.pp(&format!("layers.{}", i)), cfg)?;
            layers.push(layer);
        }

        let fln = candle_nn::layer_norm(cfg.hidden_size, 1e-5, vb.pp("pre_layrnorm"))
            .map_err(candle_err)?;
        let pln = candle_nn::layer_norm(cfg.hidden_size, 1e-5, vb.pp("post_layernorm"))
            .map_err(candle_err)?;

        Ok(Self {
            patch_embedding: patch_emb,
            position_embedding: pos_emb,
            layers,
            final_layer_norm: candle_nn::layer_norm(cfg.hidden_size, 1e-5, vb.pp("final_layer_norm"))
                .map_err(candle_err)?,
            pre_layrnorm: fln,
            post_layernorm: pln,
            num_positions,
            image_size: cfg.image_size,
            patch_size: cfg.patch_size,
        })
    }

    fn forward(&self, pixel_values: &Tensor) -> SkyResult<Tensor> {
        let x = self.patch_embedding.forward(pixel_values).map_err(candle_err)?;
        let (b, c, h, w) = x.dims4().map_err(candle_err)?;
        let x = x.reshape((b, c, h * w)).map_err(candle_err)?
            .transpose(1, 2).map_err(candle_err)?;

        let pos_ids = Tensor::arange(0u32, self.num_positions as u32, x.device()).map_err(candle_err)?;
        let pos = self.position_embedding.forward(&pos_ids).map_err(candle_err)?;
        let pos = pos.unsqueeze(0).map_err(candle_err)?;
        let x = (x + pos).map_err(candle_err)?;
        let x = self.pre_layrnorm.forward(&x).map_err(candle_err)?;

        let mut x = x;
        for layer in &self.layers {
            x = layer.forward(&x)?;
        }

        let x = self.final_layer_norm.forward(&x).map_err(candle_err)?;
        let cls_token = x.i((.., 0, ..)).map_err(candle_err)?;
        let cls_token = self.post_layernorm.forward(&cls_token).map_err(candle_err)?;
        Ok(cls_token)
    }
}

pub struct ImageEmbedder {
    model: ClipVisionTransformer,
    device: Device,
    dimension: usize,
    image_size: usize,
}

impl ImageEmbedder {
    pub fn load_or_download(models_dir: &Path, mirror: Option<&str>) -> SkyResult<Self> {
        let dir = model_manager::ensure_model_files(
            models_dir,
            model_manager::CLIP_MODEL_ID,
            &model_manager::clip_vision_files(),
            mirror,
        )?;

        let config_str = std::fs::read_to_string(dir.join("config.json"))?;
        let config_json: serde_json::Value = serde_json::from_str(&config_str)
            .map_err(|e| SkyError::InvalidInput(format!("invalid config: {}", e)))?;
        let cfg = ClipVisionConfig::from_json(&config_json);

        let model_path = dir.join("model.safetensors");
        let device = Device::Cpu;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[&model_path], DType::F32, &device)
                .map_err(candle_err)?
        };

        let model = ClipVisionTransformer::load(vb.pp("vision_model"), &cfg)?;
        let dimension = cfg.hidden_size;

        info!("loaded CLIP vision model, dimension={}", dimension);

        Ok(ImageEmbedder {
            model,
            device,
            dimension,
            image_size: cfg.image_size,
        })
    }

    pub fn embed_image(&self, image_data: &[u8]) -> SkyResult<Vec<f32>> {
        let img = image::load_from_memory(image_data)
            .map_err(|e| SkyError::InvalidInput(format!("image decode failed: {}", e)))?
            .resize_to_fill(
                self.image_size as u32,
                self.image_size as u32,
                image::imageops::FilterType::Lanczos3,
            )
            .to_rgb8();

        let (w, h) = img.dimensions();
        let mut pixels = Vec::with_capacity(3 * h as usize * w as usize);
        for y in 0..h {
            for x in 0..w {
                let px = img.get_pixel(x, y);
                pixels.push(px[0] as f32 / 255.0);
                pixels.push(px[1] as f32 / 255.0);
                pixels.push(px[2] as f32 / 255.0);
            }
        }

        let means: [f32; 3] = [0.48145466, 0.4578275, 0.40821073];
        let stds: [f32; 3] = [0.26862954, 0.26130258, 0.27577711];
        for i in 0..pixels.len() {
            let c = i % 3;
            pixels[i] = (pixels[i] - means[c]) / stds[c];
        }

        let pixel_tensor = Tensor::from_vec(
            pixels,
            (1, 3, h as usize, w as usize),
            &self.device,
        ).map_err(candle_err)?;

        let embeddings = self.model.forward(&pixel_tensor)?;
        let embeddings = normalize_l2(&embeddings)?;
        let result = embeddings.squeeze(0).map_err(candle_err)?.to_vec1::<f32>().map_err(candle_err)?;
        Ok(result)
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn model_name(&self) -> &str {
        model_manager::CLIP_MODEL_ID
    }
}

fn normalize_l2(tensor: &Tensor) -> SkyResult<Tensor> {
    let norm = tensor.sqr().map_err(candle_err)?.sum_keepdim(1).map_err(candle_err)?.sqrt().map_err(candle_err)?;
    let result = (tensor / norm).map_err(candle_err)?;
    Ok(result)
}

fn candle_err(e: candle_core::Error) -> SkyError {
    SkyError::InvalidInput(format!("candle error: {}", e))
}
