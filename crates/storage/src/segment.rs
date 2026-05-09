use serde::{Deserialize, Serialize};
use skymemory_core::{MemoryNode, NodeId, SkyError, SkyResult};
use std::fs;
use std::path::{Path, PathBuf};

use crate::wal::{Wal, WalEntry};

#[derive(Debug, Serialize, Deserialize)]
pub struct SegmentManifest {
    pub segment_id: String,
    pub node_count: usize,
    pub created_at: u64,
}

pub struct Segment {
    pub path: PathBuf,
    pub manifest: SegmentManifest,
    pub nodes: Vec<MemoryNode>,
}

impl Segment {
    pub fn new<P: AsRef<Path>>(base_dir: P, segment_id: String) -> SkyResult<Self> {
        let path = base_dir.as_ref().join("segments").join(&segment_id);
        fs::create_dir_all(&path)?;
        let manifest = SegmentManifest {
            segment_id: segment_id.clone(),
            node_count: 0,
            created_at: skymemory_core::now_millis(),
        };
        Ok(Segment { path, manifest, nodes: Vec::new() })
    }

    pub fn load<P: AsRef<Path>>(base_dir: P, segment_id: &str) -> SkyResult<Self> {
        let path = base_dir.as_ref().join("segments").join(segment_id);
        if !path.exists() {
            return Err(SkyError::NotFound(format!("segment {}", segment_id)));
        }
        let nodes_path = path.join("vectors.bin");
        let nodes: Vec<MemoryNode> = if nodes_path.exists() {
            let data = fs::read(&nodes_path)?;
            bincode::deserialize(&data).map_err(|e| SkyError::Deserialize(e.to_string()))?
        } else {
            Vec::new()
        };
        let manifest = SegmentManifest {
            segment_id: segment_id.to_string(),
            node_count: nodes.len(),
            created_at: skymemory_core::now_millis(),
        };
        Ok(Segment { path, manifest, nodes })
    }

    pub fn insert(&mut self, node: MemoryNode) -> SkyResult<()> {
        let mut wal = Wal::open(self.path.join("wal.log"))?;
        wal.append(&WalEntry::Insert(node.clone()))?;
        self.nodes.push(node);
        self.manifest.node_count = self.nodes.len();
        Ok(())
    }

    pub fn delete(&mut self, node_id: NodeId) -> SkyResult<bool> {
        let before = self.nodes.len();
        self.nodes.retain(|n| n.id != node_id);
        let removed = self.nodes.len() < before;
        if removed {
            let mut wal = Wal::open(self.path.join("wal.log"))?;
            wal.append(&WalEntry::Delete(node_id))?;
            self.manifest.node_count = self.nodes.len();
        }
        Ok(removed)
    }

    pub fn get(&self, node_id: NodeId) -> Option<&MemoryNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    pub fn get_mut(&mut self, node_id: NodeId) -> Option<&mut MemoryNode> {
        self.nodes.iter_mut().find(|n| n.id == node_id)
    }

    pub fn flush_to_disk(&mut self) -> SkyResult<()> {
        let encoded = bincode::serialize(&self.nodes)
            .map_err(|e| SkyError::Serialize(e.to_string()))?;
        fs::write(self.path.join("vectors.bin"), &encoded)?;
        let manifest_data = serde_json::to_vec_pretty(&self.manifest)
            .map_err(|e| SkyError::Serialize(e.to_string()))?;
        fs::write(self.path.join("meta.json"), &manifest_data)?;
        Ok(())
    }

    pub fn compact(&mut self) -> SkyResult<()> {
        self.flush_to_disk()?;
        let wal_path = self.path.join("wal.log");
        if wal_path.exists() {
            fs::remove_file(&wal_path)?;
        }
        Ok(())
    }
}

pub struct SegmentManager {
    pub base_dir: PathBuf,
    pub segments: Vec<Segment>,
}

impl SegmentManager {
    pub fn open<P: AsRef<Path>>(base_dir: P) -> SkyResult<Self> {
        let base = base_dir.as_ref().to_path_buf();
        let segments_dir = base.join("segments");
        fs::create_dir_all(&segments_dir)?;

        let mut segments = Vec::new();
        if segments_dir.exists() {
            for entry in fs::read_dir(&segments_dir)?.flatten() {
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let seg_id = entry.file_name().to_string_lossy().to_string();
                    let mut seg = Segment::load(&base, &seg_id)?;
                    let wal_path = seg.path.join("wal.log");
                    let wal_entries = Wal::read_all(&wal_path)?;
                    if !wal_entries.is_empty() {
                        let replayer = crate::wal::WalReplayer::new(wal_entries);
                        replayer.replay_into(&mut seg.nodes);
                        seg.manifest.node_count = seg.nodes.len();
                    }
                    segments.push(seg);
                }
            }
        }

        if segments.is_empty() {
            let seg_id = format!("seg_{}", skymemory_core::now_millis());
            let seg = Segment::new(&base, seg_id)?;
            segments.push(seg);
        }

        Ok(SegmentManager { base_dir: base, segments })
    }

    pub fn active_segment(&mut self) -> &mut Segment {
        self.segments.last_mut().unwrap()
    }

    pub fn insert(&mut self, node: MemoryNode) -> SkyResult<()> {
        self.active_segment().insert(node)
    }

    pub fn delete(&mut self, node_id: NodeId) -> SkyResult<bool> {
        for seg in &mut self.segments {
            if seg.delete(node_id)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn find_node(&self, node_id: NodeId) -> Option<&MemoryNode> {
        for seg in &self.segments {
            if let Some(node) = seg.get(node_id) {
                return Some(node);
            }
        }
        None
    }

    pub fn all_nodes(&self) -> Vec<&MemoryNode> {
        self.segments.iter().flat_map(|s| s.nodes.iter()).collect()
    }

    pub fn all_nodes_count(&self) -> usize {
        self.segments.iter().map(|s| s.nodes.len()).sum()
    }

    pub fn flush_all(&mut self) -> SkyResult<()> {
        for seg in &mut self.segments {
            seg.flush_to_disk()?;
        }
        Ok(())
    }

    pub fn compact_all(&mut self) -> SkyResult<()> {
        for seg in &mut self.segments {
            seg.compact()?;
        }
        Ok(())
    }

    pub fn snapshot(&self) -> SkyResult<Vec<MemoryNode>> {
        let mut all = Vec::new();
        for seg in &self.segments {
            all.extend(seg.nodes.clone());
        }
        Ok(all)
    }
}
