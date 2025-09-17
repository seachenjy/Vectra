use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::fmt::Debug;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use bincode;

// 定义一个枚举，表示元数据的不同类型
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum MetadataValue {
    Integer(i32),
    Float(f32),
    String(String),
    Bool(bool),
    DateTime(DateTime<Utc>),
}

// 定义一个结构体，用于保存元数据的 key 和 value
#[derive(Debug, Serialize, Deserialize)]
pub struct MetadataEntry {
    key: String,
    value: MetadataValue,
}

impl MetadataEntry {
    pub fn new(key: String, value: MetadataValue) -> Self {
        MetadataEntry { key, value }
    }

    pub fn key(&self) -> &str { &self.key }
    pub fn value(&self) -> &MetadataValue { &self.value }
}

// 定义一个向量结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct Vector<T> {
    data: Vec<T>,                          // 向量数据
    metadata: Vec<MetadataEntry>,           // 元数据，包含多个 key-value 对
}

impl<T> Vector<T> 
where T: Debug + Serialize + for<'de> Deserialize<'de> {
    // 构造函数
    pub fn new(data: Vec<T>, metadata: Vec<MetadataEntry>) -> Self {
        Vector { data, metadata }
    }

    // 打印 Vector 数据和元数据
    #[allow(dead_code)]
    pub fn print(&self) {
        println!("Vector data: {:?}", self.data);
        println!("Metadata: {:?}", self.metadata);
    }

    pub fn data(&self) -> &[T] {
        &self.data
    }

    pub fn metadata(&self) -> &[MetadataEntry] {
        &self.metadata
    }

    // **持久化：将 Vector<T> 保存到文件**
    #[allow(dead_code)]
    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let encoded = bincode::serialize(self).unwrap();
        let mut file = File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    // **从文件加载 Vector<T>**
    #[allow(dead_code)]
    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let decoded: Vector<T> = bincode::deserialize(&buffer).unwrap();
        Ok(decoded)
    }
}

impl std::fmt::Display for MetadataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetadataValue::Integer(v) => write!(f, "{}", v),
            MetadataValue::Float(v) => write!(f, "{}", v),
            MetadataValue::String(v) => write!(f, "{}", v),
            MetadataValue::Bool(v) => write!(f, "{}", v),
            MetadataValue::DateTime(dt) => write!(f, "{}", dt.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Database {
    pub name: String,
    pub dimension: usize,
    pub vectors: Vec<Vector<f64>>,
}

impl Database {
    pub fn new(name: String, dimension: usize) -> Self {
        Database { name, dimension, vectors: Vec::new() }
    }

    pub fn insert(&mut self, vector: Vector<f64>) -> io::Result<()> {
        if vector_len(&vector) != self.dimension { return Err(io::Error::new(io::ErrorKind::InvalidInput, "dimension mismatch")); }
        self.vectors.push(vector);
        Ok(())
    }

    pub fn save_to_dir(&self, dir: &str) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let path = format!("{}/{}.bin", dir, self.name);
        let encoded = bincode::serialize(self).unwrap();
        let mut file = File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    pub fn load_from_dir(dir: &str, name: &str) -> io::Result<Self> {
        let path = format!("{}/{}.bin", dir, name);
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let decoded: Database = bincode::deserialize(&buffer).unwrap();
        Ok(decoded)
    }
}

fn vector_len(v: &Vector<f64>) -> usize { v.data.len() }

pub enum Metric {
    Euclidean,
    L1,
    Cosine,
}

impl Metric {
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "eu" => Some(Metric::Euclidean),
            "l1" => Some(Metric::L1),
            "cs" => Some(Metric::Cosine),
            _ => None,
        }
    }
}

pub fn distance(a: &[f64], b: &[f64], metric: &Metric) -> f64 {
    match metric {
        Metric::Euclidean => {
            a.iter().zip(b.iter()).map(|(x,y)| (x-y)*(x-y)).sum::<f64>().sqrt()
        }
        Metric::L1 => {
            a.iter().zip(b.iter()).map(|(x,y)| (x-y).abs()).sum::<f64>()
        }
        Metric::Cosine => {
            let dot = a.iter().zip(b.iter()).map(|(x,y)| x*y).sum::<f64>();
            let na = a.iter().map(|x| x*x).sum::<f64>().sqrt();
            let nb = b.iter().map(|x| x*x).sum::<f64>().sqrt();
            1.0 - (dot / (na * nb + f64::EPSILON))
        }
    }
}