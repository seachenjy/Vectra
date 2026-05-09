use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use skymemory_core::{Edge, EdgeType, NodeId, SkyResult};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CsrGraph {
    offsets: Vec<u32>,
    edges: Vec<CompressedEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CompressedEdge {
    target: NodeId,
    edge_type: EdgeType,
    weight: f32,
}

impl CsrGraph {
    fn new() -> Self {
        CsrGraph {
            offsets: vec![0],
            edges: Vec::new(),
        }
    }

    fn neighbors(&self, node_idx: usize) -> &[CompressedEdge] {
        if node_idx + 1 >= self.offsets.len() {
            return &[];
        }
        let start = self.offsets[node_idx] as usize;
        let end = self.offsets[node_idx + 1] as usize;
        &self.edges[start..end]
    }
}

pub struct GraphStore {
    node_id_to_idx: HashMap<NodeId, usize>,
    idx_to_node_id: Vec<NodeId>,
    graph: RwLock<CsrGraph>,
    reverse_graph: RwLock<CsrGraph>,
}

impl GraphStore {
    pub fn new() -> Self {
        GraphStore {
            node_id_to_idx: HashMap::new(),
            idx_to_node_id: Vec::new(),
            graph: RwLock::new(CsrGraph::new()),
            reverse_graph: RwLock::new(CsrGraph::new()),
        }
    }

    fn ensure_node(&mut self, id: NodeId) -> usize {
        if let Some(&idx) = self.node_id_to_idx.get(&id) {
            return idx;
        }
        let idx = self.idx_to_node_id.len();
        self.node_id_to_idx.insert(id, idx);
        self.idx_to_node_id.push(id);
        let mut g = self.graph.write();
        let last = g.offsets.last().copied().unwrap_or(0);
        g.offsets.push(last);
        let mut rg = self.reverse_graph.write();
        let last = rg.offsets.last().copied().unwrap_or(0);
        rg.offsets.push(last);
        idx
    }

    pub fn add_edge(&mut self, edge: Edge) -> SkyResult<()> {
        let from_idx = self.ensure_node(edge.from);
        let to_idx = self.ensure_node(edge.to);
        {
            let mut g = self.graph.write();
            let insert_pos = g.offsets[from_idx + 1] as usize;
            g.edges.insert(insert_pos, CompressedEdge {
                target: edge.to,
                edge_type: edge.edge_type,
                weight: edge.weight,
            });
            for i in (from_idx + 1)..g.offsets.len() {
                g.offsets[i] += 1;
            }
        }
        {
            let mut rg = self.reverse_graph.write();
            let insert_pos = rg.offsets[to_idx + 1] as usize;
            rg.edges.insert(insert_pos, CompressedEdge {
                target: edge.from,
                edge_type: edge.edge_type,
                weight: edge.weight,
            });
            for i in (to_idx + 1)..rg.offsets.len() {
                rg.offsets[i] += 1;
            }
        }
        Ok(())
    }

    pub fn get_neighbors(&self, node_id: NodeId, edge_type: Option<EdgeType>) -> Vec<(NodeId, EdgeType, f32)> {
        let idx = match self.node_id_to_idx.get(&node_id) {
            Some(&i) => i,
            None => return Vec::new(),
        };
        let g = self.graph.read();
        g.neighbors(idx)
            .iter()
            .filter(|e| edge_type.map_or(true, |et| e.edge_type == et))
            .map(|e| (e.target, e.edge_type, e.weight))
            .collect()
    }

    pub fn traverse(
        &self,
        start: NodeId,
        max_depth: usize,
        edge_filter: Option<EdgeType>,
    ) -> Vec<(NodeId, usize, f32)> {
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((start, 0usize, 1.0f32));
        visited.insert(start);

        while let Some((node, depth, weight)) = queue.pop_front() {
            result.push((node, depth, weight));
            if depth >= max_depth {
                continue;
            }
            for (neighbor, _et, w) in self.get_neighbors(node, edge_filter) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    queue.push_back((neighbor, depth + 1, weight * w));
                }
            }
        }
        result
    }

    pub fn spreading_activation(
        &self,
        start: NodeId,
        decay_factor: f32,
        threshold: f32,
        max_hops: usize,
    ) -> Vec<(NodeId, f32)> {
        let mut activation: HashMap<NodeId, f32> = HashMap::new();
        activation.insert(start, 1.0);
        let mut frontier = vec![(start, 1.0f32)];

        for _ in 0..max_hops {
            let mut next_frontier = Vec::new();
            for (node, act) in &frontier {
                for (neighbor, _et, w) in self.get_neighbors(*node, None) {
                    let new_act = act * w * decay_factor;
                    if new_act >= threshold {
                        let entry = activation.entry(neighbor).or_insert(0.0);
                        if new_act > *entry {
                            *entry = new_act;
                            next_frontier.push((neighbor, new_act));
                        }
                    }
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }

        let mut result: Vec<(NodeId, f32)> = activation.into_iter().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

    pub fn detect_contradictions(&self, node_id: NodeId) -> Vec<NodeId> {
        self.get_neighbors(node_id, Some(EdgeType::Contradicts))
            .into_iter()
            .map(|(id, _, _)| id)
            .collect()
    }

    pub fn strengthen_edge(&mut self, from: NodeId, to: NodeId, reinforcement: f32) {
        let from_idx = match self.node_id_to_idx.get(&from) {
            Some(&i) => i,
            None => return,
        };
        let mut g = self.graph.write();
        let start = g.offsets[from_idx] as usize;
        let end = g.offsets[from_idx + 1] as usize;
        for edge in &mut g.edges[start..end] {
            if edge.target == to {
                edge.weight = (edge.weight + reinforcement).min(1.0);
            }
        }
    }

    pub fn node_count(&self) -> usize {
        self.idx_to_node_id.len()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.read().edges.len()
    }

    pub fn all_edges(&self) -> Vec<Edge> {
        let g = self.graph.read();
        let mut edges = Vec::new();
        for (from_idx, &from_id) in self.idx_to_node_id.iter().enumerate() {
            for e in g.neighbors(from_idx) {
                edges.push(Edge {
                    from: from_id,
                    to: e.target,
                    edge_type: e.edge_type,
                    weight: e.weight,
                });
            }
        }
        edges
    }
}

impl Default for GraphStore {
    fn default() -> Self {
        Self::new()
    }
}
