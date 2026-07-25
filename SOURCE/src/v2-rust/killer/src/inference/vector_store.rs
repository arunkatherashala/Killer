// inference/vector_store.rs — In-memory vector store for RAG
//
// Stores text chunks with their embedding vectors and retrieves the most
// semantically similar ones for a query.  No disk, no external DB — pure Rust.
//
// Typical RAG flow:
//   1. Chunk your documents with `chunk_text()`
//   2. Embed each chunk with `KillerEmbedder::embed()`
//   3. `store.add(chunk_text, embedding)`
//   4. At query time: embed the question, call `store.query(q_emb, top_k)`
//   5. Inject returned chunks into the LLM prompt as context

// --- Document chunk -----------------------------------------------------------

/// A single document fragment with its dense embedding vector.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// Original text of the chunk.
    pub text: String,
    /// L2-normalized embedding — length == embed model's hidden size.
    pub embedding: Vec<f32>,
    /// Optional metadata: source file, section title, page number, etc.
    pub source: Option<String>,
}

// --- Vector store -------------------------------------------------------------

/// In-memory vector store — add chunks, query by semantic similarity.
///
/// Backed by a flat list + brute-force cosine search (exact, O(n)).
/// Suitable for up to ~100K chunks on modern hardware.
///
/// # Example
/// ```no_run
/// let mut store = VectorStore::new();
/// store.add("Killer is a fast compiled language.", embedding_vec, None);
/// let hits = store.query(&query_embedding, 3);
/// println!("{}", hits[0]);
/// ```
pub struct VectorStore {
    chunks: Vec<Chunk>,
}

impl VectorStore {
    /// Create an empty store.
    pub fn new() -> Self { VectorStore { chunks: Vec::new() } }

    /// Create a store with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        VectorStore { chunks: Vec::with_capacity(cap) }
    }

    /// Add a chunk with its embedding.
    /// `source` is optional metadata (filename, URL, section, etc.).
    pub fn add(&mut self, text: &str, embedding: Vec<f32>, source: Option<String>) {
        self.chunks.push(Chunk {
            text:      text.to_string(),
            embedding,
            source,
        });
    }

    /// Return the top-`k` most similar chunk texts for a query embedding.
    ///
    /// Assumes L2-normalized embeddings (cosine similarity = dot product).
    pub fn query(&self, query_embedding: &[f32], top_k: usize) -> Vec<&str> {
        if self.chunks.is_empty() { return Vec::new(); }

        let mut scored: Vec<(f32, usize)> = self.chunks.iter().enumerate()
            .map(|(i, c)| (cosine_similarity(&c.embedding, query_embedding), i))
            .collect();

        // Sort descending by similarity score
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0)
                                  .unwrap_or(std::cmp::Ordering::Equal));

        scored.iter()
              .take(top_k)
              .map(|&(_, i)| self.chunks[i].text.as_str())
              .collect()
    }

    /// Return top-k chunks as full `Chunk` structs (includes source metadata).
    pub fn query_chunks(&self, query_embedding: &[f32], top_k: usize) -> Vec<&Chunk> {
        if self.chunks.is_empty() { return Vec::new(); }

        let mut scored: Vec<(f32, usize)> = self.chunks.iter().enumerate()
            .map(|(i, c)| (cosine_similarity(&c.embedding, query_embedding), i))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0)
                                  .unwrap_or(std::cmp::Ordering::Equal));

        scored.iter()
              .take(top_k)
              .map(|&(_, i)| &self.chunks[i])
              .collect()
    }

    /// Return top-k results with their similarity scores.
    pub fn query_scored(&self, query_embedding: &[f32], top_k: usize) -> Vec<(f32, &str)> {
        if self.chunks.is_empty() { return Vec::new(); }

        let mut scored: Vec<(f32, usize)> = self.chunks.iter().enumerate()
            .map(|(i, c)| (cosine_similarity(&c.embedding, query_embedding), i))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0)
                                  .unwrap_or(std::cmp::Ordering::Equal));

        scored.iter()
              .take(top_k)
              .map(|&(score, i)| (score, self.chunks[i].text.as_str()))
              .collect()
    }

    /// Total number of chunks in the store.
    pub fn len(&self) -> usize { self.chunks.len() }

    /// True if no chunks have been added yet.
    pub fn is_empty(&self) -> bool { self.chunks.is_empty() }

    /// Clear all chunks.
    pub fn clear(&mut self) { self.chunks.clear(); }
}

impl Default for VectorStore {
    fn default() -> Self { VectorStore::new() }
}

// --- Similarity metric --------------------------------------------------------

