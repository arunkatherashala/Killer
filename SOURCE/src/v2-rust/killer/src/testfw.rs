//! Test framework: `test "name" { ... }` blocks, `fn test_*()` discovery, `assert_eq` diffs,
//! `assert_throws`, per-test isolated results, name filtering, JUnit XML, `killer test`.
//!
//! How it works
//! * [`lower`] is a compiler source pass (hooked right after import resolution in
//!   `compile_killer_subset`, before `controlflow`). It rewrites each `test "name" {` header into
//!   `fn __ktest_<name>_<hash>() {` *on the same line*, so line numbers are unchanged and a plain
//!   `killer run` merely defines (and never calls) the test functions.
//! * [`discover`] finds the same blocks (and top-level zero-argument `fn test_*()` functions) in
//!   the original text.
//! * [`run_source`] appends a small harness (a results table plus one guarded call per selected
//!   test) and runs the file once in one VM. Each test runs inside `try/catch`, so a failing
//!   test never stops the others, and its result (name, ok, error, milliseconds) is recorded.
//! * Files named `test_*.killer` / `*_test.killer` without any test block or `test_` function
//!   keep the legacy behaviour of `src/test_runner.rs`: the whole file runs and passes when it
//!   finishes without an error.

use crate::value::Value;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------------------------
// Source pass and discovery

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
    /// Display name (`test "name"` or the function name).
    pub name: String,
    /// Name of the function that runs the test.
    pub func: String,
    /// 1-based line of the declaration.
    pub line: usize,
}

fn fnv(s: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn func_name_for(name: &str, occurrence: usize) -> String {
    let slug: String = name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).take(40).collect();
    if occurrence == 0 {
        format!("__ktest_{slug}_{:08x}", fnv(name))
    } else {
        format!("__ktest_{slug}_{:08x}_{occurrence}", fnv(name))
    }
}

