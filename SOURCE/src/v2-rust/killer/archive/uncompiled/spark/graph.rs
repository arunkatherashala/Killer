/// GraphX - Graph Processing for Killer Spark
/// 
/// Scalable graph analytics: PageRank, connected components, triangle counting

use std::collections::{HashMap, HashSet};

/// Vertex with ID and attributes
#[derive(Clone, Debug)]
pub struct Vertex {
    id: u64,
    value: f64,
}

impl Vertex {
    pub fn new(id: u64, value: f64) -> Self {
        Self { id, value }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

/// Edge between two vertices with weight
#[derive(Clone, Debug)]
pub struct Edge {
    src: u64,
    dst: u64,
    weight: f64,
}

impl Edge {
    pub fn new(src: u64, dst: u64, weight: f64) -> Self {
        Self { src, dst, weight }
    }

    pub fn src(&self) -> u64 {
        self.src
    }

    pub fn dst(&self) -> u64 {
        self.dst
    }

    pub fn weight(&self) -> f64 {
        self.weight
    }
}

/// Graph - vertices and edges
#[derive(Clone, Debug)]
pub struct Graph {
    vertices: HashMap<u64, Vertex>,
    edges: Vec<Edge>,
    adjacency: HashMap<u64, Vec<u64>>, // vertex -> list of neighbors
}

impl Graph {
    /// Create empty graph
    pub fn new() -> Self {
        Self {
            vertices: HashMap::new(),
            edges: Vec::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add vertex
    pub fn add_vertex(&mut self, id: u64, value: f64) {
        self.vertices.insert(id, Vertex::new(id, value));
        self.adjacency.entry(id).or_insert_with(Vec::new);
    }

    /// Add edge
    pub fn add_edge(&mut self, src: u64, dst: u64, weight: f64) {
        self.edges.push(Edge::new(src, dst, weight));
        self.adjacency.entry(src).or_insert_with(Vec::new).push(dst);
        self.adjacency.entry(dst).or_insert_with(Vec::new).push(src);
    }

    /// Get vertices
    pub fn vertices(&self) -> Vec<&Vertex> {
        self.vertices.values().collect()
    }

    /// Get edges
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Get neighbors of vertex
    pub fn neighbors(&self, vertex_id: u64) -> Vec<u64> {
        self.adjacency
            .get(&vertex_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Number of vertices
    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    /// Number of edges
    pub fn num_edges(&self) -> usize {
        self.edges.len()
    }

    /// Compute PageRank
    /// Returns (vertex_id, rank)
    pub fn page_rank(&self, iterations: usize, damping: f64) -> HashMap<u64, f64> {
        let n = self.vertices.len() as f64;
        let mut ranks: HashMap<u64, f64> = HashMap::new();

        // Initialize ranks
        for vertex_id in self.vertices.keys() {
            ranks.insert(*vertex_id, 1.0 / n);
        }

        // Iterate
        for _ in 0..iterations {
            let mut new_ranks: HashMap<u64, f64> = HashMap::new();

            for vertex_id in self.vertices.keys() {
                let mut rank = (1.0 - damping) / n;

                // Sum contributions from incoming edges
                for (src_id, edges_from) in &self.adjacency {
                    if edges_from.contains(vertex_id) {
                        let src_rank = ranks.get(src_id).copied().unwrap_or(0.0);
                        let out_degree = edges_from.len() as f64;
                        rank += damping * (src_rank / out_degree);
                    }
                }

                new_ranks.insert(*vertex_id, rank);
            }

            ranks = new_ranks;
        }

        ranks
    }

    /// Find connected components using DFS
    /// Returns (vertex_id, component_id)
    pub fn connected_components(&self) -> HashMap<u64, u64> {
        let mut visited: HashSet<u64> = HashSet::new();
        let mut components: HashMap<u64, u64> = HashMap::new();
        let mut component_id = 0u64;

        for start_id in self.vertices.keys() {
            if !visited.contains(start_id) {
                self.dfs(*start_id, &mut visited, &mut components, component_id);
                component_id += 1;
            }
        }

        components
    }

    fn dfs(
        &self,
        vertex_id: u64,
        visited: &mut HashSet<u64>,
        components: &mut HashMap<u64, u64>,
        component_id: u64,
    ) {
        if visited.contains(&vertex_id) {
            return;
        }

        visited.insert(vertex_id);
        components.insert(vertex_id, component_id);

        for neighbor_id in self.neighbors(vertex_id) {
            self.dfs(neighbor_id, visited, components, component_id);
        }
    }

    /// Count triangles
    pub fn triangle_count(&self) -> usize {
        let mut count = 0;

        for edge in &self.edges {
            let src_neighbors = self.neighbors(edge.src);
            let dst_neighbors = self.neighbors(edge.dst);

            // Check if there's a common neighbor
            for neighbor in src_neighbors {
                if dst_neighbors.contains(&neighbor) {
                    count += 1;
                }
            }
        }

        // Each triangle is counted 3 times
        count / 3
    }

    /// Compute average clustering coefficient
    pub fn clustering_coefficient(&self) -> f64 {
        if self.vertices.len() < 3 {
            return 0.0;
        }

        let mut total = 0.0;

        for vertex_id in self.vertices.keys() {
            let neighbors = self.neighbors(*vertex_id);
            if neighbors.len() < 2 {
                continue;
            }

            // Count edges between neighbors
            let mut edges_between = 0;
            for i in 0..neighbors.len() {
                for j in (i + 1)..neighbors.len() {
                    let n1 = neighbors[i];
                    let n2 = neighbors[j];
                    if self.neighbors(n1).contains(&n2) {
                        edges_between += 1;
                    }
                }
            }

            let possible_edges = neighbors.len() * (neighbors.len() - 1) / 2;
            if possible_edges > 0 {
                total += edges_between as f64 / possible_edges as f64;
            }
        }

        total / self.vertices.len() as f64
    }

    /// Shortest path using BFS
    /// Returns path from src to dst
    pub fn shortest_path(&self, src: u64, dst: u64) -> Result<Vec<u64>, String> {
        let mut visited = HashSet::new();
        let mut queue = vec![(src, vec![src])];
        visited.insert(src);

        while !queue.is_empty() {
            let (current, path) = queue.remove(0);

            if current == dst {
                return Ok(path);
            }

            for neighbor in self.neighbors(current) {
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor);
                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    queue.push((neighbor, new_path));
                }
            }
        }

        Err(format!("No path found from {} to {}", src, dst))
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_creation() {
        let mut graph = Graph::new();
        graph.add_vertex(1, 1.0);
        graph.add_vertex(2, 2.0);
        graph.add_edge(1, 2, 1.0);

        assert_eq!(graph.num_vertices(), 2);
        assert_eq!(graph.num_edges(), 1);
    }

    #[test]
    fn test_neighbors() {
        let mut graph = Graph::new();
        graph.add_vertex(1, 1.0);
        graph.add_vertex(2, 2.0);
        graph.add_vertex(3, 3.0);
        graph.add_edge(1, 2, 1.0);
        graph.add_edge(1, 3, 1.0);

        let neighbors = graph.neighbors(1);
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_page_rank() {
        let mut graph = Graph::new();
        graph.add_vertex(1, 1.0);
        graph.add_vertex(2, 1.0);
        graph.add_edge(1, 2, 1.0);

        let ranks = graph.page_rank(10, 0.85);
        assert_eq!(ranks.len(), 2);
    }

    #[test]
    fn test_connected_components() {
        let mut graph = Graph::new();
        graph.add_vertex(1, 1.0);
        graph.add_vertex(2, 1.0);
        graph.add_vertex(3, 1.0);
        graph.add_edge(1, 2, 1.0);

        let components = graph.connected_components();
        assert!(components.get(&1) == components.get(&2));
        assert!(components.get(&1) != components.get(&3));
    }

    #[test]
    fn test_triangle_count() {
        let mut graph = Graph::new();
        graph.add_vertex(1, 1.0);
        graph.add_vertex(2, 1.0);
        graph.add_vertex(3, 1.0);
        graph.add_edge(1, 2, 1.0);
        graph.add_edge(2, 3, 1.0);
        graph.add_edge(3, 1, 1.0);

        assert_eq!(graph.triangle_count(), 1);
    }

    #[test]
    fn test_clustering_coefficient() {
        let mut graph = Graph::new();
        graph.add_vertex(1, 1.0);
        graph.add_vertex(2, 1.0);
        graph.add_vertex(3, 1.0);
        graph.add_edge(1, 2, 1.0);
        graph.add_edge(1, 3, 1.0);

        let coeff = graph.clustering_coefficient();
        assert!(coeff >= 0.0 && coeff <= 1.0);
    }

    #[test]
    fn test_shortest_path() {
        let mut graph = Graph::new();
        graph.add_vertex(1, 1.0);
        graph.add_vertex(2, 1.0);
        graph.add_vertex(3, 1.0);
        graph.add_edge(1, 2, 1.0);
        graph.add_edge(2, 3, 1.0);

        let path = graph.shortest_path(1, 3).unwrap();
        assert_eq!(path, vec![1, 2, 3]);
    }
}
