use serde::{Deserialize, Serialize};
use skymemory_core::{MemoryNode, MemoryType, NodeId, TemporalInfo};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayConfig {
    pub half_life_millis: u64,
    pub access_boost: f32,
    pub min_score: f32,
}

impl Default for DecayConfig {
    fn default() -> Self {
        DecayConfig {
            half_life_millis: 24 * 60 * 60 * 1000,
            access_boost: 0.1,
            min_score: 0.01,
        }
    }
}

pub struct TemporalStore {
    records: HashMap<NodeId, TemporalInfo>,
    config: DecayConfig,
}

impl TemporalStore {
    pub fn new(config: DecayConfig) -> Self {
        TemporalStore {
            records: HashMap::new(),
            config,
        }
    }

    pub fn insert(&mut self, id: NodeId, temporal: TemporalInfo) {
        self.records.insert(id, temporal);
    }

    pub fn remove(&mut self, id: NodeId) {
        self.records.remove(&id);
    }

    pub fn get(&self, id: NodeId) -> Option<&TemporalInfo> {
        self.records.get(&id)
    }

    pub fn record_access(&mut self, id: NodeId) {
        if let Some(t) = self.records.get_mut(&id) {
            let now = skymemory_core::now_millis();
            t.last_access = now;
            t.access_count += 1;
            t.updated_at = now;
            t.decay_score = (t.decay_score + self.config.access_boost).min(1.0);
        }
    }

    pub fn compute_decay(&self, temporal: &TemporalInfo) -> f32 {
        let now = skymemory_core::now_millis();
        let elapsed = now.saturating_sub(temporal.last_access) as f64;
        let half_life = self.config.half_life_millis as f64;
        let decay_factor = 0.5f64.powf(elapsed / half_life);
        let base = temporal.decay_score as f64;
        let result = base * decay_factor;
        result.max(self.config.min_score as f64) as f32
    }

    pub fn recency_boost(&self, temporal: &TemporalInfo) -> f32 {
        let now = skymemory_core::now_millis();
        let recency = now.saturating_sub(temporal.last_access) as f32;
        let max_recency: f32 = (7 * 24 * 60 * 60 * 1000) as f32;
        1.0 - (recency / max_recency).min(1.0)
    }

    pub fn hot_cold_separation(&self, threshold: f32) -> (Vec<NodeId>, Vec<NodeId>) {
        let mut hot = Vec::new();
        let mut cold = Vec::new();
        for (id, temporal) in &self.records {
            let score = self.compute_decay(temporal);
            if score >= threshold {
                hot.push(*id);
            } else {
                cold.push(*id);
            }
        }
        (hot, cold)
    }

    pub fn forgetting_curve_score(&self, id: NodeId, now: u64) -> f32 {
        let temporal = match self.records.get(&id) {
            Some(t) => t,
            None => return 0.0,
        };
        let elapsed = now.saturating_sub(temporal.created_at) as f64;
        let half_life = self.config.half_life_millis as f64;
        let base_decay = 0.5f64.powf(elapsed / half_life);
        let access_factor = 1.0 + (temporal.access_count as f64 * 0.05).min(0.5);
        let recency_factor = self.recency_boost(temporal) as f64;
        (base_decay * access_factor * recency_factor).min(1.0) as f32
    }

    pub fn consolidate_memories(
        &self,
        nodes: &[MemoryNode],
    ) -> Vec<(NodeId, f32, MemoryType)> {
        let mut scored: Vec<(NodeId, f32, MemoryType)> = nodes
            .iter()
            .map(|n| {
                let decay = self.compute_decay(&n.temporal);
                (n.id, decay, n.memory_type)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored
    }

    pub fn update_all_decay(&mut self) {
        let now = skymemory_core::now_millis();
        let half_life = self.config.half_life_millis as f64;
        let min_score = self.config.min_score as f64;
        for temporal in self.records.values_mut() {
            let elapsed = now.saturating_sub(temporal.last_access) as f64;
            let decay_factor = 0.5f64.powf(elapsed / half_life);
            let result = temporal.decay_score as f64 * decay_factor;
            temporal.decay_score = result.max(min_score) as f32;
        }
    }

    pub fn record_count(&self) -> usize {
        self.records.len()
    }
}

impl Default for TemporalStore {
    fn default() -> Self {
        Self::new(DecayConfig::default())
    }
}
