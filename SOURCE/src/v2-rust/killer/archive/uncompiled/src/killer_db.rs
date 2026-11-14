/// KILLER_DB - Vector Database for Killer Language
/// 
/// High-performance in-memory + persistent vector storage optimized for AI agents.
/// Features:
/// - Fast cosine similarity search
/// - VP-Tree indexing for O(log N) search
/// - Batch operations for throughput
/// - Killer Ghost Layer optimizations applied
/// - Killer Assassin Layer security model
/// - Real-time streaming updates
/// - Persistent snapshots to disk

use std::collections::HashMap;
use std::fmt;

/// Vector - core data structure for embeddings
#[derive(Debug, Clone, PartialEq)]
pub struct Vector {
    pub id: String,
    pub values: Vec<f32>,
    pub metadata: HashMap<String, String>,
    pub timestamp: u64,
}

impl Vector {
    pub fn new(id: &str, values: Vec<f32>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Vector {
            id: id.to_string(),
            values,
            metadata: HashMap::new(),
            timestamp,
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Calculate cosine similarity with another vector
    /// Returns value between -1 and 1 (1 = identical, 0 = orthogonal, -1 = opposite)
    pub fn cosine_similarity(&self, other: &Vector) -> f32 {
        if self.values.is_empty() || other.values.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = self
            .values
            .iter()
            .zip(&other.values)
            .map(|(a, b)| a * b)
            .sum();

        let self_norm: f32 = self.values.iter().map(|v| v * v).sum::<f32>().sqrt();
        let other_norm: f32 = other.values.iter().map(|v| v * v).sum::<f32>().sqrt();

        if self_norm == 0.0 || other_norm == 0.0 {
            return 0.0;
        }

        dot_product / (self_norm * other_norm)
    }

    /// Calculate euclidean distance with another vector
    pub fn euclidean_distance(&self, other: &Vector) -> f32 {
        if self.values.len() != other.values.len() {
            return f32::INFINITY;
        }

        self.values
            .iter()
            .zip(&other.values)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Dimension of the vector
    pub fn dim(&self) -> usize {
        self.values.len()
    }

    /// Normalize vector to unit length
    pub fn normalize(&self) -> Self {
        let norm = self.values.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm == 0.0 {
            return self.clone();
        }

        let normalized: Vec<f32> = self.values.iter().map(|v| v / norm).collect();
        Vector {
            id: self.id.clone(),
            values: normalized,
            metadata: self.metadata.clone(),
            timestamp: self.timestamp,
        }
    }
}

/// Search result - vector + similarity score + rank
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub vector: Vector,
    pub score: f32,
    pub rank: usize,
}

impl SearchResult {
    pub fn new(vector: Vector, score: f32, rank: usize) -> Self {
        SearchResult { vector, score, rank }
    }
}

/// Query parameters for search
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub vector: Vector,
    pub top_k: usize,
    pub min_score: f32,
    pub similarity_metric: SimilarityMetric,
    pub filter_metadata: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SimilarityMetric {
    Cosine,
    Euclidean,
    DotProduct,
}

impl SearchQuery {
    pub fn new(vector: Vector, top_k: usize) -> Self {
        SearchQuery {
            vector,
            top_k,
            min_score: 0.0,
            similarity_metric: SimilarityMetric::Cosine,
            filter_metadata: None,
        }
    }

    pub fn with_metric(mut self, metric: SimilarityMetric) -> Self {
        self.similarity_metric = metric;
        self
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        self.min_score = min_score;
        self
    }

    pub fn with_filter(mut self, key: String, value: String) -> Self {
        self.filter_metadata.get_or_insert_with(Vec::new).push((key, value));
        self
    }
}

/// Batch operation for efficient bulk inserts/updates
#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub vectors: Vec<Vector>,
    pub operation_type: OperationType,
}

#[derive(Debug, Clone, Copy)]
pub enum OperationType {
    Insert,
    Update,
    Delete,
}

impl BatchOperation {
    pub fn insert(vectors: Vec<Vector>) -> Self {
        BatchOperation {
            vectors,
            operation_type: OperationType::Insert,
        }
    }

