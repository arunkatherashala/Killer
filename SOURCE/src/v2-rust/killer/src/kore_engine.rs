//! KORE FFI bridge — Killer ↔ kore-ffi static library.
//!
//! All exported functions are always present (either real or stub).
//! Real implementation is gated on `#[cfg(feature = "kore-engine")]`.
//! Without the feature the functions return a clear error string so
//! `cargo check` / `cargo test --lib` always pass.

#![allow(unsafe_code)]

use std::sync::{Mutex, OnceLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

// ── ID counters ───────────────────────────────────────────────────────────────

static SESSION_COUNTER: AtomicI64 = AtomicI64::new(1);
static MODEL_COUNTER:   AtomicI64 = AtomicI64::new(1);

fn sessions() -> &'static Mutex<HashMap<i64, usize>> {
    static S: OnceLock<Mutex<HashMap<i64, usize>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashMap::new()))
}

fn models() -> &'static Mutex<HashMap<i64, usize>> {
    static M: OnceLock<Mutex<HashMap<i64, usize>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

// ═════════════════════════════════════════════════════════════════════════════
// Real implementation (requires `kore-engine` feature + kore_ffi static lib)
// ═════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "kore-engine")]
mod ffi {
    use std::os::raw::{c_char, c_double, c_int};

    /// Opaque session type.
    #[repr(C)]
    pub struct KoreSession { _priv: [u8; 0] }

    /// Opaque model type.
    #[repr(C)]
    pub struct KoreModel { _priv: [u8; 0] }

    extern "C" {
        pub fn kore_session_new() -> *mut KoreSession;
        pub fn kore_session_free(ptr: *mut KoreSession);
        pub fn kore_session_load_csv(
            sess:  *mut KoreSession,
            table: *const c_char,
            path:  *const c_char,
        ) -> c_int;
        pub fn kore_session_query(
            sess: *mut KoreSession,
            sql:  *const c_char,
        ) -> *mut c_char;
        pub fn kore_session_row_count(
            sess:  *const KoreSession,
            table: *const c_char,
        ) -> i64;
        pub fn kore_free_string(s: *mut c_char);

        pub fn kore_model_new(model_type: c_int, param1: c_int, param2: c_int) -> *mut KoreModel;
        pub fn kore_model_free(ptr: *mut KoreModel);
        pub fn kore_model_fit(
            model:  *mut KoreModel,
            x_flat: *const c_double,
            n_rows: u64,
            n_cols: u64,
            y:      *const c_double,
        ) -> c_int;
        pub fn kore_model_predict(
            model:  *const KoreModel,
            x_flat: *const c_double,
            n_rows: u64,
            n_cols: u64,
            out:    *mut c_double,
        ) -> c_int;
    }
}

// ─── Session wrappers ─────────────────────────────────────────────────────────

/// Create a new SQL session. Returns an opaque integer handle.
pub fn session_new() -> i64 {
    #[cfg(feature = "kore-engine")]
    {
        let ptr = unsafe { ffi::kore_session_new() };
        if ptr.is_null() { return -1; }
        let id = SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
        sessions().lock().unwrap().insert(id, ptr as usize);
        id
    }
    #[cfg(not(feature = "kore-engine"))]
    { -1 }
}

/// Load a CSV file into the session as a named table.
pub fn session_load_csv(id: i64, table: &str, path: &str) -> Result<(), String> {
    #[cfg(feature = "kore-engine")]
    {
        let ptr = *sessions().lock().unwrap().get(&id)
            .ok_or_else(|| format!("kore_session: unknown id {id}"))? as *mut ffi::KoreSession;
        let t = std::ffi::CString::new(table).map_err(|e| e.to_string())?;
        let p = std::ffi::CString::new(path).map_err(|e| e.to_string())?;
        let rc = unsafe { ffi::kore_session_load_csv(ptr, t.as_ptr(), p.as_ptr()) };
        if rc == 0 { Ok(()) } else { Err(format!("kore_session_load_csv failed (rc={rc})")) }
    }
    #[cfg(not(feature = "kore-engine"))]
    { Err(kore_not_available()) }
}

