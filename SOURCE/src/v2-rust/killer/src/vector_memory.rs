// ===============================================================================
// NOVA GALAXY ENGINE v1 — Vector Memory
// KhLM stores/retrieves past results via TF-IDF embeddings — gets smarter over time
//
// Architecture:
//   vmem_store(key, text)       → store text with key + TF-IDF vector
//   vmem_recall(key)            → get exact entry by key
//   vmem_search(query, topk)    → cosine-similarity retrieval (top-K results)
//   vmem_forget(key)            → remove entry
//   vmem_list()                 → list all stored keys
//   vmem_stats()                → memory stats (entries, vocab size, avg vector len)
//   vmem_clear()                → wipe all memory
//
// KhLM Integration:
//   • khlm_polyglot auto-stores every answer → learns from past queries
//   • Before firing LLM/Ghost-108, vector search retrieves relevant past answers
//   • If similarity > 0.72 threshold → return cached answer (no LLM call needed)
//
// Implementation: TF-IDF word vectors + cosine similarity (pure Rust, no deps)
//   1. Tokenize text → word frequencies → TF-IDF using global IDF weights
//   2. Similarity = dot(a, b) / (|a| * |b|)
//   3. Global vocabulary grows with each store() call
//
// Zero external crates — pure std
// ===============================================================================

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use crate::value::Value;

// --- Types --------------------------------------------------------------------

/// A stored memory entry.
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// User-supplied key (e.g. "go_unused_var_fix")
    pub key: String,
    /// Original text
    pub text: String,
    /// TF-IDF vector: word_index → tfidf_weight
    pub vector: HashMap<usize, f64>,
    /// L2 norm of vector (precomputed for fast cosine)
    pub norm: f64,
    /// Timestamp (millis since epoch)
    pub stored_at: u128,
    /// Number of times this entry has been retrieved
    pub hits: u64,
}

/// Top-K search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub key: String,
    pub text: String,
    pub score: f64,
    pub hits: u64,
}

/// Global vector memory state.
pub struct VectorMemory {
    /// All stored entries, keyed by user key
    entries: HashMap<String, MemoryEntry>,
    /// Global vocabulary: word → index
    vocab: HashMap<String, usize>,
    /// Document frequency per word: vocab_index → doc_count
    doc_freq: HashMap<usize, usize>,
    /// Total documents stored (for IDF)
    total_docs: usize,
    /// Retrieval similarity threshold (0.0 - 1.0)
    pub similarity_threshold: f64,
}

// --- Global Singleton ---------------------------------------------------------

static VMEM: OnceLock<Mutex<VectorMemory>> = OnceLock::new();

pub fn vmem() -> &'static Mutex<VectorMemory> {
    VMEM.get_or_init(|| Mutex::new(VectorMemory::new()))
}

// --- Implementation -----------------------------------------------------------

impl VectorMemory {
    pub fn new() -> Self {
        VectorMemory {
            entries: HashMap::new(),
            vocab: HashMap::new(),
            doc_freq: HashMap::new(),
            total_docs: 0,
            similarity_threshold: 0.72,
        }
    }

