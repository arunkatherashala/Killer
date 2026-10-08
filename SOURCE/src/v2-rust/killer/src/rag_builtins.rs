//! Text chunking, token counting and RAG prompt building.
//!
//! Builtins: `chunk_text(text, size, overlap)`, `chunk_by_sentence(text, max_chars)`,
//! `count_tokens(text)`, `build_rag_prompt(question, chunks)`.
//!
//! `count_tokens` is a documented APPROXIMATION: the real tokenizer in `inference/tokenizer.rs`
//! needs a GGUF model file (its vocabulary and merges live in the model), so without one we
//! estimate with a BPE-like rule: each run of letters costs ceil(len / 4) tokens, each run of
//! digits ceil(len / 3), every other visible character (punctuation, CJK, emoji) one token and
//! whitespace is free. On English prose this lands within roughly 10-20 percent of GPT-style
//! tokenizers, which is enough for budgeting a prompt; use a model's own tokenizer when exact
//! counts matter.

use crate::error::VmError;
use crate::value::{SharedArray, Value};

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn whole(v: &Value, who: &str, what: &str) -> Result<usize, VmError> {
    let n = match v {
        Value::Number(n) => *n,
        Value::Integer(i) => *i as f64,
        other => return err(format!("{}(): {} must be a number, got {}", who, what, other.type_name())),
    };
    if n < 0.0 || n.fract() != 0.0 || !n.is_finite() {
        return err(format!("{}(): {} must be a non-negative whole number", who, what));
    }
    Ok(n as usize)
}

/// Sliding window of `size` characters with `overlap` characters shared between neighbours.
/// A window that would end mid-word is pulled back to the last whitespace in its final fifth
/// (when that does not shrink it below half of `size`). Always makes progress.
pub fn chunk_text(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < total {
        let mut end = (start + size).min(total);
        if end < total {
            let floor = (start + size / 2).max(end.saturating_sub((size / 5).max(1)));
            if let Some(p) = chars[floor.min(end)..end].iter().rposition(|c| c.is_whitespace()) {
                end = floor.min(end) + p + 1;
            }
        }
        let piece: String = chars[start..end].iter().collect();
        let piece = piece.trim();
        if !piece.is_empty() {
            out.push(piece.to_string());
        }
        if end >= total {
            break;
        }
        // next window starts `overlap` chars before this one ended, but always moves forward
        start = end.saturating_sub(overlap).max(start + 1);
        // do not begin the next chunk in the middle of a word: skip to the next word start
        if start > 0 && !chars[start - 1].is_whitespace() && !chars[start].is_whitespace() {
            if let Some(p) = chars[start..end].iter().position(|c| c.is_whitespace()) {
                start += p + 1;
            }
        }
    }
    out
}

/// Split into sentences at `.`, `!` or `?` (plus any closing quotes/brackets) followed by
/// whitespace, or at a blank line.
pub fn sentences(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        cur.push(c);
        let mut boundary = false;
        if matches!(c, '.' | '!' | '?') {
            while i + 1 < chars.len() && matches!(chars[i + 1], '"' | '\'' | ')' | ']' | '.' | '!' | '?') {
                i += 1;
                cur.push(chars[i]);
            }
            boundary = i + 1 >= chars.len() || chars[i + 1].is_whitespace();
        } else if c == '\n' && i + 1 < chars.len() && chars[i + 1] == '\n' {
            boundary = true;
        }
        if boundary {
            let t = cur.trim();
            if !t.is_empty() {
                out.push(t.to_string());
            }
            cur.clear();
        }
        i += 1;
    }
    let t = cur.trim();
    if !t.is_empty() {
        out.push(t.to_string());
    }
    out
}

