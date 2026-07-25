# killer_db API Reference

**killer_db** is a high-performance vector database for storing and searching embeddings.

## Table of Contents
1. [Vector](#vector)
2. [SearchQuery](#searchquery)
3. [KillerDB](#killerdb)
4. [Collection](#collection)
5. [Examples](#examples)

---

## Vector

### Description
Represents an embedding with metadata. Typically 1,536+ dimensions for LLM embeddings.

### Definition
```rust
pub struct Vector {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub timestamp: u64,
}
```

### Constructor

#### `Vector::new(id: &str, values: Vec<f32>) -> Self`
Create a new vector with ID and embedding values.

**Parameters:**
- `id` - Unique identifier (e.g., "doc_001", "pattern_ghost_layer")
- `values` - List of floating-point numbers (embedding dimensions)

**Example:**
```rust
let vec = Vector::new("optimization_tip_1", vec![0.5, -0.3, 0.8]);
```

### Methods

#### `with_metadata(key: &str, value: &str) -> Self`
Add metadata label to vector (chainable).

**Parameters:**
- `key` - Metadata key (e.g., "category", "language")
- `value` - Metadata value (e.g., "optimization", "killer")

**Returns:** Self (for chaining)

**Example:**
```rust
let vec = Vector::new("doc_1", values)
    .with_metadata("title", "Ghost Layer Optimization")
    .with_metadata("category", "performance")
    .with_metadata("confidence", "0.95");
```

#### `cosine_similarity(other: &Vector) -> f32`
Calculate cosine similarity with another vector.

**Returns:** Float between -1 and 1
- `1.0` = identical vectors
- `0.5` = similar
- `0.0` = orthogonal
- `-1.0` = opposite

**Example:**
```rust
let similarity = vec1.cosine_similarity(&vec2);
if similarity > 0.8 {
    println!("Very similar!");
}
```

#### `euclidean_distance(other: &Vector) -> f32`
Calculate Euclidean distance with another vector.

**Returns:** Float >= 0
- `0.0` = identical
- `< 1.0` = very similar
- `> 5.0` = dissimilar

**Example:**
```rust
let distance = vec1.euclidean_distance(&vec2);
```

#### `normalize() -> Self`
Create normalized (unit length) copy of vector.

**Returns:** New normalized Vector

**Example:**
```rust
let normalized = vec.normalize();  // Length = 1.0
```

#### `dim() -> usize`
Get vector dimensionality.

**Returns:** Number of dimensions

**Example:**
```rust
assert_eq!(vec.dim(), 1536);  // OpenAI embeddings
```

---

## SearchQuery

### Description
Parameters for searching similar vectors in killer_db.

### Definition
```rust
pub struct SearchQuery {
    pub vector: Vector,
    pub top_k: usize,
    pub min_score: f32,
    pub similarity_metric: SimilarityMetric,
    pub filter_metadata: Option<Vec<(String, String)>>,
}

pub enum SimilarityMetric {
    Cosine,      // Default
    Euclidean,
    DotProduct,
}
```

### Constructor

#### `SearchQuery::new(vector: Vector, top_k: usize) -> Self`
Create search query with defaults.

**Parameters:**
- `vector` - Query vector (same dimensions as stored vectors)
- `top_k` - Maximum results to return

**Example:**
```rust
let query = SearchQuery::new(question_vector, 5);
// Returns top 5 most similar vectors
```

### Methods

#### `with_metric(metric: SimilarityMetric) -> Self`
Set similarity metric (chainable).

**Parameters:**
- `metric` - Cosine (default), Euclidean, or DotProduct

**Returns:** Self

**Example:**
```rust
let query = SearchQuery::new(vec, 5)
    .with_metric(SimilarityMetric::Euclidean);
```

#### `with_min_score(min_score: f32) -> Self`
Filter results by minimum score (chainable).

**Parameters:**
- `min_score` - Minimum similarity threshold (0.0-1.0)

**Returns:** Self

**Example:**
```rust
let query = SearchQuery::new(vec, 10)
    .with_metric(SimilarityMetric::Cosine)
    .with_min_score(0.7);  // Only results > 0.7 similarity
```

#### `with_filter(key: String, value: String) -> Self`
Filter by metadata (chainable).

**Parameters:**
- `key` - Metadata key
- `value` - Metadata value to match

**Returns:** Self

**Example:**
```rust
let query = SearchQuery::new(vec, 10)
    .with_filter("category".to_string(), "optimization".to_string())
    .with_filter("language".to_string(), "killer".to_string());
```

---

## KillerDB

### Description
Main vector database for storing and searching vectors.

### Constructor

#### `KillerDB::new() -> Self`
Create new database with default capacity (1M vectors).

**Example:**
```rust
let mut db = KillerDB::new();
```

#### `KillerDB::with_capacity(max_vectors: u32) -> Self`
Create new database with specific capacity.

**Parameters:**
- `max_vectors` - Maximum vectors to store

**Example:**
```rust
let mut db = KillerDB::with_capacity(100_000);
```

### Methods

#### `create_collection(name: &str, dimension: usize) -> Result<(), String>`
Create named collection for organizing vectors.

**Parameters:**
- `name` - Collection name (e.g., "patterns", "examples")
- `dimension` - Vector dimensionality

**Returns:** Result

**Example:**
```rust
db.create_collection("killer_patterns", 1536)?;
db.create_collection("code_examples", 1536)?;
```

#### `get_collection(name: &str) -> Option<Collection>`
Get collection info.

**Parameters:**
- `name` - Collection name

**Returns:** Collection or None

**Example:**
```rust
if let Some(coll) = db.get_collection("patterns") {
    println!("Collection: {}, Vectors: {}", coll.name, coll.vector_count);
}
```

#### `insert(vector: Vector) -> Result<(), String>`
Insert single vector.

**Parameters:**
- `vector` - Vector to store

**Returns:** Result

**Example:**
```rust
let vec = Vector::new("doc_1", embeddings);
db.insert(vec)?;
```

#### `batch_insert(vectors: Vec<Vector>) -> Result<u32, String>`
Insert multiple vectors efficiently.

**Parameters:**
- `vectors` - List of vectors

**Returns:** Number of vectors inserted

**Example:**
```rust
let docs = vec![
    Vector::new("doc_1", vec1),
    Vector::new("doc_2", vec2),
];
let inserted = db.batch_insert(docs)?;
println!("Inserted {} vectors", inserted);
```

#### `get(id: &str) -> Option<Vector>`
Retrieve vector by ID.

**Parameters:**
- `id` - Vector ID

**Returns:** Vector or None

**Example:**
```rust
if let Some(vec) = db.get("doc_1") {
    println!("Found vector: {}", vec.id);
}
```

#### `update(vector: Vector) -> Result<(), String>`
Update existing vector.

**Parameters:**
- `vector` - Updated vector (must have same ID)

**Returns:** Result

**Example:**
```rust
let updated_vec = Vector::new("doc_1", new_embeddings);
db.update(updated_vec)?;
```

#### `delete(id: &str) -> Result<(), String>`
Delete vector by ID.

**Parameters:**
- `id` - Vector ID

**Returns:** Result

**Example:**
```rust
db.delete("doc_1")?;
```

#### `search(query: &SearchQuery) -> Vec<SearchResult>`
Search for similar vectors.

**Parameters:**
- `query` - Search parameters

**Returns:** Ranked results

**Example:**
```rust
let query = SearchQuery::new(question_vec, 5)
    .with_min_score(0.7);
let results = db.search(&query);
for (rank, result) in results.iter().enumerate() {
    println!("#{}: {} ({})", rank+1, result.vector.id, result.score);
}
```

#### `count() -> u32`
Get total vectors in database.

**Returns:** Vector count

**Example:**
```rust
println!("Database contains {} vectors", db.count());
```

#### `clear()`
Remove all vectors from database.

**Example:**
```rust
db.clear();
assert_eq!(db.count(), 0);
```

#### `stats() -> &IndexStats`
Get database statistics.

**Returns:** IndexStats

**Example:**
```rust
let stats = db.stats();
println!("Total vectors: {}", stats.total_vectors);
println!("Average search time: {}ms", stats.search_time_avg_ms);
```

---

## Collection

### Description
Named group of vectors for organizing knowledge.

### Definition
```rust
pub struct Collection {
    pub name: String,
    pub dimension: usize,
    pub vector_count: u32,
    pub indexed: bool,
    pub created_at: u64,
}
```

### Constructor

#### `Collection::new(name: &str, dimension: usize) -> Self`
Create new collection.

**Parameters:**
- `name` - Collection name
- `dimension` - Vector size

**Example:**
```rust
let coll = Collection::new("ghost_layer_docs", 1536);
```

---

## Examples

### Basic Insert & Search

```rust
use killer_db::{KillerDB, Vector, SearchQuery};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut db = KillerDB::new();
    
    // Insert vectors
    db.insert(Vector::new("doc1", vec![0.5, -0.3, 0.8])
        .with_metadata("title", "Ghost Layer")
        .with_metadata("category", "optimization"))?;
    
    db.insert(Vector::new("doc2", vec![0.51, -0.29, 0.79])
        .with_metadata("title", "Hot Path Detection")
        .with_metadata("category", "optimization"))?;
    
    // Search similar vectors
    let query = SearchQuery::new(Vector::new("q", vec![0.5, -0.3, 0.8]), 5);
    let results = db.search(&query);
    
    for result in results {
        println!("{}: {}", result.vector.id, result.score);
    }
    
    Ok(())
}
```

### Collections

```rust
// Create collections
db.create_collection("patterns", 1536)?;
db.create_collection("examples", 1536)?;

// Get collection info
if let Some(coll) = db.get_collection("patterns") {
    println!("Collection: {}, Dimension: {}", coll.name, coll.dimension);
}

// List all collections
let collections = db.list_collections();
for name in collections {
    println!("Collection: {}", name);
}
```

### Search with Filters

```rust
let query = SearchQuery::new(question_vec, 10)
    .with_metric(SimilarityMetric::Cosine)
    .with_min_score(0.75)
    .with_filter("category".to_string(), "optimization".to_string())
    .with_filter("language".to_string(), "killer".to_string());

let results = db.search(&query);
```

### Batch Operations

```rust
// Prepare vectors
let docs = vec![
    Vector::new("v1", vec1).with_metadata("type", "pattern"),
    Vector::new("v2", vec2).with_metadata("type", "example"),
    Vector::new("v3", vec3).with_metadata("type", "guide"),
];

// Insert all at once
let inserted = db.batch_insert(docs)?;
println!("Inserted {} vectors", inserted);

// Get statistics
let stats = db.stats();
println!("Total: {}, Avg search: {}ms", 
    stats.total_vectors, 
    stats.search_time_avg_ms);
```

---

## Performance Characteristics

| Operation | Complexity | Speed | Notes |
|-----------|-----------|-------|-------|
| Insert | O(1) | < 1ms | Amortized |
| Get | O(1) | < 1ms | Hash lookup |
| Search | O(N) | < 100ms * | Linear scan |
| Delete | O(1) | < 1ms | Amortized |
| Batch insert | O(M) | < 10ms * | M = batch size |

*For < 100K vectors. Phase 8 will add HNSW indexing for O(log N) search.

---

## Best Practices

1. **Use collections** - Organize vectors by type (patterns, examples, etc.)
2. **Add metadata** - Include title, category, timestamp for better search filtering
3. **Batch operations** - Insert multiple vectors together when possible
4. **Set min_score** - Filter out low-relevance results (e.g., 0.7+ for good matches)
5. **Monitor stats** - Track search time, vector count to optimize

---

**API Version**: Phase 6  
**Last Updated**: March 18, 2026
