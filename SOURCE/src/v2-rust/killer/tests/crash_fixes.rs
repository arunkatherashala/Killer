//! Regression tests for process-killing panics / hangs (release profile uses panic="abort").
use std::collections::HashMap;

use killer_native::builtin::BuiltinFunctions;
use killer_native::value::{SharedDict, Value};

fn n(x: f64) -> Value {
    Value::Number(x)
}
fn s(t: &str) -> Value {
    Value::Str(t.to_string())
}
fn dt(secs: f64) -> Value {
    let mut m = HashMap::new();
    m.insert("seconds".to_string(), Value::Number(secs));
    Value::Dict(SharedDict::new(m))
}

/// Run a Killer script; returns the global variables.
fn run(src: &str) -> HashMap<String, Value> {
    let program = killer_native::compiler::compile_killer_default(src).expect("compile");
    let mut vm = killer_native::vm::VirtualMachine::new();
    vm.run(&program).expect("run");
    vm.get_globals()
}

#[test]
fn json_parse_bad_unicode_escape_is_error() {
    let r = BuiltinFunctions::call("json_parse", &[s("\"\\u000\u{e9}\"")]);
    assert!(r.is_err());
}

#[test]
fn splice_with_huge_counts_does_not_panic() {
    let g = run("__res = []\nfn go() {\n  a = [1, 2, 3]\n  a.splice(1, 1e30)\n  push(__res, a)\n  b = [1, 2, 3]\n  b.splice(1e30, 1e30, 9)\n  push(__res, b)\n  c = [1, 2, 3]\n  c.splice(-5, -1e30)\n  push(__res, c)\n}\ngo()\n");
    let items = match &g["__res"] {
        Value::Array(a) => a.to_vec(),
        other => panic!("unexpected {:?}", other),
    };
    assert_eq!(items[0], Value::Array(vec![n(1.0)].into()));
    assert_eq!(items[1], Value::Array(vec![n(1.0), n(2.0), n(3.0), n(9.0)].into()));
    assert_eq!(items[2], Value::Array(vec![n(1.0), n(2.0), n(3.0)].into()));
}

#[test]
fn parse_datetime_rejects_bad_fields() {
    for bad in ["2020-13-01", "2020-00-01", "2000000000-01-01", "2020-02-30", "2020-01-01 25:00:00"] {
        assert!(BuiltinFunctions::call("parse_datetime", &[s(bad)]).is_err(), "{bad}");
    }
    assert!(BuiltinFunctions::call("parse_datetime", &[s("2024-02-29 12:00:00")]).is_ok());
}

#[test]
fn date_parts_are_bounded_and_correct() {
    // huge input must return promptly (previously looped ~1e11 iterations)
    for f in ["date_year", "date_month", "date_day"] {
        let _ = BuiltinFunctions::call(f, &[dt(9e18)]).unwrap();
        let _ = BuiltinFunctions::call(f, &[dt(-9e18)]).unwrap();
        let _ = BuiltinFunctions::call(f, &[dt(f64::NAN)]).unwrap();
    }
    // 2024-02-29 12:00:00 UTC = 1709208000
    assert_eq!(BuiltinFunctions::call("date_year", &[dt(1709208000.0)]).unwrap(), n(2024.0));
    assert_eq!(BuiltinFunctions::call("date_month", &[dt(1709208000.0)]).unwrap(), n(2.0));
    assert_eq!(BuiltinFunctions::call("date_day", &[dt(1709208000.0)]).unwrap(), n(29.0));
    // epoch and a pre-epoch date: 1969-12-31
    assert_eq!(BuiltinFunctions::call("date_year", &[dt(0.0)]).unwrap(), n(1970.0));
    assert_eq!(BuiltinFunctions::call("date_year", &[dt(-1.0)]).unwrap(), n(1969.0));
    assert_eq!(BuiltinFunctions::call("date_month", &[dt(-1.0)]).unwrap(), n(12.0));
    assert_eq!(BuiltinFunctions::call("date_day", &[dt(-1.0)]).unwrap(), n(31.0));
}

#[test]
fn date_add_and_diff_do_not_overflow() {
    let r = BuiltinFunctions::call("date_add", &[dt(9e18), n(9e18), s("seconds")]).unwrap();
    assert!(matches!(r, Value::Dict(_)));
    let r = BuiltinFunctions::call("date_add", &[dt(-9e18), n(-1e300), s("days")]).unwrap();
    assert!(matches!(r, Value::Dict(_)));
    let r = BuiltinFunctions::call("date_diff", &[n(9.2e18), n(-9.2e18)]).unwrap();
    assert!(matches!(r, Value::Number(x) if x > 0.0));
    let r = BuiltinFunctions::call("date_diff", &[n(i64::MIN as f64), n(i64::MAX as f64)]).unwrap();
    assert!(matches!(r, Value::Number(x) if x > 0.0));
}

#[test]
fn sorted_with_nan_does_not_panic() {
    let mut v: Vec<Value> = (0..200).map(|i| if i % 7 == 0 { n(f64::NAN) } else { n(((i * 37) % 101) as f64) }).collect();
    v.reverse();
    let out = BuiltinFunctions::call("sorted", &[Value::Array(v.into())]).unwrap();
    if let Value::Array(a) = out {
        let items = a.to_vec();
        assert_eq!(items.len(), 200);
        // non-NaN values come out in order
        let nums: Vec<f64> = items.iter().filter_map(|x| if let Value::Number(f) = x { Some(*f) } else { None }).filter(|f| !f.is_nan()).collect();
        assert!(nums.windows(2).all(|w| w[0] <= w[1]));
    } else {
        panic!("expected array");
    }
}