/// `test "name" {` -> (name, byte length of the `test "name"` prefix after the indentation).
pub fn parse_test_header(line: &str) -> Option<(String, usize, usize)> {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    let after_kw = rest.strip_prefix("test")?;
    if !after_kw.starts_with(|c: char| c == ' ' || c == '\t') {
        return None;
    }
    let ws = after_kw.len() - after_kw.trim_start().len();
    let body = &after_kw[ws..];
    let quote = body.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let mut name = String::new();
    let mut escaped = false;
    let mut end = None;
    for (i, c) in body.char_indices().skip(1) {
        if escaped {
            name.push(match c {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == quote {
            end = Some(i + 1);
            break;
        } else {
            name.push(c);
        }
    }
    let end = end?;
    let tail = body[end..].trim_start();
    if !tail.starts_with('{') {
        return None;
    }
    Some((name, indent, indent + 4 + ws + end))
}

/// Rewrite `test "name" {` headers into function headers (one output line per input line).
pub fn lower(source: &str) -> String {
    let needs_prelude = source.contains("assert_throws(") && !source.contains("fn assert_throws");
    if !source.contains("test") && !needs_prelude {
        return source.to_string();
    }
    let mut seen: Vec<String> = Vec::new();
    let mut out = String::with_capacity(source.len());
    for (i, line) in source.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match parse_test_header(line) {
            Some((name, indent, prefix_end)) => {
                let occ = seen.iter().filter(|n| **n == name).count();
                seen.push(name.clone());
                out.push_str(&line[..indent]);
                out.push_str(&format!("fn {}()", func_name_for(&name, occ)));
                out.push_str(&line[prefix_end..]);
            }
            None => out.push_str(line),
        }
    }
    if needs_prelude {
        // appended after the user's code: no existing line moves
        out.push('\n');
        out.push_str(PRELUDE);
    }
    out
}

/// Tests defined in `source`, in file order.
pub fn discover(source: &str) -> Vec<TestCase> {
    let mut tests = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut depth = 0i64;
    for (i, line) in source.split('\n').enumerate() {
        let masked = crate::formatter::mask_code(line);
        if let Some((name, _, _)) = parse_test_header(line) {
            let occ = seen.iter().filter(|n| **n == name).count();
            seen.push(name.clone());
            tests.push(TestCase { func: func_name_for(&name, occ), name, line: i + 1 });
        } else if depth == 0 {
            let t = masked.trim();
            let t = t.strip_prefix("async ").unwrap_or(t);
            for kw in ["fn ", "kfn "] {
                if let Some(rest) = t.strip_prefix(kw) {
                    let name: String = rest.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    let after = rest.trim_start()[name.len()..].trim_start();
                    if name.starts_with("test_") && after.starts_with("()") {
                        tests.push(TestCase { func: name.clone(), name, line: i + 1 });
                    }
                }
            }
        }
        for c in masked.chars() {
            match c {
                '{' | '[' | '(' => depth += 1,
                '}' | ']' | ')' => depth = (depth - 1).max(0),
                _ => {}
            }
        }
    }
    tests
}

// ---------------------------------------------------------------------------------------------
// assert_eq message

fn repr(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")),
        Value::Array(a) => format!("[{}]", a.to_vec().iter().map(repr).collect::<Vec<_>>().join(", ")),
        Value::Dict(d) => format!("{{{}}}", d.iter().map(|(k, v)| format!("\"{}\": {}", k, repr(&v))).collect::<Vec<_>>().join(", ")),
        other => format!("{other}"),
    }
}

/// The failure message of `assert_eq(a, b)`: both sides plus where they first differ.
pub fn assert_eq_message(a: &Value, b: &Value) -> String {
    let mut msg = format!("assert_eq failed\n  left:  {}\n  right: {}", repr(a), repr(b));
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => {
            let (xs, ys): (Vec<char>, Vec<char>) = (x.chars().collect(), y.chars().collect());
            let i = xs.iter().zip(ys.iter()).take_while(|(p, q)| p == q).count();
            msg.push_str(&format!("\n  diff:  strings first differ at character {i}"));
            if xs.len() != ys.len() {
                msg.push_str(&format!(" (lengths {} and {})", xs.len(), ys.len()));
            }
            let at = |v: &Vec<char>| v.get(i).map_or("<end>".to_string(), |c| format!("{c:?}"));
            msg.push_str(&format!(": left {}, right {}", at(&xs), at(&ys)));
        }
        (Value::Array(x), Value::Array(y)) => {
            let (xs, ys) = (x.to_vec(), y.to_vec());
            if let Some(i) = (0..xs.len().min(ys.len())).find(|&i| xs[i] != ys[i]) {
                msg.push_str(&format!("\n  diff:  index {i}: left {}, right {}", repr(&xs[i]), repr(&ys[i])));
            }
            if xs.len() != ys.len() {
                msg.push_str(&format!("\n  diff:  lengths differ: left {}, right {}", xs.len(), ys.len()));
            }
        }
        (Value::Dict(x), Value::Dict(y)) => {
            for (k, v) in x.iter() {
                match y.get(&k) {
                    None => msg.push_str(&format!("\n  diff:  key \"{k}\" only on the left")),
                    Some(w) if w != v => msg.push_str(&format!("\n  diff:  key \"{k}\": left {}, right {}", repr(&v), repr(&w))),
                    _ => {}
                }
            }
            for (k, _) in y.iter() {
                if !x.contains_key(&k) {
                    msg.push_str(&format!("\n  diff:  key \"{k}\" only on the right"));
                }
            }
        }
        _ => {}
    }
    msg
}

// ---------------------------------------------------------------------------------------------
// Running

#[derive(Clone, Debug)]
pub struct TestResult {
    pub file: String,
    pub name: String,
    pub ok: bool,
    pub error: String,
    pub millis: u64,
}

const PRELUDE: &str = r#"
fn assert_throws(f, expected = "") {
  threw = false
  msg = ""
  try {
    f()
  } catch e {
    threw = true
    msg = str(e)
  }
  if not threw {
    throw "assert_throws failed: the function did not throw"
  }
  if expected != "" and not contains(msg, expected) {
    throw "assert_throws failed: error `" + msg + "` does not contain `" + expected + "`"
  }
  return "OK"
}
"#;

