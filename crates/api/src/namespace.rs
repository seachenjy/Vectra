use skymemory_core::{SkyError, SkyResult};
use skymemory_query::QueryEngine;
use skymemory_storage::SegmentManager;
use skymemory_backup::BackupManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

const DEFAULT_NAMESPACE: &str = "default";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamespaceInfo {
    pub name: String,
    pub dimension: usize,
    pub node_count: usize,
    pub edge_count: usize,
    pub created_at: u64,
}

pub struct Namespace {
    pub name: String,
    pub dimension: usize,
    pub engine: QueryEngine,
    pub segment_mgr: RwLock<SegmentManager>,
    pub backup: BackupManager,
    pub created_at: u64,
}

impl Namespace {
    pub fn info(&self) -> NamespaceInfo {
        NamespaceInfo {
            name: self.name.clone(),
            dimension: self.dimension,
            node_count: self.engine.node_count(),
            edge_count: self.engine.graph_store.read().edge_count(),
            created_at: self.created_at,
        }
    }
}

pub struct NamespaceManager {
    base_dir: PathBuf,
    default_dimension: usize,
    namespaces: RwLock<HashMap<String, Arc<Namespace>>>,
}

impl NamespaceManager {
    pub fn new<P: AsRef<Path>>(base_dir: P, default_dimension: usize) -> SkyResult<Self> {
        let base = base_dir.as_ref().to_path_buf();
        let manager = NamespaceManager {
            base_dir: base,
            default_dimension,
            namespaces: RwLock::new(HashMap::new()),
        };
        Ok(manager)
    }

    pub async fn initialize(&self) -> SkyResult<()> {
        self.migrate_legacy_data().await?;
        self.load_all_namespaces().await?;

        if self.namespaces.read().await.is_empty() {
            self.create_namespace_internal(DEFAULT_NAMESPACE, self.default_dimension).await?;
        }

        Ok(())
    }

