//! Python-style builtins: sorted, sum, enumerate, all, any, zip
use std::collections::HashMap;

use killer_native::builtin::BuiltinFunctions;
use killer_native::value::Value;

fn n(x: f64) -> Value {
    Value::Number(x)
}
fn s(t: &str) -> Value {
    Value::Str(t.to_string())
}
fn arr(v: Vec<Value>) -> Value {
    Value::Array(v.into())
}

#[test]
fn sorted_numbers_and_reverse() {
    let a = arr(vec![n(3.0), n(1.0), n(2.0)]);
    let out = BuiltinFunctions::call("sorted", &[a.clone()]).unwrap();
    assert_eq!(out, arr(vec![n(1.0), n(2.0), n(3.0)]));
    let out2 = BuiltinFunctions::call("sorted", &[a, Value::Bool(true)]).unwrap();
    assert_eq!(out2, arr(vec![n(3.0), n(2.0), n(1.0)]));
}

#[test]
fn sorted_strings() {
    let a = arr(vec![s("b"), s("a")]);
    let out = BuiltinFunctions::call("sorted", &[a]).unwrap();
    assert_eq!(out, arr(vec![s("a"), s("b")]));
}

#[test]
fn sum_empty_and_values() {
    let out = BuiltinFunctions::call("sum", &[arr(vec![])]).unwrap();
    assert_eq!(out, n(0.0));
    let out2 = BuiltinFunctions::call("sum", &[arr(vec![n(2.0), n(3.0)])]).unwrap();
    assert_eq!(out2, n(5.0));
}

#[test]
fn enumerate_default_and_start() {
    let a = arr(vec![s("a"), s("b")]);
    let out = BuiltinFunctions::call("enumerate", &[a.clone()]).unwrap();
    assert_eq!(
        out,
        arr(vec![
            arr(vec![n(0.0), s("a")]),
            arr(vec![n(1.0), s("b")]),
        ])
    );
    let out2 = BuiltinFunctions::call("enumerate", &[a, n(10.0)]).unwrap();
    assert_eq!(
        out2,
        arr(vec![
            arr(vec![n(10.0), s("a")]),
            arr(vec![n(11.0), s("b")]),
        ])
    );
}

#[test]
fn all_any_empty() {
    assert_eq!(
        BuiltinFunctions::call("all", &[arr(vec![])]).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        BuiltinFunctions::call("any", &[arr(vec![])]).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        BuiltinFunctions::call("all", &[arr(vec![n(1.0), n(2.0)])]).unwrap(),
        Value::Bool(true)
    );
    assert_eq!(
        BuiltinFunctions::call("all", &[arr(vec![n(1.0), n(0.0)])]).unwrap(),
        Value::Bool(false)
    );
    assert_eq!(
        BuiltinFunctions::call("any", &[arr(vec![n(0.0), n(1.0)])]).unwrap(),
        Value::Bool(true)
    );
}

#[test]
fn zip_min_length() {
    let out = BuiltinFunctions::call(
        "zip",
        &[arr(vec![n(1.0), n(2.0)]), arr(vec![n(10.0)])],
    )
    .unwrap();
    assert_eq!(out, arr(vec![arr(vec![n(1.0), n(10.0)])]));
}

#[test]
fn dict_get_default() {
    let d = Value::Dict(Box::new(HashMap::from([("a".into(), n(1.0))])));
    let out = BuiltinFunctions::call("get", &[d.clone(), s("a")]).unwrap();
    assert_eq!(out, n(1.0));
    let out2 = BuiltinFunctions::call("get", &[d.clone(), s("missing")]).unwrap();
    assert_eq!(out2, Value::Null);
    let out3 = BuiltinFunctions::call("get", &[d, s("missing"), s("fallback")]).unwrap();
    assert_eq!(out3, s("fallback"));
}

#[test]
fn dict_setdefault_pair() {
    let d = Value::Dict(Box::new(HashMap::from([("a".into(), n(1.0))])));
    let out = BuiltinFunctions::call("setdefault", &[d.clone(), s("a"), n(99.0)]).unwrap();
    let Value::Array(parts) = out else { panic!("not array") };
    assert_eq!(parts.len(), 2);
    assert_eq!(parts.get(1).unwrap(), n(1.0));
    let out2 = BuiltinFunctions::call("setdefault", &[parts.get(0).unwrap(), s("b"), n(2.0)]).unwrap();
    let Value::Array(parts2) = out2 else { panic!("not array") };
    assert_eq!(parts2.get(1).unwrap(), n(2.0));
}

#[test]
fn copy_array_dict() {
    let a = arr(vec![n(1.0)]);
    let c = BuiltinFunctions::call("copy", &[a.clone()]).unwrap();
    assert_eq!(c, a);
    let d = Value::Dict(Box::new(HashMap::from([("x".into(), n(0.0))])));
    let c2 = BuiltinFunctions::call("copy", &[d.clone()]).unwrap();
    assert_eq!(c2, d);
}

#[test]
fn reversed_alias() {
    let a = arr(vec![n(1.0), n(2.0), n(3.0)]);
    let r1 = BuiltinFunctions::call("reverse", &[a.clone()]).unwrap();
    let r2 = BuiltinFunctions::call("reversed", &[a]).unwrap();
    assert_eq!(r1, r2);
}
