fn main() {
    let mut visited = vec![false; 1000];
    let mut result = vec![];
    dfs(0, &mut visited, &mut result);
    
    println!("DFS traversed {} nodes. Root to leaf: 0 -> {}", result.len(), result.last().unwrap_or(&0));
}

fn dfs(node: i32, visited: &mut Vec<bool>, result: &mut Vec<i32>) {
    visited[node as usize] = true;
    result.push(node);
    
    // Simulate tree children (0-1023 possible)
    let child1 = (node * 2 + 1) as i32;
    let child2 = (node * 2 + 2) as i32;
    
    if (child1 as usize) < 1000 && !visited[child1 as usize] {
        dfs(child1, visited, result);
    }
    if (child2 as usize) < 1000 && !visited[child2 as usize] {
        dfs(child2, visited, result);
    }
}
