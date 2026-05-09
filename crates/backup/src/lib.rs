use serde::{Deserialize, Serialize};
use skymemory_core::{MemoryNode, SkyError, SkyResult};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub created_at: u64,
    pub node_count: usize,
    pub nodes: Vec<MemoryNode>,
}

pub struct BackupManager {
    pub backup_dir: PathBuf,
}

impl BackupManager {
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Self {
        BackupManager {
            backup_dir: data_dir.as_ref().join("backups"),
        }
    }

    pub fn create_snapshot(&self, nodes: &[MemoryNode]) -> SkyResult<Snapshot> {
        std::fs::create_dir_all(&self.backup_dir)?;
        let snapshot_id = format!("snap_{}", skymemory_core::now_millis());
        let snapshot = Snapshot {
            snapshot_id: snapshot_id.clone(),
            created_at: skymemory_core::now_millis(),
            node_count: nodes.len(),
            nodes: nodes.to_vec(),
        };
        let path = self.backup_dir.join(format!("{}.json", snapshot_id));
        let data = serde_json::to_vec_pretty(&snapshot)
            .map_err(|e| SkyError::Serialize(e.to_string()))?;
        std::fs::write(path, data)?;
        Ok(snapshot)
    }

    pub fn list_snapshots(&self) -> SkyResult<Vec<String>> {
        std::fs::create_dir_all(&self.backup_dir)?;
        let mut snapshots = Vec::new();
        for entry in std::fs::read_dir(&self.backup_dir)?.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.ends_with(".json") {
                snapshots.push(fname.trim_end_matches(".json").to_string());
            }
        }
        snapshots.sort();
        Ok(snapshots)
    }

    pub fn load_snapshot(&self, snapshot_id: &str) -> SkyResult<Snapshot> {
        let path = self.backup_dir.join(format!("{}.json", snapshot_id));
        let data = std::fs::read(&path)?;
        serde_json::from_slice(&data)
            .map_err(|e| SkyError::Deserialize(e.to_string()))
    }

    pub fn restore_snapshot(&self, snapshot_id: &str) -> SkyResult<Vec<MemoryNode>> {
        let snapshot = self.load_snapshot(snapshot_id)?;
        Ok(snapshot.nodes)
    }

    pub fn delete_snapshot(&self, snapshot_id: &str) -> SkyResult<()> {
        let path = self.backup_dir.join(format!("{}.json", snapshot_id));
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
