// inference/rlm.rs — Reasoning Language Model (RLM) support
//
// Handles models that produce an internal reasoning scratchpad before their answer.
// Used by DeepSeek-R1, QwQ-32B, and any model trained with chain-of-thought RL.
//
// These models emit text between <think>...</think> tags (the "reasoning trace"),
// then give the final answer.  This module parses that structure and exposes
// thinking + answer as separate fields.
//
// Compatible RLM models (GGUF format, free from HuggingFace):
//   • DeepSeek-R1-Distill-Qwen-1.5B-Q4_K_M.gguf  ~1.1 GB  (cheapest)
//   • DeepSeek-R1-Distill-Qwen-7B-Q4_K_M.gguf    ~4.7 GB  (strong)
//   • QwQ-32B-Q4_K_M.gguf                          ~20 GB  (best open reasoning)
//
// Usage:
//   let resp = killer_think("deepseek-r1.gguf", "What is 17 × 23?", 1024)?;
//   println!("Thinking:\n{}", resp.thinking);
//   println!("\nAnswer: {}", resp.answer);

use super::engine::KillerInference;
use super::gguf::GgufFile;
use super::sampler::GenerateConfig;

// Qwen2 / ChatML control token IDs (fixed across all Qwen2/Qwen2.5 models):
// 151643 = <|endoftext|>  (EOS from the tokenizer's perspective)
// 151644 = <|im_start|>   (start-of-turn marker)
// 151645 = <|im_end|>     (end-of-turn marker — model outputs this after each turn)
const QWEN2_IM_END:   u32 = 151645;
const QWEN2_IM_START: u32 = 151644;

/// DeepSeek-R1 / QwQ-style reasoning delimiter tags (must stay in sync with prompts below).
pub(crate) const RLM_THINK_OPEN: &str = "<redacted_thinking>";
pub(crate) const RLM_THINK_CLOSE: &str = "</think>";

// --- Response type ------------------------------------------------------------

/// Structured output from a Reasoning Language Model.
#[derive(Debug, Clone)]
pub struct RlmResponse {
    /// The model's internal reasoning scratchpad — everything between <think>...</think>.
    /// Empty for standard (non-RLM) models.
    pub thinking: String,

    /// The final answer produced after reasoning.
    pub answer: String,

    /// Raw unprocessed model output (thinking + answer combined).
    pub raw: String,
}

impl RlmResponse {
    /// Human-readable display: shows thinking trace then the answer.
    pub fn display(&self) -> String {
        if self.thinking.is_empty() {
            self.answer.clone()
        } else {
            format!(
                "+-- Thinking ---------------------------------------\n\
                 {}\n\
                 +---------------------------------------------------\n\n\
                 {}",
                self.thinking.trim(),
                self.answer.trim()
            )
        }
    }

    /// Just the answer — no thinking trace shown.
    pub fn answer_only(&self) -> &str {
        self.answer.trim_matches(|c: char| c.is_whitespace())
    }

    /// Character count of the thinking trace.
    pub fn thinking_len(&self) -> usize { self.thinking.len() }
}

// --- Thinking-block parser ----------------------------------------------------