/// Pack whole sentences into chunks of at most `max_chars` characters. A single sentence longer
/// than `max_chars` is split with `chunk_text` (no overlap).
pub fn chunk_by_sentence(text: &str, max_chars: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut cur_len = 0usize;
    let flush = |cur: &mut String, cur_len: &mut usize, out: &mut Vec<String>| {
        if !cur.is_empty() {
            out.push(std::mem::take(cur));
            *cur_len = 0;
        }
    };
    for s in sentences(text) {
        let len = s.chars().count();
        if len > max_chars {
            flush(&mut cur, &mut cur_len, &mut out);
            out.extend(chunk_text(&s, max_chars, 0));
            continue;
        }
        if cur_len > 0 && cur_len + 1 + len > max_chars {
            flush(&mut cur, &mut cur_len, &mut out);
        }
        if cur_len > 0 {
            cur.push(' ');
            cur_len += 1;
        }
        cur.push_str(&s);
        cur_len += len;
    }
    flush(&mut cur, &mut cur_len, &mut out);
    out
}

/// Approximate token count (see the module docs).
pub fn count_tokens(text: &str) -> usize {
    let mut n = 0usize;
    let mut letters = 0usize;
    let mut digits = 0usize;
    let flush = |letters: &mut usize, digits: &mut usize, n: &mut usize| {
        *n += (*letters + 3) / 4 + (*digits + 2) / 3;
        *letters = 0;
        *digits = 0;
    };
    for c in text.chars() {
        if c.is_ascii_alphabetic() || (c.is_alphabetic() && (c as u32) < 0x2E80) {
            if digits > 0 {
                flush(&mut letters, &mut digits, &mut n);
            }
            letters += 1;
        } else if c.is_ascii_digit() {
            if letters > 0 {
                flush(&mut letters, &mut digits, &mut n);
            }
            digits += 1;
        } else {
            flush(&mut letters, &mut digits, &mut n);
            if !c.is_whitespace() {
                n += 1;
            }
        }
    }
    flush(&mut letters, &mut digits, &mut n);
    n
}

fn chunk_text_of(v: &Value) -> Option<String> {
    match v {
        Value::Str(s) => Some(s.clone()),
        Value::Dict(d) => match d.get("text") {
            Some(Value::Str(s)) => Some(s),
            // a vec_search hit: {id, score, meta: {text: ...}}
            _ => match d.get("meta") {
                Some(Value::Dict(m)) => match m.get("text") {
                    Some(Value::Str(s)) => Some(s),
                    _ => None,
                },
                _ => None,
            },
        },
        _ => None,
    }
}

/// Prompt with numbered context passages, an instruction to stay grounded, and the question.
pub fn build_rag_prompt(question: &str, chunks: &[String]) -> String {
    if chunks.is_empty() {
        return format!("Question: {}\nAnswer:", question);
    }
    let mut p = String::from(
        "Answer the question using only the context below. If the context does not contain the answer, say you do not know.\n\nContext:\n",
    );
    for (i, c) in chunks.iter().enumerate() {
        p.push_str(&format!("[{}] {}\n", i + 1, c.trim()));
    }
    p.push_str(&format!("\nQuestion: {}\nAnswer:", question));
    p
}

