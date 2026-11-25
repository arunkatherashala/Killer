// inference/pretokenizer.rs — GPT-2 / LLaMA 3 / Qwen2 style BPE pre-tokenizers
//
// Byte-level BPE tokenizers do not run merges over the whole text: the text is first split
// into "pre-tokens" by a regex, and merges only happen inside one pre-token. GGUF names the
// regex family in `tokenizer.ggml.pre`. This module implements the supported families with a
// hand-written scanner (no regex crate). Each `*_at` function returns where the reference
// regex's match starting at character index `i` ends, trying the alternatives in the same
// order as the regex (leftmost alternative wins, quantifiers greedy with backtracking).
//
// Reference regexes (as in the upstream tokenizer.json files / llama.cpp `llama-vocab.cpp`):
//   GPT-2:   's|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+
//   LLaMA 3: (?i:'s|'t|'re|'ve|'m|'ll|'d)|[^\r\n\p{L}\p{N}]?\p{L}+|\p{N}{1,3}| ?[^\s\p{L}\p{N}]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+
//   Qwen2:   same as LLaMA 3 but \p{N} (one digit per pre-token) instead of \p{N}{1,3}
//   StarCoder: first split every \p{N} into its own piece, then GPT-2 on the rest
// (The `(?i:...)` group is ASCII-only here, like llama.cpp's adapted `'[sS]|'[tT]|...`.)
//
// Character classes: \s = `char::is_whitespace` (Unicode White_Space, same set as the
// reference), \p{N} = `char::is_numeric` (Nd, Nl, No: same set), \p{L} =
// `char::is_alphabetic() && !char::is_numeric()`. The last one is an approximation: Rust's
// Alphabetic property also contains Other_Alphabetic combining marks (Mn/Mc, for example
// Devanagari vowel signs), which the reference treats as "other". ASCII and most
// letters without combining marks classify exactly.

/// A pre-tokenizer family from `tokenizer.ggml.pre`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreTokenizer {
    Gpt2,
    Llama3,
    Qwen2,
    StarCoder,
}

/// Names accepted for each family (subset of the names llama.cpp maps to the same regex).
const GPT2_NAMES: &[&str] = &["gpt-2", "gpt2", "phi-2", "mpt", "olmo", "roberta-bpe"];
const LLAMA3_NAMES: &[&str] = &["llama3", "llama-v3", "llama-bpe", "falcon3", "dbrx", "smaug-bpe"];
const QWEN2_NAMES: &[&str] = &["qwen2", "deepseek-r1-qwen", "stablelm2"];
const STARCODER_NAMES: &[&str] = &["starcoder", "refact", "command-r", "smollm"];

impl PreTokenizer {
    /// Resolve `tokenizer.ggml.pre`. Returns the pre-tokenizer to use and, when the name is
    /// missing or not supported, a warning: GPT-2 splitting is used then, and token ids can
    /// differ from the reference tokenizer.
    pub fn from_gguf_name(name: Option<&str>) -> (PreTokenizer, Option<String>) {
        let n = match name {
            Some(n) if !n.is_empty() => n,
            _ => {
                return (PreTokenizer::Gpt2, Some(
                    "tokenizer.ggml.pre is missing: using the GPT-2 pre-tokenizer; token ids may \
                     differ from the model's reference tokenizer".to_string()));
            }
        };
        let lower = n.to_ascii_lowercase();
        let l = lower.as_str();
        if GPT2_NAMES.contains(&l) { return (PreTokenizer::Gpt2, None); }
        if LLAMA3_NAMES.contains(&l) { return (PreTokenizer::Llama3, None); }
        if QWEN2_NAMES.contains(&l) { return (PreTokenizer::Qwen2, None); }
        if STARCODER_NAMES.contains(&l) { return (PreTokenizer::StarCoder, None); }
        (PreTokenizer::Gpt2, Some(format!(
            "unsupported pre-tokenizer '{}' (tokenizer.ggml.pre): using the GPT-2 pre-tokenizer; \
             token ids may differ from the model's reference tokenizer. Supported: {}",
            n, Self::supported_names().join(", "))))
    }

