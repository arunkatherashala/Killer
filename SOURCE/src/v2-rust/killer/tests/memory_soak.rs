//! Memory behaviour of long-running scripts: reference cycles (dicts, objects, closures, arrays), abandoned
//! generators, and the eval memory cap across spawned threads. Every loop below allocates several times more than
//! the cap it runs under if (and only if) the memory is never returned, so the cap error is the leak detector.
// The cap needs the counting allocator; a test binary using the library directly installs it (the library does not).
#[global_allocator]
static ALLOC: killer_native::memcap::CountingAlloc = killer_native::memcap::CountingAlloc;
use killer_native::builtin::BuiltinFunctions;
use killer_native::value::{SharedDict, Value};
use std::collections::HashMap;

fn run_capped(code: &str, mb: i64) -> Result<String, String> {
    let code = code.to_string();
    let h = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || {
            let opts = Value::Dict(SharedDict::new(HashMap::from([("max_memory_mb".to_string(), Value::Integer(mb))])));
            let args = vec![Value::Str(code), Value::Null, opts];
            BuiltinFunctions::call("exec", &args).map(|_| "ok".to_string()).map_err(|e| format!("{e}"))
        })
        .unwrap();
    h.join().unwrap()
}

#[test]
fn dict_cycles_are_collected() {
    let code = "i = 0\nwhile i < 150000 {\n  a = {\"n\": i}\n  b = {\"a\": a}\n  a[\"b\"] = b\n  i = i + 1\n}\n\"done\"\n";
    let r = run_capped(code, 32);
    assert!(r.is_ok(), "dict cycles leaked: {r:?}");
}

#[test]
fn object_cycles_are_collected() {
    let code = "class Node {\n  kfn init() {\n    this.other = null\n    this.pad = \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"\n  }\n}\ni = 0\nwhile i < 60000 {\n  a = new Node()\n  b = new Node()\n  a.other = b\n  b.other = a\n  i = i + 1\n}\n\"done\"\n";
    let r = run_capped(code, 40);
    assert!(r.is_ok(), "object cycles leaked: {r:?}");
}

#[test]
fn self_referencing_closures_are_collected() {
    let code = "kfn make(k) {\n  fn walk(n) {\n    if n <= 0 {\n      return k\n    }\n    return walk(n - 1)\n  }\n  return walk(2)\n}\ni = 0\ns = 0\nwhile i < 150000 {\n  s = s + make(i)\n  i = i + 1\n}\ns\n";
    let r = run_capped(code, 32);
    assert!(r.is_ok(), "closure cycles leaked: {r:?}");
}

#[test]
fn array_cycles_are_collected() {
    let code = "i = 0\nwhile i < 150000 {\n  a = [i, \"xxxxxxxxxxxxxxxxxxxxxxxxxxxx\"]\n  b = [a]\n  push(a, b)\n  i = i + 1\n}\n\"done\"\n";
    let r = run_capped(code, 32);
    assert!(r.is_ok(), "array cycles leaked: {r:?}");
}

#[test]
fn array_dict_mixed_cycles_are_collected() {
    let code = "i = 0\nwhile i < 150000 {\n  a = [i]\n  d = {\"list\": a}\n  push(a, d)\n  i = i + 1\n}\n\"done\"\n";
    let r = run_capped(code, 32);
    assert!(r.is_ok(), "array/dict cycles leaked: {r:?}");
}

#[test]
fn live_data_survives_collection() {
    // Cyclic structures that stay reachable must never be cleared by a collection that runs in between.
    let code = "keep = []\ni = 0\nwhile i < 60000 {\n  a = {\"n\": i}\n  b = {\"a\": a}\n  a[\"b\"] = b\n  if i % 1000 == 0 {\n    push(keep, a)\n  }\n  junk = {\"j\": {}}\n  junk[\"j\"][\"back\"] = junk\n  i = i + 1\n}\nok = 0\nfor k in keep {\n  if k[\"b\"][\"a\"][\"n\"] == k[\"n\"] {\n    ok = ok + 1\n  }\n}\nok\n";
    let r = run_capped(code, 256);
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn abandoned_generators_are_freed() {
    // Each generator is advanced once and then dropped; its suspended frame must go with it.
    let code = "fn gen(k) {\n  pad = \"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"\n  yield k\n  yield pad\n}\ni = 0\ns = 0\nwhile i < 150000 {\n  g = gen(i)\n  s = s + next(g)\n  i = i + 1\n}\ns\n";
    let r = run_capped(code, 32);
    assert!(r.is_ok(), "abandoned generators leaked: {r:?}");
}

