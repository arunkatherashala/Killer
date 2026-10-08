//! Dense vector store and vector math builtins.
//!
//! `vec_new(dim)` returns a handle (a dict `{type: "vec_store", id, dim}`); the data lives in a
//! process-wide registry behind a `Mutex`, so handles are valid on every thread and no Killer
//! value is ever stored there (metadata is kept as JSON text). Free a store with `vec_free`.
//!
//! Builtins: vec_new, vec_add, vec_search, vec_get, vec_delete, vec_len, vec_clear, vec_free,
//! vec_save, vec_load, cosine, dot, norm, top_k.
//!
//! File format (little endian): "KVEC1\n", u32 dim, u64 count, then per entry
//! u32 id_len, id bytes, u32 meta_len, meta JSON bytes, dim x f32. The loader validates every
//! length against the bytes that remain, so a corrupt or hostile file cannot over-allocate.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::error::VmError;
use crate::inference::vector_store::cosine_similarity;
use crate::value::{SharedArray, SharedDict, Value};

const MAGIC: &[u8] = b"KVEC1\n";
const MAX_DIM: usize = 65_536;

struct Entry {
    id: String,
    vec: Vec<f32>,
    meta: String, // JSON text
}

struct Store {
    dim: usize,
    entries: Vec<Entry>,
    index: HashMap<String, usize>,
}

static REGISTRY: Mutex<Option<HashMap<u64, Store>>> = Mutex::new(None);
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn err<T>(msg: impl Into<String>) -> Result<T, VmError> {
    Err(VmError::runtime_error(msg.into()))
}

fn with_store<R>(id: u64, f: impl FnOnce(&mut Store) -> Result<R, VmError>) -> Result<R, VmError> {
    let mut g = REGISTRY.lock().unwrap_or_else(|p| p.into_inner());
    match g.as_mut().and_then(|m| m.get_mut(&id)) {
        Some(s) => f(s),
        None => err("vector store handle is not valid (freed, or never created)"),
    }
}

fn register(store: Store) -> Value {
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dim = store.dim;
    REGISTRY.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(HashMap::new).insert(id, store);
    Value::Dict(SharedDict::from_pairs(vec![
        ("type".to_string(), Value::Str("vec_store".to_string())),
        ("id".to_string(), Value::Number(id as f64)),
        ("dim".to_string(), Value::Number(dim as f64)),
    ]))
}

fn handle_id(v: &Value, who: &str) -> Result<u64, VmError> {
    if let Value::Dict(d) = v {
        if let (Some(Value::Str(t)), Some(Value::Number(id))) = (d.get("type"), d.get("id")) {
            if t == "vec_store" {
                return Ok(id as u64);
            }
        }
    }
    err(format!("{}() expects a store from vec_new()/vec_load()", who))
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => Some(*n),
        Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

fn floats(v: &Value, who: &str) -> Result<Vec<f32>, VmError> {
    match v {
        Value::Array(a) => a
            .clone_elements()
            .iter()
            .map(|x| match num(x) {
                Some(n) if n.is_finite() => Ok(n as f32),
                Some(_) => err(format!("{}(): vector contains NaN or infinity", who)),
                None => err(format!("{}(): vector elements must be numbers, got {}", who, x.type_name())),
            })
            .collect(),
        other => err(format!("{}() expects an array of numbers, got {}", who, other.type_name())),
    }
}

fn to_array(v: &[f32]) -> Value {
    Value::Array(SharedArray::new(v.iter().map(|x| Value::Number(*x as f64)).collect()))
}

fn id_string(v: &Value, who: &str) -> Result<String, VmError> {
    match v {
        Value::Str(s) => Ok(s.clone()),
        Value::Number(n) if n.fract() == 0.0 => Ok(format!("{}", *n as i64)),
        Value::Integer(i) => Ok(i.to_string()),
        other => err(format!("{}(): id must be a string or whole number, got {}", who, other.type_name())),
    }
}

fn meta_to_value(meta: &str) -> Value {
    crate::json_lang::parse(meta).unwrap_or(Value::Null)
}

/// Does `meta` contain every key/value pair of `filter`? (equality on top-level keys)
fn matches_filter(meta: &Value, filter: &SharedDict) -> bool {
    let Value::Dict(m) = meta else { return filter.is_empty() };
    filter.iter().all(|(k, want)| match m.get(&k) {
        Some(have) => values_equal(&have, &want),
        None => false,
    })
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (num(a), num(b)) {
        (Some(x), Some(y)) => x == y,
        _ => match (a, b) {
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Null, Value::Null) => true,
            // anything else (arrays, dicts) compares by JSON text
            _ => match (crate::json_lang::stringify(a, 0), crate::json_lang::stringify(b, 0)) {
                (Ok(x), Ok(y)) => x == y,
                _ => false,
            },
        },
    }
}

