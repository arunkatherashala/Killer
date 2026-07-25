// inference/tokenizer.rs — BPE Tokenizer loaded from GGUF embedded vocabulary
//
// Most modern LLMs (LLaMA, Phi, Mistral, Gemma) embed the complete tokenizer
// vocabulary inside the .gguf file itself.  No separate tokenizer.json needed.
//
// Algorithm: SentencePiece-compatible BPE
//   1. Convert spaces → ▁ (U+2581)
//   2. Start with character-level tokens
//   3. Repeatedly merge the highest-scoring consecutive pair until no merges remain

use std::collections::HashMap;
use super::gguf::GgufFile;

/// BPE tokenizer — loaded directly from a GGUF file.
pub struct KillerTokenizer {
    pub vocab:         Vec<String>,   // token_id → piece string
    pub token_scores:  Vec<f32>,      // token_id → BPE merge score
    pub token_types:   Vec<u32>,      // 1=normal 2=unknown 3=control 4=user_defined
    pub vocab_size:    usize,
    pub bos_id:        u32,
    pub eos_id:        u32,
    pub pad_id:        u32,
    pub unk_id:        u32,
    token_to_id:       HashMap<String, u32>,
    /// Whether this tokenizer uses SentencePiece (▁ word markers) or plain BPE (GPT-2/tiktoken style).
    /// SentencePiece: LLaMA 1/2, TinyLlama, Mistral, Gemma.
    /// BPE: Qwen2, GPT-2, LLaMA 3 (uses `<|begin_of_text|>` etc).
    is_spm: bool,
}

impl KillerTokenizer {
    /// Load tokenizer from an already-opened GGUF file.
    pub fn from_gguf(gguf: &GgufFile) -> Result<Self, String> {
        let vocab = gguf.meta_array_strings("tokenizer.ggml.tokens");
        if vocab.is_empty() {
            return Err(
                "No vocabulary found in model file. \
                 Ensure the model includes an embedded tokenizer (all recent GGUF models do)."
                .to_string()
            );
        }

        let scores      = gguf.meta_array_f32("tokenizer.ggml.scores");
        let token_types = gguf.meta_array_u32("tokenizer.ggml.token_type");
        let vocab_size  = vocab.len();

        let bos_id = gguf.meta_u64("tokenizer.ggml.bos_token_id").unwrap_or(1)  as u32;
        let eos_id = gguf.meta_u64("tokenizer.ggml.eos_token_id").unwrap_or(2)  as u32;
        let pad_id = gguf.meta_u64("tokenizer.ggml.padding_token_id").unwrap_or(0) as u32;
        let unk_id = gguf.meta_u64("tokenizer.ggml.unknown_token_id").unwrap_or(0) as u32;

        let mut token_to_id = HashMap::with_capacity(vocab_size);
        for (id, piece) in vocab.iter().enumerate() {
            token_to_id.insert(piece.clone(), id as u32);
        }

        let token_scores = if scores.len() == vocab_size { scores } else { vec![0.0f32; vocab_size] };
        let token_types  = if token_types.len() == vocab_size { token_types } else { vec![1u32; vocab_size] };

        // Detect tokenizer type: "llama" and "bpe" with ▁ in vocab → SentencePiece;
        // otherwise ("gpt2", "qwen2") → plain BPE without ▁ word markers.
        let tok_model = gguf.meta_str("tokenizer.ggml.model").unwrap_or_default();
        let is_spm = matches!(tok_model.as_str(), "llama" | "spm" | "sentencepiece")
            || vocab.iter().take(1000).any(|s| s.contains('\u{2581}'));

        eprintln!("[tok-info] is_spm={} bos={} eos={} vocab_size={}", is_spm, bos_id, eos_id, vocab_size);
        Ok(KillerTokenizer {
            vocab, token_scores, token_types, vocab_size,
            bos_id, eos_id, pad_id, unk_id, token_to_id, is_spm,
        })
    }

    // --- Encoding: text → token IDs -----------------------------------------

    /// Encode text into a token ID sequence.
    /// `add_bos`: prepend the beginning-of-sequence token.
    /// NOTE: For BPE models (Qwen2, GPT-2), BOS is not used — the prompt template
    /// already starts with the appropriate special token (`<|im_start|>` etc.).
    pub fn encode(&self, text: &str, add_bos: bool) -> Vec<u32> {
        let mut result = Vec::new();
        // Only add BOS for SentencePiece models; BPE models embed it in the template
        if add_bos && self.is_spm { result.push(self.bos_id); }

        if self.is_spm {
            // -- SentencePiece path (LLaMA 1/2, TinyLlama, Mistral, Gemma) --
            // SentencePiece: spaces → ▁ (U+2581), prepend one ▁ at start
            let normalized = format!("\u{2581}{}", text.replace(' ', "\u{2581}"));
            let mut pieces = self.initial_tokenize(&normalized);
            self.bpe_merge(&mut pieces);
            for piece in pieces {
                match self.token_to_id.get(&piece) {
                    Some(&id) => result.push(id),
                    None      => result.push(self.unk_id),
                }
            }
        } else {
            // -- BPE path (Qwen2, GPT-2, LLaMA 3) --
            // Special tokens (enclosed in <| |> or similar) are found first,
            // then remaining text segments are tokenized with plain BPE.
            self.encode_bpe(text, &mut result);
        }

        result
    }

