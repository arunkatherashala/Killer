// inference/mod.rs — Killer Native LLM Inference Engine
//
// Killer's own transformer inference — loads .gguf weights, runs BPE tokenization,
// executes the forward pass, samples the next token. No Ollama. No cloud. No deps.
//
// KV cache compression (same *goal* as Google TurboQuant — smaller KV RAM; training-free):
//   • Set env `KILLER_KV_Q8=1` (or `true` / `yes` / `on`) for int8 K/V + per-step f32 scales (~4× KV vs f32).
//   • Or `GenerateConfig::with_kv_q8(true)` with `killer_generate`. Full `embed_text` stays f32 KV.
//
// Supported model families (all use the same LLaMA tensor naming convention):
//   • TinyLlama 1.1B  (tinyllama-1.1b-chat-v1.0.Q8_0.gguf  — ~1.2 GB)
//   • Phi-3 mini      (Phi-3-mini-4k-instruct-q4.gguf       — ~2.2 GB)
//   • Mistral 7B      (Mistral-7B-Instruct-v0.3.Q4_K_M.gguf — ~4.4 GB)
//   • Gemma 2B        (gemma-2b-it-q4_k_m.gguf              — ~1.7 GB)
//   • LLaMA 3 8B      (Meta-Llama-3-8B-Instruct.Q4_K_M.gguf — ~4.9 GB)
//
// RLM (Reasoning Language Models):
//   • DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf  ~1.1 GB
//   • DeepSeek-R1-Distill-Qwen-7B-Q4_K_M.gguf    ~4.7 GB
//   • QwQ-32B-Q4_K_M.gguf                          ~20 GB
//
// Embedding models (for RAG):
//   • nomic-embed-text-v1.5.Q4_K_M.gguf   ~100 MB  (recommended)
//   • all-MiniLM-L6-v2-Q4_K_M.gguf         ~20 MB  (tiny + fast)
//   • bge-base-en-v1.5.Q4_K_M.gguf        ~120 MB  (strong retrieval)
//
// Quick start:
//   Download any of the above from https://huggingface.co/TheBloke (or similar).
//   Run: killer-native --model path/to/model.gguf "Your question here"
//   RLM: killer-native --think path/to/deepseek-r1.gguf "Solve: 2x+5=13"

pub mod gguf;
pub mod quant;
pub mod tokenizer;
pub mod sampler;
pub mod engine;
pub mod rlm;            // Reasoning Language Model — <think>...</think> support
pub mod vector_store;   // In-memory vector store for RAG
pub mod embeddings;     // Text embedding engine + RAG pipeline
pub mod model_registry; // ~/.killer/models/ path resolution + install + list

// --- Re-exports for convenience -----------------------------------------------

pub use gguf::{GgufFile, GgufValue, GgmlType, TensorInfo};
pub use tokenizer::KillerTokenizer;
pub use sampler::{GenerateConfig, SamplingStrategy};
pub use engine::{KillerInference, ModelConfig, ArchType};
pub use rlm::{RlmResponse, split_thinking, killer_think, killer_think_with_system};
pub use vector_store::{VectorStore, Chunk, cosine_similarity, chunk_text, build_rag_prompt};
pub use embeddings::{KillerEmbedder, killer_rag, killer_rag_single_model};
pub use model_registry::{
    resolve_model_path, list_models, install_model, killer_models_dir, migrate_local_models,
    pick_default_gguf_for_khlm,
};

// --- Top-level convenience functions -----------------------------------------

/// Ask a model a question, get a string answer back.
///
/// Loads the model on every call — use [`KillerInference::load`] directly if you
/// want to ask multiple questions without reloading.
///
/// # Example (Killer CLI)
/// ```no_run
/// // killer-native --model tinyllama.gguf "What is 2+2?"
/// let answer = killer_ask("tinyllama.gguf", "What is 2+2?", 128).unwrap();
/// println!("{}", answer);  // "4"
/// ```
pub fn killer_ask(model_path: &str, prompt: &str, max_tokens: usize) -> Result<String, String> {
    let mut engine = KillerInference::load(model_path)?;
    Ok(engine.ask(prompt, max_tokens))
}

/// Full-control generation with a custom [`GenerateConfig`].
pub fn killer_generate(model_path: &str, prompt: &str, cfg: &GenerateConfig) -> Result<String, String> {
    let mut engine = KillerInference::load(model_path)?;
    Ok(engine.generate(prompt, cfg))
}

