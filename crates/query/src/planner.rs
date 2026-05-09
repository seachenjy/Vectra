use parking_lot::RwLock;
use skymemory_core::{Metric, MemoryNode, NodeId, QueryResult, SkyResult, Value};
use skymemory_graph::GraphStore;
use skymemory_property::PropertyStore;
use skymemory_temporal::TemporalStore;
use skymemory_vector::VectorStore;

use crate::parser::{FilterOp, QueryNode};

pub struct QueryEngine {
    pub vector_store: VectorStore,
    pub property_store: RwLock<PropertyStore>,
    pub graph_store: RwLock<GraphStore>,
    pub temporal_store: RwLock<TemporalStore>,
    pub node_data: RwLock<std::collections::HashMap<NodeId, MemoryNode>>,
}

impl QueryEngine {
    pub fn new(dimension: usize) -> Self {
        QueryEngine {
            vector_store: VectorStore::new(dimension),
            property_store: RwLock::new(PropertyStore::new()),
            graph_store: RwLock::new(GraphStore::new()),
            temporal_store: RwLock::new(TemporalStore::default()),
            node_data: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn insert_node(&self, node: MemoryNode) -> SkyResult<()> {
        self.vector_store.insert(node.id, node.vector.clone())?;
        self.property_store.write().insert(node.id, node.properties.clone());
        self.temporal_store.write().insert(node.id, node.temporal.clone());
        self.node_data.write().insert(node.id, node);
        Ok(())
    }

    pub fn remove_node(&self, id: NodeId) {
        self.vector_store.remove(id);
        self.property_store.write().remove(id);
        self.temporal_store.write().remove(id);
        self.node_data.write().remove(&id);
    }

    pub fn get_node(&self, id: NodeId) -> Option<MemoryNode> {
        self.node_data.read().get(&id).cloned()
    }

    pub fn node_count(&self) -> usize {
        self.node_data.read().len()
    }

    pub fn execute(&self, query: &QueryNode, k: usize, metric: &Metric) -> SkyResult<Vec<QueryResult>> {
        match query {
            QueryNode::VectorSimilar { terms } => {
                self.execute_vector_similar(terms, k, metric)
            }
            QueryNode::PropertyFilter { key, op, value } => {
                self.execute_property_filter(key, op, value)
            }
            QueryNode::BoostRecent { weight } => {
                self.execute_boost_recent(*weight, k)
            }
            QueryNode::Expand { depth } => {
                self.execute_expand(*depth, k)
            }
            QueryNode::And(left, right) => {
                let left_results = self.execute(left, k * 2, metric)?;
                let right_results = self.execute(right, k * 2, metric)?;
                let right_ids: std::collections::HashSet<NodeId> =
                    right_results.iter().map(|r| r.node_id).collect();
                let mut combined: Vec<QueryResult> = left_results
                    .into_iter()
                    .filter(|r| right_ids.contains(&r.node_id))
                    .collect();
                combined.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                combined.truncate(k);
                Ok(combined)
            }
            QueryNode::Or(left, right) => {
                let mut left_results = self.execute(left, k, metric)?;
                let right_results = self.execute(right, k, metric)?;
                let left_ids: std::collections::HashSet<NodeId> =
                    left_results.iter().map(|r| r.node_id).collect();
                for r in right_results {
                    if !left_ids.contains(&r.node_id) {
                        left_results.push(r);
                    }
                }
                left_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                left_results.truncate(k);
                Ok(left_results)
            }
        }
    }

    fn execute_vector_similar(&self, _terms: &str, k: usize, metric: &Metric) -> SkyResult<Vec<QueryResult>> {
        let node_data = self.node_data.read();
        let nodes: Vec<&MemoryNode> = node_data.values().collect();
        if nodes.is_empty() {
            return Ok(Vec::new());
        }
        let dim = self.vector_store.dimension;
        let query_vec = vec![0.0f32; dim];
        let raw = self.vector_store.search(&query_vec, k * 2, metric)?;
        let mut results = Vec::new();
        for (node_id, dist) in raw {
            if let Some(node) = node_data.get(&node_id) {
                let temporal_score = self.temporal_store.read().forgetting_curve_score(node_id, skymemory_core::now_millis());
                let score = (1.0 / (1.0 + dist as f32)) * temporal_score;
                results.push(QueryResult {
                    node_id,
                    score,
                    distance: dist,
                    vector: node.vector.clone(),
                    properties: node.properties.clone(),
                    temporal: node.temporal.clone(),
                    memory_type: node.memory_type,
                    expanded_from: None,
                });
            }
        }
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        Ok(results)
    }

    fn execute_property_filter(&self, key: &str, op: &FilterOp, value: &str) -> SkyResult<Vec<QueryResult>> {
        let node_data = self.node_data.read();
        let prop_store = self.property_store.read();
        let matched_ids = match op {
            FilterOp::Eq => {
                let val = parse_filter_value(value);
                prop_store.filter(key, &val)
            }
            FilterOp::Gt | FilterOp::Gte | FilterOp::Lt | FilterOp::Lte => {
                if let Ok(threshold) = value.parse::<f64>() {
                    match op {
                        FilterOp::Gt | FilterOp::Gte => prop_store.filter_range(key, threshold, f64::MAX),
                        FilterOp::Lt | FilterOp::Lte => prop_store.filter_range(key, f64::MIN, threshold),
                        _ => std::collections::HashSet::new(),
                    }
                } else {
                    std::collections::HashSet::new()
                }
            }
            FilterOp::Neq => {
                let val = parse_filter_value(value);
                let matched = prop_store.filter(key, &val);
                node_data.keys().filter(|id| !matched.contains(id)).copied().collect()
            }
        };
        let mut results = Vec::new();
        for id in matched_ids {
            if let Some(node) = node_data.get(&id) {
                results.push(QueryResult {
                    node_id: id,
                    score: 1.0,
                    distance: 0.0,
                    vector: node.vector.clone(),
                    properties: node.properties.clone(),
                    temporal: node.temporal.clone(),
                    memory_type: node.memory_type,
                    expanded_from: None,
                });
            }
        }
        Ok(results)
    }

    fn execute_boost_recent(&self, weight: f32, k: usize) -> SkyResult<Vec<QueryResult>> {
        let node_data = self.node_data.read();
        let temporal = self.temporal_store.read();
        let mut results: Vec<QueryResult> = node_data
            .values()
            .map(|node| {
                let boost = temporal.recency_boost(&node.temporal) * weight;
                QueryResult {
                    node_id: node.id,
                    score: boost,
                    distance: 0.0,
                    vector: node.vector.clone(),
                    properties: node.properties.clone(),
                    temporal: node.temporal.clone(),
                    memory_type: node.memory_type,
                    expanded_from: None,
                }
            })
            .collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        Ok(results)
    }

    fn execute_expand(&self, depth: usize, k: usize) -> SkyResult<Vec<QueryResult>> {
        let node_data = self.node_data.read();
        let graph = self.graph_store.read();
        let temporal = self.temporal_store.read();
        let mut results = Vec::new();
        let start_nodes: Vec<NodeId> = node_data.keys().copied().take(5).collect();
        for start in start_nodes {
            let traversed = graph.traverse(start, depth, None);
            for (node_id, hop_dist, weight) in traversed {
                if let Some(node) = node_data.get(&node_id) {
                    let decay = temporal.compute_decay(&node.temporal);
                    let score = weight * decay / (1.0 + hop_dist as f32);
                    results.push(QueryResult {
                        node_id,
                        score,
                        distance: hop_dist as f64,
                        vector: node.vector.clone(),
                        properties: node.properties.clone(),
                        temporal: node.temporal.clone(),
                        memory_type: node.memory_type,
                        expanded_from: Some(start),
                    });
                }
            }
        }
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(k);
        Ok(results)
    }
}

fn parse_filter_value(s: &str) -> Value {
    if s.eq_ignore_ascii_case("true") || s == "1" {
        return Value::Bool(true);
    }
    if s.eq_ignore_ascii_case("false") || s == "0" {
        return Value::Bool(false);
    }
    if let Ok(i) = s.parse::<i64>() {
        return Value::Int(i);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Value::Float(skymemory_core::OrderedF64(f));
    }
    Value::String(s.to_string())
}
