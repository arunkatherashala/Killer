// inference/sampler.rs — Token sampling strategies
//
// After the transformer produces logits (one score per vocab token),
// the sampler picks which token comes next.

// --- Strategy ----------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum SamplingStrategy {
    /// Always pick the highest-probability token (deterministic, fast).
    Greedy,
    /// Temperature + top-k + top-p (nucleus) sampling — varied, creative output.
    Sample { temperature: f32, top_k: usize, top_p: f32 },
}

impl SamplingStrategy {
    /// Good defaults for chat / instruction following.
    pub fn chat() -> Self {
        SamplingStrategy::Sample { temperature: 0.7, top_k: 40, top_p: 0.9 }
    }
    /// Deterministic greedy decoding.
    pub fn greedy() -> Self { SamplingStrategy::Greedy }
    /// Creative mode (higher temperature).
    pub fn creative() -> Self {
        SamplingStrategy::Sample { temperature: 1.0, top_k: 50, top_p: 0.95 }
    }
}

// --- Generation config --------------------------------------------------------

#[derive(Clone, Debug)]
pub struct GenerateConfig {
    pub max_new_tokens: usize,
    pub sampling:       SamplingStrategy,
    pub stop_tokens:    Vec<u32>,
    /// Store self-attention K/V cache as int8 with one f32 scale per token position per layer
    /// (~4× smaller KV RAM vs f32). Same *goal* as Google TurboQuant-style KV compression; this is a
    /// simpler training-free quant path (not bit-identical to Google's algorithm).
    pub kv_q8: bool,
}

impl GenerateConfig {
    /// Default: chat mode, max 256 new tokens.
    pub fn new(max_tokens: usize) -> Self {
        GenerateConfig {
            max_new_tokens: max_tokens,
            sampling:       SamplingStrategy::chat(),
            stop_tokens:    Vec::new(),
            kv_q8:          false,
        }
    }

    /// Greedy decoding.
    pub fn greedy(max_tokens: usize) -> Self {
        GenerateConfig {
            max_new_tokens: max_tokens,
            sampling:       SamplingStrategy::Greedy,
            stop_tokens:    Vec::new(),
            kv_q8:          false,
        }
    }

    pub fn with_temperature(mut self, t: f32) -> Self {
        self.sampling = SamplingStrategy::Sample { temperature: t, top_k: 40, top_p: 0.9 };
        self
    }

    pub fn with_stop_tokens(mut self, tokens: Vec<u32>) -> Self {
        self.stop_tokens = tokens;
        self
    }

    /// Enable int8 KV cache (see [`GenerateConfig::kv_q8`]). Can also set env `KILLER_KV_Q8=1`.
    pub fn with_kv_q8(mut self, on: bool) -> Self {
        self.kv_q8 = on;
        self
    }
}

// --- Sampling functions -------------------------------------------------------

/// Sample the next token from a logits vector.
/// `step` is used as a deterministic seed (no external RNG dependency).
/// `recent` is a recent-token window used for repetition penalty and LCG mixing.
pub fn sample(logits: &[f32], strategy: &SamplingStrategy, step: usize, recent: &[u32]) -> u32 {
    match strategy {
        SamplingStrategy::Greedy => {
            // Apply light repetition penalty even for greedy to avoid dead loops
            let penalized = apply_rep_penalty(logits, recent, 1.2);
            argmax(&penalized)
        }
        SamplingStrategy::Sample { temperature, top_k, top_p } => {
            let penalized = apply_rep_penalty(logits, recent, 1.3);
            // Mix last token into LCG seed for better variety across the sequence
            let seed = step ^ (recent.last().copied().unwrap_or(0) as usize).wrapping_mul(2654435761);
            sample_nucleus(&penalized, *temperature, *top_k, *top_p, seed)
        }
    }
}

