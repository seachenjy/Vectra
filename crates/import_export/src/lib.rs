use serde::{Deserialize, Serialize};
use skymemory_core::{MemoryNode, Property, Value, MemoryType, TemporalInfo, generate_node_id, now_millis, SkyError, SkyResult};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct SkyArchive {
    pub manifest: ArchiveManifest,
    pub nodes: Vec<MemoryNode>,
}

#[derive(Serialize, Deserialize)]
pub struct ArchiveManifest {
    pub version: String,
    pub node_count: usize,
    pub dimension: usize,
    pub created_at: u64,
}

pub fn export_archive(nodes: &[MemoryNode], dimension: usize) -> SkyArchive {
    SkyArchive {
        manifest: ArchiveManifest {
            version: "0.2.0".into(),
            node_count: nodes.len(),
            dimension,
            created_at: now_millis(),
        },
        nodes: nodes.to_vec(),
    }
}

pub fn save_archive<P: AsRef<Path>>(archive: &SkyArchive, path: P) -> SkyResult<()> {
    let data = serde_json::to_vec_pretty(archive)
        .map_err(|e| SkyError::Serialize(e.to_string()))?;
    std::fs::write(path, data)?;
    Ok(())
}

pub fn load_archive<P: AsRef<Path>>(path: P) -> SkyResult<SkyArchive> {
    let data = std::fs::read(path)?;
    serde_json::from_slice(&data)
        .map_err(|e| SkyError::Deserialize(e.to_string()))
}

pub fn import_jsonl<P: AsRef<Path>>(path: P, dimension: usize) -> SkyResult<Vec<MemoryNode>> {
    let content = std::fs::read_to_string(path)?;
    let mut nodes = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        let record: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| SkyError::Deserialize(e.to_string()))?;
        if let Some(node) = parse_json_record(&record, dimension)? {
            nodes.push(node);
        }
    }
    Ok(nodes)
}

pub fn import_csv<P: AsRef<Path>>(path: P, dimension: usize) -> SkyResult<Vec<MemoryNode>> {
    let content = std::fs::read_to_string(path)?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| SkyError::InvalidInput("empty CSV".into()))?;
    let headers: Vec<&str> = header.split(',').map(|s| s.trim()).collect();
    let mut nodes = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() { continue; }
        let fields: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if fields.len() < dimension {
            return Err(SkyError::DimensionMismatch { expected: dimension, got: fields.len() });
        }
        let mut vector = Vec::with_capacity(dimension);
        for i in 0..dimension {
            let val: f32 = fields[i].parse()
                .map_err(|_| SkyError::InvalidInput(format!("invalid float at col {}", i)))?;
            vector.push(val);
        }
        let mut properties = Vec::new();
        for i in dimension..fields.len().min(headers.len()) {
            properties.push(Property {
                key: headers[i].to_string(),
                value: Value::String(fields[i].to_string()),
            });
        }
        nodes.push(MemoryNode {
            id: generate_node_id(),
            vector,
            properties,
            temporal: TemporalInfo::default(),
            memory_type: MemoryType::Semantic,
        });
    }
    Ok(nodes)
}

fn parse_json_record(record: &serde_json::Value, dimension: usize) -> SkyResult<Option<MemoryNode>> {
    let obj = match record.as_object() {
        Some(o) => o,
        None => return Ok(None),
    };
    let vector: Vec<f32> = if let Some(arr) = obj.get("vector").and_then(|v| v.as_array()) {
        arr.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect()
    } else if let Some(arr) = obj.get("values").and_then(|v| v.as_array()) {
        arr.iter().map(|v| v.as_f64().unwrap_or(0.0) as f32).collect()
    } else {
        return Ok(None);
    };
    if vector.len() != dimension {
        return Err(SkyError::DimensionMismatch { expected: dimension, got: vector.len() });
    }
    let properties = if let Some(meta) = obj.get("metadata").and_then(|v| v.as_object()) {
        meta.iter().map(|(k, v)| Property {
            key: k.clone(),
            value: match v {
                serde_json::Value::String(s) => Value::String(s.clone()),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() { Value::Int(i) }
                    else { Value::Float(skymemory_core::OrderedF64(n.as_f64().unwrap_or(0.0))) }
                }
                serde_json::Value::Bool(b) => Value::Bool(*b),
                _ => Value::String(v.to_string()),
            },
        }).collect()
    } else {
        Vec::new()
    };
    Ok(Some(MemoryNode {
        id: generate_node_id(),
        vector,
        properties,
        temporal: TemporalInfo::default(),
        memory_type: MemoryType::Semantic,
    }))
}