    /// Tokenize text into lowercase words (removes punctuation, stop words).
    fn tokenize(text: &str) -> Vec<String> {
        static STOP_WORDS: &[&str] = &[
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to",
            "for", "of", "with", "is", "are", "was", "were", "be", "been",
            "have", "has", "had", "do", "does", "did", "not", "by", "from",
            "this", "that", "it", "its", "as", "if", "use", "can", "will",
        ];
        text.split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 2)
            .map(|w| w.to_lowercase())
            .filter(|w| !STOP_WORDS.contains(&w.as_str()))
            .collect()
    }

    /// Compute term-frequency map for a token list.
    fn term_freq(tokens: &[String]) -> HashMap<String, f64> {
        let mut freq: HashMap<String, f64> = HashMap::new();
        for t in tokens {
            *freq.entry(t.clone()).or_insert(0.0) += 1.0;
        }
        let total = tokens.len().max(1) as f64;
        freq.values_mut().for_each(|v| *v /= total);
        freq
    }

    /// Get or create vocab index for a word.
    fn word_index(&mut self, word: &str) -> usize {
        let next = self.vocab.len();
        *self.vocab.entry(word.to_string()).or_insert(next)
    }

    /// Compute IDF for a word index: log((N+1) / (df+1)) + 1
    fn idf(&self, word_idx: usize) -> f64 {
        let df = *self.doc_freq.get(&word_idx).unwrap_or(&0) as f64;
        let n = self.total_docs.max(1) as f64;
        ((n + 1.0) / (df + 1.0)).ln() + 1.0
    }

    /// Build TF-IDF vector for a token list (using CURRENT vocabulary+IDF).
    fn build_vector(&mut self, tokens: &[String]) -> (HashMap<usize, f64>, f64) {
        let tf = Self::term_freq(tokens);
        let mut vec_map: HashMap<usize, f64> = HashMap::new();

        // First pass: assign indices and accumulate doc_freq updates
        let mut word_indices: Vec<(usize, f64)> = Vec::new();
        for (word, &tf_val) in &tf {
            let idx = self.word_index(word);
            word_indices.push((idx, tf_val));
        }

        // Compute TF-IDF weights
        for (idx, tf_val) in &word_indices {
            let idf = self.idf(*idx);
            vec_map.insert(*idx, tf_val * idf);
        }

        // L2 norm
        let norm = (vec_map.values().map(|v| v * v).sum::<f64>()).sqrt();
        (vec_map, norm.max(1e-10))
    }

    /// Store a text entry. Rebuilds existing entry if key already exists.
    pub fn store(&mut self, key: &str, text: &str) {
        let tokens = Self::tokenize(text);
        if tokens.is_empty() { return; }

        // Register new words in vocabulary
        let tf = Self::term_freq(&tokens);
        for word in tf.keys() {
            let idx = self.word_index(word);
            // Increment doc_freq only if NEW entry (not update)
            if !self.entries.contains_key(key) {
                *self.doc_freq.entry(idx).or_insert(0) += 1;
            }
        }

        if !self.entries.contains_key(key) {
            self.total_docs += 1;
        }

        let (vector, norm) = self.build_vector(&tokens);
        let stored_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        self.entries.insert(key.to_string(), MemoryEntry {
            key: key.to_string(),
            text: text.to_string(),
            vector,
            norm,
            stored_at,
            hits: 0,
        });
    }

    /// Recall exact entry by key.
    pub fn recall(&mut self, key: &str) -> Option<String> {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.hits += 1;
            Some(entry.text.clone())
        } else {
            None
        }
    }

    /// Semantic search: find top-K entries most similar to query text.
    pub fn search(&mut self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let tokens = Self::tokenize(query);
        if tokens.is_empty() || self.entries.is_empty() {
            return Vec::new();
        }

        // Build query vector using current vocab (no new words added for queries)
        let tf = Self::term_freq(&tokens);
        let mut q_vec: HashMap<usize, f64> = HashMap::new();
        for (word, &tf_val) in &tf {
            if let Some(&idx) = self.vocab.get(word) {
                let idf = self.idf(idx);
                q_vec.insert(idx, tf_val * idf);
            }
        }
        let q_norm = (q_vec.values().map(|v| v * v).sum::<f64>()).sqrt().max(1e-10);

        // Score all entries via cosine similarity
        let mut scores: Vec<(String, f64)> = self.entries
            .values()
            .map(|entry| {
                let dot: f64 = q_vec.iter()
                    .filter_map(|(idx, qw)| entry.vector.get(idx).map(|ew| qw * ew))
                    .sum();
                let sim = dot / (q_norm * entry.norm);
                (entry.key.clone(), sim)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let k = top_k.min(scores.len());
        let keys_to_bump: Vec<String> = scores[..k].iter()
            .filter(|(_, s)| *s > 0.0)
            .map(|(k, _)| k.clone())
            .collect();

        let mut results = Vec::new();
        for (key, score) in &scores[..k] {
            if *score <= 0.0 { continue; }
            if let Some(entry) = self.entries.get(key) {
                results.push(SearchResult {
                    key: key.clone(),
                    text: entry.text.clone(),
                    score: *score,
                    hits: entry.hits,
                });
            }
        }

        // Bump hit counters
        for key in keys_to_bump {
            if let Some(e) = self.entries.get_mut(&key) { e.hits += 1; }
        }

        results
    }

    /// Remove an entry by key.
    pub fn forget(&mut self, key: &str) -> bool {
        if self.entries.remove(key).is_some() {
            self.total_docs = self.total_docs.saturating_sub(1);
            true
        } else {
            false
        }
    }

    /// Return all stored keys.
    pub fn list_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.entries.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Return stats as formatted string.
    pub fn stats(&self) -> String {
        let avg_tokens = if self.entries.is_empty() {
            0.0
        } else {
            self.entries.values().map(|e| e.vector.len()).sum::<usize>() as f64
                / self.entries.len() as f64
        };
        format!(
            "Vector Memory Stats\n  Entries:    {}\n  Vocabulary: {} words\n  Avg vec dim: {:.0} features\n  Total hits: {}\n  Sim threshold: {:.2}\n",
            self.entries.len(),
            self.vocab.len(),
            avg_tokens,
            self.entries.values().map(|e| e.hits).sum::<u64>(),
            self.similarity_threshold,
        )
    }

    /// Clear all memory.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.vocab.clear();
        self.doc_freq.clear();
        self.total_docs = 0;
    }
}

// --- KhLM Integration ---------------------------------------------------------

/// Auto-store a KhLM answer into vector memory for future recall.
/// Called by khlm_polyglot after every successful answer.
pub fn auto_store_khlm_answer(operation: &str, lang: &str, query: &str, answer: &str) {
    let key = format!("khlm:{}:{}:{}", operation, lang,
        &query.chars().take(40).collect::<String>().replace(' ', "_"));
    let combined = format!("{} {} {}", query, answer, lang);
    if let Ok(mut mem) = vmem().lock() {
        mem.store(&key, &combined);
    }
}