/// Parse raw model output into thinking + answer blocks.
///
/// Handles:
/// - `<think>...</think>`         (DeepSeek-R1, QwQ)
/// - `<|thinking|>...</|thinking|>`  (alternative token variant)
/// - No tags → entire output goes to `answer`, `thinking` is empty
pub fn split_thinking(raw: &str) -> RlmResponse {
    const ALT_OPEN: &str = "<|thinking|>";
    const ALT_CLOSE: &str = "</|thinking|>";

    // Primary format: <redacted_thinking> ... </redacted_thinking>
    if let Some(start) = raw.find(RLM_THINK_OPEN) {
        let body = &raw[start + RLM_THINK_OPEN.len()..];
        if let Some(end) = body.find(RLM_THINK_CLOSE) {
            return RlmResponse {
                thinking: body[..end].trim().to_string(),
                answer:   body[end + RLM_THINK_CLOSE.len()..].trim().to_string(),
                raw:      raw.to_string(),
            };
        }
        // Model is still mid-reasoning (no closing tag generated yet)
        return RlmResponse {
            thinking: body.trim().to_string(),
            answer:   String::new(),
            raw:      raw.to_string(),
        };
    }

    // Alternative format: <|thinking|> ... </|thinking|>
    if let Some(start) = raw.find(ALT_OPEN) {
        let body = &raw[start + ALT_OPEN.len()..];
        if let Some(end) = body.find(ALT_CLOSE) {
            return RlmResponse {
                thinking: body[..end].trim().to_string(),
                answer:   body[end + ALT_CLOSE.len()..].trim().to_string(),
                raw:      raw.to_string(),
            };
        }
    }

    // Standard model — no thinking tags
    RlmResponse {
        thinking: String::new(),
        answer:   raw.trim().to_string(),
        raw:      raw.to_string(),
    }
}

// --- RLM-specific chat templates ---------------------------------------------

/// DeepSeek-R1 prompt template.
///
/// Primes the model to begin its thinking block immediately after the assistant
/// header — the model will close the `<think>` block itself before answering.
///
/// Works with: DeepSeek-R1-Distill-* models (Qwen2 architecture + ChatML format)
///
/// NOTE: We skip the system turn — R1-Distill 7B/8B models were fine-tuned
/// without system prompts in most benchmarks. Including one causes the model
/// to reason *about* the system prompt text rather than the user's question.
/// Pass `system: Some(...)` only when you need domain-specific instructions.
pub fn deepseek_r1_template(user_msg: &str, system: Option<&str>) -> String {
    if let Some(sys) = system {
        // Explicit system prompt requested: include it
        format!(
            "<|im_start|>system\n{sys}<|im_end|>\n\
             <|im_start|>user\n{user_msg}<|im_end|>\n\
             <|im_start|>assistant\n<think>\n"
        )
    } else {
        // Default: no system turn — cleaner reasoning for R1-Distill models
        format!(
            "<|im_start|>user\n{user_msg}<|im_end|>\n\
             <|im_start|>assistant\n<think>\n"
        )
    }
}

/// QwQ-32B prompt template (Qwen-based reasoning model by Alibaba).
pub fn qwq_template(user_msg: &str, system: Option<&str>) -> String {
    let sys = system.unwrap_or(
        "You are a helpful and harmless assistant that thinks carefully \
         before answering."
    );
    format!(
        "<|im_start|>system\n{sys}<|im_end|>\n\
         <|im_start|>user\n{user_msg}<|im_end|>\n\
         <|im_start|>assistant\n"
    )
}

/// Auto-select the right RLM template based on the model's architecture string.
pub fn rlm_template_auto(model_path: &str, user_msg: &str, system: Option<&str>)
    -> Result<String, String>
{
    let gguf = GgufFile::open(model_path)?;
    let name = gguf.meta_str("general.name").unwrap_or_default().to_lowercase();
    let arch = gguf.meta_str("general.architecture").unwrap_or_default().to_lowercase();

    let prompt = if name.contains("deepseek") || name.contains("r1") {
        deepseek_r1_template(user_msg, system)
    } else if name.contains("qwq") {
        qwq_template(user_msg, system)
    } else if arch.contains("qwen") {
        // Generic Qwen reasoning model
        deepseek_r1_template(user_msg, system)
    } else {
        // Fallback: standard [INST] format
        format!("<s>[INST] {user_msg} [/INST]")
    };

    Ok(prompt)
}

// --- Public API ---------------------------------------------------------------