    /// Every accepted `tokenizer.ggml.pre` value.
    pub fn supported_names() -> Vec<&'static str> {
        GPT2_NAMES.iter().chain(LLAMA3_NAMES).chain(QWEN2_NAMES).chain(STARCODER_NAMES).copied().collect()
    }

    /// Split `text` into pre-tokens. The pieces are contiguous and concatenate to `text`.
    pub fn split<'a>(&self, text: &'a str) -> Vec<&'a str> {
        let cs: Vec<(usize, char)> = text.char_indices().collect();
        let byte_at = |i: usize| if i < cs.len() { cs[i].0 } else { text.len() };
        let mut out = Vec::new();
        match self {
            PreTokenizer::StarCoder => {
                // Pass 1: every \p{N} char is its own piece; pass 2: GPT-2 on the gaps.
                let mut i = 0;
                while i < cs.len() {
                    if is_number(cs[i].1) {
                        out.push(&text[byte_at(i)..byte_at(i + 1)]);
                        i += 1;
                        continue;
                    }
                    let mut j = i;
                    while j < cs.len() && !is_number(cs[j].1) { j += 1; }
                    let gap = &text[byte_at(i)..byte_at(j)];
                    out.extend(PreTokenizer::Gpt2.split(gap));
                    i = j;
                }
            }
            _ => {
                let chars: Vec<char> = cs.iter().map(|&(_, c)| c).collect();
                let mut i = 0;
                while i < chars.len() {
                    let end = match self {
                        PreTokenizer::Gpt2 => gpt2_at(&chars, i),
                        PreTokenizer::Llama3 => llama3_at(&chars, i, 3),
                        PreTokenizer::Qwen2 => llama3_at(&chars, i, 1),
                        PreTokenizer::StarCoder => unreachable!(),
                    };
                    // Every character matches some alternative; `max` guarantees progress anyway.
                    let end = end.max(i + 1).min(chars.len());
                    out.push(&text[byte_at(i)..byte_at(end)]);
                    i = end;
                }
            }
        }
        out
    }
}

const CR: char = 13u8 as char;
const LF: char = 10u8 as char;

fn is_letter(c: char) -> bool { c.is_alphabetic() && !c.is_numeric() }
fn is_number(c: char) -> bool { c.is_numeric() }
fn is_ws(c: char) -> bool { c.is_whitespace() }
fn is_other(c: char) -> bool { !is_ws(c) && !is_letter(c) && !is_number(c) }
fn is_newline(c: char) -> bool { c == CR || c == LF }

fn run(cs: &[char], from: usize, pred: fn(char) -> bool) -> usize {
    let mut j = from;
    while j < cs.len() && pred(cs[j]) { j += 1; }
    j
}

/// `'s|'t|'re|'ve|'m|'ll|'d` at `i`; returns the match length in chars.
fn contraction(cs: &[char], i: usize, ascii_ci: bool) -> Option<usize> {
    if cs.get(i) != Some(&'\'') { return None; }
    let at = |k: usize| cs.get(i + k).map(|&c| if ascii_ci { c.to_ascii_lowercase() } else { c });
    for suffix in ["s", "t", "re", "ve", "m", "ll", "d"] {
        if suffix.chars().enumerate().all(|(k, sc)| at(k + 1) == Some(sc)) {
            return Some(1 + suffix.len());
        }
    }
    None
}

/// `\s+(?!\S)|\s+` at `i` (cs[i] is whitespace): a whitespace run that stops one short of a
/// following non-space, so the last space can prefix the next word.
fn whitespace_tail(cs: &[char], i: usize) -> usize {
    let end = run(cs, i, is_ws);
    if end == i { return i + 1; }
    if end == cs.len() || end - i == 1 { end } else { end - 1 }
}