/// Cosine similarity between two vectors.
///
/// For L2-normalized vectors this reduces to a plain dot product — O(n), exact.
/// Returns a value in [-1.0, 1.0].  Higher = more similar.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 { return 0.0; }

    let dot: f32   = a[..len].iter().zip(&b[..len]).map(|(&ai, &bi)| ai * bi).sum();
    let mag_a: f32 = a[..len].iter().map(|&v| v * v).sum::<f32>().sqrt();
    let mag_b: f32 = b[..len].iter().map(|&v| v * v).sum::<f32>().sqrt();

    if mag_a < 1e-8 || mag_b < 1e-8 { 0.0 } else { dot / (mag_a * mag_b) }
}

/// L2 distance between two vectors (Euclidean).  Lower = more similar.
pub fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(&ai, &bi)| (ai - bi).powi(2)).sum::<f32>().sqrt()
}

// --- Document chunker ---------------------------------------------------------

/// Split a document string into overlapping text chunks.
///
/// # Parameters
/// - `text`       — source document text
/// - `chunk_size` — target number of characters per chunk
/// - `overlap`    — number of characters to repeat at the start of each new chunk
///                  (preserves context across chunk boundaries)
///
/// # Example
/// ```
/// let chunks = chunk_text("The quick brown fox...", 100, 20);
/// // Each chunk is ~100 chars, consecutive chunks share 20 chars of context.
/// ```
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() || chunk_size == 0 { return Vec::new(); }

    // Work on character boundaries (safe for Unicode)
    let chars: Vec<char> = text.chars().collect();
    let total  = chars.len();
    let step   = chunk_size.saturating_sub(overlap).max(1);
    let mut chunks = Vec::new();
    let mut start  = 0usize;

    while start < total {
        let end = (start + chunk_size).min(total);
        // Extend to the next whitespace boundary for cleaner splits
        let end = if end < total {
            let scan_from = end.saturating_sub(20);
            chars[scan_from..end].iter().rposition(|&c| c.is_whitespace())
                .map(|p| scan_from + p + 1)
                .unwrap_or(end)
        } else { end };

        chunks.push(chars[start..end].iter().collect::<String>().trim().to_string());
        if end >= total { break; }
        start = (start + step).min(end.saturating_sub(overlap));
    }

    chunks.into_iter().filter(|c| !c.is_empty()).collect()
}

/// Split text on sentence boundaries (`.`, `!`, `?` followed by whitespace).
/// Useful when you want each chunk to be a complete thought.
pub fn chunk_by_sentence(text: &str, max_sentences: usize) -> Vec<String> {
    let mut chunks   = Vec::new();
    let mut current  = String::new();
    let mut count    = 0usize;

    for ch in text.chars() {
        current.push(ch);
        if (ch == '.' || ch == '!' || ch == '?') && count + 1 >= max_sentences {
            let s = current.trim().to_string();
            if !s.is_empty() { chunks.push(s); }
            current.clear();
            count = 0;
        } else if ch == '.' || ch == '!' || ch == '?' {
            count += 1;
        }
    }
    if !current.trim().is_empty() { chunks.push(current.trim().to_string()); }
    chunks
}

// --- RAG context builder ------------------------------------------------------

/// Build a RAG-augmented prompt from retrieved chunks and the user question.
///
/// Produces a prompt in the format:
/// ```text
/// Context:
/// [1] <chunk 1>
/// [2] <chunk 2>
/// ...
///
/// Question: <question>
/// Answer:
/// ```
pub fn build_rag_prompt(chunks: &[&str], question: &str) -> String {
    if chunks.is_empty() {
        return format!("Question: {question}\nAnswer:");
    }

    let mut ctx = String::from("Context:\n");
    for (i, chunk) in chunks.iter().enumerate() {
        ctx.push_str(&format!("[{}] {}\n", i + 1, chunk.trim()));
    }
    ctx.push_str(&format!("\nQuestion: {question}\nAnswer:"));
    ctx
}

// --- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_identical() {
        let v = vec![0.6, 0.8];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn test_vector_store_query() {
        let mut store = VectorStore::new();
        store.add("cats",    vec![1.0, 0.0, 0.0], None);
        store.add("dogs",    vec![0.9, 0.1, 0.0], None);
        store.add("physics", vec![0.0, 0.0, 1.0], None);

        let query = vec![1.0, 0.0, 0.0]; // close to "cats"
        let results = store.query(&query, 2);
        assert_eq!(results[0], "cats");
        assert_eq!(results[1], "dogs");
    }

    #[test]
    fn test_chunk_text_basic() {
        let text = "Hello world this is a test of the chunking function";
        let chunks = chunk_text(text, 20, 5);
        assert!(!chunks.is_empty());
        for c in &chunks { assert!(!c.is_empty()); }
    }

    #[test]
    fn test_build_rag_prompt() {
        let chunks = vec!["Killer is compiled.", "Killer runs on Rust."];
        let prompt = build_rag_prompt(&chunks, "What is Killer?");
        assert!(prompt.contains("[1]"));
        assert!(prompt.contains("[2]"));
        assert!(prompt.contains("What is Killer?"));
    }
}
