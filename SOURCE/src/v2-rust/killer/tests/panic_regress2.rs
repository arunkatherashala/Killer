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
