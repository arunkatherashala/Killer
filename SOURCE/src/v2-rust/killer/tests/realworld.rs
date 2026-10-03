//! Real-world programs (parity/realworld/*.killer) run end to end and compared with their reviewed
//! expected output, plus a regression test for every bug those programs exposed.
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn scratch_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("killer_rw_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run_file(path: &std::path::Path, cwd: &std::path::Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(path)
        .arg("--run")
        .current_dir(cwd)
        .output()
        .unwrap();
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}

fn run(src: &str) -> String {
    let dir = scratch_dir();
    let path = dir.join("main.killer");
    std::fs::write(&path, src).unwrap();
    let out = run_file(&path, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

/// Run parity/realworld/<name>.killer in a scratch directory and compare with <name>.expected.
fn program(name: &str) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("parity").join("realworld");
    let dir = scratch_dir();
    let actual = run_file(&base.join(format!("{name}.killer")), &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let expected = std::fs::read_to_string(base.join(format!("{name}.expected")))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(actual, expected.trim(), "output of {name} changed");
}

#[test]
fn assigning_through_a_field_and_an_index() {
    // `this.cells[i] = x` was rejected as an invalid assignment target
    let src = "class B {\n  fn init() {\n    this.cells = [0, 0, 0]\n    this.pos = {\"x\": 1}\n  }\n  fn set(i, v) {\n    this.cells[i] = v\n    this.pos.x = v\n  }\n}\nb = new B()\nb.set(1, 7)\nprintln(b.cells)\nprintln(b.pos)\nb.cells[2] = 9\nb.pos.y = 5\nprintln(b.cells)\nprintln(b.pos)\nm = {\"rows\": [[1, 2], [3, 4]]}\nm.rows[1][0] = 30\nprintln(m)\n";
    assert_eq!(run(src), "[0, 7, 0]\n{x: 7}\n[0, 7, 9]\n{x: 7, y: 5}\n{rows: [[1, 2], [30, 4]]}");
}

#[test]
fn contains_and_in_accept_number_keys_on_dicts() {
    let src = "memo = {}\nmemo[5] = \"five\"\nprintln(5 in memo)\nprintln(6 in memo)\nprintln(contains(memo, 5))\nprintln(memo[5])\n";
    assert_eq!(run(src), "true\nfalse\ntrue\nfive");
}

#[test]
fn and_or_short_circuit() {
    // the right operand used to run even when the left decided the result
    let src = "fn boom() {\n  println(\"boom\")\n  return true\n}\nprintln(false && boom())\nprintln(true || boom())\nprintln(true && boom())\nprintln(false || boom())\nxs = [1]\nj = 3\nprintln(j < len(xs) && xs[j] > 0)\nprintln(j >= len(xs) || xs[j] > 0)\nif j < len(xs) && xs[j] > 0 {\n  println(\"no\")\n}\nwhile j < len(xs) && xs[j] > 0 {\n  j = j + 1\n}\nprintln(j)\n";
    assert_eq!(run(src), "false\ntrue\nboom\ntrue\nboom\ntrue\nfalse\ntrue\n3");
}

#[test]
fn null_coalescing_only_evaluates_the_fallback_when_needed() {
    let src = "fn fallback() {\n  println(\"fallback\")\n  return 0\n}\nx = 5\nprintln(x ?? fallback())\ny = null\nprintln(y ?? fallback())\nd = {\"a\": 1}\nprintln((d[\"a\"] ?? 0) + 1)\nprintln((d[\"zz\"] ?? 10) + 1)\n";
    assert_eq!(run(src), "5\nfallback\n0\n2\n11");
}

#[test]
fn short_circuit_keeps_three_valued_logic() {
    let src = "u = T_ZERO\nprintln(u && true)\nprintln(u || false)\nprintln(T_POS && T_NEG)\nprintln(T_NEG || T_ZERO)\n";
    assert_eq!(run(src), "T_ZERO\nT_ZERO\nT_NEG\nT_ZERO");
}

#[test]
fn short_circuit_inside_natively_compiled_functions() {
    // hot enough for the JIT; the guard must still protect the array read
    let src = "fn ok(xs, i) {\n  if i < len(xs) && xs[i] > 0 {\n    return 1\n  }\n  return 0\n}\nfn main() {\n  s = 0\n  for k in range(600) {\n    s = s + ok([1, 2, 3], k % 6)\n  }\n  for k in range(500) {\n    if k > 3 || k < 2 {\n      s = s + 1\n    }\n  }\n  return s\n}\nprintln(main())\n";
    assert_eq!(run(src), "798");
}

#[test]
fn constant_folding_does_not_cross_a_branch_target() {
    // `0` (end of `z ?? 0`) and `1` were folded together although a jump lands between them
    let src = "z = 3\nprintln((z ?? 0) + 1)\nc = true\nprintln((c ? 5 : 6) + 7)\nc = false\nprintln((c ? 5 : 6) + 7)\nprintln((null ?? 2) * 4)\n";
    assert_eq!(run(src), "4\n12\n13\n8");
}

#[test]
fn loop_and_comprehension_variables_can_be_called() {
    let src = "fn inc(x) {\n  return x + 1\n}\nfn dbl(x) {\n  return x * 2\n}\nfn run(fs, v) {\n  for g in fs {\n    v = g(v)\n  }\n  return v\n}\nprintln(run([inc, dbl, inc], 3))\nfs = [inc, dbl]\nprintln([f(10) for f in fs])\nclass K {\n  fn init() {\n    this.fs = [inc, dbl]\n  }\n  fn go(v) {\n    for g in reverse(this.fs) {\n      v = g(v)\n    }\n    return v\n  }\n}\nprintln(new K().go(5))\n";
    assert_eq!(run(src), "9\n[11, 20]\n11");
}

#[test]
fn string_literal_ending_in_an_escaped_quote() {
    let src = "println(\"say \\\"hi\\\"\")\nprintln(\"quote:\\\"\")\nprintln(\"a\" + \"q:\\\"\" + \"b\")\nprintln(\"x,\\\"y\", \"z\")\n";
    assert_eq!(run(src), "say \"hi\"\nquote:\"\naq:\"b\nx,\"y z");
}

#[test]
fn methods_on_string_literals_and_python_style_string_methods() {
    let src = "s = \"hello world\"\nprintln(\"abc\".upper())\nprintln(\"a-b\".split(\"-\"))\nprintln(\", \".join([\"a\", \"b\"]))\nprintln(s.startswith(\"he\"))\nprintln(s.endswith(\"d\"))\nprintln(s.find(\"o\"))\nprintln(s.rfind(\"o\"))\nprintln(s.count(\"o\"))\nprintln(s.title())\nprintln(s.capitalize())\nprintln(s.center(15, \"*\"))\nprintln(\"7\".zfill(3))\nprintln(\"-7\".zfill(4))\nprintln(\"42\".isdigit())\nprintln(\"4x\".isdigit())\nprintln(\"abc\".isalpha())\nprintln(len(\"a b  c\".split()))\nprintln(\"x\".ljust(3, \".\") + \"|\")\nprintln(s.swapcase())\nprintln(\"a\\nb\".splitlines())\n";
    assert_eq!(
        run(src),
        "ABC\n[a, b]\na, b\ntrue\ntrue\n4\n7\n2\nHello World\nHello world\n**hello world**\n007\n-007\ntrue\nfalse\ntrue\n3\nx..|\nHELLO WORLD\n[a, b]"
    );
}

#[test]
fn text_search_returns_character_indexes_for_unicode() {
    let src = "u = \"h\u{e9}llo w\u{f6}rld\"\nprintln(index_of(u, \"w\"))\nprintln(u.find(\"w\"))\nprintln(split(\"abc\", \"\"))\n";
    assert_eq!(run(src), "6\n6\n[a, b, c]");
}

#[test]
fn builtin_fallback_works_for_arrays_and_dicts() {
    let src = "w = [\"ab\", \"cd\"]\nd = {\"k\": 1}\nprintln(w.contains(\"ab\"))\nprintln(w.slice(0, 1))\nprintln(w.append(\"zz\"))\nprintln(d.has(\"k\"))\nprintln(d.get(\"zz\", 5))\nprintln(w.count(\"ab\"))\n";
    assert_eq!(run(src), "true\n[ab]\n[ab, cd, zz]\ntrue\n5\n1");
}

#[test]
fn sorting_and_extremes_over_numbers_arrays_and_rows() {
    let src = "n = [10, 9, 2, 33]\nprintln(n.sort())\nprintln(n)\nprintln(max(n))\nprintln(min(n))\nprintln(max(\"b\", \"a\"))\nprintln(sorted([[2, \"b\"], [1, \"z\"], [1, \"a\"]]))\nprintln(sorted([\"b\", \"a\"], true))\n";
    assert_eq!(run(src), "[2, 9, 10, 33]\n[2, 9, 10, 33]\n33\n2\nb\n[[1, a], [1, z], [2, b]]\n[b, a]");
}

#[test]
fn dicts_keep_insertion_order() {
    let src = "d = {\"z\": 1, \"a\": 2}\nd[\"m\"] = 3\nprintln(d)\nprintln(keys(d))\nprintln(values(d))\ndelete(d, \"z\")\nd[\"z\"] = 9\nprintln(d)\nprintln(json_stringify(d))\nprintln(json_stringify(json_parse('{\"b\": 1, \"a\": {\"y\": 1, \"x\": 2}}')))\nprintln({\"a\": 1, \"a\": 2})\nprintln({\"a\": 1, \"b\": 2} == {\"b\": 2, \"a\": 1})\nprintln({k: k * 2 for k in range(3)})\nc = d\nc[\"q\"] = 1\nprintln(len(d))\n";
    assert_eq!(
        run(src),
        "{z: 1, a: 2, m: 3}\n[z, a, m]\n[1, 2, 3]\n{a: 2, m: 3, z: 9}\n{\"a\":2,\"m\":3,\"z\":9}\n{\"b\":1,\"a\":{\"y\":1,\"x\":2}}\n{a: 2}\ntrue\n{0: 0, 1: 2, 2: 4}\n4"
    );
}

#[test]
fn iterating_a_dict_or_set_walks_keys_and_members() {
    let src = "d = {\"x\": 1, \"y\": 2}\nfor k in d {\n  println(k + \"=\" + str(d[k]))\n}\ns = set([3, 1, 2])\ntotal = 0\nfor v in s {\n  total = total + v\n}\nprintln(total)\n";
    assert_eq!(run(src), "x=1\ny=2\n6");
}

#[test]
fn spawned_tasks_see_globals_classes_and_raise_their_errors() {
    let src = "m = mutex_new(0)\nscale = 10\nclass Acc {\n  fn init(n) {\n    this.n = n\n  }\n  fn double() {\n    return this.n * 2\n  }\n}\nfn job(k) {\n  mutex_add(m, k)\n  return new Acc(k * scale).double()\n}\nhs = [async_spawn(job, 1), async_spawn(job, 2)]\nprintln(async_await(hs[0]) + async_await(hs[1]))\nprintln(mutex_get(m))\nfn bad() {\n  throw {\"code\": 7}\n}\ntry {\n  async_await(async_spawn(bad))\n} catch e {\n  println(e[\"code\"])\n}\nfn bad2() {\n  return nosuch + 1\n}\ntry {\n  async_await(async_spawn(bad2))\n} catch e {\n  println(\"caught\")\n}\n";
    assert_eq!(run(src), "60\n3\n7\ncaught");
}

#[test]
fn spawned_tasks_get_their_own_copy_of_arguments() {
    let src = "fn mutate(xs) {\n  push(xs, 99)\n  return len(xs)\n}\nxs = [1, 2]\nprintln(async_await(async_spawn(mutate, xs)))\nprintln(xs)\n";
    assert_eq!(run(src), "3\n[1, 2]");
}

#[test]
fn dictionary_literals_can_span_lines() {
    let src = "b = {\n  \"x\": 1,\n  \"y\": {\"k\": [1, 2]}\n}\nprintln(b)\nfn make() {\n  return {\n    \"a\": 1,\n    // a comment inside\n    \"b\": 2\n  }\n}\nprintln(make())\nprintln(max(1, 5,\n  3))\nrows = []\npush(rows, {\n  \"n\": 1\n})\nprintln(rows)\nif true {\n  println(\"block still works\")\n}\n";
    assert_eq!(run(src), "{x: 1, y: {k: [1, 2]}}\n{a: 1, b: 2}\n5\n[{n: 1}]\nblock still works");
}

#[test]
fn fstrings_accept_single_quoted_strings_inside_the_braces() {
    let src = "d = {\"k\": 5, \"name\": \"Bob\"}\nprintln(f\"v={d['k']}\")\nprintln(f\"{d['name']} has {d['k'] + 1}\")\nprintln(f\"{pad_left('a', 3, '*')}|\")\n";
    assert_eq!(run(src), "v=5\nBob has 6\n**a|");
}

#[test]
fn bracketed_destructuring_assignment() {
    let src = "pair = [3, 4]\n[a, b] = pair\nprintln(a + b)\n(c, d) = d_swap(a, b)\nprintln(str(c) + str(d))\nfn d_swap(x, y) {\n  return [y, x]\n}\n";
    assert_eq!(run(src), "7\n43");
}

#[test]
fn sets_are_shared_and_mutated_in_place() {
    let src = "s = set([1])\nset_add(s, 2)\ns.add(3)\nprintln(s)\nfn addto(x) {\n  set_add(x, 99)\n}\naddto(s)\nprintln(s)\nprintln(3 in s)\ns.remove(1)\nprintln(len(s))\nt = set([2, 3, 4])\nprintln(s | t)\nprintln(s & t)\nprintln(t - s)\nprintln(s.contains(99))\nfor v in t {\n  println(v)\n}\n";
    assert_eq!(run(src), "{1, 2, 3}\n{1, 2, 3, 99}\ntrue\n3\n{2, 3, 4, 99}\n{2, 3}\n{4}\ntrue\n2\n3\n4");
}

#[test]
fn finally_runs_when_return_break_or_continue_leave_the_try() {
    let src = "fn a() {\n  try {\n    return 1\n  } finally {\n    println(\"fin a\")\n  }\n}\nprintln(a())\nfn b() {\n  try {\n    throw \"x\"\n  } catch e {\n    return \"caught \" + e\n  } finally {\n    println(\"fin b\")\n  }\n}\nprintln(b())\nfn nested() {\n  try {\n    try {\n      return \"inner\"\n    } finally {\n      println(\"inner finally\")\n    }\n  } finally {\n    println(\"outer finally\")\n  }\n}\nprintln(nested())\nfor i in range(3) {\n  try {\n    if i == 1 {\n      continue\n    }\n    if i == 2 {\n      break\n    }\n    println(\"body \" + str(i))\n  } finally {\n    println(\"fin \" + str(i))\n  }\n}\nfn ft() {\n  try {\n    try {\n      return 1\n    } finally {\n      throw \"from finally\"\n    }\n  } catch e {\n    return \"caught \" + e\n  }\n}\nprintln(ft())\n";
    assert_eq!(
        run(src),
        "fin a\n1\nfin b\ncaught x\ninner finally\nouter finally\ninner\nbody 0\nfin 0\nfin 1\nfin 2\ncaught from finally"
    );
}

#[test]
fn read_file_raises_for_a_missing_file() {
    // the VM used to answer null (and skip the sandbox capability check)
    let src = "try {\n  readFile(\"definitely_missing_zz.txt\")\n  println(\"no error\")\n} catch e {\n  println(\"caught\")\n}\nwriteFile(\"rw_tmp.txt\", \"hi\")\nprintln(readFile(\"rw_tmp.txt\"))\ndeleteFile(\"rw_tmp.txt\")\n";
    assert_eq!(run(src), "caught\nhi");
}