#[test]
fn generators_still_work_when_kept_and_when_overwritten() {
    // a generator kept in a variable and one stored in a dict survive the churn of dropped ones around them
    let code = "fn gen(k) {
  yield k
  yield k + 1
  yield k + 2
}
kept = gen(10)
next(kept)
holder = {\"g\": gen(100)}
next(holder[\"g\"])
i = 0
while i < 5000 {
  g = gen(i)
  next(g)
  i = i + 1
}
if next(kept) != 11 || next(kept) != 12 {
  throw \"kept generator broke\"
}
if next(holder[\"g\"]) != 101 {
  throw \"dict-held generator broke\"
}
";
    let r = run_capped(code, 256);
    assert!(r.is_ok(), "{r:?}");
}

/// Each task holds roughly 5 MB for a while; the tasks run at the same time.
const TASK_FN: &str = "kfn hold(n) {\n  a = []\n  i = 0\n  while i < n {\n    push(a, i)\n    i = i + 1\n  }\n  sleep_ms(1500)\n  return len(a)\n}\n";

#[test]
fn spawned_threads_cannot_bypass_the_cap() {
    // 16 tasks x ~5 MB = ~80 MB held at once under a 32 MB cap: the threads must be stopped, not run uncounted.
    let code = format!("{TASK_FN}hs = []\ni = 0\nwhile i < 16 {{\n  push(hs, spawn hold(150000))\n  i = i + 1\n}}\ntotal = 0\nfor h in hs {{\n  total = total + await h\n}}\n");
    let r = run_capped(&code, 32);
    assert!(matches!(&r, Err(m) if m.contains("memory limit exceeded")), "threads escaped the cap: {r:?}");
}

#[test]
fn spawned_threads_within_the_cap_still_work() {
    // 4 tasks x ~5 MB = ~20 MB under a 256 MB cap, and again after they ended (their memory is returned to the budget)
    let code = format!("{TASK_FN}round = 0\nwhile round < 2 {{\n  hs = []\n  i = 0\n  while i < 4 {{\n    push(hs, spawn hold(50000))\n    i = i + 1\n  }}\n  total = 0\n  for h in hs {{\n    total = total + await h\n  }}\n  if total != 200000 {{\n    throw \"wrong total\"\n  }}\n  round = round + 1\n}}\n");
    let r = run_capped(&code, 256);
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn many_short_tasks_do_not_leave_the_budget_drifting_upwards() {
    // 150 tasks, one after another, each allocating and freeing ~1 MB under an 8 MB cap: if every ended task left
    // even a few KB counted but never freed, the cap would eventually fire although the script holds nothing.
    let code = "kfn work(n) {\n  a = []\n  i = 0\n  while i < n {\n    push(a, i)\n    i = i + 1\n  }\n  return len(a)\n}\nround = 0\ntotal = 0\nwhile round < 150 {\n  total = total + await spawn work(2000)\n  round = round + 1\n}\nif total != 300000 {\n  throw \"wrong total\"\n}\n";
    let r = run_capped(code, 8);
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn the_shared_counter_returns_to_its_baseline_after_threads_end() {
    use killer_native::memcap;
    let _g = memcap::enter(256);
    let keep = vec![0u8; 1024];
    let before = memcap::live_bytes();
    for _ in 0..300 {
        let cap = memcap::inherit();
        std::thread::spawn(move || {
            let _c = memcap::adopt(cap);
            let v: Vec<u64> = (0..20000).collect();
            std::hint::black_box(&v);
        })
        .join()
        .unwrap();
    }
    let drift = memcap::live_bytes() - before;
    drop(keep);
    // a thread leaves a few hundred bytes of bookkeeping at most; 300 threads must stay well below 1 MB in total
    assert!(drift.abs() < 512 * 1024, "the counted bytes drifted by {drift} over 300 threads");
}