/// Run a reasoning model, return structured thinking + answer.
///
/// Automatically selects the correct chat template from the model's metadata.
///
/// # Example
/// ```no_run
/// let resp = killer_think("deepseek-r1-1.5b.gguf", "Solve: 2x + 5 = 13", 1024)?;
/// println!("Thinking:\n{}", resp.thinking);
/// println!("Answer: {}", resp.answer);
/// ```
pub fn killer_think(model_path: &str, question: &str, max_tokens: usize)
    -> Result<RlmResponse, String>
{
    let prompt = rlm_template_auto(model_path, question, None)?;
    let mut engine = KillerInference::load(model_path)?;
    // Stop at <|im_end|> and <|im_start|> — ChatML control tokens that mark the
    // end of the assistant's turn.  Without these, the model generates garbage
    // after closing </think> (it tries to start a new conversation turn).
    let cfg = GenerateConfig::new(max_tokens)
        .with_stop_tokens(vec![QWEN2_IM_END, QWEN2_IM_START]);
    let raw = engine.generate(&prompt, &cfg);
    Ok(split_thinking(&raw))
}

/// Run a reasoning model with a custom system prompt.
pub fn killer_think_with_system(model_path: &str, question: &str,
                                 system: &str, max_tokens: usize)
    -> Result<RlmResponse, String>
{
    let prompt = rlm_template_auto(model_path, question, Some(system))?;
    let mut engine = KillerInference::load(model_path)?;
    let cfg = GenerateConfig::new(max_tokens)
        .with_stop_tokens(vec![QWEN2_IM_END, QWEN2_IM_START]);
    let raw = engine.generate(&prompt, &cfg);
    Ok(split_thinking(&raw))
}

/// Turn any standard LLM into a pseudo-RLM via a chain-of-thought system prompt.
///
/// Works with Qwen, TinyLlama, Mistral, Llama — any model you already have.
/// No special reasoning-trained model needed.
///
/// Uses the model's NATIVE chat template (auto-detected from GGUF metadata).
/// The system prompt instructs the model to reason inside <think>...</think> tags
/// then give a clean final answer.  Does NOT prefill <think> (unlike RLM models)
/// so the model generates the tags itself without crashing.
///
/// The `split_thinking()` parser then extracts thinking vs answer.
///
/// # Example
/// ```no_run
/// let resp = killer_llm_as_rlm("qwen2.5-0.5b.gguf", "What is the capital of France?", 512)?;
/// println!("Thinking:\n{}", resp.thinking);
/// println!("Answer: {}", resp.answer);
/// ```
pub fn killer_llm_as_rlm(model_path: &str, question: &str, max_tokens: usize)
    -> Result<RlmResponse, String>
{
    let system = "You are a careful reasoning assistant. \
        Before giving your final answer, think through the problem step by step. \
        Write your entire reasoning process inside <think> and </think> tags. \
        After the closing </think> tag, write only your final answer — concise and direct. \
        Always use exactly this format: <think>your reasoning here</think> final answer here.";

    // Use the model's NATIVE chat template (ChatML for Qwen, Zephyr for TinyLlama).
    // This avoids the RLM template that prefills <think> and crashes non-reasoning models.
    let raw = super::killer_chat_auto(model_path, question, Some(system), max_tokens)?;
    Ok(split_thinking(&raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_redacted_thinking_parses_answer() {
        let raw = format!(
            "preamble {}step1{}final reply",
            RLM_THINK_OPEN,
            RLM_THINK_CLOSE
        );
        let r = split_thinking(&raw);
        assert_eq!(r.thinking, "step1");
        assert_eq!(r.answer, "final reply");
    }

    #[test]
    fn split_alt_thinking_tags() {
        let raw = "<|thinking|>work</|thinking|>Paris";
        let r = split_thinking(raw);
        assert_eq!(r.thinking, "work");
        assert_eq!(r.answer, "Paris");
    }

    #[test]
    fn split_no_tags_whole_answer() {
        let r = split_thinking("  hello  ");
        assert!(r.thinking.is_empty());
        assert_eq!(r.answer, "hello");
    }
}