fn kstr(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

/// Run the selected tests of one source text; `Err` when the file itself fails to compile or its
/// top-level code throws (so no test could start).
pub fn run_source(source: &str, selected: &[TestCase]) -> Result<Vec<(String, bool, String, u64)>, String> {
    let mut program_text = String::from(source);
    if !program_text.ends_with('\n') {
        program_text.push('\n');
    }
    program_text.push_str(
        "__ktest_results = []\nfn __ktest_run(name, f) {\n  t0 = time_ms()\n  try {\n    f()\n    push(__ktest_results, {\"name\": name, \"ok\": true, \"error\": \"\", \"ms\": time_ms() - t0})\n  } catch e {\n    push(__ktest_results, {\"name\": name, \"ok\": false, \"error\": str(e), \"ms\": time_ms() - t0})\n  }\n}\n",
    );
    for t in selected {
        program_text.push_str(&format!("__ktest_run({}, {})\n", kstr(&t.name), t.func));
    }
    let src = program_text;
    let outcome = std::panic::catch_unwind(move || -> Result<Vec<(String, bool, String, u64)>, String> {
        let program = crate::compiler::compile_killer_default(&src).map_err(|e| e.to_string())?;
        let mut vm = crate::vm::VirtualMachine::new();
        vm.run(&program).map_err(|e| e.to_string())?;
        let globals = vm.get_globals();
        let mut out = Vec::new();
        if let Some(Value::Array(items)) = globals.get("__ktest_results") {
            for item in items.to_vec() {
                if let Value::Dict(d) = item {
                    let s = |k: &str| match d.get(k) {
                        Some(Value::Str(s)) => s,
                        _ => String::new(),
                    };
                    let ok = matches!(d.get("ok"), Some(Value::Bool(true)));
                    let ms = match d.get("ms") {
                        Some(Value::Number(n)) if n >= 0.0 => n as u64,
                        Some(Value::Integer(n)) if n >= 0 => n as u64,
                        _ => 0,
                    };
                    out.push((s("name"), ok, s("error"), ms));
                }
            }
        }
        Ok(out)
    });
    match outcome {
        Ok(r) => r,
        Err(_) => Err("internal error: the test run panicked".to_string()),
    }
}

fn is_legacy_test_file(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).map_or(false, |name| {
        name.ends_with(".killer") && (name.starts_with("test_") || name.starts_with("_test_") || name.ends_with("_test.killer"))
    })
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name == "target" || name.starts_with('.') || name == "node_modules" {
                continue;
            }
            collect_files(&p, out);
        } else if is_legacy_test_file(&p) {
            out.push(p);
        }
    }
}