/// Run a SQL query and return the result as a JSON string.
pub fn session_query(id: i64, sql: &str) -> Result<String, String> {
    #[cfg(feature = "kore-engine")]
    {
        let ptr = *sessions().lock().unwrap().get(&id)
            .ok_or_else(|| format!("kore_session: unknown id {id}"))? as *mut ffi::KoreSession;
        let s = std::ffi::CString::new(sql).map_err(|e| e.to_string())?;
        let raw = unsafe { ffi::kore_session_query(ptr, s.as_ptr()) };
        if raw.is_null() {
            return Err("kore_session_query returned null".into());
        }
        let result = unsafe { std::ffi::CStr::from_ptr(raw).to_string_lossy().into_owned() };
        unsafe { ffi::kore_free_string(raw); }
        Ok(result)
    }
    #[cfg(not(feature = "kore-engine"))]
    { Err(kore_not_available()) }
}

/// Return the row count of a registered table, or -1 if not found.
pub fn session_row_count(id: i64, table: &str) -> i64 {
    #[cfg(feature = "kore-engine")]
    {
        let guard = sessions().lock().unwrap();
        let ptr = match guard.get(&id) {
            Some(&p) => p as *const ffi::KoreSession,
            None     => return -1,
        };
        let t = match std::ffi::CString::new(table) { Ok(c) => c, Err(_) => return -1 };
        unsafe { ffi::kore_session_row_count(ptr, t.as_ptr()) }
    }
    #[cfg(not(feature = "kore-engine"))]
    { -1 }
}

/// Free a session and remove it from the registry.
pub fn session_free(id: i64) {
    #[cfg(feature = "kore-engine")]
    {
        if let Some(ptr) = sessions().lock().unwrap().remove(&id) {
            unsafe { ffi::kore_session_free(ptr as *mut ffi::KoreSession); }
        }
    }
    #[cfg(not(feature = "kore-engine"))]
    { let _ = id; }
}

// ─── Model wrappers ───────────────────────────────────────────────────────────

/// Create a new ML model.
/// model_type: 0=RF-reg  1=RF-clf  2=GBM  3=LinReg  4=Logistic  5=KNN-reg  6=KNN-clf  7=SVM
pub fn model_new(model_type: i32, param1: i32, param2: i32) -> i64 {
    #[cfg(feature = "kore-engine")]
    {
        let ptr = unsafe {
            ffi::kore_model_new(model_type as _, param1 as _, param2 as _)
        };
        if ptr.is_null() { return -1; }
        let id = MODEL_COUNTER.fetch_add(1, Ordering::Relaxed);
        models().lock().unwrap().insert(id, ptr as usize);
        id
    }
    #[cfg(not(feature = "kore-engine"))]
    { -1 }
}

/// Fit a model on training data.
pub fn model_fit(id: i64, x: &[Vec<f64>], y: &[f64]) -> Result<(), String> {
    #[cfg(feature = "kore-engine")]
    {
        let ptr = *models().lock().unwrap().get(&id)
            .ok_or_else(|| format!("kore_model: unknown id {id}"))? as *mut ffi::KoreModel;
        if x.is_empty() || y.is_empty() { return Err("empty training data".into()); }
        let n_rows = x.len() as u64;
        let n_cols = x[0].len() as u64;
        let x_flat: Vec<f64> = x.iter().flat_map(|r| r.iter().copied()).collect();
        let rc = unsafe {
            ffi::kore_model_fit(ptr, x_flat.as_ptr(), n_rows, n_cols, y.as_ptr())
        };
        if rc == 0 { Ok(()) } else { Err(format!("kore_model_fit failed (rc={rc})")) }
    }
    #[cfg(not(feature = "kore-engine"))]
    { Err(kore_not_available()) }
}

/// Predict from a fitted model.
pub fn model_predict(id: i64, x: &[Vec<f64>]) -> Result<Vec<f64>, String> {
    #[cfg(feature = "kore-engine")]
    {
        let ptr = *models().lock().unwrap().get(&id)
            .ok_or_else(|| format!("kore_model: unknown id {id}"))? as *const ffi::KoreModel;
        if x.is_empty() { return Ok(vec![]); }
        let n_rows = x.len() as u64;
        let n_cols = x[0].len() as u64;
        let x_flat: Vec<f64> = x.iter().flat_map(|r| r.iter().copied()).collect();
        let mut out = vec![0.0f64; x.len()];
        let rc = unsafe {
            ffi::kore_model_predict(ptr, x_flat.as_ptr(), n_rows, n_cols, out.as_mut_ptr())
        };
        if rc == 0 { Ok(out) } else { Err(format!("kore_model_predict failed (rc={rc})")) }
    }
    #[cfg(not(feature = "kore-engine"))]
    { Err(kore_not_available()) }
}

