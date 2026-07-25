fn main() {
    let mut visited = vec![false; 1000];
    let mut queue = vec![0];
    let mut result = vec![];
    
    while !queue.is_empty() {
        let node = queue.remove(0);
        if visited[node as usize] { continue; }
        visited[node as usize] = true;
        result.push(node);
        
        let child1 = node * 2 + 1;
        let child2 = node * 2 + 2;
        
        if (child1 as usize) < 1000 { queue.push(child1); }
        if (child2 as usize) < 1000 { queue.push(child2); }
    }
    
    println!("BFS traversed {} nodes", result.len());
}
