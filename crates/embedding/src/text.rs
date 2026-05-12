use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config as BertConfig};
use skymemory_core::{SkyError, SkyResult};
use std::path::Path;
use tokenizers::Tokenizer;
use tracing::info;

use crate::model_manager;

pub struct TextEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
    dimension: usize,
}

impl TextEmbedder {
    pub fn load_or_download(models_dir: &Path, mirror: Option<&str>) -> SkyResult<Self> {
        let dir = model_manager::ensure_model_files(
            models_dir,
            model_manager::BGE_MODEL_ID,
            &model_manager::bge_files(),
            mirror,
        )?;

        let config_path = dir.join("config.json");
        let tokenizer_path = dir.join("tokenizer.json");
        let model_path = dir.join("model.safetensors");

        let config_str = std::fs::read_to_string(&config_path)?;
        let config: BertConfig = serde_json::from_str(&config_str)
            .map_err(|e| SkyError::InvalidInput(format!("invalid bert config: {}", e)))?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| SkyError::InvalidInput(format!("failed to load tokenizer: {}", e)))?;

        let device = Device::Cpu;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[&model_path], DType::F32, &device)
                .map_err(candle_err)?
        };

        let model = BertModel::load(vb, &config).map_err(candle_err)?;
        let dimension = config.hidden_size;
        info!("loaded BGE text embedding model, dimension={}", dimension);

        Ok(TextEmbedder {
            model,
            tokenizer,
            device,
            dimension,
        })
    }

    pub fn embed_text(&self, text: &str) -> SkyResult<Vec<f32>> {
        let encoding = self.tokenizer
            .encode(text, true)
            .map_err(|e| SkyError::InvalidInput(format!("tokenization failed: {}", e)))?;

        let token_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        let token_type_ids = vec![0u32; token_ids.len()];

        let input_ids = Tensor::new(token_ids, &self.device).map_err(candle_err)?.unsqueeze(0).map_err(candle_err)?;
        let attention_mask = Tensor::new(attention_mask, &self.device).map_err(candle_err)?.unsqueeze(0).map_err(candle_err)?;
        let token_type_ids = Tensor::new(token_type_ids.as_slice(), &self.device).map_err(candle_err)?.unsqueeze(0).map_err(candle_err)?;

        let output = self.model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(candle_err)?;

        let embeddings = mean_pooling(&output, &attention_mask)?;
        let embeddings = normalize_l2(&embeddings)?;

        let result = embeddings.squeeze(0).map_err(candle_err)?.to_vec1::<f32>().map_err(candle_err)?;
        Ok(result)
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn model_name(&self) -> &str {
        model_manager::BGE_MODEL_ID
    }
}

fn mean_pooling(hidden_states: &Tensor, attention_mask: &Tensor) -> SkyResult<Tensor> {
    let mask = attention_mask.unsqueeze(2).map_err(candle_err)?.to_dtype(DType::F32).map_err(candle_err)?;
    let masked = (hidden_states * &mask).map_err(candle_err)?;
    let sum = masked.sum(1).map_err(candle_err)?;
    let count = mask.sum(1).map_err(candle_err)?;
    let count = count.clamp(1e-9, f32::MAX).map_err(candle_err)?;
    let result = (sum / count).map_err(candle_err)?;
    Ok(result)
}

fn normalize_l2(tensor: &Tensor) -> SkyResult<Tensor> {
    let norm = tensor.sqr().map_err(candle_err)?.sum_keepdim(1).map_err(candle_err)?.sqrt().map_err(candle_err)?;
    let result = (tensor / norm).map_err(candle_err)?;
    Ok(result)
}

fn candle_err(e: candle_core::Error) -> SkyError {
    SkyError::InvalidInput(format!("candle error: {}", e))
}
