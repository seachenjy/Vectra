use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use skymemory_core::{Metric, NodeId, SkyError, SkyResult};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

use crate::distance::distance;

const MAX_LEVEL: usize = 16;
const M: usize = 16;
const EF_CONSTRUCTION: usize = 200;
const M_MAX0: usize = M * 2;

#[derive(Clone, Serialize, Deserialize)]
struct HnswNode {
    id: NodeId,
    vector: Vec<f32>,
    level: usize,
    neighbors: Vec<Vec<NodeId>>,
}

struct Scored {
    id: NodeId,
    dist: f64,
}

impl PartialEq for Scored {
    fn eq(&self, other: &Self) -> bool {
        self.dist == other.dist
    }
}
impl Eq for Scored {}

impl PartialOrd for Scored {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.dist.partial_cmp(&self.dist)
    }
}

impl Ord for Scored {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

#[derive(Serialize, Deserialize)]
pub struct HnswIndex {
    nodes: HashMap<NodeId, HnswNode>,
    entry_point: Option<NodeId>,
    max_level: usize,
    dimension: usize,
}

impl HnswIndex {
    pub fn new(dimension: usize) -> Self {
        HnswIndex {
            nodes: HashMap::new(),
            entry_point: None,
            max_level: 0,
            dimension,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }

    pub fn insert(&mut self, id: NodeId, vector: Vec<f32>) -> SkyResult<()> {
        if vector.len() != self.dimension {
            return Err(SkyError::DimensionMismatch {
                expected: self.dimension,
                got: vector.len(),
            });
        }

        let level = self.random_level();

        if let Some(ep) = self.entry_point {
            let mut curr = ep;
            let mut curr_dist = distance(&self.nodes[&ep].vector, &vector, &Metric::Cosine);

            for l in (level + 1..=self.max_level).rev() {
                let nearest = self.search_layer(curr, curr_dist, &vector, 1, l);
                if let Some(best) = nearest.first() {
                    curr = best.0;
                    curr_dist = best.1;
                }
            }

            let mut neighbors = vec![Vec::new(); level + 1];
            for l in 0..=level.min(self.max_level) {
                let ef = if l == 0 { M_MAX0 } else { EF_CONSTRUCTION };
                let candidates = self.search_layer(curr, curr_dist, &vector, ef, l);
                let selected = self.select_neighbors_heuristic(&candidates, M, l);
                neighbors[l] = selected.iter().map(|(id, _)| *id).collect();
                for (nid, _) in &selected {
                    if let Some(neighbor) = self.nodes.get_mut(nid) {
                        while neighbor.neighbors.len() <= l {
                            neighbor.neighbors.push(Vec::new());
                        }
                        neighbor.neighbors[l].push(id);
                        let max_n = if l == 0 { M_MAX0 } else { M };
                        if neighbor.neighbors[l].len() > max_n {
                            neighbor.neighbors[l].truncate(max_n);
                        }
                    }
                }
                if let Some(cand) = candidates.first() {
                    curr = cand.0;
                    curr_dist = cand.1;
                }
            }

            if level > self.max_level {
                self.max_level = level;
                self.entry_point = Some(id);
            }

            self.nodes.insert(id, HnswNode { id, vector, level, neighbors });
        } else {
            let neighbors = vec![Vec::new(); level + 1];
            self.nodes.insert(id, HnswNode { id, vector, level, neighbors });
            self.entry_point = Some(id);
            self.max_level = level;
        }

        Ok(())
    }

    pub fn remove(&mut self, id: NodeId) -> bool {
        if self.nodes.remove(&id).is_some() {
            for node in self.nodes.values_mut() {
                for layer in &mut node.neighbors {
                    layer.retain(|nid| *nid != id);
                }
            }
            if self.entry_point == Some(id) {
                self.entry_point = self.nodes.keys().next().copied();
            }
            return true;
        }
        false
    }

    pub fn search(
        &self,
        query: &[f32],
        k: usize,
        ef_search: usize,
        metric: &Metric,
    ) -> SkyResult<Vec<(NodeId, f64)>> {
        if query.len() != self.dimension {
            return Err(SkyError::DimensionMismatch {
                expected: self.dimension,
                got: query.len(),
            });
        }

        let ep = match self.entry_point {
            Some(ep) => ep,
            None => return Ok(Vec::new()),
        };

        let mut curr = ep;
        let mut curr_dist = distance(&self.nodes[&ep].vector, query, metric);

        for l in (1..=self.max_level).rev() {
            let nearest = self.search_layer(curr, curr_dist, query, 1, l);
            if let Some(best) = nearest.first() {
                curr = best.0;
                curr_dist = best.1;
            }
        }

        let candidates = self.search_layer(curr, curr_dist, query, ef_search.max(k), 0);
        Ok(candidates.into_iter().take(k).collect())
    }

    fn search_layer(
        &self,
        entry: NodeId,
        entry_dist: f64,
        query: &[f32],
        ef: usize,
        level: usize,
    ) -> Vec<(NodeId, f64)> {
        let mut visited = HashSet::new();
        visited.insert(entry);

        let mut candidates = BinaryHeap::new();
        candidates.push(Scored { id: entry, dist: entry_dist });

        let mut results = vec![(entry, entry_dist)];

        while let Some(curr) = candidates.pop() {
            if curr.dist > results.last().map(|r| r.1).unwrap_or(f64::MAX) && results.len() >= ef {
                break;
            }

            if let Some(node) = self.nodes.get(&curr.id) {
                if level < node.neighbors.len() {
                    for &neighbor_id in &node.neighbors[level] {
                        if visited.contains(&neighbor_id) {
                            continue;
                        }
                        visited.insert(neighbor_id);

                        if let Some(neighbor) = self.nodes.get(&neighbor_id) {
                            let d = distance(&neighbor.vector, query, &Metric::Cosine);
                            if results.len() < ef || d < results.last().map(|r| r.1).unwrap_or(f64::MAX) {
                                candidates.push(Scored { id: neighbor_id, dist: d });
                                results.push((neighbor_id, d));
                                results.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
                                if results.len() > ef {
                                    results.truncate(ef);
                                }
                            }
                        }
                    }
                }
            }
        }

        results
    }

    fn select_neighbors_heuristic(
        &self,
        candidates: &[(NodeId, f64)],
        m: usize,
        _level: usize,
    ) -> Vec<(NodeId, f64)> {
        let mut selected = Vec::new();
        for &(id, dist) in candidates {
            if selected.len() >= m {
                break;
            }
            let mut good = true;
            for &(sel_id, sel_dist) in &selected {
                if sel_dist < dist {
                    if let (Some(node_a), Some(node_b)) = (self.nodes.get(&sel_id), self.nodes.get(&id)) {
                        let d = distance(&node_a.vector, &node_b.vector, &Metric::Cosine);
                        if d < dist {
                            good = false;
                            break;
                        }
                    }
                }
            }
            if good {
                selected.push((id, dist));
            }
        }
        selected
    }

    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let mut level = 0;
        while rng.gen::<f64>() < 0.5 && level < MAX_LEVEL - 1 {
            level += 1;
        }
        level
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.keys().copied().collect()
    }
}

pub struct VectorStore {
    pub index: RwLock<HnswIndex>,
    pub dimension: usize,
}

impl VectorStore {
    pub fn new(dimension: usize) -> Self {
        VectorStore {
            index: RwLock::new(HnswIndex::new(dimension)),
            dimension,
        }
    }

    pub fn insert(&self, id: NodeId, vector: Vec<f32>) -> SkyResult<()> {
        self.index.write().insert(id, vector)
    }

    pub fn remove(&self, id: NodeId) -> bool {
        self.index.write().remove(id)
    }

    pub fn search(&self, query: &[f32], k: usize, metric: &Metric) -> SkyResult<Vec<(NodeId, f64)>> {
        self.index.read().search(query, k, 200, metric)
    }

    pub fn len(&self) -> usize {
        self.index.read().len()
    }
}
