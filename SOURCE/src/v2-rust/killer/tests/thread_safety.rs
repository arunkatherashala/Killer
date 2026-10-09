//! Thread-safety contract: values never share reference counts between OS threads.
//!
//! Everything that crosses a thread boundary (spawn arguments, captured variables, global
//! snapshots, channel messages, future results) is a deep copy (`Value::detached`). A task
//! therefore cannot alias its parent's arrays, dicts, objects or sets; use `mutex_*` cells or
//! channels to communicate.
use killer_native::value::{SharedDict, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let dir: PathBuf = std::env::temp_dir().join(format!("killer_ts_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.killer");
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(&path)
        .arg("--run")
        .current_dir(&dir)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}

#[test]
fn arguments_of_every_kind_are_copied() {
    let src = "class Box {\n  fn init() {\n    this.n = 1\n  }\n}\nfn child(a, d, o, s, nested) {\n  push(a, 99)\n  d[\"k\"] = 99\n  o.n = 99\n  set_add(s, 99)\n  push(nested[0], 7)\n  nested[1][\"z\"] = 7\n  return [len(a), d[\"k\"], o.n, set_size(s), len(nested[0])]\n}\na = [1, 2]\nd = {\"k\": 1}\no = new Box()\ns = set([1])\nnested = [[1], {\"y\": 1}]\nprintln(async_await(async_spawn(child, a, d, o, s, nested)))\nprintln(a)\nprintln(d[\"k\"])\nprintln(o.n)\nprintln(set_size(s))\nprintln(nested)\n";
    assert_eq!(run(src), "[3, 99, 99, 2, 2]\n[1, 2]\n1\n1\n1\n[[1], {y: 1}]");
}

#[test]
fn globals_are_snapshots_not_shared() {
    let src = "shared = [1, 2, 3]\ncfg = {\"hits\": 0}\nfn bump() {\n  push(shared, 4)\n  cfg[\"hits\"] = cfg[\"hits\"] + 1\n  return len(shared)\n}\nprintln(async_await(async_spawn(bump)))\nprintln(len(shared))\nprintln(cfg[\"hits\"])\n";
    assert_eq!(run(src), "4\n3\n0");
}

#[test]
fn parent_mutation_after_spawn_is_invisible_to_the_child() {
    let src = "fn slow_len(xs) {\n  sleep_ms(150)\n  return len(xs)\n}\nxs = [1, 2]\nh = async_spawn(slow_len, xs)\npush(xs, 3)\npush(xs, 4)\nprintln(async_await(h))\nprintln(len(xs))\n";
    assert_eq!(run(src), "2\n4");
}

#[test]
fn values_passed_as_the_task_itself_are_snapshots() {
    let src = "xs = [1, 2]\nh = async_spawn(xs)\npush(xs, 3)\nr = async_await(h)\nprintln(r)\nprintln(xs)\npush(r, 100)\nprintln(xs)\n";
    assert_eq!(run(src), "[1, 2]\n[1, 2, 3]\n[1, 2, 3]");
}

#[test]
fn results_are_independent_copies() {
    let src = "fn echo(xs) {\n  return xs\n}\nxs = [1, 2]\nr = async_await(async_spawn(echo, xs))\npush(r, 3)\nprintln(xs)\nprintln(r)\n";
    assert_eq!(run(src), "[1, 2]\n[1, 2, 3]");
}

#[test]
fn closures_capture_a_private_copy() {
    let src = "fn make() {\n  items = [1]\n  fn add(v) {\n    push(items, v)\n    return len(items)\n  }\n  return add\n}\nf = make()\nprintln(f(5))\nprintln(async_await(async_spawn(f, 7)))\nprintln(f(9))\n";
    assert_eq!(run(src), "2\n3\n3");
}

#[test]
fn self_referencing_values_can_cross_threads() {
    let src = "a = []\npush(a, a)\nd = {}\nd[\"me\"] = d\nfn f(x, y) {\n  return len(x) + len(y)\n}\nprintln(async_await(async_spawn(f, a, d)))\n";
    assert_eq!(run(src), "2");
}

#[test]
fn channels_deliver_copies() {
    let src = "ch = chan_new()\nxs = [1, 2]\nchan_send(ch, xs)\npush(xs, 3)\nr = chan_recv(ch)\nprintln(r)\nprintln(xs)\nfn producer(c, items) {\n  push(items, 5)\n  chan_send(c, items)\n  return len(items)\n}\nys = [4]\nprintln(async_await(async_spawn(producer, ch, ys)))\nprintln(chan_recv(ch))\nprintln(ys)\n";
    assert_eq!(run(src), "[1, 2]\n[1, 2, 3]\n2\n[4, 5]\n[4]");
}

#[test]
fn mutex_cells_are_the_way_to_share_state() {
    let src = "m = mutex_new(0)\nfn add(k) {\n  mutex_add(m, k)\n  return k\n}\nhs = []\nfor i in range(1, 11) {\n  push(hs, async_spawn(add, i))\n}\nfor h in hs {\n  async_await(h)\n}\nprintln(mutex_get(m))\n";
    assert_eq!(run(src), "55");
}

#[test]
fn many_spawned_tasks_stress() {
    let src = "fn work(id, data, table) {\n  total = 0\n  for v in data {\n    total = total + v * id\n  }\n  push(data, id)\n  table[\"id\"] = id\n  return [id, total, len(data), table[\"id\"]]\n}\ndata = [1, 2, 3, 4]\ntable = {\"id\": -1}\nhs = []\nfor id in range(200) {\n  push(hs, async_spawn(work, id, data, table))\n}\ns = 0\nbad = 0\nfor h in hs {\n  r = async_await(h)\n  s = s + r[1] + r[2]\n  if r[3] != r[0] {\n    bad = bad + 1\n  }\n}\nprintln(s)\nprintln(bad)\nprintln(data)\nprintln(table[\"id\"])\n";
    assert_eq!(run(src), "200000\n0\n[1, 2, 3, 4]\n-1");
}

#[test]
fn nested_tasks_and_repeated_runs_are_stable() {
    let src = "fn leaf(n) {\n  return n * 2\n}\nfn mid(n) {\n  h = async_spawn(leaf, n + 1)\n  return async_await(h) + 1\n}\ntotal = 0\nfor round in range(5) {\n  hs = []\n  for i in range(20) {\n    push(hs, async_spawn(mid, i))\n  }\n  for h in hs {\n    total = total + async_await(h)\n  }\n}\nprintln(total)\n";
    // each mid(i) = 2*(i+1) + 1 ; sum over i in 0..20 = 2*210 + 20 = 440 ; 5 rounds
    assert_eq!(run(src), "2200");
}

#[test]
fn a_failing_task_does_not_poison_later_tasks() {
    let src = "fn bad() {\n  throw \"boom\"\n}\nfn good(x) {\n  return x + 1\n}\nfor i in range(10) {\n  try {\n    async_await(async_spawn(bad))\n  } catch e {\n    print(\"c\")\n  }\n  print(async_await(async_spawn(good, i)))\n}\nprintln(\"\")\n";
    assert_eq!(run(src), "c
1
c
2
c
3
c
4
c
5
c
6
c
7
c
8
c
9
c
10");
}

// ---------------------------------------------------------------- Value::detached itself

#[test]
fn detached_values_share_no_storage() {
    let inner = Value::from(vec![Value::Number(1.0)]);
    let dict = SharedDict::empty();
    dict.insert("list".to_string(), inner.clone());
    let original = Value::from(vec![inner.clone(), Value::Dict(dict.clone())]);
    let copy = original.detached();

    // mutate every level of the original
    if let Value::Array(a) = &inner {
        a.push(Value::Number(2.0));
    }
    dict.insert("extra".to_string(), Value::Number(1.0));
    if let Value::Array(a) = &original {
        a.push(Value::Null);
    }

    assert_eq!(format!("{}", copy), "[[1], {list: [1]}]");
    assert_eq!(format!("{}", original), "[[1, 2], {list: [1, 2], extra: 1}, null]");
}

#[test]
fn detached_preserves_sharing_and_cycles_inside_the_copy() {
    let shared = Value::from(vec![Value::Number(1.0)]);
    let outer = Value::from(vec![shared.clone(), shared.clone()]);
    if let Value::Array(a) = &outer {
        a.push(outer.clone()); // a cycle
    }
    let copy = outer.detached();
    let Value::Array(c) = &copy else { panic!("not an array") };
    let items = c.to_vec();
    assert_eq!(items.len(), 3);
    // the two references to the same list are still the same list in the copy...
    if let (Value::Array(x), Value::Array(y)) = (&items[0], &items[1]) {
        x.push(Value::Number(2.0));
        assert_eq!(y.len(), 2);
    } else {
        panic!("expected arrays");
    }
    // ...the cycle points at the copy, not at the original
    if let Value::Array(me) = &items[2] {
        assert_eq!(me.len(), 3);
    } else {
        panic!("expected the cycle");
    }
    if let Value::Array(orig_first) = &shared {
        assert_eq!(orig_first.len(), 1, "the original must be untouched");
    }
}
