//! Regression tests for reachable panics (release profile uses panic="abort").
use killer_native::builtin::BuiltinFunctions;
use killer_native::value::Value;

fn n(x: f64) -> Value {
    Value::Number(x)
}
fn s(t: &str) -> Value {
    Value::Str(t.to_string())
}
fn arr(v: Vec<Value>) -> Value {
    Value::from(v)
}

#[test]
fn enumerate_huge_start_does_not_overflow() {
    for start in [1e300, -1e300, f64::NAN, 9.3e18] {
        let r = BuiltinFunctions::call("enumerate", &[arr(vec![n(1.0), n(2.0)]), n(start)]);
        assert!(r.is_ok() || r.is_err());
    }
}

#[test]
fn json_pretty_huge_indent_is_clamped() {
    for ind in [1e300, 1e18, 1e9] {
        let r = BuiltinFunctions::call("json_pretty", &[s("{\"a\":[1]}"), n(ind)]).expect("ok");
        match r {
            Value::Str(t) => assert!(t.len() < 10_000, "len {}", t.len()),
            other => panic!("{:?}", other),
        }
    }
}

#[test]
fn json_pretty_deep_nesting_with_big_indent_terminates() {
    let t = std::time::Instant::now();
    let _ = BuiltinFunctions::call("json_pretty", &[s("[[1]]"), n(1e9)]);
    let deep = "[".repeat(100_000);
    assert!(BuiltinFunctions::call("json_pretty", &[s(&deep), n(2.0)]).is_err());
    assert!(t.elapsed().as_secs() < 20);
}

#[test]
fn to_yaml_huge_indent_is_clamped() {
    let mut m = std::collections::HashMap::new();
    m.insert("a".to_string(), n(1.0));
    let d = Value::Dict(killer_native::value::SharedDict::new(m));
    let r = BuiltinFunctions::call("to_yaml", &[d, n(1e300)]).expect("ok");
    match r {
        Value::Str(t) => assert!(t.len() < 1000),
        other => panic!("{:?}", other),
    }
}

fn run_bin(dir: &std::path::Path, file: &str) -> (Option<i32>, String) {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_killer"))
        .arg(file)
        .current_dir(dir)
        .output()
        .expect("spawn killer");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

#[test]
fn runtime_self_import_is_an_error_not_a_stack_overflow() {
    let dir = std::env::temp_dir().join(format!("killer_selfimp_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("x.killer"), "import 'x.killer'\nprint(1)\n").unwrap();
    std::fs::write(dir.join("a.killer"), "import 'b.killer'\n").unwrap();
    std::fs::write(dir.join("b.killer"), "import 'a.killer'\n").unwrap();
    for f in ["x.killer", "a.killer"] {
        let (code, text) = run_bin(&dir, f);
        assert!(!text.contains("overflowed") && !text.contains("panicked"), "{f}: {text}");
        assert!(text.to_lowercase().contains("circular import"), "{f}: {text}");
        assert_ne!(code, Some(0));
    }
    let _ = std::fs::remove_dir_all(&dir);
}
