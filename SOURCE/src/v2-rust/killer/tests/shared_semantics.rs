use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_shared_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    String::from_utf8_lossy(&out.stdout).trim().replace("\r\n", "\n")
}

const COUNTER: &str = "class Counter {\n  fn init(start) {\n    this.n = start\n  }\n  fn inc() {\n    this.n = this.n + 1\n    return this.n\n  }\n}\n";

#[test]
fn methods_mutate_the_callers_object() {
    let src = format!("{}c = new Counter(0)\nc.inc()\nc.inc()\nprintln(c.n)\n", COUNTER);
    assert_eq!(run(&src), "2");
}

#[test]
fn objects_are_shared_through_function_arguments_and_aliases() {
    let src = format!("{}fn bump(o) {{\n  o.inc()\n}}\nc = new Counter(10)\nbump(c)\nbump(c)\nd = c\nd.inc()\nprintln(c.n)\n", COUNTER);
    assert_eq!(run(&src), "13");
}

#[test]
fn objects_keep_state_in_hot_loops() {
    let src = format!("{}fn main() {{\n  c = new Counter(0)\n  for i in range(50000) {{\n    c.inc()\n  }}\n  println(c.n)\n}}\nmain()\n", COUNTER);
    assert_eq!(run(&src), "50000");
}

#[test]
fn dicts_are_shared_references() {
    let src = "fn put(d) {\n  d[\"k\"] = 42\n}\nm = {}\nput(m)\nalias = m\nalias[\"z\"] = 1\nprintln(m[\"k\"])\nprintln(m[\"z\"])\n";
    assert_eq!(run(src), "42\n1");
}

#[test]
fn contains_works_for_dict_keys() {
    let src = "d = {}\nd[\"a\"] = 1\nprintln(contains(d, \"a\"))\nprintln(contains(d, \"zz\"))\n";
    assert_eq!(run(src), "true\nfalse");
}

#[test]
fn data_survives_garbage_collection_cycles() {
    // keys(d) allocates one array per iteration, so the collector runs several times; the
    // counts held in function locals must come through intact
    let src = "fn main() {\n  d = {}\n  n = 0\n  for i in range(120000) {\n    k = str(i % 50)\n    v = d[k]\n    if v == null {\n      d[k] = 1\n    } else {\n      d[k] = v + 1\n    }\n    n = len(keys(d))\n  }\n  println(d[\"7\"])\n  println(n)\n}\nmain()\n";
    assert_eq!(run(src), "2400\n50");
}

#[test]
fn arrays_in_locals_survive_gc() {
    let src = "fn main() {\n  keep = [1, 2, 3]\n  total = 0\n  for i in range(120000) {\n    tmp = [i]\n    total = total + len(tmp)\n  }\n  println(len(keep))\n  println(total)\n}\nmain()\n";
    assert_eq!(run(src), "3\n120000");
}

#[test]
fn property_reads_return_fields() {
    let src = "class P {\n  fn init(a, b) {\n    this.x = a\n    this.y = b\n  }\n  fn sum() {\n    return this.x + this.y\n  }\n}\np = new P(3, 4)\nprintln(p.x)\nprintln(p.y)\nprintln(p.sum())\n";
    assert_eq!(run(src), "3\n4\n7");
}

#[test]
fn a_method_with_the_same_name_as_a_property_read_still_wins() {
    // zero-arg property syntax on a name that some class defines as a method calls the method
    let src = "class A {\n  fn init() {\n    this.size = 99\n  }\n  fn size() {\n    return 5\n  }\n}\na = new A()\nprintln(a.size())\n";
    assert_eq!(run(src), "5");
}

#[test]
fn copy_makes_an_independent_dict_and_array() {
    let src = "d = {\"a\": 1}\ne = copy(d)\ne[\"a\"] = 99\ne[\"b\"] = 2\nprintln(d[\"a\"])\nprintln(len(keys(d)))\nxs = [1, 2]\nys = copy(xs)\nys[0] = 7\nprintln(xs[0])\n";
    assert_eq!(run(src), "1\n1\n1");
}
