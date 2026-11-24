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
