# Killer Standard Library — Algorithm Releases

## v1.2.0 — "Enterprise" (March 28, 2026) ✅ STABLE

### New in v1.2
| Feature | File | Performance |
|---------|------|-------------|
| Native HashMap (O(1)) | `builtin.rs` — `hm_*` | 8–12 ms/lookup |
| Dijkstra shortest path | `builtin.rs` — `dijkstra`, `dijkstra_path` | 8–9 ms / 100 vertices |
| Binary Search Tree | `algorithms/bst.killer` | O(log n) insert/search |
| Dynamic Programming | `algorithms/dynamic_programming.killer` | fib/knapsack/LCS/coin_change |

### Native Builtins (Rust-backed, zero VM overhead)
```
hash_map_new()                      → Dict
hash_map_insert(map, key, value)    → Dict (updated)
hash_map_get(map, key)              → Value | Null
hash_map_contains(map, key)         → Bool
hash_map_remove(map, key)           → Dict (without key)
hash_map_size(map)                  → Number
hash_map_keys(map)                  → Array<Str>
hash_map_values(map)                → Array<Value>

dijkstra(adj_list, source)               → Array<Number> (distances)
dijkstra_path(adj_list, source, target)  → Array<Number> (vertex path)
```
`adj_list` format: `Array<Array<Dict>>` where each inner Dict is `{to: Int, weight: Int}`.

### Dynamic Programming (`dynamic_programming.killer`)
- `fibonacci_dp(n)` — O(n) with HashMap memoization
- `knapsack(items, capacity)` — 0/1 knapsack O(n×W) DP table
- `lcs(a, b)` — Longest Common Subsequence O(m×n)
- `coin_change(coins, amount)` — Min coins O(amount×|coins|)

### Binary Search Tree (`bst.killer`)
- `BST.insert(key, value)` / `BST.search(key)`
- `BST.inorder()` → sorted key array
- `BST.height()`, `BST.get_size()`
- Test suite: insert/search, inorder sorted, height, key update

### Version Constants (version.rs)
```rust
pub const STDLIB_VERSION: &str = "1.2.0";
pub const STDLIB_CODENAME: &str = "Enterprise";
```

### Test Results
```
cargo test --lib
639 passed; 0 failed; 2 ignored
```

---

## v1.1.0 — "General Purpose" (March 17, 2026) ✅ STABLE

**Avg latency: 72.78 ms**

| Algorithm | Latency |
|-----------|---------|
| DFS (depth-first search) | 50 ms |
| BFS (breadth-first search) | 56 ms |
| Quicksort | 80 ms |
| Mergesort | 55 ms |
| Binary Search | 62 ms |
| Prime Sieve (from v1.01) | 52.73 ms |
| Fibonacci (from v1.01) | 88.62 ms |
| Factorial (from v1.01) | 108.07 ms |
| Matrix Multiply (from v1.01) | 91.92 ms |
| Bubble Sort (from v1.01) | 143 ms |

---

## v1.01 — "Mathematics" (March 16, 2026) ✅ STABLE

**Avg latency: 96.87 ms**

| Algorithm | Latency |
|-----------|---------|
| Prime Sieve | 52.73 ms |
| Fibonacci | 88.62 ms |
| Factorial | 108.07 ms |
| Matrix Multiply | 91.92 ms |
| Bubble Sort | 143 ms |

---

## Roadmap

| Milestone | Target | Status |
|-----------|--------|--------|
| v1.01 Math algorithms | March 16, 2026 | ✅ Released |
| v1.1 General purpose | March 17, 2026 | ✅ Released |
| v1.2 Enterprise (Maps + Graphs + Trees + DP) | March 28, 2026 | ✅ Released |
| v2.0 AI-native (Async, LLM, Vectors, Memory) | June 2026 | 🔵 Planned |