    async fn migrate_legacy_data(&self) -> SkyResult<()> {
        let legacy_segments = self.base_dir.join("segments");
        if !legacy_segments.exists() {
            return Ok(());
        }

        let default_dir = self.base_dir.join("namespaces").join(DEFAULT_NAMESPACE);
        if default_dir.exists() {
            info!("default namespace already exists, skipping migration");
            return Ok(());
        }

        info!("migrating legacy data to namespaces/default/");
        std::fs::create_dir_all(&default_dir)?;

        for entry in std::fs::read_dir(&self.base_dir)?.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "segments" || name_str == "backups" {
                let dest = default_dir.join(&*name_str);
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    copy_dir_recursive(&entry.path(), &dest)?;
                }
            }
        }

        info!("migration complete");
        Ok(())
    }

    async fn load_all_namespaces(&self) -> SkyResult<()> {
        let namespaces_dir = self.base_dir.join("namespaces");
        if !namespaces_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&namespaces_dir)?.flatten() {
            if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                continue;
            }
            let ns_name = entry.file_name().to_string_lossy().to_string();
            let ns_dir = entry.path();

            let meta_path = ns_dir.join("meta.json");
            let (dimension, created_at) = if meta_path.exists() {
                let data = std::fs::read(&meta_path)?;
                let meta: NsMeta = serde_json::from_slice(&data)
                    .unwrap_or(NsMeta { dimension: self.default_dimension, created_at: skymemory_core::now_millis() });
                (meta.dimension, meta.created_at)
            } else {
                (self.default_dimension, skymemory_core::now_millis())
            };

            match self.load_namespace(&ns_name, dimension, created_at).await {
                Ok(ns) => {
                    self.namespaces.write().await.insert(ns_name.clone(), ns);
                    info!("loaded namespace: {}", ns_name);
                }
                Err(e) => {
                    warn!("failed to load namespace '{}': {}", ns_name, e);
                }
            }
        }

        Ok(())
    }

    async fn load_namespace(&self, name: &str, dimension: usize, created_at: u64) -> SkyResult<Arc<Namespace>> {
        let ns_dir = self.base_dir.join("namespaces").join(name);

        let seg_mgr = SegmentManager::open(&ns_dir)?;

        let backup = BackupManager::new(&ns_dir);

        let engine = QueryEngine::new(dimension);
        let nodes = seg_mgr.snapshot()?;
        let node_count = nodes.len();
        for node in nodes {
            let _ = engine.insert_node(node);
        }

        info!("namespace '{}' loaded with {} nodes", name, node_count);

        Ok(Arc::new(Namespace {
            name: name.to_string(),
            dimension,
            engine,
            segment_mgr: RwLock::new(seg_mgr),
            backup,
            created_at,
        }))
    }

    async fn create_namespace_internal(&self, name: &str, dimension: usize) -> SkyResult<Arc<Namespace>> {
        validate_namespace_name(name)?;

        {
            let namespaces = self.namespaces.read().await;
            if namespaces.contains_key(name) {
                return Err(SkyError::InvalidInput(format!("namespace '{}' already exists", name)));
            }
        }

        let ns_dir = self.base_dir.join("namespaces").join(name);
        std::fs::create_dir_all(&ns_dir)?;

        let meta = NsMeta {
            dimension,
            created_at: skymemory_core::now_millis(),
        };
        let meta_data = serde_json::to_vec_pretty(&meta)
            .map_err(|e| SkyError::Serialize(e.to_string()))?;
        std::fs::write(ns_dir.join("meta.json"), meta_data)?;

        let seg_mgr = SegmentManager::open(&ns_dir)?;
        let backup = BackupManager::new(&ns_dir);
        let engine = QueryEngine::new(dimension);

        let ns = Arc::new(Namespace {
            name: name.to_string(),
            dimension,
            engine,
            segment_mgr: RwLock::new(seg_mgr),
            backup,
            created_at: meta.created_at,
        });

        self.namespaces.write().await.insert(name.to_string(), ns.clone());
        info!("created namespace: {} (dimension={})", name, dimension);
        Ok(ns)
    }

    pub async fn create_namespace(&self, name: &str, dimension: usize) -> SkyResult<NamespaceInfo> {
        let ns = self.create_namespace_internal(name, dimension).await?;
        Ok(ns.info())
    }

    pub async fn delete_namespace(&self, name: &str) -> SkyResult<()> {
        if name == DEFAULT_NAMESPACE {
            return Err(SkyError::InvalidInput("cannot delete the default namespace".to_string()));
        }

        let ns = {
            let mut namespaces = self.namespaces.write().await;
            namespaces.remove(name)
        };

        if ns.is_none() {
            return Err(SkyError::NotFound(format!("namespace '{}'", name)));
        }

        let ns_dir = self.base_dir.join("namespaces").join(name);
        if ns_dir.exists() {
            std::fs::remove_dir_all(&ns_dir)?;
        }

        info!("deleted namespace: {}", name);
        Ok(())
    }

    pub async fn get_namespace(&self, name: &str) -> Option<Arc<Namespace>> {
        self.namespaces.read().await.get(name).cloned()
    }

    pub async fn list_namespaces(&self) -> Vec<NamespaceInfo> {
        self.namespaces.read().await.values().map(|ns| ns.info()).collect()
    }

    pub async fn default_dimension(&self) -> usize {
        self.default_dimension
    }
}

fn validate_namespace_name(name: &str) -> SkyResult<()> {
    if name.is_empty() || name.len() > 64 {
        return Err(SkyError::InvalidInput("namespace name must be 1-64 characters".to_string()));
    }

    let first = name.as_bytes()[0];
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return Err(SkyError::InvalidInput("namespace name must start with lowercase letter or digit".to_string()));
    }

    if !name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_') {
        return Err(SkyError::InvalidInput("namespace name can only contain lowercase letters, digits, hyphens, and underscores".to_string()));
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> SkyResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let dest_path = dst.join(entry.file_name());
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct NsMeta {
    dimension: usize,
    created_at: u64,
}