// --- math -----------------------------------------------------------------------

fn pair(args: &[Value], who: &str) -> Result<(Vec<f64>, Vec<f64>), VmError> {
    let get = |v: &Value| -> Result<Vec<f64>, VmError> { Ok(floats(v, who)?.into_iter().map(|x| x as f64).collect()) };
    match args {
        [a, b] => {
            let (a, b) = (get(a)?, get(b)?);
            if a.len() != b.len() {
                return err(format!("{}(): vectors differ in length ({} vs {})", who, a.len(), b.len()));
            }
            Ok((a, b))
        }
        _ => err(format!("{}(a, b) expects two arrays", who)),
    }
}

pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

pub fn norm(a: &[f64]) -> f64 {
    dot(a, a).sqrt()
}

pub fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let (na, nb) = (norm(a), norm(b));
    if na < 1e-12 || nb < 1e-12 { 0.0 } else { dot(a, b) / (na * nb) }
}

/// Indices of the `k` largest scores, best first (ties keep the earlier index first).
pub fn top_k_indices(scores: &[f64], k: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..scores.len()).filter(|&i| !scores[i].is_nan()).collect();
    idx.sort_by(|&a, &b| scores[b].partial_cmp(&scores[a]).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(&b)));
    idx.truncate(k);
    idx
}

// --- persistence ------------------------------------------------------------------