/// Inspect basic model information from a GGUF file without loading weights.
/// Returns a human-readable summary string.
pub fn killer_model_info(model_path: &str) -> Result<String, String> {
    let gguf = GgufFile::open(model_path)?;

    let arch    = gguf.meta_str("general.architecture").unwrap_or_else(|| "unknown".to_string());
    let name    = gguf.meta_str("general.name").unwrap_or_else(|| model_path.to_string());
    let quant   = gguf.meta_str("general.quantization_version").unwrap_or_default();
    let ctx     = gguf.meta_u64(&format!("{}.context_length", arch)).unwrap_or(0);
    let embd    = gguf.meta_u64(&format!("{}.embedding_length", arch)).unwrap_or(0);
    let layers  = gguf.meta_u64(&format!("{}.block_count", arch)).unwrap_or(0);
    let heads   = gguf.meta_u64(&format!("{}.attention.head_count", arch)).unwrap_or(0);
    // vocab_size: try metadata key, then fall back to counting the token array
    let vocab = gguf.meta_u64(&format!("{}.vocab_size", arch))
        .unwrap_or_else(|| gguf.meta_array_strings("tokenizer.ggml.tokens").len() as u64);

    Ok(format!(
        "Model : {}\nArch  : {}\nQuant : {}\nLayers: {}  Heads: {}  Embd: {}  Vocab: {}  Ctx: {}  Tensors: {}",
        name, arch, quant, layers, heads, embd, vocab, ctx, gguf.n_tensors
    ))
}

// --- Chat templates -----------------------------------------------------------
//
// Each model family expects its own prompt wrapper format.
// Without the right template, the model outputs garbage.
//
// Usage:
//   let prompt = apply_chat_template("qwen2", "What is 2+2?", None);
//   let answer = killer_ask("qwen2.5-0.5b.gguf", &prompt, 256)?;

/// Wrap a user message in the model's expected chat format.
///
/// `arch` matches `general.architecture` from the GGUF metadata —
/// call `killer_model_info()` first if you're not sure.
///
/// `system` — optional system/instruction prompt (e.g. "You are a helpful assistant.").
pub fn apply_chat_template(arch: &str, user_msg: &str, system: Option<&str>) -> String {
    let sys = system.unwrap_or("You are a helpful assistant.");
    match arch {
        // Qwen2 / Qwen2.5 — ChatML format
        "qwen2" | "qwen" | "qwen2_5" => format!(
            "<|im_start|>system\n{sys}<|im_end|>\n\
             <|im_start|>user\n{user_msg}<|im_end|>\n\
             <|im_start|>assistant\n"
        ),
        // LLaMA 3 — header tag format
        "llama3" => format!(
            "<|begin_of_text|>\
             <|start_header_id|>system<|end_header_id|>\n\n{sys}<|eot_id|>\
             <|start_header_id|>user<|end_header_id|>\n\n{user_msg}<|eot_id|>\
             <|start_header_id|>assistant<|end_header_id|>\n\n"
        ),
        // Phi-3
        "phi" | "phi2" | "phi3" => format!(
            "<|system|>\n{sys}<|end|>\n\
             <|user|>\n{user_msg}<|end|>\n\
             <|assistant|>\n"
        ),
        // Gemma
        "gemma" | "gemma2" => format!(
            "<bos><start_of_turn>user\n{user_msg}<end_of_turn>\n\
             <start_of_turn>model\n"
        ),
        // TinyLlama / Zephyr — <|user|> tag format (detected via killer_chat_auto)
        "zephyr" | "tinyllama" => format!(
            "<|system|>\n{sys}</s>\n\
             <|user|>\n{user_msg}</s>\n\
             <|assistant|>\n"
        ),
        // Mistral / LLaMA 2 — [INST] format
        _ => format!("<s>[INST] {user_msg} [/INST]"),
    }
}

/// High-level chat: applies the correct template then calls the model.
///
/// # Example
/// ```no_run
/// let answer = killer_chat("qwen2.5-0.5b.Q4_K_M.gguf", "qwen2",
///                          "What is the capital of France?", None, 256).unwrap();
/// println!("{}", answer);  // "Paris"
/// ```
pub fn killer_chat(model_path: &str, arch: &str, user_msg: &str,
                   system: Option<&str>, max_tokens: usize) -> Result<String, String> {
    let prompt = apply_chat_template(arch, user_msg, system);
    killer_ask(model_path, &prompt, max_tokens)
}