/// Search vector memory for a relevant past answer before calling LLM.
/// Returns `Some(answer)` if similarity >= threshold, else `None`.
pub fn recall_for_khlm(operation: &str, lang: &str, query: &str) -> Option<String> {
    let search_text = format!("{} {} {}", operation, lang, query);
    if let Ok(mut mem) = vmem().lock() {
        let threshold = mem.similarity_threshold;
        let results = mem.search(&search_text, 1);
        if let Some(top) = results.first() {
            if top.score >= threshold {
                return Some(format!(
                    "[vmem sim {:.0}%] {}",
                    top.score * 100.0,
                    top.text
                ));
            }
        }
    }
    None
}

// --- Builtin dispatch functions -----------------------------------------------

use crate::error::VmError;

/// vmem_store(key, text) → "Stored: key"
pub fn builtin_vmem_store(args: &[Value]) -> Result<Value, VmError> {
    let key = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: vmem_store(key, text) — key must be a string".to_string())),
    };
    let text = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: vmem_store(key, text) — text must be a string".to_string())),
    };
    if let Ok(mut mem) = vmem().lock() {
        mem.store(&key, &text);
        Ok(Value::Str(format!("Vector Memory: stored '{}'", key)))
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}

/// vmem_recall(key) → String (text) or "Not found: key"
pub fn builtin_vmem_recall(args: &[Value]) -> Result<Value, VmError> {
    let key = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: vmem_recall(key) — key must be a string".to_string())),
    };
    if let Ok(mut mem) = vmem().lock() {
        match mem.recall(&key) {
            Some(text) => Ok(Value::Str(text)),
            None => Ok(Value::Str(format!("Vector Memory: key '{}' not found", key))),
        }
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}

/// vmem_search(query, top_k?) → String (formatted results)
pub fn builtin_vmem_search(args: &[Value]) -> Result<Value, VmError> {
    let query = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: vmem_search(query) — query must be a string".to_string())),
    };
    let top_k = match args.get(1) {
        Some(Value::Number(n)) => *n as usize,
        _ => 3,
    };
    if let Ok(mut mem) = vmem().lock() {
        let results = mem.search(&query, top_k);
        if results.is_empty() {
            return Ok(Value::Str("Vector Memory: no results found".to_string()));
        }
        let mut out = format!("Vector Memory Search (top {} results):\n", results.len());
        for (i, r) in results.iter().enumerate() {
            let preview = if r.text.len() > 120 {
                format!("{}...", &r.text[..120])
            } else {
                r.text.clone()
            };
            out.push_str(&format!("  {}. [sim {:.0}%] {} (hits: {})\n     {}\n",
                i + 1, r.score * 100.0, r.key, r.hits, preview));
        }
        Ok(Value::Str(out))
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}

/// vmem_forget(key) → "Forgot: key" or "Not found: key"
pub fn builtin_vmem_forget(args: &[Value]) -> Result<Value, VmError> {
    let key = match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Str("Error: vmem_forget(key) — key must be a string".to_string())),
    };
    if let Ok(mut mem) = vmem().lock() {
        if mem.forget(&key) {
            Ok(Value::Str(format!("Vector Memory: forgot '{}'", key)))
        } else {
            Ok(Value::Str(format!("Vector Memory: key '{}' not found", key)))
        }
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}

/// vmem_list() → String (all keys, one per line)
pub fn builtin_vmem_list(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    if let Ok(mem) = vmem().lock() {
        let keys = mem.list_keys();
        if keys.is_empty() {
            Ok(Value::Str("Vector Memory: empty (no entries stored)".to_string()))
        } else {
            Ok(Value::Str(format!("Vector Memory keys ({}):\n  {}", keys.len(), keys.join("\n  "))))
        }
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}

/// vmem_stats() → String (formatted stats)
pub fn builtin_vmem_stats(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    if let Ok(mem) = vmem().lock() {
        Ok(Value::Str(mem.stats()))
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}

/// vmem_clear() → "Vector Memory cleared"
pub fn builtin_vmem_clear(args: &[Value]) -> Result<Value, VmError> {
    let _ = args;
    if let Ok(mut mem) = vmem().lock() {
        mem.clear();
        Ok(Value::Str("Vector Memory: cleared all entries".to_string()))
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}

/// vmem_set_threshold(f64) → "Threshold set to N%"
pub fn builtin_vmem_set_threshold(args: &[Value]) -> Result<Value, VmError> {
    let t = match args.first() {
        Some(Value::Number(n)) => *n,
        _ => return Ok(Value::Str("Error: vmem_set_threshold(0.0..1.0) — requires a float".to_string())),
    };
    if !(0.0..=1.0).contains(&t) {
        return Ok(Value::Str(format!("Error: threshold must be 0.0–1.0, got {}", t)));
    }
    if let Ok(mut mem) = vmem().lock() {
        mem.similarity_threshold = t;
        Ok(Value::Str(format!("Vector Memory: threshold set to {:.0}%", t * 100.0)))
    } else {
        Ok(Value::Str("Error: vector memory lock failed".to_string()))
    }
}