/// Run every test of one file, honouring `filter` (a substring of `file::test name`).
pub fn run_file(path: &Path, filter: Option<&str>) -> Vec<TestResult> {
    let file = path.display().to_string();
    let source = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => return vec![TestResult { file: file.clone(), name: "(read)".into(), ok: false, error: format!("cannot read {file}: {e}"), millis: 0 }],
    };
    let tests = discover(&source);
    let started = std::time::Instant::now();
    let matches = |name: &str| filter.map_or(true, |f| f.is_empty() || format!("{file}::{name}").contains(f));
    if tests.is_empty() {
        // legacy file: the whole program is the test
        if !matches(&file) && !matches("") {
            return Vec::new();
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or(&file).to_string();
        let result = std::panic::catch_unwind(|| crate::run_killer_source(&source));
        let (ok, error) = match result {
            Ok(Ok(())) => (true, String::new()),
            Ok(Err(e)) => (false, e.to_string()),
            Err(_) => (false, "panic".to_string()),
        };
        return vec![TestResult { file, name, ok, error, millis: started.elapsed().as_millis() as u64 }];
    }
    let selected: Vec<TestCase> = tests.into_iter().filter(|t| matches(&t.name)).collect();
    if selected.is_empty() {
        return Vec::new();
    }
    match run_source(&source, &selected) {
        Ok(results) => results
            .into_iter()
            .map(|(name, ok, error, millis)| TestResult { file: file.clone(), name, ok, error, millis })
            .collect(),
        Err(e) => vec![TestResult { file, name: "(file)".into(), ok: false, error: e, millis: started.elapsed().as_millis() as u64 }],
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
        .chars().filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t')).collect()
}

/// JUnit XML for a run.
pub fn junit_xml(results: &[TestResult]) -> String {
    let mut files: Vec<&str> = Vec::new();
    for r in results {
        if !files.contains(&r.file.as_str()) {
            files.push(&r.file);
        }
    }
    let total_fail = results.iter().filter(|r| !r.ok).count();
    let total_ms: u64 = results.iter().map(|r| r.millis).sum();
    let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.push_str(&format!("<testsuites tests=\"{}\" failures=\"{}\" time=\"{:.3}\">\n", results.len(), total_fail, total_ms as f64 / 1000.0));
    for f in files {
        let rs: Vec<&TestResult> = results.iter().filter(|r| r.file == f).collect();
        let fails = rs.iter().filter(|r| !r.ok).count();
        let ms: u64 = rs.iter().map(|r| r.millis).sum();
        x.push_str(&format!(
            "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" time=\"{:.3}\">\n",
            xml_escape(f), rs.len(), fails, ms as f64 / 1000.0
        ));
        for r in rs {
            x.push_str(&format!("    <testcase classname=\"{}\" name=\"{}\" time=\"{:.3}\"", xml_escape(&r.file), xml_escape(&r.name), r.millis as f64 / 1000.0));
            if r.ok {
                x.push_str("/>\n");
            } else {
                let first = r.error.lines().next().unwrap_or("");
                x.push_str(&format!(">\n      <failure message=\"{}\">{}</failure>\n    </testcase>\n", xml_escape(first), xml_escape(&r.error)));
            }
        }
        x.push_str("  </testsuite>\n");
    }
    x.push_str("</testsuites>\n");
    x
}

/// `killer test [--junit FILE] [--list] [filter] [paths...]`. Returns the exit code.
pub fn cmd_test(args: &[String]) -> i32 {
    let mut junit: Option<String> = None;
    let mut list = false;
    let mut filter: Option<String> = None;
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--junit" => {
                i += 1;
                match args.get(i) {
                    Some(p) => junit = Some(p.clone()),
                    None => {
                        eprintln!("killer test: --junit needs a file name");
                        return 2;
                    }
                }
            }
            "--list" => list = true,
            "-h" | "--help" => {
                println!("usage: killer test [--junit FILE] [--list] [filter] [file-or-dir...]\n\nRuns `test \"name\" {{ }}` blocks, `fn test_*()` functions and legacy test_*.killer files.\nThe filter is a substring of `file::test name`.");
                return 0;
            }
            a => {
                let p = Path::new(a);
                if p.exists() {
                    paths.push(p.to_path_buf());
                } else {
                    filter = Some(a.to_string());
                }
            }
        }
        i += 1;
    }
    let mut files: Vec<PathBuf> = Vec::new();
    if paths.is_empty() {
        collect_files(Path::new("."), &mut files);
    }
    for p in &paths {
        if p.is_dir() {
            collect_files(p, &mut files);
        } else {
            files.push(p.clone());
        }
    }
    if files.is_empty() {
        println!("no test files found");
        return 0;
    }
    if list {
        for f in &files {
            if let Ok(src) = std::fs::read_to_string(f) {
                let tests = discover(&src);
                if tests.is_empty() {
                    println!("{}", f.display());
                }
                for t in tests {
                    println!("{}::{}", f.display(), t.name);
                }
            }
        }
        return 0;
    }
    let started = std::time::Instant::now();
    let mut all: Vec<TestResult> = Vec::new();
    for f in &files {
        let results = run_file(f, filter.as_deref());
        for r in &results {
            if r.ok {
                println!("  ok    {}::{} ({} ms)", r.file, r.name, r.millis);
            } else {
                println!("  FAIL  {}::{}", r.file, r.name);
                for l in r.error.lines() {
                    println!("          {l}");
                }
            }
        }
        all.extend(results);
    }
    let failed = all.iter().filter(|r| !r.ok).count();
    println!(
        "\n{} passed, {} failed, {} total in {:.2}s",
        all.len() - failed,
        failed,
        all.len(),
        started.elapsed().as_secs_f64()
    );
    if let Some(path) = junit {
        if let Err(e) = std::fs::write(&path, junit_xml(&all)) {
            eprintln!("killer test: cannot write {path}: {e}");
            return 2;
        }
    }
    if failed > 0 { 1 } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_parsing() {
        let (name, indent, end) = parse_test_header("  test \"adds two\" {").unwrap();
        assert_eq!((name.as_str(), indent), ("adds two", 2));
        assert_eq!(&"  test \"adds two\" {"[end..], " {");
        assert!(parse_test_header("test_something()").is_none());
        assert!(parse_test_header("testing \"x\" {").is_none());
        assert!(parse_test_header("test \"x\"").is_none());
    }

    #[test]
    fn lowering_keeps_lines() {
        let src = "x = 1\ntest \"a b\" {\n  assert_eq(x, 1)\n}\n";
        let out = lower(src);
        assert_eq!(out.lines().count(), src.lines().count());
        assert!(out.lines().nth(1).unwrap().starts_with("fn __ktest_a_b_"));
        assert!(out.lines().nth(1).unwrap().ends_with("() {"));
    }

    #[test]
    fn discovers_blocks_and_functions() {
        let src = "fn helper(x) {\n  return x\n}\nfn test_one() {\n  assert_eq(1, 1)\n}\ntest \"two\" {\n  assert_eq(2, 2)\n}\n";
        let t = discover(src);
        assert_eq!(t.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(), vec!["test_one", "two"]);
    }

    #[test]
    fn assert_eq_diff_is_readable() {
        let m = assert_eq_message(&Value::from(vec![Value::Number(1.0), Value::Number(2.0)]), &Value::from(vec![Value::Number(1.0), Value::Number(3.0)]));
        assert!(m.contains("left:  [1, 2]") && m.contains("right: [1, 3]") && m.contains("index 1"), "{m}");
        let m = assert_eq_message(&Value::Str("hello".into()), &Value::Str("help".into()));
        assert!(m.contains("character 3"), "{m}");
    }

    #[test]
    fn runs_tests_in_isolation() {
        let src = "fn double(x) {\n  return x * 2\n}\ntest \"passes\" {\n  assert_eq(double(2), 4)\n}\ntest \"fails\" {\n  assert_eq(double(2), 5)\n}\nfn thrower() {
  throw \"boom\"
}
test \"throws\" {\n  assert_throws(thrower, \"boom\")\n}\nfn test_plain() {\n  assert_eq(1, 1)\n}\n";
        let tests = discover(src);
        assert_eq!(tests.len(), 4);
        let results = run_source(src, &tests).unwrap();
        let by: std::collections::HashMap<_, _> = results.iter().map(|r| (r.0.clone(), r.1)).collect();
        assert_eq!(by["passes"], true);
        assert_eq!(by["fails"], false);
        assert_eq!(by["throws"], true);
        assert_eq!(by["test_plain"], true);
        assert!(results.iter().find(|r| r.0 == "fails").unwrap().2.contains("assert_eq failed"));
    }

    #[test]
    fn junit_is_escaped() {
        let x = junit_xml(&[
            TestResult { file: "a.killer".into(), name: "x<y".into(), ok: true, error: String::new(), millis: 5 },
            TestResult { file: "a.killer".into(), name: "bad".into(), ok: false, error: "oops & \"more\"\nline2".into(), millis: 1 },
        ]);
        assert!(x.contains("tests=\"2\" failures=\"1\""));
        assert!(x.contains("name=\"x&lt;y\"") && x.contains("&amp;") && x.contains("<failure message=\"oops &amp; &quot;more&quot;\">"));
    }
}
