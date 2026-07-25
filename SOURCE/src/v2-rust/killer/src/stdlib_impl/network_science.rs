// ================================================================
// NETWORK SCIENCE SOLVER - Phase 21.3
// Graph algorithms, network metrics, community detection
// Ported from: solver_network_science.killer
// ================================================================

use std::collections::{HashMap, HashSet, VecDeque};

pub type Vector = Vec<f64>;

/// Network Science Solver
pub struct NetworkScienceSolver;

impl NetworkScienceSolver {
    // ================================================================
    // BASIC GRAPH METRICS (1-20)
    // ================================================================

    /// Problem 1: Degree Centrality
    pub fn degree_centrality(adj_matrix: &[Vec<u32>], node: usize) -> f64 {
        if node >= adj_matrix.len() { return 0.0; }
        
        let n = adj_matrix.len();
        let degree: u32 = adj_matrix[node].iter().sum();
        degree as f64 / (n - 1) as f64
    }

    /// Problem 2: Betweenness Centrality (shortest paths)
    pub fn betweenness_centrality(adj_matrix: &[Vec<u32>], node: usize) -> f64 {
        if node >= adj_matrix.len() { return 0.0; }
        
        let n = adj_matrix.len();
        let mut betweenness = 0.0;
        
        // Count shortest paths through node
        for i in 0..n {
            for j in i + 1..n {
                if i == node || j == node { continue; }
                // Simplified: increment if node is on path
                if adj_matrix[i][node] > 0 && adj_matrix[node][j] > 0 {
                    betweenness += 1.0;
                }
            }
        }
        
        betweenness / ((n - 1) * (n - 2)) as f64
    }

    /// Problem 3: Closeness Centrality
    pub fn closeness_centrality(adj_matrix: &[Vec<f64>], node: usize) -> f64 {
        if node >= adj_matrix.len() { return 0.0; }
        
        let n = adj_matrix.len();
        let sum_distances: f64 = adj_matrix[node].iter().sum::<f64>();
        
        if sum_distances <= 0.0 { return 0.0; }
        (n - 1) as f64 / sum_distances
    }

    /// Problem 4: Eigenvector Centrality (power iteration)
    pub fn eigenvector_centrality(adj_matrix: &[Vec<f64>]) -> Vec<f64> {
        let n = adj_matrix.len();
        if n == 0 { return vec![]; }
        
        let mut x = vec![1.0; n];
        
        for _ in 0..20 {  // 20 iterations
            let mut new_x = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    new_x[i] += adj_matrix[i][j] * x[j];
                }
            }
            
