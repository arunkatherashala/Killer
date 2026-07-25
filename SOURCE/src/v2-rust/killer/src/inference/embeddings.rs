// inference/embeddings.rs — Text embedding engine
//
// Wraps KillerInference with an embedding-focused API.
// Loads any GGUF model and produces fixed-length dense vectors from text.
//
// Best results with dedicated embedding models (small, fast, ~20-150 MB):
//   • nomic-embed-text-v1.5.Q4_K_M.gguf   ~100 MB  (recommended, 768-dim)
//   • all-MiniLM-L6-v2-Q4_K_M.gguf         ~20 MB  (very fast, 384-dim)
//   • bge-base-en-v1.5.Q4_K_M.gguf        ~120 MB  (strong retrieval, 768-dim)
//   • bge-small-en-v1.5.Q4_K_M.gguf        ~30 MB  (compact, 384-dim)
//   • e5-small-v2.Q4_K_M.gguf              ~25 MB  (multilingual)
//
// Any LLaMA-family generation model also works as an embedder — quality is lower
// than dedicated embed models but still useful for prototyping.
//
// Usage:
//   let embedder = KillerEmbedder::load("nomic-embed-text.gguf")?;
//   let vec = embedder.embed("What is a transformer?");
//   // vec is a 768-dim L2-normalized Vec<f32>

use super::engine::KillerInference;
use super::vector_store::{VectorStore, cosine_similarity, chunk_text, build_rag_prompt};

// --- Embedding model ----------------------------------------------------------

/// A text encoder — wraps `KillerInference` to produce semantic embedding vectors.
///
/// Output vectors are:
/// - L2-normalized (magnitude ≈ 1.0)
/// - Fixed length = `self.dims` (the model's hidden size)
/// - Ready for cosine similarity: `sim(a, b) = dot(a, b)`
pub struct KillerEmbedder {
    engine: KillerInference,
    /// Output dimensionality — equal to the model's embedding size.
    pub dims: usize,
    /// Path the model was loaded from (for diagnostics).
    pub model_path: String,
}

impl KillerEmbedder {
    /// Load a GGUF embedding model.
    ///
    /// # Errors
    /// Returns an error string if the file is missing, corrupt, or uses an
    /// unsupported tensor format.
    pub fn load(path: &str) -> Result<Self, String> {
        let engine = KillerInference::load(path)?;
        let dims   = engine.config.n_embd;
        eprintln!("[KillerEmbed] Ready — dims={}", dims);
        Ok(KillerEmbedder { engine, dims, model_path: path.to_string() })
    }

    /// Convert a text string to a dense L2-normalized float vector.
    ///
    /// Length of the returned `Vec<f32>` equals `self.dims`.
    pub fn embed(&mut self, text: &str) -> Vec<f32> {
        self.engine.embed_text(text)
    }

    /// Embed multiple texts.  Returns one vector per input.
    pub fn embed_batch(&mut self, texts: &[&str]) -> Vec<Vec<f32>> {
        texts.iter().map(|t| self.engine.embed_text(t)).collect()
    }

    /// Compute cosine similarity between two texts (embeds both, then dots).
    pub fn similarity(&mut self, a: &str, b: &str) -> f32 {
        let va = self.embed(a);
        let vb = self.embed(b);
        cosine_similarity(&va, &vb)
    }

    /// Embed and add a single document string to a `VectorStore`.
    /// Splits into chunks of `chunk_size` chars with `overlap` char context window.
    pub fn index_document(&mut self,
                           store: &mut VectorStore,
                           text: &str,
                           chunk_size: usize,
                           overlap: usize,
                           source: Option<String>) {
        let chunks = chunk_text(text, chunk_size, overlap);
        let source_ref = source.as_deref();

        for chunk in &chunks {
            let emb = self.embed(chunk);
            store.add(chunk, emb, source_ref.map(|s| s.to_string()));
        }

        eprintln!("[KillerEmbed] Indexed {} chunks from {:?}",
                  chunks.len(), source.unwrap_or_else(|| "(no source)".into()));
    }

    /// Index multiple documents at once.
    pub fn index_documents(&mut self,
                            store: &mut VectorStore,
                            docs: &[(&str, Option<&str>)],
                            chunk_size: usize,
                            overlap: usize) {
        for (text, source) in docs {
            self.index_document(store, text, chunk_size, overlap,
                                source.map(|s| s.to_string()));
        }
    }

