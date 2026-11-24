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
    /// BPE merge ranks from `tokenizer.ggml.merges`: (left, right) -> rank (lower merges first).
    /// Empty when the model ships no merge table (SentencePiece models use `token_scores`).
    merge_ranks:       HashMap<(String, String), u32>,
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

        let mut merge_ranks = HashMap::new();
        for (rank, m) in gguf.meta_array_strings("tokenizer.ggml.merges").iter().enumerate() {
            if let Some((a, b)) = m.split_once(' ') {
                merge_ranks.entry((a.to_string(), b.to_string())).or_insert(rank as u32);
            }
        }

        eprintln!("[tok-info] is_spm={} bos={} eos={} vocab_size={}", is_spm, bos_id, eos_id, vocab_size);
        Ok(KillerTokenizer {
            vocab, token_scores, token_types, vocab_size,
            bos_id, eos_id, pad_id, unk_id, token_to_id, merge_ranks, is_spm,
        })
    }

    // --- Encoding: text → token IDs -----------------------------------------

    /// Encode text into a token ID sequence.
    /// `add_bos`: prepend the beginning-of-sequence token.
    /// NOTE: For BPE models (Qwen2, GPT-2), BOS is not used — the prompt template
    /// already starts with the appropriate special token (`<|im_start|>` etc.).
    pub fn encode(&self, text: &str, add_bos: bool) -> Vec<u32> {
        self.encode_with_special(text, add_bos, false)
    }

    /// Like [`encode`], but when `parse_special` is true, literal special-token strings in
    /// `text` (`<|im_start|>`, `</s>`, ...) are turned into their control token ids. With the
    /// default (`false`) user text is always encoded as plain text, so untrusted input cannot
    /// inject control tokens.
    pub fn encode_with_special(&self, text: &str, add_bos: bool, parse_special: bool) -> Vec<u32> {
        let mut result = Vec::new();
        // Only add BOS for SentencePiece models; BPE models embed it in the template
        if add_bos && self.is_spm { result.push(self.bos_id); }

        if self.is_spm {
            // -- SentencePiece path (LLaMA 1/2, TinyLlama, Mistral, Gemma) --
            // SentencePiece: spaces → ▁ (U+2581), prepend one ▁ at start
            let normalized = format!("\u{2581}{}", text.replace(' ', "\u{2581}"));
            let mut pieces = self.initial_tokenize(&normalized, parse_special);
            self.bpe_merge(&mut pieces, parse_special);
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
            self.encode_bpe(text, &mut result, parse_special);
        }

        result
    }

    /// True for control / user-defined tokens, which plain-text encoding must never produce
    /// from literal text.
    fn is_special_id(&self, id: u32) -> bool {
        matches!(self.token_types.get(id as usize), Some(3) | Some(4))
    }

    /// Initial tokenization: break text into characters/multi-char vocab entries (greedy longest match).
    fn initial_tokenize(&self, text: &str, allow_special: bool) -> Vec<String> {
        let mut pieces = Vec::new();
        let mut remaining = text;

        while !remaining.is_empty() {
            // Try longest match first (up to 32 chars)
            let limit = remaining.len().min(64);
            let mut found = false;

            for end in (1..=limit).rev() {
                if !remaining.is_char_boundary(end) { continue; }
                let candidate = &remaining[..end];
                let usable = match self.token_to_id.get(candidate) {
                    Some(&id) => allow_special || !self.is_special_id(id),
                    None => false,
                };
                if usable {
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
    fn bpe_merge(&self, pieces: &mut Vec<String>, allow_special: bool) {
        loop {
            if pieces.len() < 2 { break; }
            let mut best_score = f32::NEG_INFINITY;
            let mut best_i = usize::MAX;
            for i in 0..pieces.len() - 1 {
                let merged = format!("{}{}", pieces[i], pieces[i + 1]);
                if let Some(&id) = self.token_to_id.get(&merged) {
                    if !allow_special && self.is_special_id(id) { continue; }
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

    /// Rank-based BPE (GPT-2 style): repeatedly merge the adjacent pair with the lowest
    /// rank in `tokenizer.ggml.merges` until no listed pair remains. Ties go leftmost.
    fn bpe_merge_ranked(&self, pieces: &mut Vec<String>) {
        loop {
            if pieces.len() < 2 { break; }
            let mut best: Option<(u32, usize)> = None;
            for i in 0..pieces.len() - 1 {
                if let Some(&r) = self.merge_ranks.get(&(pieces[i].clone(), pieces[i + 1].clone())) {
                    if best.map_or(true, |(br, _)| r < br) { best = Some((r, i)); }
                }
            }
            match best {
                Some((_, i)) => {
                    let right = pieces.remove(i + 1);
                    pieces[i].push_str(&right);
                }
                None => break,
            }
        }
    }

    /// BPE encoding for tiktoken/GPT-2 style models (Qwen2, LLaMA 3).
    /// Two-phase: first split on ONLY true special tokens (`<|...|>`, `<s>`, `</s>`),
    /// then apply GPT-2 byte-level encoding + BPE on each regular text segment.
    fn encode_bpe(&self, text: &str, out: &mut Vec<u32>, parse_special: bool) {
        let n = text.len();
        let mut pos = 0usize;

        while pos < n {
            // Phase 1: Try to match a SPECIAL token at the current position (opt-in only).
            let rem = &text[pos..];
            if parse_special && rem.starts_with('<') {
                if let Some(close) = rem.find('>') {
                    let candidate = &rem[..close + 1];
                    if let Some(&id) = self.token_to_id.get(candidate) {
                        out.push(id);
                        pos += close + 1;
                        continue;
                    }
                }
            }

            // Phase 2: Collect regular text. The first character is always consumed (so a `<`
            // that does not start a special token is kept as text and progress is guaranteed);
            // after that stop at the next potential special token start.
            let seg_start = pos;
            if let Some(c) = text[pos..].chars().next() {
                pos += c.len_utf8();
            }
            while pos < n {
                let rem2 = &text[pos..];
                if parse_special && rem2.starts_with('<') && rem2.find('>').is_some() {
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
                let mut pieces = if self.merge_ranks.is_empty() {
                    let mut p = self.initial_tokenize(&gpt2_seg, parse_special);
                    self.bpe_merge(&mut p, parse_special);
                    p
                } else {
                    // Real BPE: start from single (byte-level) characters, merge by rank.
                    let mut p: Vec<String> = gpt2_seg.chars().map(|c| c.to_string()).collect();
                    self.bpe_merge_ranked(&mut p);
                    p
                };
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
        // Accumulate raw bytes across tokens first: a multi-byte UTF-8 character can be split
        // over several byte-level tokens, and each token alone is not valid UTF-8.
        let mut bytes: Vec<u8> = Vec::new();
        for &id in tokens {
            if id == self.bos_id || id == self.eos_id { continue; }
            self.push_token_bytes(id, &mut bytes);
        }
        let out = String::from_utf8_lossy(&bytes).into_owned();
        out.trim_start_matches(' ').to_string()
    }

    /// Append the raw bytes a token stands for (GPT-2 byte-level mapping, SPM `▁` and
    /// `<0xXX>` byte-fallback pieces).
    fn push_token_bytes(&self, id: u32, bytes: &mut Vec<u8>) {
        let raw = match self.vocab.get(id as usize) { Some(r) => r, None => return };
        if self.is_spm {
            if raw.len() == 6 && raw.starts_with("<0x") && raw.ends_with('>') {
                if let Ok(b) = u8::from_str_radix(&raw[3..5], 16) {
                    bytes.push(b);
                    return;
                }
            }
            bytes.extend_from_slice(raw.replace('\u{2581}', " ").as_bytes());
        } else {
            for c in raw.chars() {
                match Self::unicode_to_byte(c as u32) {
                    Some(b) => bytes.push(b),
                    None => {
                        let mut buf = [0u8; 4];
                        bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                    }
                }
            }
        }
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
            vocab, token_to_id, merge_ranks: HashMap::new(),
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

    fn make_bpe_tokenizer() -> KillerTokenizer {
        let vocab: Vec<String> = ["<unk>", "<s>", "</s>", "<|im_start|>", "<", "a", "b", "\u{c3}", "\u{a9}"]
            .iter().map(|s| s.to_string()).collect();
        let vocab_size = vocab.len();
        let mut token_to_id = HashMap::new();
        for (i, v) in vocab.iter().enumerate() { token_to_id.insert(v.clone(), i as u32); }
        let mut token_types = vec![1u32; vocab_size];
        token_types[3] = 3;
        KillerTokenizer {
            token_scores: vec![0.0; vocab_size],
            token_types,
            vocab_size,
            bos_id: 1, eos_id: 2, pad_id: 0, unk_id: 0,
            vocab, token_to_id, merge_ranks: HashMap::new(),
            is_spm: false,
        }
    }

    #[test]
    fn lt_that_is_not_a_special_token_is_kept() {
        let tok = make_bpe_tokenizer();
        assert_eq!(tok.encode("a<b", false), vec![5, 4, 6]);
        assert_eq!(tok.encode("a<b>", false).len(), 4); // '>' unknown -> unk, '<' kept
        assert_eq!(tok.encode_with_special("a<b", false, true), vec![5, 4, 6]);
    }

    #[test]
    fn literal_special_strings_are_plain_text_unless_opted_in() {
        let tok = make_bpe_tokenizer();
        assert!(!tok.encode("<|im_start|>a", false).contains(&3));
        assert_eq!(tok.encode_with_special("<|im_start|>a", false, true), vec![3, 5]);
    }

    #[test]
    fn decode_accumulates_bytes_across_tokens() {
        let tok = make_bpe_tokenizer();
        // 0xC3 0xA9 is "é": neither byte alone is valid UTF-8
        assert_eq!(tok.decode(&[7, 8]), "\u{e9}");
    }

    #[test]
    fn spm_byte_fallback_decodes_to_one_char() {
        let mut tok = make_tokenizer();
        tok.vocab.push("<0xC3>".to_string());
        tok.vocab.push("<0xA9>".to_string());
        assert_eq!(tok.decode(&[8, 9]), "\u{e9}");
    }

    // Tiny merge table, ranks by order: 0:"a b" 1:"ab c" 2:"b c".
    // Python reference (GPT-2 BPE):
    //   w=list("abc"); ranks={("a","b"):0,("ab","c"):1,("b","c"):2}
    //   loop: pick min-rank adjacent pair, merge  -> ["ab","c"] -> ["abc"]
    // so "abc" -> [abc]. (Lowest rank wins over leftmost/longest: "bc" would merge
    // only if "a b" were absent; "abcab" -> [abc, ab].)
    fn make_ranked_tokenizer() -> KillerTokenizer {
        let vocab: Vec<String> = ["<unk>", "<s>", "</s>", "a", "b", "c", "ab", "bc", "abc"]
            .iter().map(|s| s.to_string()).collect();
        let vocab_size = vocab.len();
        let mut token_to_id = HashMap::new();
        for (i, v) in vocab.iter().enumerate() { token_to_id.insert(v.clone(), i as u32); }
        let mut merge_ranks = HashMap::new();
        merge_ranks.insert(("a".to_string(), "b".to_string()), 0);
        merge_ranks.insert(("ab".to_string(), "c".to_string()), 1);
        merge_ranks.insert(("b".to_string(), "c".to_string()), 2);
        KillerTokenizer {
            token_scores: vec![0.0; vocab_size],
            token_types: vec![1u32; vocab_size],
            vocab_size, bos_id: 1, eos_id: 2, pad_id: 0, unk_id: 0,
            vocab, token_to_id, merge_ranks, is_spm: false,
        }
    }

    #[test]
    fn bpe_uses_merge_ranks_not_vocab_greedy() {
        let tok = make_ranked_tokenizer();
        // ids: a=3 b=4 c=5 ab=6 bc=7 abc=8
        assert_eq!(tok.encode("abc", false), vec![8]);
        assert_eq!(tok.encode("abcab", false), vec![8, 6]);
        // "bc" alone: only rank-2 merge applies
        assert_eq!(tok.encode("bc", false), vec![7]);
        // "cab": "a b" (rank 0) merges, "c" stays
        assert_eq!(tok.encode("cab", false), vec![5, 6]);
    }

    #[test]
    fn bpe_rank_beats_longest_match() {
        // ranks: "b c"=0, "a b"=1, "ab c"=2. Python: abc -> a,bc (bc merges first, then
        // "a bc" is not a listed pair) => ["a","bc"]. Greedy longest-match gives ["abc"].
        let mut tok = make_ranked_tokenizer();
        tok.merge_ranks.clear();
        tok.merge_ranks.insert(("b".to_string(), "c".to_string()), 0);
        tok.merge_ranks.insert(("a".to_string(), "b".to_string()), 1);
        tok.merge_ranks.insert(("ab".to_string(), "c".to_string()), 2);
        assert_eq!(tok.encode("abc", false), vec![3, 7]);
    }

    #[test]
    fn test_is_eos() {
        let tok = make_tokenizer();
        assert!(tok.is_eos(2));
        assert!(!tok.is_eos(1));
    }
}