/// Auto-detect arch from the model file, apply template, then generate.
pub fn killer_chat_auto(model_path: &str, user_msg: &str,
                        system: Option<&str>, max_tokens: usize) -> Result<String, String> {
    let gguf = GgufFile::open(model_path)?;
    let arch  = gguf.meta_str("general.architecture").unwrap_or_else(|| "llama".to_string());
    // Detect Zephyr-style template (TinyLlama, Zephyr, StableLM-Zephyr, etc.)
    // by reading the embedded chat template from metadata.
    let chat_tmpl = gguf.meta_str("tokenizer.chat_template").unwrap_or_default();
    let effective_arch = if arch == "llama" && chat_tmpl.contains("<|user|>") {
        "zephyr".to_string()
    } else {
        arch
    };
    drop(gguf);  // release file handle before loading weights
    let prompt = apply_chat_template(&effective_arch, user_msg, system);
    killer_ask(model_path, &prompt, max_tokens)
}

// --- RLM convenience wrappers -------------------------------------------------

/// Run a reasoning model (DeepSeek-R1, QwQ, etc.) and return thinking + answer.
///
/// Equivalent to calling `rlm::killer_think()` directly.
///
/// # Example
/// ```no_run
/// let resp = killer_think_rlm("deepseek-r1-1.5b.gguf", "What is 17 × 23?", 1024)?;
/// println!("Thinking:\n{}", resp.thinking);
/// println!("Answer: {}", resp.answer);
/// ```
pub fn killer_think_rlm(model_path: &str, question: &str, max_tokens: usize)
    -> Result<RlmResponse, String>
{
    killer_think(model_path, question, max_tokens)
}

/// Turn any standard LLM into a pseudo-RLM using a chain-of-thought system prompt.
///
/// Works with Qwen, TinyLlama, Mistral, Llama — any GGUF model you already have.
/// No DeepSeek-R1 or dedicated reasoning model needed.
///
/// The model is instructed to output reasoning inside <think>...</think> tags,
/// then a clean final answer.  The result is parsed into `thinking` + `answer`.
///
/// # Example
/// ```no_run
/// let resp = killer_llm_as_rlm("qwen2.5-0.5b.gguf", "What is 2 + 2?", 512)?;
/// println!("Thinking:\n{}", resp.thinking);
/// println!("Answer: {}", resp.answer);
/// ```
pub fn killer_llm_as_rlm(model_path: &str, question: &str, max_tokens: usize)
    -> Result<RlmResponse, String>
{
    rlm::killer_llm_as_rlm(model_path, question, max_tokens)
}

// --- Embedding convenience wrappers ------------------------------------------

/// Embed a single text string into a dense vector using a GGUF model.
///
/// Returns an L2-normalized `Vec<f32>` of length == model's hidden size.
/// Use `cosine_similarity()` to compare two embeddings.
///
/// # Example
/// ```no_run
/// let vec = killer_embed("nomic-embed-text.gguf", "What is a transformer?")?;
/// println!("Dims: {}", vec.len());
/// ```
pub fn killer_embed(model_path: &str, text: &str) -> Result<Vec<f32>, String> {
    let mut engine = KillerInference::load(model_path)?;
    Ok(engine.embed_text(text))
}

/// Full RAG pipeline: index docs → embed question → retrieve → generate answer.
///
/// `embed_model` and `llm_model` can be the same path (single-model RAG)
/// or separate files (dedicated embedder + generation model).
///
/// # Example
/// ```no_run
/// let answer = killer_rag_ask(
///     "nomic-embed.gguf",                               // embed model
///     "tinyllama.gguf",                                 // generation model
///     &[("Killer is a compiled language.", Some("docs"))],
///     "What kind of language is Killer?",
///     3,    // top-k chunks
///     256,  // max answer tokens
/// )?;
/// println!("{}", answer);
/// ```
pub fn killer_rag_ask(embed_model: &str,
                      llm_model:   &str,
                      docs:        &[(&str, Option<&str>)],
                      question:    &str,
                      top_k:       usize,
                      max_tokens:  usize) -> Result<String, String>
{
    killer_rag(embed_model, llm_model, docs, question, top_k, max_tokens)
}
