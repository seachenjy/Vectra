use serde::{Deserialize, Serialize};
use skymemory_core::{NodeId, Property, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnIndex {
    pub column_name: String,
    pub values: HashMap<Value, Vec<NodeId>>,
}

impl ColumnIndex {
    pub fn new(name: String) -> Self {
        ColumnIndex {
            column_name: name,
            values: HashMap::new(),
        }
    }

    pub fn insert(&mut self, value: Value, node_id: NodeId) {
        self.values.entry(value).or_default().push(node_id);
    }

    pub fn remove(&mut self, node_id: NodeId) {
        for ids in self.values.values_mut() {
            ids.retain(|id| *id != node_id);
        }
        self.values.retain(|_, v| !v.is_empty());
    }

    pub fn lookup(&self, value: &Value) -> Option<&Vec<NodeId>> {
        self.values.get(value)
    }

    pub fn scan_gt(&self, threshold: f64) -> Vec<NodeId> {
        let mut result = Vec::new();
        for (val, ids) in &self.values {
            if let Value::Float(f) = val {
                if f.0 > threshold {
                    result.extend(ids);
                }
            }
            if let Value::Int(i) = val {
                if *i as f64 > threshold {
                    result.extend(ids);
                }
            }
        }
        result
    }
}

pub struct PropertyStore {
    columns: HashMap<String, ColumnIndex>,
    node_properties: HashMap<NodeId, Vec<Property>>,
}

impl PropertyStore {
    pub fn new() -> Self {
        PropertyStore {
            columns: HashMap::new(),
            node_properties: HashMap::new(),
        }
    }

    pub fn insert(&mut self, node_id: NodeId, properties: Vec<Property>) {
        for prop in &properties {
            let col = self.columns
                .entry(prop.key.clone())
                .or_insert_with(|| ColumnIndex::new(prop.key.clone()));
            col.insert(prop.value.clone(), node_id);
        }
        self.node_properties.insert(node_id, properties);
    }

    pub fn remove(&mut self, node_id: NodeId) {
        if let Some(_props) = self.node_properties.remove(&node_id) {
            for col in self.columns.values_mut() {
                col.remove(node_id);
            }
        }
    }

    pub fn get(&self, node_id: NodeId) -> Option<&Vec<Property>> {
        self.node_properties.get(&node_id)
    }

    pub fn filter(&self, key: &str, value: &Value) -> HashSet<NodeId> {
        self.columns
            .get(key)
            .and_then(|col| col.lookup(value))
            .map(|ids| ids.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn filter_range(&self, key: &str, min: f64, max: f64) -> HashSet<NodeId> {
        let mut result = HashSet::new();
        if let Some(col) = self.columns.get(key) {
            for (val, ids) in &col.values {
                let numeric = match val {
                    Value::Float(f) => Some(f.0),
                    Value::Int(i) => Some(*i as f64),
                    _ => None,
                };
                if let Some(n) = numeric {
                    if n >= min && n <= max {
                        result.extend(ids);
                    }
                }
            }
        }
        result
    }

    pub fn schema(&self) -> HashMap<String, Vec<String>> {
        let mut schema = HashMap::new();
        for (name, col) in &self.columns {
            let types: Vec<String> = col.values.keys().map(|v| v.type_name().to_string()).collect();
            let mut types: Vec<String> = types.into_iter().collect::<HashSet<_>>().into_iter().collect();
            types.sort();
            schema.insert(name.clone(), types);
        }
        schema
    }

    pub fn node_count(&self) -> usize {
        self.node_properties.len()
    }
}

impl Default for PropertyStore {
    fn default() -> Self {
        Self::new()
    }
}
