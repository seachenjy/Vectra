use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type NodeId = u64;
pub type SymbolId = u32;
pub type TextId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    SimilarTo,
    DerivedFrom,
    PartOf,
    TemporallyAfter,
    Contradicts,
    References,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub edge_type: EdgeType,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(OrderedF64),
    Bool(bool),
    Symbol(SymbolId),
    Text(TextId),
    Ref(NodeId),
    Timestamp(u64),
    String(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrderedF64(pub f64);

impl PartialEq for OrderedF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}
impl Eq for OrderedF64 {}

impl std::hash::Hash for OrderedF64 {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

impl std::fmt::Display for OrderedF64 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::Symbol(_) => "Symbol",
            Value::Text(_) => "Text",
            Value::Ref(_) => "Ref",
            Value::Timestamp(_) => "Timestamp",
            Value::String(_) => "String",
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::Symbol(v) => write!(f, "sym:{}", v),
            Value::Text(v) => write!(f, "txt:{}", v),
            Value::Ref(v) => write!(f, "ref:{}", v),
            Value::Timestamp(v) => write!(f, "ts:{}", v),
            Value::String(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalInfo {
    pub created_at: u64,
    pub updated_at: u64,
    pub access_count: u32,
    pub last_access: u64,
    pub decay_score: f32,
}

impl Default for TemporalInfo {
    fn default() -> Self {
        let now = now_millis();
        TemporalInfo {
            created_at: now,
            updated_at: now,
            access_count: 0,
            last_access: now,
            decay_score: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNode {
    pub id: NodeId,
    pub vector: Vec<f32>,
    pub properties: Vec<Property>,
    pub temporal: TemporalInfo,
    pub memory_type: MemoryType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    Episodic,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMeta {
    pub segment_id: String,
    pub node_count: usize,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Metric {
    Euclidean,
    Cosine,
    DotProduct,
}

impl Metric {
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "eu" | "euclidean" => Some(Metric::Euclidean),
            "cs" | "cosine" => Some(Metric::Cosine),
            "dot" | "dot_product" => Some(Metric::DotProduct),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub node_id: NodeId,
    pub score: f32,
    pub distance: f64,
    pub vector: Vec<f32>,
    pub properties: Vec<Property>,
    pub temporal: TemporalInfo,
    pub memory_type: MemoryType,
    pub expanded_from: Option<NodeId>,
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

pub fn generate_node_id() -> NodeId {
    let uuid = Uuid::new_v4();
    let bytes = uuid.as_bytes();
    u64::from_be_bytes(bytes[..8].try_into().unwrap())
}
