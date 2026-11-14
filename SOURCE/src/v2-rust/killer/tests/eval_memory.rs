//! The memory cap of `eval(code[, vars[, opts]])` / `exec`: allocations inside the evaluated text are counted and
//! stopped with a catchable error ("memory limit exceeded (N MB)"); the host process is never aborted.
// The cap needs the counting allocator; a test binary using the library directly installs it (the library does not).
#[global_allocator]
static ALLOC: killer_native::memcap::CountingAlloc = killer_native::memcap::CountingAlloc;
use killer_native::builtin::BuiltinFunctions;
use killer_native::value::{SharedDict, Value};
use std::collections::HashMap;

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn opts(mb: i64) -> Value {
    Value::Dict(SharedDict::new(HashMap::from([("max_memory_mb".to_string(), Value::Integer(mb))])))
}

fn ev(name: &str, code: &str, mb: Option<i64>) -> Result<Value, String> {
    let mut args = vec![s(code), Value::Null];
    if let Some(mb) = mb {
        args.push(opts(mb));
    }
    BuiltinFunctions::call(name, &args).map_err(|e| format!("{e}"))
}

/// Run on a thread with a large stack, like the interpreter's own main thread does (nested VMs are deep).
fn big<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new().stack_size(512 * 1024 * 1024).spawn(f).unwrap().join().unwrap();
}

fn is_mem(r: &Result<Value, String>, mb: u64) -> bool {
    matches!(r, Err(m) if m.contains(&format!("memory limit exceeded ({} MB)", mb)))
}

#[test]
fn huge_string_repeat_is_refused_without_crashing() {
    big(|| {
        // far beyond any limit: refused by the fixed string-size limit or the cap, never attempted
        for call in ["eval", "exec"] {
            let r = ev(call, "x = 'a' * 100000000000\nx", None);
            assert!(r.is_err(), "{call}: {r:?}");
        }
        // under the fixed 256 MB string limit but over a small cap: the cap refuses it
        let r = ev("eval", "len('a' * 100000000)", Some(16));
        assert!(is_mem(&r, 16), "{r:?}");
        let r = ev("eval", "len('a' * 300000000)", None);
        assert!(r.is_err(), "{r:?}");
    });
}

#[test]
fn doubling_list_is_stopped() {
    big(|| {
        let r = ev("exec", "x = [1, 2, 3]\nwhile true {\n  x = x + x\n}\n", None);
        assert!(is_mem(&r, 256), "{r:?}");
        let r = ev("exec", "x = [1]\nwhile true {\n  x = x * 2\n}\n", Some(32));
        assert!(r.is_err(), "{r:?}");
        let r = ev("exec", "x = [1, 2, 3]\nwhile true {\n  x = x + x\n}\n", Some(8));
        assert!(is_mem(&r, 8), "{r:?}");
    });
}

#[test]
fn giant_range_is_refused() {
    big(|| {
        let r = ev("eval", "len(range(1000000000000))", None);
        assert!(r.is_err(), "{r:?}");
        // below the fixed range limit but above the cap
        let r = ev("eval", "len(range(30000000))", Some(8));
        assert!(is_mem(&r, 8), "{r:?}");
        // and fine with a cap that fits
        let r = ev("eval", "len(range(100000))", Some(8));
        assert_eq!(r.map(|v| format!("{v}")), Ok("100000".to_string()));
    });
}

#[test]
fn many_small_allocations_in_a_loop_are_counted() {
    big(|| {
        let code = "l = []\ni = 0\nwhile i < 40000000 {\n  l.push('item' + str(i))\n  i = i + 1\n}\nlen(l)\n";
        let r = ev("exec", code, Some(24));
        assert!(is_mem(&r, 24), "{r:?}");
    });
}

#[test]
fn string_built_in_a_loop_is_stopped() {
    big(|| {
        let code = "s = ''\nwhile true {\n  s = s + 'abcdefghij' * 1000\n}\n";
        let r = ev("exec", code, Some(20));
        assert!(is_mem(&r, 20), "{r:?}");
    });
}

#[test]
fn a_moderate_allocation_still_works() {
    big(|| {
        let r = ev("eval", "len('a' * 10000000)", None);
        assert_eq!(r.map(|v| format!("{v}")), Ok("10000000".to_string()));
        let r = ev("exec", "l = []\nfor i in range(200000) {\n  l.push(i)\n}\nn = len(l)\n", None).unwrap();
        assert!(format!("{r}").contains("200000"), "{r}");
    });
}

#[test]
fn the_configured_cap_is_respected() {
    big(|| {
        let code = "len('a' * 40000000)";
        assert_eq!(ev("eval", code, Some(128)).map(|v| format!("{v}")), Ok("40000000".to_string()));
        assert!(is_mem(&ev("eval", code, Some(16)), 16));
        assert!(is_mem(&ev("eval", code, Some(1)), 1));
    });
}

