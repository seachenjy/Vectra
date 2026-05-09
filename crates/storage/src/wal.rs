use serde::{Deserialize, Serialize};
use skymemory_core::{MemoryNode, NodeId, SkyError, SkyResult};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub enum WalEntry {
    Insert(MemoryNode),
    Delete(NodeId),
    UpdateProperties { id: NodeId, properties: Vec<skymemory_core::Property> },
    UpdateTemporal { id: NodeId, temporal: skymemory_core::TemporalInfo },
    Checkpoint,
}

pub struct Wal {
    path: PathBuf,
    writer: BufWriter<File>,
}

impl Wal {
    pub fn open<P: AsRef<Path>>(path: P) -> SkyResult<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())?;
        Ok(Wal {
            path: path.as_ref().to_path_buf(),
            writer: BufWriter::new(file),
        })
    }

    pub fn append(&mut self, entry: &WalEntry) -> SkyResult<()> {
        let encoded = bincode::serialize(entry)
            .map_err(|e| SkyError::Serialize(e.to_string()))?;
        let len = encoded.len() as u32;
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&encoded)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> SkyResult<()> {
        self.writer.flush()?;
        Ok(())
    }

    pub fn read_all<P: AsRef<Path>>(path: P) -> SkyResult<Vec<WalEntry>> {
        let file = match File::open(path.as_ref()) {
            Ok(f) => f,
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();
        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf) {
                Ok(()) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut data = vec![0u8; len];
            reader.read_exact(&mut data)?;
            let entry: WalEntry = bincode::deserialize(&data)
                .map_err(|e| SkyError::Deserialize(e.to_string()))?;
            entries.push(entry);
        }
        Ok(entries)
    }

    pub fn truncate(&mut self) -> SkyResult<()> {
        self.writer.flush()?;
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        self.writer = BufWriter::new(file);
        Ok(())
    }
}

pub struct WalReplayer {
    entries: Vec<WalEntry>,
}

impl WalReplayer {
    pub fn new(entries: Vec<WalEntry>) -> Self {
        WalReplayer { entries }
    }

    pub fn replay_into(&self, nodes: &mut Vec<MemoryNode>) {
        for entry in &self.entries {
            match entry {
                WalEntry::Insert(node) => {
                    nodes.push(node.clone());
                }
                WalEntry::Delete(id) => {
                    nodes.retain(|n| n.id != *id);
                }
                WalEntry::UpdateProperties { id, properties } => {
                    if let Some(node) = nodes.iter_mut().find(|n| n.id == *id) {
                        node.properties = properties.clone();
                    }
                }
                WalEntry::UpdateTemporal { id, temporal } => {
                    if let Some(node) = nodes.iter_mut().find(|n| n.id == *id) {
                        node.temporal = temporal.clone();
                    }
                }
                WalEntry::Checkpoint => {}
            }
        }
    }
}