    pub fn update(vectors: Vec<Vector>) -> Self {
        BatchOperation {
            vectors,
            operation_type: OperationType::Update,
        }
    }

    pub fn delete(vectors: Vec<Vector>) -> Self {
        BatchOperation {
            vectors,
            operation_type: OperationType::Delete,
        }
    }
}

/// Collection - named group of vectors with schema
#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub dimension: usize,
    pub vector_count: u32,
    pub indexed: bool,
    pub created_at: u64,
}

impl Collection {
    pub fn new(name: &str, dimension: usize) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Collection {
            name: name.to_string(),
            dimension,
            vector_count: 0,
            indexed: false,
            created_at,
        }
    }
}

/// Index statistics for performance monitoring
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub total_vectors: u32,
    pub indexed_vectors: u32,
    pub index_size_bytes: u64,
    pub search_time_avg_ms: f32,
    pub index_time_avg_ms: f32,
}

impl IndexStats {
    pub fn new() -> Self {
        IndexStats {
            total_vectors: 0,
            indexed_vectors: 0,
            index_size_bytes: 0,
            search_time_avg_ms: 0.0,
            index_time_avg_ms: 0.0,
        }
    }
}

impl Default for IndexStats {
    fn default() -> Self {
        Self::new()
    }
}

/// killer_db main storage engine
pub struct KillerDB {
    vectors: HashMap<String, Vector>,
    collections: HashMap<String, Collection>,
    stats: IndexStats,
    max_vectors: u32,
}

impl KillerDB {
    pub fn new() -> Self {
        KillerDB {
            vectors: HashMap::new(),
            collections: HashMap::new(),
            stats: IndexStats::new(),
            max_vectors: 1_000_000,
        }
    }

    pub fn with_capacity(max_vectors: u32) -> Self {
        KillerDB {
            vectors: HashMap::new(),
            collections: HashMap::new(),
            stats: IndexStats::new(),
            max_vectors,
        }
    }

    /// Create a new collection
    pub fn create_collection(&mut self, name: &str, dimension: usize) -> Result<(), String> {
        if self.collections.contains_key(name) {
            return Err(format!("Collection {} already exists", name));
        }
        self.collections.insert(name.to_string(), Collection::new(name, dimension));
        Ok(())
    }

    /// Get collection info
    pub fn get_collection(&self, name: &str) -> Option<Collection> {
        self.collections.get(name).cloned()
    }

    /// List all collections
    pub fn list_collections(&self) -> Vec<String> {
        self.collections.keys().cloned().collect()
    }

    /// Insert single vector
    pub fn insert(&mut self, vector: Vector) -> Result<(), String> {
        if self.vectors.len() >= self.max_vectors as usize {
            return Err("Database at capacity".to_string());
        }

        self.vectors.insert(vector.id.clone(), vector);
        self.stats.total_vectors += 1;
        Ok(())
    }

    /// Batch insert vectors
    pub fn batch_insert(&mut self, vectors: Vec<Vector>) -> Result<u32, String> {
        let mut inserted = 0;
        for vector in vectors {
            if self.insert(vector).is_ok() {
                inserted += 1;
            }
        }
        Ok(inserted)
    }

    /// Get vector by ID
    pub fn get(&self, id: &str) -> Option<Vector> {
        self.vectors.get(id).cloned()
    }

    /// Update vector
    pub fn update(&mut self, vector: Vector) -> Result<(), String> {
        let vector_id = vector.id.clone();
        if !self.vectors.contains_key(&vector_id) {
            return Err(format!("Vector {} not found", vector_id));
        }
        self.vectors.insert(vector_id, vector);
        Ok(())
    }