    /// Find the `top_k` chunks from `store` most similar to `question`.
    pub fn retrieve<'a>(&mut self, store: &'a VectorStore, question: &str, top_k: usize)
        -> Vec<&'a str>
    {
        let q_emb = self.embed(question);
        store.query(&q_emb, top_k)
    }
}

// --- Standalone helpers -------------------------------------------------------

/// Compute cosine similarity between two pre-computed embedding vectors.
/// Identical to `vector_store::cosine_similarity` — re-exported for convenience.
pub use super::vector_store::cosine_similarity as embed_cosine;

/// L2-normalize a vector in-place (makes magnitude = 1.0).
pub fn l2_normalize(v: &mut Vec<f32>) {
    let norm: f32 = v.iter().map(|&x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 { for x in v.iter_mut() { *x /= norm; } }
}

/// Top-k most similar indices from a list of candidate embeddings.
///
/// `query`      — the query embedding (L2-normalized)
/// `candidates` — list of candidate embeddings (L2-normalized)
/// Returns indices into `candidates`, sorted by decreasing similarity.
pub fn top_k_similar(query: &[f32], candidates: &[Vec<f32>], k: usize) -> Vec<usize> {
    let mut scored: Vec<(f32, usize)> = candidates.iter().enumerate()
        .map(|(i, c)| (cosine_similarity(query, c), i))
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.iter().take(k).map(|&(_, i)| i).collect()
}

// --- RAG pipeline (embed model + LLM in one call) ----------------------------

/// Full RAG pipeline — index documents, retrieve, augment prompt, generate answer.
///
/// # Parameters
/// - `embed_model` — path to embedding GGUF (nomic-embed-text, all-MiniLM, etc.)
/// - `llm_model`   — path to generation GGUF (any chat model)
/// - `docs`        — list of `(text, source_label)` document pairs to index
/// - `question`    — user question
/// - `top_k`       — how many chunks to retrieve and inject as context
/// - `max_tokens`  — max generation tokens for the answer
///
/// # Example
/// ```no_run
/// let answer = killer_rag(
///     "nomic-embed.gguf",
///     "tinyllama.gguf",
///     &[("Killer is a compiled language with actor-based concurrency.", Some("docs"))],
///     "What runtime model does Killer use?",
///     3,
///     256,
/// )?;
/// println!("{}", answer);
/// ```
pub fn killer_rag(embed_model: &str,
                  llm_model:   &str,
                  docs:         &[(&str, Option<&str>)],
                  question:    &str,
                  top_k:       usize,
                  max_tokens:  usize) -> Result<String, String>
{
    // Step 1: Load embed model and index all documents
    let mut embedder = KillerEmbedder::load(embed_model)?;
    let mut store    = VectorStore::new();
    embedder.index_documents(&mut store, docs, 512, 64);

    eprintln!("[KillerRAG] Indexed {} chunks total.", store.len());

    // Step 2: Embed the question and retrieve top-K relevant chunks
    let relevant = embedder.retrieve(&store, question, top_k);

    if relevant.is_empty() {
        eprintln!("[KillerRAG] No relevant chunks found — falling back to direct question.");
    } else {
        eprintln!("[KillerRAG] Retrieved {} chunks.", relevant.len());
    }

    // Step 3: Build augmented prompt
    let prompt = build_rag_prompt(&relevant, question);
    eprintln!("[KillerRAG] Prompt length: {} chars", prompt.len());

    // Step 4: Generate answer with the LLM
    let mut llm = KillerInference::load(llm_model)?;
    use super::sampler::GenerateConfig;
    let answer = llm.generate(&prompt, &GenerateConfig::new(max_tokens));

    Ok(answer)
}

/// RAG with a single model doing both embedding and generation.
/// Lower quality embeddings than a dedicated model but zero extra setup.
pub fn killer_rag_single_model(model:      &str,
                                docs:      &[(&str, Option<&str>)],
                                question:  &str,
                                top_k:     usize,
                                max_tokens: usize) -> Result<String, String>
{
    killer_rag(model, model, docs, question, top_k, max_tokens)
}
