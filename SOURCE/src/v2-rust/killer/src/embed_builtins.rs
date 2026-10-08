//! Embedding builtins: a deterministic offline `mock_embed`, plus `llm_embed_batch` and the
//! options form of `llm_embed(opts, text)` for real providers.
//!
//! Options dict: `provider` ("openai" default, "ollama", "openai_compatible" / "compat", "mock"),
//! `model`, `base_url` (any OpenAI-compatible server: vLLM, TEI, LM Studio, Together, ...),
//! `key` or `key_env` (the NAME of an environment variable holding the key; provider default
//! OPENAI_API_KEY), `dim` (mock dimension, or OpenAI `dimensions`), `timeout`, `retries`,
//! `batch_size` (default 64), `insecure`, `allow_http`.
//!
//! Keys are only ever sent through `http_req` (curl's stdin), never on a command line, and they
//! are never echoed into error messages.

use crate::error::VmError;
use crate::http_req::{exec, ReqSpec};
use crate::value::{SharedArray, SharedDict, Value};

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

/// Deterministic hashed bag-of-words embedding, L2-normalized. Texts that share words (and, to a
/// lesser degree, word fragments) get a high cosine similarity, so retrieval logic can be tested
/// offline with realistic-looking results. NOT semantic: synonyms are unrelated.
pub fn mock_embed(text: &str, dim: usize) -> Vec<f64> {
    let mut v = vec![0.0f64; dim];
    if dim == 0 {
        return v;
    }
    let lower = text.to_lowercase();
    for word in lower.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()) {
        let h = fnv1a(word.as_bytes());
        // two projections per word keep the vector smooth even at small dims
        for k in 0..2u64 {
            let r = splitmix(h.wrapping_add(k));
            let idx = (r % dim as u64) as usize;
            v[idx] += if (r >> 63) & 1 == 0 { 1.0 } else { -1.0 };
        }
        // character trigrams give partial credit to related word forms (retrieve/retrieval)
        let padded: Vec<char> = format!("^{}$", word).chars().collect();
        if padded.len() >= 3 {
            for w in padded.windows(3) {
                let s: String = w.iter().collect();
                let r = splitmix(fnv1a(s.as_bytes()));
                let idx = (r % dim as u64) as usize;
                v[idx] += if (r >> 63) & 1 == 0 { 0.25 } else { -0.25 };
            }
        }
    }
    let n = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if n > 1e-12 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
    v
}

fn vec_value(v: &[f64]) -> Value {
    Value::Array(SharedArray::new(v.iter().map(|x| Value::Number(*x)).collect()))
}

fn opt_str(d: &SharedDict, k: &str) -> Option<String> {
    match d.get(k) {
        Some(Value::Str(s)) if !s.is_empty() => Some(s),
        _ => None,
    }
}

fn opt_num(d: &SharedDict, k: &str) -> Option<f64> {
    match d.get(k) {
        Some(Value::Number(n)) => Some(n),
        Some(Value::Integer(i)) => Some(i as f64),
        _ => None,
    }
}

fn json_str(s: &str) -> String {
    crate::json_lang::stringify(&Value::Str(s.to_string()), 0).unwrap_or_else(|_| "\"\"".to_string())
}

/// Resolve the API key from `key`, else the env var named by `key_env`, else the default env var.
pub fn resolve_key(opts: &SharedDict, default_env: Option<&str>) -> Result<Option<String>, String> {
    if let Some(k) = opt_str(opts, "key") {
        return Ok(Some(k));
    }
    let env_name = opt_str(opts, "key_env").or_else(|| default_env.map(|s| s.to_string()));
    match env_name {
        None => Ok(None),
        Some(name) => match std::env::var(&name) {
            Ok(v) if !v.is_empty() => Ok(Some(v)),
            _ => {
                if opt_str(opts, "key_env").is_some() {
                    Err(format!("environment variable {} is not set", name))
                } else {
                    Ok(None)
                }
            }
        },
    }
}