fn gpt2_at(cs: &[char], i: usize) -> usize {
    if let Some(n) = contraction(cs, i, false) { return i + n; }
    let j = if cs[i] == ' ' && i + 1 < cs.len() { i + 1 } else { i };
    let c = cs[j];
    if is_letter(c) { return run(cs, j, is_letter); }
    if is_number(c) { return run(cs, j, is_number); }
    if is_other(c) { return run(cs, j, is_other); }
    whitespace_tail(cs, i)
}

fn llama3_at(cs: &[char], i: usize, max_digits: usize) -> usize {
    if let Some(n) = contraction(cs, i, true) { return i + n; }
    let c = cs[i];
    // [^\r\n\p{L}\p{N}]?\p{L}+
    if is_letter(c) { return run(cs, i, is_letter); }
    if !is_newline(c) && !is_number(c) && i + 1 < cs.len() && is_letter(cs[i + 1]) {
        return run(cs, i + 1, is_letter);
    }
    // \p{N}{1,max}
    if is_number(c) { return run(cs, i, is_number).min(i + max_digits); }
    //  ?[^\s\p{L}\p{N}]+[\r\n]*
    let j = if c == ' ' && i + 1 < cs.len() { i + 1 } else { i };
    if is_other(cs[j]) {
        let k = run(cs, j, is_other);
        return run(cs, k, is_newline);
    }
    // \s*[\r\n]+ : up to and including the last newline of the whitespace run
    let end = run(cs, i, is_ws);
    if let Some(last_nl) = (i..end).rev().find(|&m| is_newline(cs[m])) {
        return last_nl + 1;
    }
    whitespace_tail(cs, i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(p: PreTokenizer, t: &str) -> Vec<String> {
        let v = p.split(t);
        assert_eq!(v.concat(), t, "pieces must concatenate to the input");
        v.into_iter().map(String::from).collect()
    }

    // Expected splits below were produced with Python `re` (the third-party `regex` module
    // is not installed here), using ASCII approximations of the classes:
    // \p{L} -> [A-Za-z], \p{N} -> [0-9], [^\s\p{L}\p{N}] -> [^\sA-Za-z0-9]. For ASCII input
    // these are exact. Non-ASCII cases are reasoned by hand from the regex and noted.
    //   GPT2 = r"'s|'t|'re|'ve|'m|'ll|'d| ?[A-Za-z]+| ?[0-9]+| ?[^\sA-Za-z0-9]+|\s+(?!\S)|\s+"
    //   L3   = r"(?:'[sS]|'[tT]|'[rR][eE]|'[vV][eE]|'[mM]|'[lL][lL]|'[dD])|[^\r\nA-Za-z0-9]?[A-Za-z]+|[0-9]{1,3}| ?[^\sA-Za-z0-9]+[\r\n]*|\s*[\r\n]+|\s+(?!\S)|\s+"
    //   QW   = L3 with [0-9] instead of [0-9]{1,3}

    #[test]
    fn gpt2_contractions_words_numbers_symbols() {
        let p = PreTokenizer::Gpt2;
        assert_eq!(s(p, "I'm don't they're we've he'll she'd it's"),
            ["I", "'m", " don", "'t", " they", "'re", " we", "'ve", " he", "'ll", " she", "'d", " it", "'s"]);
        assert_eq!(s(p, "Hello world 12345 ok!!"), ["Hello", " world", " 12345", " ok", "!!"]);
        // case-sensitive contractions: 'S is not one
        assert_eq!(s(p, "IT'S"), ["IT", "'", "S"]);
        assert_eq!(s(p, "a+b=c;"), ["a", "+", "b", "=", "c", ";"]);
        assert_eq!(s(p, " ?!x"), [" ?!", "x"]);
    }

    #[test]
    fn gpt2_whitespace_runs() {
        let p = PreTokenizer::Gpt2;
        // \s+(?!\S) leaves the last space to prefix the next word
        assert_eq!(s(p, "a   b"), ["a", "  ", " b"]);
        assert_eq!(s(p, "a  "), ["a", "  "]);
        assert_eq!(s(p, "a\n\nb"), ["a", "\n", "\n", "b"]);
        assert_eq!(s(p, "a \tb"), ["a", " ", "\t", "b"]);
        assert_eq!(s(p, "  "), ["  "]);
        assert_eq!(s(p, " "), [" "]);
        assert_eq!(s(p, ""), Vec::<String>::new());
    }

    #[test]
    fn gpt2_non_ascii_letters_and_digits() {
        // Hand-reasoned: e-acute and CJK are \p{L}; Arabic-Indic digits are \p{N}.
        let p = PreTokenizer::Gpt2;
        assert_eq!(s(p, "caf\u{e9} \u{4f60}\u{597d}"), ["caf\u{e9}", " \u{4f60}\u{597d}"]);
        assert_eq!(s(p, "x \u{661}\u{662}"), ["x", " \u{661}\u{662}"]);
        // emoji is "other"
        assert_eq!(s(p, "hi \u{1f600}"), ["hi", " \u{1f600}"]);
    }

    #[test]
    fn llama3_splits() {
        let p = PreTokenizer::Llama3;
        assert_eq!(s(p, "Hello world 12345 ok!!"), ["Hello", " world", " ", "123", "45", " ok", "!!"]);
        // case-insensitive contractions
        assert_eq!(s(p, "IT'S I'M"), ["IT", "'S", " I", "'M"]);
        // any non-newline non-alnum char may prefix a word
        assert_eq!(s(p, "(hello)\tworld"), ["(hello", ")", "\tworld"]);
        // symbols swallow trailing newlines; whitespace+newlines up to the last newline
        assert_eq!(s(p, "x;\n\ny"), ["x", ";\n\n", "y"]);
        assert_eq!(s(p, "a \n  b"), ["a", " \n", " ", " b"]);
        assert_eq!(s(p, "a\r\nb"), ["a", "\r\n", "b"]);
        assert_eq!(s(p, "a   b"), ["a", "  ", " b"]);
    }

    #[test]
    fn qwen2_splits_digits_individually() {
        let p = PreTokenizer::Qwen2;
        assert_eq!(s(p, "Hello world 12345 ok!!"), ["Hello", " world", " ", "1", "2", "3", "4", "5", " ok", "!!"]);
        assert_eq!(s(p, "x=42;"), ["x", "=", "4", "2", ";"]);
        assert_eq!(s(p, "I've"), ["I", "'ve"]);
    }

    #[test]
    fn starcoder_digits_then_gpt2() {
        // Pass 1 (\p{N}) isolates each digit, pass 2 runs GPT-2 on the gaps.
        let p = PreTokenizer::StarCoder;
        assert_eq!(s(p, "x = 123 ok"), ["x", " =", " ", "1", "2", "3", " ok"]);
    }

    #[test]
    fn pre_name_resolution() {
        assert_eq!(PreTokenizer::from_gguf_name(Some("gpt-2")), (PreTokenizer::Gpt2, None));
        assert_eq!(PreTokenizer::from_gguf_name(Some("qwen2")), (PreTokenizer::Qwen2, None));
        assert_eq!(PreTokenizer::from_gguf_name(Some("llama-bpe")), (PreTokenizer::Llama3, None));
        assert_eq!(PreTokenizer::from_gguf_name(Some("starcoder")), (PreTokenizer::StarCoder, None));
        let (p, w) = PreTokenizer::from_gguf_name(Some("deepseek-coder"));
        assert_eq!(p, PreTokenizer::Gpt2);
        let w = w.expect("unsupported name must warn");
        assert!(w.contains("unsupported pre-tokenizer 'deepseek-coder'") && w.contains("qwen2"), "{}", w);
        let (_, w) = PreTokenizer::from_gguf_name(None);
        assert!(w.unwrap().contains("missing"));
    }
}
