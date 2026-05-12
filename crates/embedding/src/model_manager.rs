use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub const BGE_MODEL_ID: &str = "BAAI/bge-small-zh-v1.5";
pub const CLIP_MODEL_ID: &str = "openai/clip-vit-base-patch32";

pub struct ModelPaths {
    pub tokenizer_path: PathBuf,
    pub model_path: PathBuf,
}

pub fn model_dir(base: &Path, model_id: &str) -> PathBuf {
    let safe_name = model_id.replace('/', "--");
    base.join(safe_name)
}

pub fn ensure_model_files(
    base: &Path,
    model_id: &str,
    files: &[&str],
    mirror: Option<&str>,
) -> Result<PathBuf, skymemory_core::SkyError> {
    let dir = model_dir(base, model_id);
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }

    let all_exist = files.iter().all(|f| dir.join(f).exists());
    if all_exist {
        return Ok(dir);
    }

    let base_url = match mirror {
        Some(m) => format!("{}/{}", m.trim_end_matches('/'), model_id),
        None => format!("https://huggingface.co/{}/resolve/main", model_id),
    };

    for file in files {
        let dest = dir.join(file);
        if dest.exists() {
            continue;
        }
        let url = format!("{}/{}", base_url, file);
        info!("downloading {} from {}", file, url);
        match download_file_sync(&url, &dest) {
            Ok(()) => info!("downloaded {}", file),
            Err(e) => {
                warn!("failed to download {}: {}", file, e);
                return Err(skymemory_core::SkyError::InvalidInput(
                    format!("failed to download model file {}: {}", file, e),
                ));
            }
        }
    }

    Ok(dir)
}

fn download_file_sync(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let parent = dest.parent().unwrap();
    if !parent.exists() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = dest.with_extension("tmp");
    let resp = reqwest::blocking::get(url)?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()).into());
    }
    let bytes = resp.bytes()?;
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, dest)?;
    Ok(())
}

pub fn bge_files() -> Vec<&'static str> {
    vec![
        "tokenizer.json",
        "model.safetensors",
        "config.json",
    ]
}

pub fn clip_vision_files() -> Vec<&'static str> {
    vec![
        "preprocessor_config.json",
        "model.safetensors",
        "config.json",
    ]
}
