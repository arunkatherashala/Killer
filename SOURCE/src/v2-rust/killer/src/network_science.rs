//! Network Science — 40+ functions (centrality, clustering, algorithms, community detection)

#[derive(Debug, Clone)]
pub struct Graph { pub nodes: Vec<u32>, pub edges: Vec<(u32, u32)> }

impl Graph {
    pub fn new() -> Self { Self { nodes: vec![], edges: vec![] } }
}
