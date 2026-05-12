pub mod chunker;
pub mod model_manager;
pub mod text;
pub mod image;

use skymemory_core::SkyResult;

pub trait Embedder: Send + Sync {
    fn embed_text(&self, text: &str) -> SkyResult<Vec<f32>>;
    fn embed_image(&self, image_data: &[u8]) -> SkyResult<Vec<f32>>;
    fn dimension(&self) -> usize;
    fn model_info(&self) -> EmbeddingModelInfo;
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EmbeddingModelInfo {
    pub text_model: Option<String>,
    pub image_model: Option<String>,
    pub text_ready: bool,
    pub image_ready: bool,
    pub text_dimension: usize,
    pub image_dimension: usize,
}

pub struct EmbeddingEngine {
    text_engine: Option<text::TextEmbedder>,
    image_engine: Option<image::ImageEmbedder>,
}

impl EmbeddingEngine {
    pub fn new(
        models_dir: &std::path::Path,
        hf_mirror: Option<&str>,
    ) -> SkyResult<Self> {
        let text_engine = text::TextEmbedder::load_or_download(models_dir, hf_mirror).ok();
        let image_engine = image::ImageEmbedder::load_or_download(models_dir, hf_mirror).ok();
        Ok(EmbeddingEngine { text_engine, image_engine })
    }

    pub fn new_unavailable() -> Self {
        EmbeddingEngine { text_engine: None, image_engine: None }
    }

    pub fn embed_text(&self, text: &str) -> SkyResult<Vec<f32>> {
        match &self.text_engine {
            Some(e) => e.embed_text(text),
            None => Err(skymemory_core::SkyError::InvalidInput(
                "text embedding model not loaded".into(),
            )),
        }
    }

    pub fn embed_image(&self, image_data: &[u8]) -> SkyResult<Vec<f32>> {
        match &self.image_engine {
            Some(e) => e.embed_image(image_data),
            None => Err(skymemory_core::SkyError::InvalidInput(
                "image embedding model not loaded".into(),
            )),
        }
    }

    pub fn model_info(&self) -> EmbeddingModelInfo {
        EmbeddingModelInfo {
            text_model: self.text_engine.as_ref().map(|e| e.model_name().to_string()),
            image_model: self.image_engine.as_ref().map(|e| e.model_name().to_string()),
            text_ready: self.text_engine.is_some(),
            image_ready: self.image_engine.is_some(),
            text_dimension: self.text_engine.as_ref().map(|e| e.dimension()).unwrap_or(0),
            image_dimension: self.image_engine.as_ref().map(|e| e.dimension()).unwrap_or(0),
        }
    }

    pub fn text_dimension(&self) -> Option<usize> {
        self.text_engine.as_ref().map(|e| e.dimension())
    }

    pub fn image_dimension(&self) -> Option<usize> {
        self.image_engine.as_ref().map(|e| e.dimension())
    }
}