/// Free a model and remove it from the registry.
pub fn model_free(id: i64) {
    #[cfg(feature = "kore-engine")]
    {
        if let Some(ptr) = models().lock().unwrap().remove(&id) {
            unsafe { ffi::kore_model_free(ptr as *mut ffi::KoreModel); }
        }
    }
    #[cfg(not(feature = "kore-engine"))]
    { let _ = id; }
}

// ─── JSON row parser ──────────────────────────────────────────────────────────

/// Parse a JSON array-of-objects string into rows of string values.
/// Field order within each row matches insertion order of the first object's keys.
pub fn parse_json_rows(json: &str) -> Vec<Vec<String>> {
    // Minimal hand-rolled parser — no serde/json dependency needed.
    // Falls back to an empty vec on malformed input.
    parse_json_rows_impl(json).unwrap_or_default()
}

fn parse_json_rows_impl(json: &str) -> Option<Vec<Vec<String>>> {
    let json = json.trim();
    if !json.starts_with('[') { return None; }
    // Use stdlib JSON-like splitting — simple but correct for the flat objects kore returns.
    // We rely on the fact that kore serialises values without nested objects.
    let mut rows = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    let mut keys_set = false;

    // Strip outer brackets, split objects by "},"
    let inner = &json[1..json.len().saturating_sub(1)];
    for obj_raw in split_json_objects(inner) {
        let obj_raw = obj_raw.trim().trim_matches(|c| c == '{' || c == '}');
        let fields = split_json_fields(obj_raw);
        if !keys_set {
            for (k, _) in &fields { keys.push(k.clone()); }
            keys_set = true;
        }
        let mut row: Vec<String> = keys.iter().map(|_| String::new()).collect();
        for (k, v) in fields {
            if let Some(pos) = keys.iter().position(|x| x == &k) {
                row[pos] = v;
            }
        }
        rows.push(row);
    }
    Some(rows)
}

/// Split a JSON array body into individual object strings.
fn split_json_objects(s: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    result.push(&s[start..=i]);
                    start = i + 1;
                }
            }
            _ => {}
        }
    }
    result
}

/// Split a JSON object body (no braces) into (key, value) pairs.
fn split_json_fields(s: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();
    let s = s.trim();
    if s.is_empty() { return fields; }

    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        // skip whitespace and commas
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b',' || bytes[i] == b'\n' || bytes[i] == b'\r' || bytes[i] == b'\t') {
            i += 1;
        }
        if i >= bytes.len() { break; }
        // key (must be quoted)
        if bytes[i] != b'"' { i += 1; continue; }
        i += 1;
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'"' { i += 1; }
        let key = s[key_start..i].to_string();
        i += 1; // closing "
        // colon
        while i < bytes.len() && bytes[i] != b':' { i += 1; }
        i += 1;
        // value
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') { i += 1; }
        let val = if i < bytes.len() && bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' { i += 1; } // skip escaped char
                i += 1;
            }
            let v = s[start..i].to_string();
            i += 1; // closing "
            v
        } else {
            let start = i;
            while i < bytes.len() && bytes[i] != b',' && bytes[i] != b'}' { i += 1; }
            s[start..i].trim().to_string()
        };
        fields.push((key, val));
    }
    fields
}

// ─── Internal ─────────────────────────────────────────────────────────────────

#[cold]
fn kore_not_available() -> String {
    "kore engine not available — build with feature 'kore-engine'".into()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_rows_empty() {
        let rows = parse_json_rows("[]");
        assert!(rows.is_empty());
    }

    #[test]
    fn test_parse_json_rows_basic() {
        let json = r#"[{"name":"Alice","age":"30"},{"name":"Bob","age":"25"}]"#;
        let rows = parse_json_rows(json);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0], "Alice");
        assert_eq!(rows[0][1], "30");
        assert_eq!(rows[1][0], "Bob");
    }

    #[test]
    fn test_parse_json_rows_numeric() {
        let json = r#"[{"x":1.5,"y":2}]"#;
        let rows = parse_json_rows(json);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "1.5");
        assert_eq!(rows[0][1], "2");
    }
}