fn strings(chunks: Vec<String>) -> Value {
    Value::Array(SharedArray::new(chunks.into_iter().map(Value::Str).collect()))
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    Some(match name {
        "chunk_text" => (|| match args {
            [Value::Str(t), size, overlap] | [Value::Str(t), size, overlap, ..] => {
                let size = whole(size, "chunk_text", "size")?;
                let overlap = whole(overlap, "chunk_text", "overlap")?;
                if size == 0 {
                    return err("chunk_text(): size must be at least 1");
                }
                if overlap >= size {
                    return err("chunk_text(): overlap must be smaller than size");
                }
                Ok(strings(chunk_text(t, size, overlap)))
            }
            [Value::Str(t), size] => {
                let size = whole(size, "chunk_text", "size")?;
                if size == 0 {
                    return err("chunk_text(): size must be at least 1");
                }
                Ok(strings(chunk_text(t, size, 0)))
            }
            _ => err("chunk_text(text, size, overlap) expects a string and numbers"),
        })(),
        "chunk_by_sentence" => (|| match args {
            [Value::Str(t), max] => {
                let max = whole(max, "chunk_by_sentence", "max_chars")?;
                if max == 0 {
                    return err("chunk_by_sentence(): max_chars must be at least 1");
                }
                Ok(strings(chunk_by_sentence(t, max)))
            }
            _ => err("chunk_by_sentence(text, max_chars) expects a string and a number"),
        })(),
        "count_tokens" => match args {
            [Value::Str(t)] | [Value::Str(t), _] => Ok(Value::Number(count_tokens(t) as f64)),
            _ => err("count_tokens(text) expects a string"),
        },
        "build_rag_prompt" => (|| match args {
            [Value::Str(q), Value::Array(a)] => {
                let mut chunks = Vec::new();
                for item in a.clone_elements() {
                    match chunk_text_of(&item) {
                        Some(s) => chunks.push(s),
                        None => return err("build_rag_prompt(): chunks must be strings, {text: ...} dicts or vec_search hits with meta.text"),
                    }
                }
                Ok(Value::Str(build_rag_prompt(q, &chunks)))
            }
            _ => err("build_rag_prompt(question, chunks) expects a string and an array"),
        })(),
        _ => return None,
    })
}

pub const NAMES: &[&str] = &["chunk_text", "chunk_by_sentence", "count_tokens", "build_rag_prompt"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_text_windows_and_overlap() {
        let t = "abcdefghijklmnopqrstuvwxyz";
        let c = chunk_text(t, 10, 3);
        assert_eq!(c[0], "abcdefghij");
        assert_eq!(c[1], "hijklmnopq");
        assert!(c.last().unwrap().ends_with('z'));
        assert_eq!(chunk_text(t, 100, 0), vec![t.to_string()]);
        assert!(chunk_text("", 10, 2).is_empty());
    }

    #[test]
    fn chunk_text_prefers_word_boundaries_and_always_progresses() {
        let c = chunk_text("the quick brown fox jumps over the lazy dog", 16, 0);
        assert_eq!(c, vec!["the quick brown", "fox jumps over", "the lazy dog"]);
        // worst case: overlap = size - 1 still terminates
        let c = chunk_text(&"x".repeat(50), 5, 4);
        assert!(c.len() >= 10 && c.len() <= 50);
        // unicode safe
        let c = chunk_text("héllo wörld ünïcode tëxt", 8, 2);
        assert!(c.iter().all(|s| !s.is_empty()));
    }

    #[test]
    fn sentence_chunking() {
        let t = "First one. Second one! Is this third? Yes it is. Done";
        assert_eq!(sentences(t), vec!["First one.", "Second one!", "Is this third?", "Yes it is.", "Done"]);
        let c = chunk_by_sentence(t, 25);
        assert_eq!(c, vec!["First one. Second one!", "Is this third? Yes it is.", "Done"]);
        // an over-long sentence is hard split
        let long = "word ".repeat(30);
        let c = chunk_by_sentence(&long, 40);
        assert!(c.len() >= 3 && c.iter().all(|s| s.chars().count() <= 40));
        assert!(chunk_by_sentence("", 10).is_empty());
    }

    #[test]
    fn token_estimate() {
        assert_eq!(count_tokens(""), 0);
        assert_eq!(count_tokens("hello"), 2);
        assert_eq!(count_tokens("a b c"), 3);
        assert_eq!(count_tokens("Hello, world!"), 2 + 1 + 2 + 1);
        assert_eq!(count_tokens("12345"), 2);
        assert!(count_tokens("日本語") >= 3);
    }

    #[test]
    fn rag_prompt_shape() {
        let p = build_rag_prompt("Why?", &["Because.".to_string(), " Reasons. ".to_string()]);
        assert!(p.contains("[1] Because.\n[2] Reasons.\n"));
        assert!(p.ends_with("Question: Why?\nAnswer:"));
        assert_eq!(build_rag_prompt("Q", &[]), "Question: Q\nAnswer:");
    }
}
