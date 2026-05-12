use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct CreateMemoryReq {
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub memory_type: Option<String>,
}

#[derive(Deserialize)]
pub struct ImportTextReq {
    pub text: String,
    #[serde(default = "default_true")]
    pub chunk: bool,
    pub chunk_size: Option<usize>,
    pub chunk_overlap: Option<usize>,
    pub memory_type: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

fn default_true() -> bool { true }


#[derive(Deserialize)]
pub struct SearchReq {
    pub vector: Vec<f32>,
    pub k: Option<usize>,
    pub metric: Option<String>,
}

#[derive(Deserialize)]
pub struct QueryReq {
    pub query: String,
    pub k: Option<usize>,
    pub metric: Option<String>,
}

#[derive(Serialize)]
pub struct MemoryItem {
    pub id: u64,
    pub score: f32,
    pub distance: f64,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub memory_type: String,
    pub created_at: u64,
    pub access_count: u32,
    pub decay_score: f32,
}

#[derive(Serialize)]
pub struct InfoResp {
    pub node_count: usize,
    pub edge_count: usize,
    pub dimension: usize,
    pub property_schema: HashMap<String, Vec<String>>,
}

#[derive(Serialize)]
pub struct SystemMetrics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub segment_count: usize,
    pub hot_memories: usize,
    pub cold_memories: usize,
}

#[derive(Deserialize)]
pub struct ImportReq {
    pub format: String,
    pub data: Option<String>,
    pub path: Option<String>,
}

#[derive(Deserialize)]
pub struct EdgeReq {
    pub from: u64,
    pub to: u64,
    pub edge_type: String,
    pub weight: Option<f32>,
}

#[derive(Serialize)]
pub struct EdgeItem {
    pub from: u64,
    pub to: u64,
    pub edge_type: String,
    pub weight: f32,
}

#[derive(Deserialize)]
pub struct SnapshotReq {
    pub action: String,
    pub snapshot_id: Option<String>,
}

// ── Database types ──────────────────────────────────────────

#[derive(Deserialize)]
pub struct DbConnectReq {
    pub name: String,
    pub db_type: String,
    pub dsn: String,
    pub max_connections: Option<u32>,
}

#[derive(Deserialize)]
pub struct DbQueryReq {
    pub connection: String,
    pub sql: String,
}

#[derive(Deserialize)]
pub struct DbDisconnectReq {
    pub name: String,
}

#[derive(Deserialize)]
pub struct DbTableReq {
    pub connection: String,
    pub table: String,
}

/// Request to import SQL query results into the vector store
#[derive(Deserialize)]
pub struct DbImportReq {
    pub connection: String,
    pub sql: String,
    /// Column name that contains the vector data (comma-separated floats or JSON array)
    pub vector_column: String,
    /// Columns to store as metadata (if empty, all non-vector columns are used)
    pub metadata_columns: Option<Vec<String>>,
    /// Memory type: "semantic" or "episodic"
    pub memory_type: Option<String>,
}