    /// Initial tokenization: break text into characters/multi-char vocab entries (greedy longest match).
    fn initial_tokenize(&self, text: &str) -> Vec<String> {
        let mut pieces = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            // Try longest match first (up to 32 chars)
            let limit = remaining.len().min(64);
            let mut found = false;

            for end in (1..=limit).rev() {
                if !remaining.is_char_boundary(end) { continue; }
                let candidate = &remaining[..end];
                if self.token_to_id.contains_key(candidate) {
                    pieces.push(candidate.to_string());
                    remaining = &remaining[end..];
                    found = true;
                    break;
                }
            }

            if !found {
                // Byte fallback: encode as <0xXX>
                if let Some(c) = remaining.chars().next() {
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    for byte in s.bytes() {
                        let hex = format!("<0x{:02X}>", byte);
                        pieces.push(hex);
                    }
                    let char_len = c.len_utf8();
                    remaining = &remaining[char_len..];
                } else {
                    break;
                }
            }
        }

        pieces
    }

    // --- BPE helpers ---------------------------------------------------------

    /// Run BPE merges on a pre-tokenized piece list (SentencePiece model).
    fn bpe_merge(&self, pieces: &mut Vec<String>) {
        loop {
            if pieces.len() < 2 { break; }
            let mut best_score = f32::NEG_INFINITY;
            let mut best_i = usize::MAX;
            for i in 0..pieces.len() - 1 {
                let merged = format!("{}{}", pieces[i], pieces[i + 1]);
                if let Some(&id) = self.token_to_id.get(&merged) {
                    let s = self.token_scores.get(id as usize).copied().unwrap_or(0.0);
                    if s > best_score { best_score = s; best_i = i; }
                }
            }
            if best_i == usize::MAX { break; }
            let merged = format!("{}{}", pieces[best_i], pieces[best_i + 1]);
            pieces[best_i] = merged;
            pieces.remove(best_i + 1);
        }
    }

    /// BPE encoding for tiktoken/GPT-2 style models (Qwen2, LLaMA 3).
    /// Two-phase: first split on ONLY true special tokens (`<|...|>`, `<s>`, `</s>`),
    /// then apply GPT-2 byte-level encoding + BPE on each regular text segment.
    fn encode_bpe(&self, text: &str, out: &mut Vec<u32>) {
        let n = text.len();
        let mut pos = 0usize;

        while pos < n {
            // Phase 1: Try to match a SPECIAL token at the current position.
            let rem = &text[pos..];
            if rem.starts_with('<') {
                if let Some(close) = rem.find('>') {
                    let candidate = &rem[..close + 1];
                    if let Some(&id) = self.token_to_id.get(candidate) {
                        out.push(id);
                        pos += close + 1;
                        continue;
                    }
                }
                // No matching special token (e.g. <think>, </think>, <br>, etc.).
                // Advance past just the `<` so Phase 2 can encode it as plain text.
                // Without this, pos never moves and we spin forever.
                pos += 1;
            }

            // Phase 2: Collect regular text until the next potential special token start.
            let seg_start = pos;
            while pos < n {
                let rem2 = &text[pos..];
                if rem2.starts_with('<') && rem2.find('>').is_some() {
                    break;
                }
                if let Some(c) = text[pos..].chars().next() {
                    pos += c.len_utf8();
                } else { break; }
            }
            let seg = &text[seg_start..pos];
            if !seg.is_empty() {
                // Convert to GPT-2 byte-level encoding before BPE lookup:
                // Qwen2/GPT-2 stores tokens with bytes mapped to specific Unicode chars
                // (e.g. '\n' → 'Ċ', ' ' → 'Ġ'). We must encode the text the same way.
                let gpt2_seg: String = seg.bytes().map(|b| Self::byte_to_unicode(b)).collect();
                let mut pieces = self.initial_tokenize(&gpt2_seg);
                self.bpe_merge(&mut pieces);
                for piece in pieces {
                    match self.token_to_id.get(&piece) {
                        Some(&id) => out.push(id),
                        None      => out.push(self.unk_id),
                    }
                }
            }
        }
    }

    /// GPT-2 byte-level encoding: map each byte to its canonical Unicode char.
    /// Printable ASCII/Latin-1 map directly; control and high-Latin bytes → U+0100+.
    /// This matches the Python `bytes_to_unicode()` function in GPT-2/tiktoken.
    fn byte_to_unicode(b: u8) -> char {
        match b {
            33..=126 | 161..=172 | 174..=255 => b as char,
            _ => {
                // Count how many bytes before `b` are in the "special" range
                let n = (0..b).filter(|&x| !(33..=126).contains(&x)
                    && !(161..=172).contains(&x) && !(174..=255).contains(&x)).count();
                char::from_u32(256 + n as u32).unwrap_or('\u{FFFD}')
            }
        }
    }

    // --- Decoding: token IDs → text -----------------------------------------

    /// Decode a sequence of token IDs back to a String.
    pub fn decode(&self, tokens: &[u32]) -> String {
        let mut out = String::new();
        for &id in tokens {
            if id == self.bos_id || id == self.eos_id { continue; }
            // Use token_str for GPT-2/tiktoken models (handles Ġ→space byte decoding).
            // For SPM models token_str also handles ▁→space.
            out.push_str(&self.token_str(id));
        }
        out.trim_start_matches(' ').to_string()
    }

    /// Decode a single token ID (no special handling, for streaming).
    pub fn decode_token(&self, id: u32) -> &str {
        self.vocab.get(id as usize).map(|s| s.as_str()).unwrap_or("")
    }

    /// Returns true if this token ends generation.
    /// Check if a token ID signals end-of-generation.
    /// Handles models with multiple EOS tokens (e.g. Qwen2 uses both
    /// 151643 <|endoftext|> and 151645 <|im_end|>).
    pub fn is_eos(&self, id: u32) -> bool {
        if id == self.eos_id { return true; }
        // Check by token string for models with multiple EOS tokens
        if let Some(piece) = self.vocab.get(id as usize) {
            matches!(piece.as_str(),
                "<|im_end|>" | "<|endoftext|>" | "</s>" |
                "<|eot_id|>" | "<end_of_turn>" | "<|end|>"
            )
        } else {
            false
        }
    }

    pub fn token_str(&self, id: u32) -> String {
        let raw = self.vocab.get(id as usize).cloned().unwrap_or_default();
        if self.is_spm {
            // SentencePiece: ▁ → space
            raw.replace('\u{2581}', " ")
        } else {
            // GPT-2/tiktoken: reverse byte-level encoding
            // U+0100–U+017F and printable-direct ranges back to bytes, then UTF-8 decode
            let bytes: Vec<u8> = raw.chars()
                .filter_map(|c| Self::unicode_to_byte(c as u32))
                .collect();
            String::from_utf8(bytes).unwrap_or_else(|_| raw)
        }
    }

    /// Reverse of byte_to_unicode: given a GPT-2-encoded char, return the original byte.
    fn unicode_to_byte(c: u32) -> Option<u8> {
        match c {
            33..=126 | 161..=172 | 174..=255 => Some(c as u8),
            256..=323 => {
                // Reconstruct the original byte for "special" chars
                let mut n = (c - 256) as usize;
                for b in 0u32..=255 {
                    let is_direct = (33..=126).contains(&b)
                        || (161..=172).contains(&b)
                        || (174..=255).contains(&b);
                    if !is_direct {
                        if n == 0 { return Some(b as u8); }
                        n -= 1;
                    }
                }
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tokenizer() -> KillerTokenizer {
        // Minimal synthetic vocab for testing
        let vocab = vec![
            "<unk>".to_string(),          // 0
            "<s>".to_string(),            // 1 = BOS
            "</s>".to_string(),           // 2 = EOS
            "\u{2581}".to_string(),       // 3 = ▁ (space prefix)
            "\u{2581}hello".to_string(),  // 4
            "\u{2581}world".to_string(),  // 5
            "hello".to_string(),          // 6
            "world".to_string(),          // 7
        ];
        let vocab_size = vocab.len();
        let mut token_to_id = HashMap::new();
        for (i, v) in vocab.iter().enumerate() { token_to_id.insert(v.clone(), i as u32); }
        KillerTokenizer {
            token_scores: vec![0.0; vocab_size],
            token_types:  vec![1u32; vocab_size],
            vocab_size,
            bos_id: 1, eos_id: 2, pad_id: 0, unk_id: 0,
            vocab, token_to_id,
            is_spm: true,
        }
    }

    #[test]
    fn test_decode_strips_space() {
        let tok = make_tokenizer();
        // Token 4 = "▁hello" → " hello" → strip leading space → "hello"
        let decoded = tok.decode(&[4]);
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn test_is_eos() {
        let tok = make_tokenizer();
        assert!(tok.is_eos(2));
        assert!(!tok.is_eos(1));
    }
}