/// Apply repetition penalty: divide logits of recently-seen tokens by `penalty`.
/// penalty > 1.0 discourages repetition (1.2–1.4 is a common range).
fn apply_rep_penalty(logits: &[f32], recent: &[u32], penalty: f32) -> Vec<f32> {
    if recent.is_empty() || penalty <= 1.0 { return logits.to_vec(); }
    let mut out = logits.to_vec();
    for &tok in recent {
        let idx = tok as usize;
        if idx < out.len() {
            // Positive logit → divide (reduce probability); negative → multiply (further reduce)
            if out[idx] > 0.0 { out[idx] /= penalty; } else { out[idx] *= penalty; }
        }
    }
    out
}

/// argmax — token with highest logit (greedy decoding).
pub fn argmax(logits: &[f32]) -> u32 {
    logits.iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as u32)
        .unwrap_or(0)
}

fn sample_nucleus(logits: &[f32], temperature: f32, top_k: usize, top_p: f32, step: usize) -> u32 {
    // 1. Temperature scaling
    let temp = temperature.max(1e-6);
    let max_l = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let exp: Vec<f32> = logits.iter().map(|&l| ((l - max_l) / temp).exp()).collect();
    let sum: f32 = exp.iter().sum();
    if sum == 0.0 { return argmax(logits); }
    let probs: Vec<f32> = exp.iter().map(|&e| e / sum).collect();

    // 2. Sort descending by probability (keep original indices)
    let mut indexed: Vec<(usize, f32)> = probs.into_iter().enumerate().collect();
    indexed.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // 3. Top-k
    let k = top_k.clamp(1, indexed.len());
    let candidates = &indexed[..k];

    // 4. Top-p (nucleus): keep only enough tokens to reach cumulative prob top_p
    let mut cumsum = 0.0f32;
    let mut nucleus_end = 1usize;
    for (i, (_, p)) in candidates.iter().enumerate() {
        cumsum += p;
        nucleus_end = i + 1;
        if cumsum >= top_p { break; }
    }
    let nucleus = &candidates[..nucleus_end];

    // 5. Renormalize nucleus
    let nucleus_total: f32 = nucleus.iter().map(|(_, p)| p).sum();
    if nucleus_total == 0.0 { return nucleus[0].0 as u32; }

    // 6. Sample: pseudo-random via LCG seeded by `step`
    let r = lcg_f32(step) * nucleus_total;
    let mut acc = 0.0f32;
    for (idx, prob) in nucleus {
        acc += prob;
        if acc >= r { return *idx as u32; }
    }
    nucleus.last().map(|(i, _)| *i as u32).unwrap_or(0)
}

/// Deterministic LCG pseudo-random float in [0, 1).
/// No external `rand` crate — entirely self-contained.
fn lcg_f32(seed: usize) -> f32 {
    let x = (seed as u64)
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (x >> 33) as f32 / u32::MAX as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argmax_basic() {
        let logits = vec![0.1f32, 5.0, 0.3, 1.0];
        assert_eq!(argmax(&logits), 1);
    }

    #[test]
    fn test_greedy_sample() {
        let logits = vec![0.1f32, 5.0, 0.3, 1.0];
        assert_eq!(sample(&logits, &SamplingStrategy::Greedy, 0, &[]), 1);
    }

    #[test]
    fn test_nucleus_sample_returns_valid_token() {
        let logits = vec![1.0f32, 2.0, 3.0, 0.5, 0.1];
        let strategy = SamplingStrategy::chat();
        for step in 0..10 {
            let tok = sample(&logits, &strategy, step, &[]);
            assert!((tok as usize) < logits.len(), "token {} out of range", tok);
        }
    }

    #[test]
    fn test_lcg_range() {
        for seed in 0..100 {
            let r = lcg_f32(seed);
            assert!(r >= 0.0 && r < 1.0, "lcg out of [0,1): {}", r);
        }
    }

    #[test]
    fn test_generate_config() {
        let cfg = GenerateConfig::new(128).with_temperature(0.5);
        assert_eq!(cfg.max_new_tokens, 128);
    }
}