fn numbers_of(v: &Value) -> Option<Vec<f64>> {
    match v {
        Value::Array(a) => a
            .clone_elements()
            .iter()
            .map(|x| match x {
                Value::Number(n) => Some(*n),
                Value::Integer(i) => Some(*i as f64),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

/// Parse an OpenAI-style `{"data":[{"index":0,"embedding":[...]}, ...]}` response.
pub fn parse_openai_embeddings(body: &str, expected: usize) -> Result<Vec<Vec<f64>>, String> {
    let v = crate::json_lang::parse(body).map_err(|e| format!("invalid JSON response: {}", e))?;
    let d = match &v {
        Value::Dict(d) => d.clone(),
        _ => return Err("unexpected response shape".to_string()),
    };
    if let Some(e) = d.get("error") {
        let msg = match &e {
            Value::Dict(ed) => match ed.get("message") {
                Some(Value::Str(m)) => m,
                _ => "unknown error".to_string(),
            },
            Value::Str(s) => s.clone(),
            _ => "unknown error".to_string(),
        };
        return Err(format!("provider error: {}", msg));
    }
    let items = match d.get("data") {
        Some(Value::Array(a)) => a.clone_elements(),
        _ => return Err("response has no \"data\" array".to_string()),
    };
    let mut rows: Vec<(usize, Vec<f64>)> = Vec::new();
    for (pos, it) in items.iter().enumerate() {
        let Value::Dict(id) = it else { return Err("bad item in data".to_string()) };
        let emb = id.get("embedding").as_ref().and_then(numbers_of).ok_or("item has no numeric embedding")?;
        let idx = match id.get("index") {
            Some(Value::Number(n)) => n as usize,
            _ => pos,
        };
        rows.push((idx, emb));
    }
    rows.sort_by_key(|r| r.0);
    if rows.len() != expected {
        return Err(format!("expected {} embeddings, got {}", expected, rows.len()));
    }
    Ok(rows.into_iter().map(|r| r.1).collect())
}

/// Parse Ollama's `{"embeddings":[[...],...]}` response.
pub fn parse_ollama_embeddings(body: &str, expected: usize) -> Result<Vec<Vec<f64>>, String> {
    let v = crate::json_lang::parse(body).map_err(|e| format!("invalid JSON response: {}", e))?;
    let Value::Dict(d) = &v else { return Err("unexpected response shape".to_string()) };
    if let Some(Value::Str(e)) = d.get("error") {
        return Err(format!("provider error: {}", e));
    }
    let Some(Value::Array(a)) = d.get("embeddings") else { return Err("response has no \"embeddings\" array".to_string()) };
    let rows: Vec<Vec<f64>> = a.clone_elements().iter().filter_map(numbers_of).collect();
    if rows.len() != expected {
        return Err(format!("expected {} embeddings, got {}", expected, rows.len()));
    }
    Ok(rows)
}

fn embed_remote(opts: &SharedDict, provider: &str, texts: &[String]) -> Result<Vec<Vec<f64>>, String> {
    let timeout = opt_num(opts, "timeout").unwrap_or(30.0);
    let retries = opt_num(opts, "retries").unwrap_or(0.0).clamp(0.0, 10.0) as u32;
    let compat = matches!(provider, "openai_compatible" | "compat");
    let ollama = provider == "ollama";
    let base = match opt_str(opts, "base_url") {
        Some(b) => b.trim_end_matches('/').to_string(),
        None if ollama => "http://127.0.0.1:11434".to_string(),
        None if compat => return Err("provider openai_compatible needs a base_url".to_string()),
        None => "https://api.openai.com/v1".to_string(),
    };
    let default_env = if compat || ollama { None } else { Some("OPENAI_API_KEY") };
    let key = resolve_key(opts, default_env)?;
    if key.is_none() && !ollama && !compat && opt_str(opts, "base_url").is_none() {
        return Err("no API key: pass key, key_env, or set OPENAI_API_KEY".to_string());
    }
    let model = opt_str(opts, "model").unwrap_or_else(|| if ollama { "nomic-embed-text".to_string() } else { "text-embedding-3-small".to_string() });
    let batch = opt_num(opts, "batch_size").unwrap_or(64.0).max(1.0) as usize;
    let mut all: Vec<Vec<f64>> = Vec::with_capacity(texts.len());
    for group in texts.chunks(batch) {
        let input: Vec<String> = group.iter().map(|t| json_str(t)).collect();
        let (url, body) = if ollama {
            (format!("{}/api/embed", base), format!("{{\"model\":{},\"input\":[{}]}}", json_str(&model), input.join(",")))
        } else {
            let dims = match opt_num(opts, "dim") {
                Some(d) if d >= 1.0 && !compat => format!(",\"dimensions\":{}", d as u64),
                _ => String::new(),
            };
            (format!("{}/embeddings", base), format!("{{\"model\":{},\"input\":[{}]{}}}", json_str(&model), input.join(","), dims))
        };
        let mut spec = ReqSpec::new("POST", &url);
        spec.headers.push(("Content-Type".to_string(), "application/json".to_string()));
        if let Some(k) = &key {
            spec.headers.push(("Authorization".to_string(), format!("Bearer {}", k)));
        }
        spec.body = Some(body);
        spec.timeout_s = timeout;
        spec.retries = retries;
        spec.insecure = matches!(opts.get("insecure"), Some(Value::Bool(true)));
        spec.allow_http = matches!(opts.get("allow_http"), Some(Value::Bool(true)));
        let resp = exec(&spec)?;
        if !(200..300).contains(&resp.status) {
            let snippet: String = resp.body.chars().take(200).collect();
            return Err(format!("HTTP {} from {}: {}", resp.status, base, snippet));
        }
        let rows = if ollama { parse_ollama_embeddings(&resp.body, group.len())? } else { parse_openai_embeddings(&resp.body, group.len())? };
        all.extend(rows);
    }
    Ok(all)
}

/// Embed `texts` according to `opts`.
pub fn embed_many(opts: &SharedDict, texts: &[String]) -> Result<Vec<Vec<f64>>, String> {
    let provider = opt_str(opts, "provider").unwrap_or_else(|| "openai".to_string()).to_lowercase();
    match provider.as_str() {
        "mock" => {
            let dim = opt_num(opts, "dim").unwrap_or(128.0) as usize;
            if dim == 0 || dim > 65_536 {
                return Err("dim must be between 1 and 65536".to_string());
            }
            Ok(texts.iter().map(|t| mock_embed(t, dim)).collect())
        }
        "openai" | "openai_compatible" | "compat" | "ollama" => embed_remote(opts, &provider, texts),
        other => Err(format!("provider {} is not supported (openai, ollama, openai_compatible, mock)", other)),
    }
}

/// `llm_embed(opts, text)` -> vector. Returns `None` for the legacy 4-argument form.
pub fn llm_embed_opts(args: &[Value]) -> Option<Result<Value, VmError>> {
    match args {
        [Value::Dict(opts), Value::Str(text)] => Some((|| {
            crate::security::require_llm()?;
            let mut rows = embed_many(opts, std::slice::from_ref(text)).map_err(|e| VmError::runtime_error(format!("llm_embed: {}", e)))?;
            Ok(vec_value(&rows.remove(0)))
        })()),
        _ => None,
    }
}

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    Some(match name {
        "mock_embed" => (|| match args {
            [Value::Str(t), dim] => match dim {
                Value::Number(n) if *n >= 1.0 && *n <= 65_536.0 && n.fract() == 0.0 => Ok(vec_value(&mock_embed(t, *n as usize))),
                Value::Integer(n) if (1..=65_536).contains(n) => Ok(vec_value(&mock_embed(t, *n as usize))),
                _ => err("mock_embed(text, dim): dim must be a whole number from 1 to 65536"),
            },
            _ => err("mock_embed(text, dim) expects a string and a number"),
        })(),
        "llm_embed_batch" => (|| match args {
            [Value::Dict(opts), Value::Array(texts)] => {
                crate::security::require_llm()?;
                let mut list = Vec::new();
                for t in texts.clone_elements() {
                    match t {
                        Value::Str(s) => list.push(s),
                        other => return err(format!("llm_embed_batch(): texts must be strings, got {}", other.type_name())),
                    }
                }
                let rows = embed_many(opts, &list).map_err(|e| VmError::runtime_error(format!("llm_embed_batch: {}", e)))?;
                Ok(Value::Array(SharedArray::new(rows.iter().map(|r| vec_value(r)).collect())))
            }
            _ => err("llm_embed_batch(opts, texts) expects an options dict and an array of strings"),
        })(),
        _ => return None,
    })
}

pub const NAMES: &[&str] = &["mock_embed", "llm_embed_batch"];

#[cfg(test)]
mod tests {
    use super::*;

    fn cos(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    #[test]
    fn mock_embed_is_deterministic_and_normalized() {
        let a = mock_embed("The quick brown fox", 64);
        let b = mock_embed("The quick brown fox", 64);
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert!((cos(&a, &a) - 1.0).abs() < 1e-9);
        assert!(mock_embed("", 8).iter().all(|x| *x == 0.0));
        // case and punctuation do not matter
        assert_eq!(mock_embed("Hello, WORLD!", 32), mock_embed("hello world", 32));
    }

    #[test]
    fn shared_words_score_higher_than_unrelated_text() {
        let q = mock_embed("how do I reset my password", 256);
        let near = mock_embed("To reset your password open the settings page", 256);
        let far = mock_embed("The cafeteria serves lunch from noon", 256);
        assert!(cos(&q, &near) > cos(&q, &far) + 0.1, "{} vs {}", cos(&q, &near), cos(&q, &far));
    }

    #[test]
    fn parses_openai_responses_in_index_order_and_errors() {
        let body = r#"{"data":[{"index":1,"embedding":[3,4]},{"index":0,"embedding":[1,2]}]}"#;
        assert_eq!(parse_openai_embeddings(body, 2).unwrap(), vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
        assert!(parse_openai_embeddings(body, 3).is_err());
        let e = parse_openai_embeddings(r#"{"error":{"message":"bad key"}}"#, 1).unwrap_err();
        assert!(e.contains("bad key"));
        assert!(parse_openai_embeddings("not json", 1).is_err());
        assert_eq!(parse_ollama_embeddings(r#"{"embeddings":[[1,2]]}"#, 1).unwrap(), vec![vec![1.0, 2.0]]);
    }

    #[test]
    fn key_resolution_prefers_explicit_key_then_named_env_var() {
        let d = SharedDict::from_pairs(vec![("key".to_string(), Value::Str("direct".to_string()))]);
        assert_eq!(resolve_key(&d, Some("KILLER_TEST_NOPE")).unwrap().as_deref(), Some("direct"));
        std::env::set_var("KILLER_EMBED_TEST_KEY", "from-env");
        let d = SharedDict::from_pairs(vec![("key_env".to_string(), Value::Str("KILLER_EMBED_TEST_KEY".to_string()))]);
        assert_eq!(resolve_key(&d, None).unwrap().as_deref(), Some("from-env"));
        let d = SharedDict::from_pairs(vec![("key_env".to_string(), Value::Str("KILLER_EMBED_UNSET_VAR".to_string()))]);
        let e = resolve_key(&d, None).unwrap_err();
        assert!(e.contains("KILLER_EMBED_UNSET_VAR") && !e.contains("from-env"));
    }
}