fn serialize(s: &Store) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&(s.dim as u32).to_le_bytes());
    out.extend_from_slice(&(s.entries.len() as u64).to_le_bytes());
    for e in &s.entries {
        out.extend_from_slice(&(e.id.len() as u32).to_le_bytes());
        out.extend_from_slice(e.id.as_bytes());
        out.extend_from_slice(&(e.meta.len() as u32).to_le_bytes());
        out.extend_from_slice(e.meta.as_bytes());
        for x in &e.vec {
            out.extend_from_slice(&x.to_le_bytes());
        }
    }
    out
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if n > self.b.len() - self.pos {
            return Err("file is truncated or corrupt".to_string());
        }
        let s = &self.b[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

fn deserialize(bytes: &[u8]) -> Result<Store, String> {
    if !bytes.starts_with(MAGIC) {
        return Err("not a Killer vector store file".to_string());
    }
    let mut r = Reader { b: bytes, pos: MAGIC.len() };
    let dim = r.u32()? as usize;
    if dim == 0 || dim > MAX_DIM {
        return Err("invalid dimension".to_string());
    }
    let count = r.u64()?;
    let mut store = Store { dim, entries: Vec::new(), index: HashMap::new() };
    for _ in 0..count {
        let idl = r.u32()? as usize;
        let id = String::from_utf8(r.take(idl)?.to_vec()).map_err(|_| "id is not UTF-8".to_string())?;
        let ml = r.u32()? as usize;
        let meta = String::from_utf8(r.take(ml)?.to_vec()).map_err(|_| "metadata is not UTF-8".to_string())?;
        let raw = r.take(dim.checked_mul(4).ok_or("bad dimension")?)?;
        let vec: Vec<f32> = raw.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
        if store.index.insert(id.clone(), store.entries.len()).is_some() {
            return Err("duplicate id in file".to_string());
        }
        store.entries.push(Entry { id, vec, meta });
    }
    if r.pos != bytes.len() {
        return Err("trailing bytes after the last entry".to_string());
    }
    Ok(store)
}

// --- builtins -----------------------------------------------------------------------

pub fn call(name: &str, args: &[Value]) -> Option<Result<Value, VmError>> {
    Some(match name {
        "vec_new" => (|| match args {
            [d] => match num(d) {
                Some(n) if n >= 1.0 && n.fract() == 0.0 && (n as usize) <= MAX_DIM => {
                    Ok(register(Store { dim: n as usize, entries: Vec::new(), index: HashMap::new() }))
                }
                _ => err(format!("vec_new(dim): dim must be a whole number from 1 to {}", MAX_DIM)),
            },
            _ => err("vec_new(dim) expects 1 argument"),
        })(),
        "vec_add" => (|| {
            if args.len() < 3 || args.len() > 4 {
                return err("vec_add(store, id, vector[, meta]) expects 3 or 4 arguments");
            }
            let sid = handle_id(&args[0], "vec_add")?;
            let id = id_string(&args[1], "vec_add")?;
            let vec = floats(&args[2], "vec_add")?;
            let meta = match args.get(3) {
                None | Some(Value::Null) => "null".to_string(),
                Some(m) => crate::json_lang::stringify(m, 0).map_err(VmError::runtime_error)?,
            };
            with_store(sid, |s| {
                if vec.len() != s.dim {
                    return err(format!("vec_add(): vector has {} elements but the store dimension is {}", vec.len(), s.dim));
                }
                match s.index.get(&id) {
                    Some(&i) => s.entries[i] = Entry { id, vec, meta },
                    None => {
                        s.index.insert(id.clone(), s.entries.len());
                        s.entries.push(Entry { id, vec, meta });
                    }
                }
                Ok(Value::Number(s.entries.len() as f64))
            })
        })(),
        "vec_search" => (|| {
            if args.len() < 3 || args.len() > 4 {
                return err("vec_search(store, query, k[, filter]) expects 3 or 4 arguments");
            }
            let sid = handle_id(&args[0], "vec_search")?;
            let q = floats(&args[1], "vec_search")?;
            let k = match num(&args[2]) {
                Some(n) if n >= 0.0 => n as usize,
                _ => return err("vec_search(): k must be a non-negative number"),
            };
            let filter = match args.get(3) {
                None | Some(Value::Null) => None,
                Some(Value::Dict(d)) => Some(d.clone()),
                Some(_) => return err("vec_search(): filter must be a dict"),
            };
            let hits: Vec<(f32, String, String)> = with_store(sid, |s| {
                if q.len() != s.dim {
                    return err(format!("vec_search(): query has {} elements but the store dimension is {}", q.len(), s.dim));
                }
                let mut scored: Vec<(f32, usize)> = Vec::new();
                for (i, e) in s.entries.iter().enumerate() {
                    if let Some(f) = &filter {
                        if !matches_filter(&meta_to_value(&e.meta), f) {
                            continue;
                        }
                    }
                    scored.push((cosine_similarity(&e.vec, &q), i));
                }
                scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1)));
                Ok(scored.into_iter().take(k).map(|(sc, i)| (sc, s.entries[i].id.clone(), s.entries[i].meta.clone())).collect())
            })?;
            Ok(Value::Array(SharedArray::new(
                hits.into_iter()
                    .map(|(score, id, meta)| {
                        Value::Dict(SharedDict::from_pairs(vec![
                            ("id".to_string(), Value::Str(id)),
                            ("score".to_string(), Value::Number(score as f64)),
                            ("meta".to_string(), meta_to_value(&meta)),
                        ]))
                    })
                    .collect(),
            )))
        })(),
        "vec_get" => (|| match args {
            [h, id] => {
                let sid = handle_id(h, "vec_get")?;
                let id = id_string(id, "vec_get")?;
                with_store(sid, |s| {
                    Ok(match s.index.get(&id) {
                        Some(&i) => Value::Dict(SharedDict::from_pairs(vec![
                            ("id".to_string(), Value::Str(id.clone())),
                            ("vector".to_string(), to_array(&s.entries[i].vec)),
                            ("meta".to_string(), meta_to_value(&s.entries[i].meta)),
                        ])),
                        None => Value::Null,
                    })
                })
            }
            _ => err("vec_get(store, id) expects 2 arguments"),
        })(),
        "vec_delete" => (|| match args {
            [h, id] => {
                let sid = handle_id(h, "vec_delete")?;
                let id = id_string(id, "vec_delete")?;
                with_store(sid, |s| match s.index.remove(&id) {
                    Some(i) => {
                        s.entries.swap_remove(i);
                        if i < s.entries.len() {
                            let moved = s.entries[i].id.clone();
                            s.index.insert(moved, i);
                        }
                        Ok(Value::Bool(true))
                    }
                    None => Ok(Value::Bool(false)),
                })
            }
            _ => err("vec_delete(store, id) expects 2 arguments"),
        })(),
        "vec_len" => (|| match args {
            [h] => with_store(handle_id(h, "vec_len")?, |s| Ok(Value::Number(s.entries.len() as f64))),
            _ => err("vec_len(store) expects 1 argument"),
        })(),
        "vec_clear" => (|| match args {
            [h] => with_store(handle_id(h, "vec_clear")?, |s| {
                s.entries.clear();
                s.index.clear();
                Ok(Value::Null)
            }),
            _ => err("vec_clear(store) expects 1 argument"),
        })(),
        "vec_free" => (|| match args {
            [h] => {
                let id = handle_id(h, "vec_free")?;
                let removed = REGISTRY.lock().unwrap_or_else(|p| p.into_inner()).as_mut().and_then(|m| m.remove(&id)).is_some();
                Ok(Value::Bool(removed))
            }
            _ => err("vec_free(store) expects 1 argument"),
        })(),
        "vec_save" => (|| match args {
            [h, Value::Str(path)] => {
                crate::security::require_file_write()?;
                let bytes = with_store(handle_id(h, "vec_save")?, |s| Ok(serialize(s)))?;
                // write to a sibling temp file first so a crash never leaves a half-written store
                let tmp = format!("{}.tmp", path);
                std::fs::write(&tmp, &bytes).and_then(|_| std::fs::rename(&tmp, path)).map_err(|e| {
                    let _ = std::fs::remove_file(&tmp);
                    VmError::runtime_error(format!("vec_save: {}", e))
                })?;
                Ok(Value::Number(bytes.len() as f64))
            }
            _ => err("vec_save(store, path) expects a store and a path string"),
        })(),
        "vec_load" => (|| match args {
            [Value::Str(path)] => {
                crate::security::require_file_read()?;
                let bytes = std::fs::read(path).map_err(|e| VmError::runtime_error(format!("vec_load: {}", e)))?;
                let store = deserialize(&bytes).map_err(|e| VmError::runtime_error(format!("vec_load: {}", e)))?;
                Ok(register(store))
            }
            _ => err("vec_load(path) expects a path string"),
        })(),
        "cosine" => pair(args, "cosine").map(|(a, b)| Value::Number(cosine(&a, &b))),
        "dot" => pair(args, "dot").map(|(a, b)| Value::Number(dot(&a, &b))),
        "norm" => (|| match args {
            [a] => Ok(Value::Number(norm(&floats(a, "norm")?.into_iter().map(|x| x as f64).collect::<Vec<_>>()))),
            _ => err("norm(a) expects 1 argument"),
        })(),
        "top_k" => (|| match args {
            [a, k] => {
                let scores: Vec<f64> = floats(a, "top_k")?.into_iter().map(|x| x as f64).collect();
                let k = match num(k) {
                    Some(n) if n >= 0.0 => n as usize,
                    _ => return err("top_k(scores, k): k must be a non-negative number"),
                };
                Ok(Value::Array(SharedArray::new(top_k_indices(&scores, k).into_iter().map(|i| Value::Number(i as f64)).collect())))
            }
            _ => err("top_k(scores, k) expects 2 arguments"),
        })(),
        _ => return None,
    })
}

