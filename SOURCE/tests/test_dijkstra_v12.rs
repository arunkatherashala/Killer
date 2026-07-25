use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

fn main() {
    // Test Dijkstra on 100 vertex graph
    let mut graph = Graph::new(100);
    
    // Add edges to create a realistic graph
    for i in 0..99 {
        graph.add_edge(i, i+1, 1);  // Linear path
        if i % 10 == 0 {
            graph.add_edge(i, (i+10) % 100, 2);  // Cross-edges
        }
    }
    
    let start = std::time::Instant::now();
    let distances = dijkstra(&graph, 0);
    let elapsed = start.elapsed().as_millis();
    
    println!("Dijkstra 100 vertices: {}ms", elapsed);
    println!("Shortest path to vertex 99: {}", distances.get(&99).unwrap_or(&-1));
}

struct Graph {
    vertices: usize,
    adj: HashMap<usize, Vec<(usize, i32)>>,
}

impl Graph {
    fn new(vertices: usize) -> Self {
        Graph { vertices, adj: HashMap::new() }
    }
    
    fn add_edge(&mut self, u: usize, v: usize, weight: i32) {
        self.adj.entry(u).or_insert_with(Vec::new).push((v, weight));
    }
}

fn dijkstra(graph: &Graph, src: usize) -> HashMap<usize, i32> {
    let mut dist = HashMap::new();
    for i in 0..graph.vertices {
        dist.insert(i, i32::MAX);
    }
    dist.insert(src, 0);
    
    let mut pq = BinaryHeap::new();
    pq.push(Reverse((0, src)));
    
    while let Some(Reverse((d, u))) = pq.pop() {
        if d > dist[&u] { continue; }
        
        if let Some(edges) = graph.adj.get(&u) {
            for &(v, weight) in edges {
                let new_dist = dist[&u] + weight;
                if new_dist < dist[&v] {
                    dist.insert(v, new_dist);
                    pq.push(Reverse((new_dist, v)));
                }
            }
        }
    }
    
    dist
}
