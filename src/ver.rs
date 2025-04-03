use std::fs::File;
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

impl MetadataValue {
    pub fn size(&self) -> usize {
        match self {
            MetadataValue::Integer(_) => std::mem::size_of::<i32>(),
            MetadataValue::Float(_) => std::mem::size_of::<f32>(),
            MetadataValue::String(s) => s.len(),
            MetadataValue::Bool(_) => std::mem::size_of::<bool>(),
            MetadataValue::DateTime(_) => std::mem::size_of::<DateTime<Utc>>(),
        }
    }
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

    pub fn size(&self) -> usize {
        self.value.size() + self.key.len()
    }
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
    pub fn print(&self) {
        println!("Vector data: {:?}", self.data);
        println!("Metadata: {:?}", self.metadata);
    }

    // 获取元数据的大小
    pub fn metadata_size(&self) -> usize {
        self.metadata.iter().map(|entry| entry.size()).sum()
    }

    // **持久化：将 Vector<T> 保存到文件**
    pub fn save_to_file(&self, path: &str) -> io::Result<()> {
        let encoded = bincode::serialize(self).unwrap();
        let mut file = File::create(path)?;
        file.write_all(&encoded)?;
        Ok(())
    }

    // **从文件加载 Vector<T>**
    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let decoded: Vector<T> = bincode::deserialize(&buffer).unwrap();
        Ok(decoded)
    }
}