pub const NAMES: &[&str] = &[
    "vec_new", "vec_add", "vec_search", "vec_get", "vec_delete", "vec_len", "vec_clear", "vec_free", "vec_save",
    "vec_load", "cosine", "dot", "norm", "top_k",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn arr(v: &[f64]) -> Value {
        Value::Array(SharedArray::new(v.iter().map(|x| Value::Number(*x)).collect()))
    }
    fn s(x: &str) -> Value {
        Value::Str(x.to_string())
    }
    fn n(x: f64) -> Value {
        Value::Number(x)
    }
    fn ok(name: &str, args: &[Value]) -> Value {
        call(name, args).unwrap().unwrap()
    }

    #[test]
    fn math_basics() {
        assert!((cosine(&[1.0, 0.0], &[1.0, 0.0]) - 1.0).abs() < 1e-12);
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-12);
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
        assert_eq!(dot(&[1.0, 2.0], &[3.0, 4.0]), 11.0);
        assert_eq!(norm(&[3.0, 4.0]), 5.0);
        assert_eq!(top_k_indices(&[0.1, 0.9, 0.5, 0.9], 3), vec![1, 3, 2]);
        assert_eq!(top_k_indices(&[], 3), Vec::<usize>::new());
        assert!(call("dot", &[arr(&[1.0]), arr(&[1.0, 2.0])]).unwrap().is_err());
    }

    #[test]
    fn add_search_filter_delete_upsert() {
        let st = ok("vec_new", &[n(2.0)]);
        let meta = |c: &str| Value::Dict(SharedDict::from_pairs(vec![("cat".to_string(), s(c))]));
        ok("vec_add", &[st.clone(), s("a"), arr(&[1.0, 0.0]), meta("x")]);
        ok("vec_add", &[st.clone(), s("b"), arr(&[0.9, 0.1]), meta("y")]);
        ok("vec_add", &[st.clone(), s("c"), arr(&[0.0, 1.0]), meta("x")]);
        assert_eq!(ok("vec_len", &[st.clone()]), n(3.0));
        let hits = match ok("vec_search", &[st.clone(), arr(&[1.0, 0.0]), n(2.0)]) { Value::Array(a) => a.clone_elements(), _ => panic!() };
        assert_eq!(hits.len(), 2);
        let first = match &hits[0] { Value::Dict(d) => d.get("id").unwrap(), _ => panic!() };
        assert_eq!(first, s("a"));
        // filter on metadata
        let f = Value::Dict(SharedDict::from_pairs(vec![("cat".to_string(), s("y"))]));
        let hits = match ok("vec_search", &[st.clone(), arr(&[0.0, 1.0]), n(5.0), f]) { Value::Array(a) => a.clone_elements(), _ => panic!() };
        assert_eq!(hits.len(), 1);
        // upsert keeps the count
        ok("vec_add", &[st.clone(), s("a"), arr(&[0.0, 1.0])]);
        assert_eq!(ok("vec_len", &[st.clone()]), n(3.0));
        // delete (swap_remove) keeps the index consistent
        assert_eq!(ok("vec_delete", &[st.clone(), s("a")]), Value::Bool(true));
        assert_eq!(ok("vec_delete", &[st.clone(), s("a")]), Value::Bool(false));
        assert!(matches!(ok("vec_get", &[st.clone(), s("c")]), Value::Dict(_)));
        assert!(matches!(ok("vec_get", &[st.clone(), s("b")]), Value::Dict(_)));
        // dimension mismatch is an error
        assert!(call("vec_add", &[st.clone(), s("z"), arr(&[1.0])]).unwrap().is_err());
        assert!(call("vec_search", &[st.clone(), arr(&[1.0, 2.0, 3.0]), n(1.0)]).unwrap().is_err());
        assert_eq!(ok("vec_free", &[st.clone()]), Value::Bool(true));
        assert!(call("vec_len", &[st]).unwrap().is_err());
    }

    #[test]
    fn save_and_load_round_trip_and_reject_corruption() {
        let st = ok("vec_new", &[n(3.0)]);
        ok("vec_add", &[st.clone(), s("one"), arr(&[1.0, 2.0, 3.0]), Value::Dict(SharedDict::from_pairs(vec![("k".to_string(), n(7.0))]))]);
        ok("vec_add", &[st.clone(), s("two"), arr(&[0.5, 0.5, 0.5])]);
        let path = std::env::temp_dir().join(format!("killer_vec_{}.kvec", std::process::id()));
        let p = path.to_string_lossy().to_string();
        ok("vec_save", &[st, s(&p)]);
        let loaded = ok("vec_load", &[s(&p)]);
        assert_eq!(ok("vec_len", &[loaded.clone()]), n(2.0));
        let hit = match ok("vec_get", &[loaded, s("one")]) { Value::Dict(d) => d, _ => panic!() };
        match hit.get("meta").unwrap() { Value::Dict(m) => assert_eq!(m.get("k").unwrap(), n(7.0)), _ => panic!() }
        // every truncation of the file must fail cleanly (no panic, no huge allocation)
        let bytes = std::fs::read(&path).unwrap();
        for cut in 0..bytes.len() {
            assert!(deserialize(&bytes[..cut]).is_err(), "cut {}", cut);
        }
        let mut hostile = bytes.clone();
        hostile[MAGIC.len() + 4..MAGIC.len() + 12].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(deserialize(&hostile).is_err());
        let _ = std::fs::remove_file(&path);
    }
}