#[test]
fn the_hard_maximum_and_bad_values_are_rejected() {
    for bad in [2049, 4096, 1_000_000, 0, -5] {
        let r = ev("eval", "1", Some(bad));
        assert!(matches!(&r, Err(m) if m.contains("from 1 to 2048")), "{bad}: {r:?}");
    }
    assert_eq!(ev("eval", "1", Some(2048)).map(|v| format!("{v}")), Ok("1".to_string()));
    let bad = Value::Dict(SharedDict::new(HashMap::from([("max_memory".to_string(), Value::Integer(5))])));
    assert!(BuiltinFunctions::call("eval", &[s("1"), Value::Null, bad]).is_err());
    assert!(BuiltinFunctions::call("eval", &[s("1"), Value::Null, Value::Integer(5)]).is_err());
    let fl = Value::Dict(SharedDict::new(HashMap::from([("max_memory_mb".to_string(), Value::Number(1.5))])));
    assert!(BuiltinFunctions::call("eval", &[s("1"), Value::Null, fl]).is_err());
}

#[test]
fn nested_evals_have_their_own_cap_and_a_shared_overall_bound() {
    big(|| {
        // the inner cap (2 MB) is smaller than the outer one (64 MB): the inner call fails, the outer one continues
        let inner = "len('a' * 5000000)";
        let code = format!("r = ''\ntry {{\n  eval(\"{}\", null, {{\"max_memory_mb\": 2}})\n}} catch e {{\n  r = e\n}}\nr\n", inner);
        let r = ev("exec", &code, Some(64)).unwrap();
        assert!(format!("{r}").contains("memory limit exceeded (2 MB)"), "{r}");
        // the inner call asks for much more than the outer cap: the outer bound still applies
        let code = "r = ''\ntry {\n  eval(\"len('a' * 100000000)\", null, {\"max_memory_mb\": 1024})\n} catch e {\n  r = e\n}\nr\n";
        let r = ev("exec", code, Some(32)).unwrap();
        assert!(format!("{r}").contains("memory limit exceeded (32 MB)"), "{r}");
        // an inner call that fits runs, and its memory is released afterwards
        let code = "a = eval(\"len('a' * 5000000)\", null, {\"max_memory_mb\": 16})\nb = eval(\"len('a' * 5000000)\", null, {\"max_memory_mb\": 16})\nn = a + b\n";
        let r = ev("exec", code, Some(24)).unwrap();
        assert!(format!("{r}").contains("10000000"), "{r}");
    });
}

#[test]
fn the_error_is_catchable_in_the_caller_which_continues() {
    big(|| {
        let src = "__msg = ''\ntry {\n  eval(\"len('a' * 100000000)\", null, {\"max_memory_mb\": 8})\n} catch e {\n  __msg = e\n}\n__after = eval(\"1 + 1\")\n__again = 0\ntry {\n  exec(\"x = [1]\\nwhile true {\\n  x = x + x\\n}\", null, {\"max_memory_mb\": 8})\n} catch e {\n  __again = 1\n}\n";
        let program = killer_native::compiler::compile_killer_default(src).expect("compile");
        let mut vm = killer_native::vm::VirtualMachine::new();
        vm.run(&program).expect("the script itself must not fail");
        let g = vm.get_globals();
        let msg = format!("{}", g.get("__msg").unwrap());
        assert!(msg.contains("memory limit exceeded (8 MB)"), "{msg}");
        assert_eq!(format!("{}", g.get("__after").unwrap()), "2");
        assert_eq!(format!("{}", g.get("__again").unwrap()), "1");
    });
}

#[test]
fn evaluated_code_cannot_swallow_the_cap_error() {
    big(|| {
        let code = "n = 0\nwhile true {\n  try {\n    x = 'a' * 100000000\n  } catch e {\n    n = n + 1\n  }\n}\n";
        let r = ev("exec", code, Some(8));
        assert!(is_mem(&r, 8), "{r:?}");
    });
}

#[test]
fn normal_evaluations_are_unaffected_after_a_failure() {
    big(|| {
        for _ in 0..3 {
            assert!(is_mem(&ev("eval", "len('a' * 100000000)", Some(8)), 8));
            assert_eq!(ev("eval", "2 + 2", None).map(|v| format!("{v}")), Ok("4".to_string()));
        }
        // outside an evaluation the cap is inactive: the host may allocate freely
        let v: Vec<u8> = vec![1u8; 64 * 1024 * 1024];
        assert_eq!(v.len(), 64 * 1024 * 1024);
    });
}
