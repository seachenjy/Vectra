use thiserror::Error;

#[derive(Error, Debug)]
pub enum SkyError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialize(String),

    #[error("deserialization error: {0}")]
    Deserialize(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("index out of bounds: {index} >= {len}")]
    OutOfBounds { index: usize, len: usize },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("wal error: {0}")]
    Wal(String),

    #[error("segment error: {0}")]
    Segment(String),

    #[error("graph error: {0}")]
    Graph(String),

    #[error("query parse error: {0}")]
    QueryParse(String),
}

pub type SkyResult<T> = Result<T, SkyError>;