            let norm = new_x.iter().map(|xi| xi * xi).sum::<f64>().sqrt();
            if norm > 1e-14 {
                x = new_x.iter().map(|xi| xi / norm).collect();
            }
        }
        x
    }

    /// Problem 5: Pagerank Algorithm
    pub fn pagerank(adj_matrix: &[Vec<f64>], damping: f64, iterations: usize) -> Vec<f64> {
        let n = adj_matrix.len();
        if n == 0 { return vec![]; }
        
        let mut ranks = vec![1.0 / n as f64; n];
        let base_score = (1.0 - damping) / n as f64;
        
        for _ in 0..iterations {
            let mut new_ranks = vec![base_score; n];
            
            for i in 0..n {
                let outgoing: u32 = adj_matrix[i].iter().map(|&x| if x > 0.0 { 1 } else { 0 }).sum();
                if outgoing > 0 {
                    let contribution = ranks[i] / outgoing as f64;
                    for j in 0..n {
                        if adj_matrix[i][j] > 0.0 {
                            new_ranks[j] += damping * contribution;
                        }
                    }
                }
            }
            ranks = new_ranks;
        }
        ranks
    }

    // ================================================================
    // GRAPH PROPERTIES (10-25)
    // ================================================================

    /// Problem 6: Average Clustering Coefficient
    pub fn avg_clustering_coefficient(adj_matrix: &[Vec<u32>]) -> f64 {
        let n = adj_matrix.len();
        if n < 3 { return 0.0; }
        
        let mut total = 0.0;
        for i in 0..n {
            let neighbors: Vec<usize> = (0..n)
                .filter(|j| adj_matrix[i][*j] > 0)
                .collect();
            
            if neighbors.len() < 2 { continue; }
            
            let mut connections = 0;
            for j in 0..neighbors.len() {
                for k in j + 1..neighbors.len() {
                    if adj_matrix[neighbors[j]][neighbors[k]] > 0 {
                        connections += 1;
                    }
                }
            }
            
            let max_connections = neighbors.len() * (neighbors.len() - 1) / 2;
            total += connections as f64 / max_connections as f64;
        }
        
        total / n as f64
    }

    /// Problem 7: Diameter (longest shortest path)
    pub fn network_diameter(distances: &[Vec<f64>]) -> f64 {
        distances.iter()
            .flat_map(|row| row.iter())
            .filter(|&&d| d > 0.0 && d < f64::INFINITY)
            .cloned()
            .fold(0.0, f64::max)
    }

    /// Problem 8: Average Path Length
    pub fn avg_path_length(distances: &[Vec<f64>]) -> f64 {
        let (sum, count) = distances.iter()
            .flat_map(|row| row.iter())
            .filter(|&&d| d > 0.0 && d < f64::INFINITY)
            .fold((0.0, 0), |(s, c), &d| (s + d, c + 1));
        
        if count == 0 { return 0.0; }
        sum / count as f64
    }

    /// Problem 9: Density
    pub fn density(adj_matrix: &[Vec<u32>]) -> f64 {
        let n = adj_matrix.len();
        if n < 2 { return 0.0; }
        
        let edges: u32 = adj_matrix.iter().flat_map(|row| row.iter()).sum();
        let max_edges = (n * (n - 1)) as u32;
        
        edges as f64 / max_edges as f64
    }

    /// Problem 10: Assortativity (degree correlation)
    pub fn assortativity(adj_matrix: &[Vec<u32>]) -> f64 {
        let n = adj_matrix.len();
        if n < 2 { return 0.0; }
        
        let degrees: Vec<f64> = adj_matrix.iter()
            .map(|row| row.iter().map(|&x| x as f64).sum())
            .collect();
        
        let avg_deg = degrees.iter().sum::<f64>() / n as f64;
        
        let mut numerator = 0.0;
        let mut denominator = 0.0;
        
        for i in 0..n {
            for j in 0..n {
                if adj_matrix[i][j] > 0 {
                    numerator += (degrees[i] - avg_deg) * (degrees[j] - avg_deg);
                    denominator += (degrees[i] - avg_deg).powi(2);
                }
            }
        }
        
        if denominator.abs() < 1e-14 { return 0.0; }
        numerator / denominator
    }

    // ================================================================
    // ALGORITHMS (20-40)
    // ================================================================

    /// Problem 11: Breadth-First Search (BFS)
    pub fn bfs_distances(adj_matrix: &[Vec<u32>], start: usize) -> Vec<u32> {
        let n = adj_matrix.len();
        if start >= n { return vec![]; }
        
        let mut distances = vec![u32::MAX; n];
        let mut queue = VecDeque::new();
        
        distances[start] = 0;
        queue.push_back(start);
        
        while let Some(node) = queue.pop_front() {
            for next in 0..n {
                if adj_matrix[node][next] > 0 && distances[next] == u32::MAX {
                    distances[next] = distances[node] + 1;
                    queue.push_back(next);
                }
            }
        }
        distances
    }

    /// Problem 12: Depth-First Search (DFS)
    pub fn dfs_visit_order(adj_matrix: &[Vec<u32>], start: usize) -> Vec<usize> {
        let n = adj_matrix.len();
        if start >= n { return vec![]; }
        
        let mut visited = vec![false; n];
        let mut order = Vec::new();
        
        fn dfs(node: usize, adj: &[Vec<u32>], visited: &mut [bool], order: &mut Vec<usize>) {
            visited[node] = true;
            order.push(node);
            
            for next in 0..adj.len() {
                if adj[node][next] > 0 && !visited[next] {
                    dfs(next, adj, visited, order);
                }
            }
        }
        
        dfs(start, adj_matrix, &mut visited, &mut order);
        order
    }

    /// Problem 13: Floyd-Warshall All-Pairs Shortest Path
    pub fn floyd_warshall(adj_matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let n = adj_matrix.len();
        let mut dist = adj_matrix.to_vec();
        
        // Initialize diagonal to 0
        for i in 0..n {
            dist[i][i] = 0.0;
        }
        
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if dist[i][k] + dist[k][j] < dist[i][j] {
                        dist[i][j] = dist[i][k] + dist[k][j];
                    }
                }
            }
        }
        dist
    }

    /// Problem 14: Minimum Spanning Tree (Tarjan-style)
    pub fn is_connected(adj_matrix: &[Vec<u32>]) -> bool {
        if adj_matrix.is_empty() { return true; }
        
        let visited_count = Self::dfs_visit_order(adj_matrix, 0).len();
        visited_count == adj_matrix.len()
    }

    /// Problem 15: Connected Components Count
    pub fn num_components(adj_matrix: &[Vec<u32>]) -> usize {
        let n = adj_matrix.len();
        if n == 0 { return 0; }
        
        let mut visited = vec![false; n];
        let mut components = 0;
        
        for i in 0..n {
            if !visited[i] {
                let order = Self::dfs_visit_order(adj_matrix, i);
                for node in order {
                    visited[node] = true;
                }
                components += 1;
            }
        }
        components
    }

    // ================================================================
    // COMMUNITY DETECTION (30-45)
    // ================================================================

    /// Problem 16: Modularity Measure
    pub fn modularity(adj_matrix: &[Vec<u32>], partition: &[u32]) -> f64 {
        let n = adj_matrix.len();
        let m: u32 = adj_matrix.iter().flat_map(|row| row.iter()).sum::<u32>() / 2;
        
        if m == 0 { return 0.0; }
        
        let mut q = 0.0;
        for i in 0..n {
            for j in 0..n {
                if partition[i] == partition[j] {
                    let deg_i: u32 = adj_matrix[i].iter().sum();
                    let deg_j: u32 = adj_matrix[j].iter().sum();
                    q += (adj_matrix[i][j] as f64 - deg_i as f64 * deg_j as f64 / (2.0 * m as f64));
                }
            }
        }
        
        q / (2.0 * m as f64)
    }

    // ================================================================
    // SMALL-WORLD NETWORKS (35-50)
    // ================================================================

    /// Problem 17: Small-World Coefficient
    pub fn small_world_coefficient(clustering: f64, avg_path_len: f64, 
                                   random_clustering: f64, random_path_len: f64) -> f64 {
        if random_path_len.abs() < 1e-14 || random_clustering.abs() < 1e-14 {
            return 0.0;
        }
        
        let sigma = (clustering / random_clustering) / (avg_path_len / random_path_len);
        sigma
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_density() {
        let adj = vec![
            vec![0, 1, 1],
            vec![1, 0, 1],
            vec![1, 1, 0],
        ];
        let d = NetworkScienceSolver::density(&adj);
        assert!((d - 1.0).abs() < 1e-10);  // Complete graph
    }

    #[test]
    fn test_bfs() {
        let adj = vec![
            vec![0, 1, 0],
            vec![1, 0, 1],
            vec![0, 1, 0],
        ];
        let dist = NetworkScienceSolver::bfs_distances(&adj, 0);
        assert_eq!(dist[0], 0);
        assert_eq!(dist[1], 1);
        assert_eq!(dist[2], 2);
    }

    #[test]
    fn test_connected() {
        let adj = vec![
            vec![0, 1, 0],
            vec![1, 0, 1],
            vec![0, 1, 0],
        ];
        assert!(NetworkScienceSolver::is_connected(&adj));
    }
}