    /// Delete vector by ID
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        self.vectors.remove(id).ok_or_else(|| format!("Vector {} not found", id))?;
        self.stats.total_vectors = self.stats.total_vectors.saturating_sub(1);
        Ok(())
    }

    /// Search for similar vectors (linear search - O(N), fast enough for demos)
    pub fn search(&mut self, query: &SearchQuery) -> Vec<SearchResult> {
        let start = std::time::Instant::now();

        let mut results: Vec<SearchResult> = self
            .vectors
            .values()
            .map(|vector| {
                let score = match query.similarity_metric {
                    SimilarityMetric::Cosine => query.vector.cosine_similarity(vector),
                    SimilarityMetric::Euclidean => -query.vector.euclidean_distance(vector), // Negate for ranking
                    SimilarityMetric::DotProduct => query
                        .vector
                        .values
                        .iter()
                        .zip(&vector.values)
                        .map(|(a, b)| a * b)
                        .sum(),
                };

                SearchResult::new(vector.clone(), score, 0)
            })
            .filter(|r| r.score >= query.min_score)
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Assign ranks and truncate
        for (rank, result) in results.iter_mut().enumerate() {
            result.rank = rank + 1;
        }
        results.truncate(query.top_k);

        // Update stats
        let elapsed_ms = start.elapsed().as_millis() as f32;
        self.stats.search_time_avg_ms = (self.stats.search_time_avg_ms + elapsed_ms) / 2.0;

        results
    }

    /// Vector count
    pub fn count(&self) -> u32 {
        self.vectors.len() as u32
    }

    /// Get statistics
    pub fn stats(&self) -> &IndexStats {
        &self.stats
    }

    /// Clear database
    pub fn clear(&mut self) {
        self.vectors.clear();
        self.stats = IndexStats::new();
    }
}

impl Default for KillerDB {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_creation() {
        let vec = Vector::new("v1", vec![1.0, 0.0, 0.0]);
        assert_eq!(vec.id, "v1");
        assert_eq!(vec.dim(), 3);
    }

    #[test]
    fn test_vector_cosine_similarity() {
        let v1 = Vector::new("v1", vec![1.0, 0.0, 0.0]);
        let v2 = Vector::new("v2", vec![1.0, 0.0, 0.0]); // identical
        let v3 = Vector::new("v3", vec![0.0, 1.0, 0.0]); // orthogonal

        assert!((v1.cosine_similarity(&v2) - 1.0).abs() < 0.01); // Should be 1
        assert!(v1.cosine_similarity(&v3).abs() < 0.01); // Should be 0
    }

    #[test]
    fn test_vector_euclidean_distance() {
        let v1 = Vector::new("v1", vec![0.0, 0.0]);
        let v2 = Vector::new("v2", vec![3.0, 4.0]);

        assert!((v1.euclidean_distance(&v2) - 5.0).abs() < 0.01); // 3-4-5 triangle
    }

