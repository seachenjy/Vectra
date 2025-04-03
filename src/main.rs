mod ver;
use ver::{Vector, MetadataEntry, MetadataValue};
use chrono::Utc;
fn main() -> Result<(), Box<dyn std::error::Error>>{
    // 创建一个示例 Vector
    let data = vec![1.0, 2.0, 3.0];  // 例子中是 f64 向量
    let metadata = vec![
        MetadataEntry::new("created_at".to_string(), MetadataValue::DateTime(Utc::now())),
        MetadataEntry::new("source".to_string(), MetadataValue::String("sensor_x".to_string())),
    ];

    let vector = Vector::new(data, metadata);
    vector.print();
    

    // 保存到文件
    let file_path = "vector_data.bin";
    vector.save_to_file(file_path)?;

    // 从文件加载
    let loaded_vector: Vector<f64> = Vector::load_from_file(file_path)?;
    println!("Loaded Vector:");
    loaded_vector.print();

    Ok(())
}