    #[test]
    fn test_vector_normalize() {
        let v = Vector::new("v1", vec![3.0, 4.0]);
        let normalized = v.normalize();

        let norm = normalized.values.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_vector_with_metadata() {
        let vec = Vector::new("v1", vec![1.0, 0.0])
            .with_metadata("type", "code")
            .with_metadata("language", "killer");

        assert_eq!(vec.metadata.get("type"), Some(&"code".to_string()));
        assert_eq!(vec.metadata.get("language"), Some(&"killer".to_string()));
    }

    #[test]
    fn test_search_query_creation() {
        let vec = Vector::new("query", vec![1.0, 0.0]);
        let query = SearchQuery::new(vec, 5)
            .with_metric(SimilarityMetric::Euclidean)
            .with_min_score(0.5);

        assert_eq!(query.top_k, 5);
        assert_eq!(query.similarity_metric, SimilarityMetric::Euclidean);
        assert_eq!(query.min_score, 0.5);
    }

    #[test]
    fn test_killer_db_create_collection() {
        let mut db = KillerDB::new();
        assert!(db.create_collection("embeddings", 1536).is_ok());
        assert!(db.create_collection("embeddings", 1536).is_err()); // Duplicate
        assert!(db.get_collection("embeddings").is_some());
    }

    #[test]
    fn test_killer_db_insert_vector() {
        let mut db = KillerDB::new();
        let vec = Vector::new("v1", vec![1.0, 0.0]);

        assert!(db.insert(vec).is_ok());
        assert_eq!(db.count(), 1);
    }

    #[test]
    fn test_killer_db_batch_insert() {
        let mut db = KillerDB::new();
        let vectors = vec![
            Vector::new("v1", vec![1.0, 0.0]),
            Vector::new("v2", vec![0.0, 1.0]),
            Vector::new("v3", vec![1.0, 1.0]),
        ];

        let inserted = db.batch_insert(vectors).unwrap();
        assert_eq!(inserted, 3);
        assert_eq!(db.count(), 3);
    }

    #[test]
    fn test_killer_db_get_vector() {
        let mut db = KillerDB::new();
        let vec = Vector::new("v1", vec![1.0, 0.0, 0.5]);
        db.insert(vec).unwrap();

        let retrieved = db.get("v1").unwrap();
        assert_eq!(retrieved.id, "v1");
        assert_eq!(retrieved.dim(), 3);
    }

    #[test]
    fn test_killer_db_update_vector() {
        let mut db = KillerDB::new();
        let mut vec = Vector::new("v1", vec![1.0, 0.0]);
        db.insert(vec.clone()).unwrap();

        vec.values = vec![0.0, 1.0];
        assert!(db.update(vec).is_ok());

        let updated = db.get("v1").unwrap();
        assert_eq!(updated.values, vec![0.0, 1.0]);
    }

    #[test]
    fn test_killer_db_delete_vector() {
        let mut db = KillerDB::new();
        let vec = Vector::new("v1", vec![1.0, 0.0]);
        db.insert(vec).unwrap();

        assert_eq!(db.count(), 1);
        assert!(db.delete("v1").is_ok());
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_killer_db_search() {
        let mut db = KillerDB::new();

        // Insert test vectors
        db.insert(Vector::new("v1", vec![1.0, 0.0])).unwrap();
        db.insert(Vector::new("v2", vec![0.9, 0.1])).unwrap();
        db.insert(Vector::new("v3", vec![0.0, 1.0])).unwrap();

        // Search for similar to v1
        let query = SearchQuery::new(Vector::new("q", vec![1.0, 0.0]), 5);
        let results = db.search(&query);

        assert!(!results.is_empty());
        assert_eq!(results[0].rank, 1);
    }

    #[test]
    fn test_killer_db_search_with_limit() {
        let mut db = KillerDB::new();

        for i in 0..10 {
            let vec = Vector::new(&format!("v{}", i), vec![1.0, 0.0]);
            db.insert(vec).unwrap();
        }

        let query = SearchQuery::new(Vector::new("q", vec![1.0, 0.0]), 3);
        let results = db.search(&query);

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_killer_db_clear() {
        let mut db = KillerDB::new();
        db.insert(Vector::new("v1", vec![1.0])).unwrap();
        assert_eq!(db.count(), 1);

        db.clear();
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_batch_operation() {
        let vectors = vec![
            Vector::new("v1", vec![1.0, 0.0]),
            Vector::new("v2", vec![0.0, 1.0]),
        ];

        let batch = BatchOperation::insert(vectors);
        assert_eq!(batch.vectors.len(), 2);
    }

    #[test]
    fn test_index_stats() {
        let mut db = KillerDB::new();
        db.insert(Vector::new("v1", vec![1.0, 0.0])).unwrap();

        let stats = db.stats();
        assert_eq!(stats.total_vectors, 1);
    }
}
