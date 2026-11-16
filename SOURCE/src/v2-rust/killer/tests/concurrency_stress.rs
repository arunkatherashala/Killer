//! Concurrency stress tests: threads, spawn, channels, futures and mutex cells driven from Killer
//! scripts, many iterations each, to catch lost updates, lost messages, hangs and flakiness.
//! Every script runs in its own process with a generous watchdog so a hang fails instead of stalling.
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let dir: PathBuf = std::env::temp_dir().join(format!("killer_cs_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.killer");
    std::fs::write(&path, src).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(&path)
        .arg("--run")
        .current_dir(&dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = std::fs::remove_dir_all(&dir);
            panic!("script hung (killed after 240s):\n{src}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr))
        .replace("\r\n", "\n")
        .trim()
        .to_string()
}

#[test]
fn shared_counter_has_no_lost_updates() {
    let src = r#"
fn bump(m, k) {
  for i in range(k) {
    mutex_add(m, 1)
  }
  return k
}
bad = 0
for round in range(20) {
  m = mutex_new(0)
  hs = []
  for t in range(16) {
    push(hs, async_spawn(bump, m, 250))
  }
  done = 0
  for h in hs {
    done = done + async_await(h)
  }
  if mutex_get(m) != 4000 || done != 4000 {
    bad = bad + 1
  }
  mutex_free(m)
}
println(bad)
"#;
    assert_eq!(run(src), "0");
}

#[test]
fn many_tasks_return_their_own_results_in_order() {
    let src = r#"
fn sq(x) {
  return x * x
}
bad = 0
for round in range(20) {
  hs = []
  for i in range(100) {
    push(hs, async_spawn(sq, i))
  }
  for i in range(100) {
    if async_await(hs[i]) != i * i {
      bad = bad + 1
    }
  }
}
println(bad)
"#;
    assert_eq!(run(src), "0");
}

#[test]
fn producers_and_consumers_lose_no_messages() {
    // 4 producers x 500 messages, 3 consumers, one sentinel per consumer
    let src = r#"
fn producer(ch, id, n) {
  for i in range(n) {
    chan_send(ch, id * 1000 + i)
  }
  return n
}
fn consumer(ch) {
  count = 0
  total = 0
  while true {
    v = chan_recv(ch)
    if v == -1 {
      break
    }
    count = count + 1
    total = total + v
  }
  return [count, total]
}
bad = 0
for round in range(5) {
  ch = chan_new()
  cs = []
  for c in range(3) {
    push(cs, async_spawn(consumer, ch))
  }
  ps = []
  for p in range(1, 5) {
    push(ps, async_spawn(producer, ch, p, 500))
  }
  for p in ps {
    async_await(p)
  }
  for c in range(3) {
    chan_send(ch, -1)
  }
  count = 0
  total = 0
  for c in cs {
    r = async_await(c)
    count = count + r[0]
    total = total + r[1]
  }
  expected = 0
  for p in range(1, 5) {
    expected = expected + p * 1000 * 500 + 499 * 500 / 2
  }
  if count != 2000 || total != expected {
    bad = bad + 1
    println("round " + str(round) + ": " + str(count) + " " + str(total) + " " + str(expected))
  }
  chan_close(ch)
}
println(bad)
"#;
    assert_eq!(run(src), "0");
}

#[test]
fn a_single_producer_is_received_in_order() {
    let src = r#"
ch = chan_new()
fn produce(c) {
  for i in range(1000) {
    chan_send(c, i)
  }
  return 0
}
h = async_spawn(produce, ch)
ok = true
for i in range(1000) {
  if chan_recv(ch) != i {
    ok = false
  }
}
async_await(h)
println(ok)
println(chan_try_recv(ch))
"#;
    assert_eq!(run(src), "true\nnull");
}

#[test]
fn closed_channels_drain_then_report_closed() {
    let src = r#"
ch = chan_new()
chan_send(ch, 1)
chan_send(ch, 2)
chan_close(ch)
println(chan_recv(ch))
println(chan_recv(ch))
try {
  chan_recv(ch)
  println("no error")
} catch e {
  println("recv: " + str(e))
}
try {
  chan_send(ch, 3)
  println("no error")
} catch e {
  println("send: " + str(e))
}
chan_close(ch)
println("double close ok")
"#;
    assert_eq!(run(src), "1\n2\nrecv: channel 1 is closed\nsend: channel 1 is closed\ndouble close ok");
}

#[test]
fn closing_a_channel_wakes_a_blocked_receiver() {
    let src = r#"
fn waiter(c) {
  try {
    return chan_recv(c)
  } catch e {
    return "woken"
  }
}
bad = 0
for round in range(10) {
  c = chan_new()
  w = async_spawn(waiter, c)
  sleep_ms(30)
  chan_close(c)
  if async_await(w) != "woken" {
    bad = bad + 1
  }
}
println(bad)
"#;
    assert_eq!(run(src), "0");
}

#[test]
fn messages_are_independent_copies() {
    let src = r#"
ch = chan_new()
xs = [1, 2]
chan_send(ch, xs)
push(xs, 3)
got = chan_recv(ch)
push(got, 99)
println(xs)
println(got)
"#;
    assert_eq!(run(src), "[1, 2, 3]\n[1, 2, 99]");
}

#[test]
fn tasks_return_every_kind_of_value() {
    let src = r#"
fn num() { return 2.5 }
fn str_() { return "text" }
fn nothing() { return null }
fn arr() { return [1, [2, 3], {"k": "v"}] }
fn dct() { return {"a": [1, 2], "b": {"c": true}} }
fn flag() { return false }
println(async_await(async_spawn(num)))
println(async_await(async_spawn(str_)))
println(async_await(async_spawn(nothing)))
println(async_await(async_spawn(arr)))
println(async_await(async_spawn(dct)))
println(async_await(async_spawn(flag)))
"#;
    assert_eq!(run(src), "2.5\ntext\nnull\n[1, [2, 3], {k: v}]\n{a: [1, 2], b: {c: true}}\nfalse");
}

#[test]
fn a_future_can_be_awaited_more_than_once() {
    // the second await used to wait 30 seconds and answer null
    let src = r#"
fn add(a, b) { return a + b }
h = async_spawn(add, 1, 2)
println(async_await(h))
println(async_await(h))
println(await h)
fn boom() { throw "again" }
f = async_spawn(boom)
for i in range(3) {
  try {
    async_await(f)
    println("no error")
  } catch e {
    println("caught " + str(e))
  }
}
"#;
    assert_eq!(run(src), "3\n3\n3\ncaught again\ncaught again\ncaught again");
}

#[test]
fn exceptions_in_tasks_reach_the_awaiter_every_time() {
    let src = r#"
fn bad_str() { throw "plain" }
fn bad_dict() { throw {"code": 7} }
fn bad_var() { return nosuch + 1 }
fn bad_div() { return 1 / 0 }
fn bad_call() {
  f = 5
  return f(1)
}
fn ok(x) { return x }
caught = 0
for round in range(10) {
  try { async_await(async_spawn(bad_str)) } catch e { if e == "plain" { caught = caught + 1 } }
  try { async_await(async_spawn(bad_dict)) } catch e { if e["code"] == 7 { caught = caught + 1 } }
  try { async_await(async_spawn(bad_var)) } catch e { caught = caught + 1 }
  try { async_await(async_spawn(bad_div)) } catch e { caught = caught + 1 }
  try { async_await(async_spawn(bad_call)) } catch e { caught = caught + 1 }
  if async_await(async_spawn(ok, round)) != round { caught = caught - 100 }
}
println(caught)
"#;
    assert_eq!(run(src), "50");
}

#[test]
fn gather_helpers_raise_task_errors_instead_of_returning_them() {
    // async_all / async_race / async_timeout used to hand back an internal "<task error>" value
    let src = r#"
fn bad() { throw "boom" }
fn good(x) { return x }
try {
  r = async_all([async_spawn(good, 1), async_spawn(bad)])
  println("no error " + str(r))
} catch e {
  println("all: " + str(e))
}
try {
  r = async_timeout(async_spawn(bad), 5000)
  println("no error " + str(r))
} catch e {
  println("timeout: " + str(e))
}
try {
  r = async_race([async_spawn(bad)])
  println("no error " + str(r))
} catch e {
  println("race: " + str(e))
}
println(async_all([async_spawn(good, 1), async_spawn(good, 2), 3]))
"#;
    assert_eq!(run(src), "all: boom\ntimeout: boom\nrace: boom\n[1, 2, 3]");
}

#[test]
fn async_all_keeps_the_order_of_its_futures() {
    let src = r#"
fn delayed(i, ms) {
  sleep_ms(ms)
  return i
}
bad = 0
for round in range(5) {
  fs = []
  for i in range(8) {
    push(fs, async_spawn(delayed, i, (8 - i) * 5))
  }
  if async_all(fs) != [0, 1, 2, 3, 4, 5, 6, 7] {
    bad = bad + 1
  }
}
println(bad)
"#;
    assert_eq!(run(src), "0");
}

#[test]
fn timeouts_and_races() {
    let src = r#"
fn slow() {
  sleep_ms(1500)
  return "slow"
}
fn quick() {
  return "quick"
}
println(async_timeout(async_spawn(slow), 50))
println(async_timeout(async_spawn(quick), 5000))
println(async_race([async_spawn(slow), async_spawn(quick)]))
fn waiter(c) {
  return chan_recv(c)
}
c = chan_new()
w = async_spawn(waiter, c)
println(async_timeout(w, 100))
chan_send(c, "late")
println(async_await(w))
"#;
    assert_eq!(run(src), "timeout\nquick\nquick\ntimeout\nlate");
}

#[test]
fn fan_out_and_nested_tasks() {
    let src = r#"
fn leaf(n) {
  return n + 1
}
fn mid(n) {
  hs = []
  for i in range(5) {
    push(hs, async_spawn(leaf, n * 10 + i))
  }
  s = 0
  for h in hs {
    s = s + async_await(h)
  }
  return s
}
total = 0
for round in range(5) {
  hs = []
  for i in range(10) {
    push(hs, async_spawn(mid, i))
  }
  for h in hs {
    total = total + async_await(h)
  }
}
println(total)
"#;
    // mid(i) = sum_{j<5} (10 i + j + 1) = 50 i + 15 ; sum over i<10 = 50*45 + 150 = 2400 ; 5 rounds
    assert_eq!(run(src), "12000");
}

#[test]
fn spawn_and_await_keywords_with_closures_and_classes() {
    let src = r#"
class Acc {
  fn init(n) {
    this.n = n
  }
  fn total() {
    s = 0
    for i in range(this.n) {
      s = s + i
    }
    return s
  }
}
fn run_acc(n) {
  return new Acc(n).total()
}
base = 100
adder = fn(x) {
  return x + base
}
hs = []
for i in range(1, 11) {
  push(hs, spawn run_acc(i * 10))
}
s = 0
for h in hs {
  s = s + await h
}
println(s)
println(await async_spawn(adder, 5))
async fn double(x) {
  return x * 2
}
println(await double(21))
"#;
    // sum_{i=1..10} (10i)(10i-1)/2 = sum (50 i^2 - 5 i) = 50*385 - 5*55 = 18975
    assert_eq!(run(src), "18975\n105\n42");
}

#[test]
fn compute_heavy_tasks_in_parallel() {
    // hot loops inside several tasks at once (JIT / fast paths must not share state)
    let src = r#"
fn crunch(seed) {
  s = 0
  for i in range(60000) {
    s = (s + i * seed) % 1000003
  }
  return s
}
bad = 0
for round in range(3) {
  hs = []
  for k in range(1, 9) {
    push(hs, async_spawn(crunch, k))
  }
  for k in range(1, 9) {
    if async_await(hs[k - 1]) != crunch(k) {
      bad = bad + 1
    }
  }
}
println(bad)
"#;
    assert_eq!(run(src), "0");
}

#[test]
fn mutex_cells_hold_data_across_tasks() {
    let src = r#"
cell = mutex_new({"items": []})
fn put(c, i) {
  mutex_set(c, {"items": [i], "who": i})
  return mutex_get(c)["who"] >= 0
}
hs = []
for i in range(30) {
  push(hs, async_spawn(put, cell, i))
}
ok = 0
for h in hs {
  if async_await(h) {
    ok = ok + 1
  }
}
v = mutex_get(cell)
println(ok)
println(len(v["items"]))
println(mutex_free(cell))
try {
  mutex_get(cell)
  println("no error")
} catch e {
  println("freed")
}
"#;
    assert_eq!(run(src), "30\n1\ntrue\nfreed");
}

#[test]
fn repeated_process_runs_are_deterministic() {
    let src = r#"
fn work(id) {
  s = 0
  for i in range(200) {
    s = s + i * id
  }
  return s
}
m = mutex_new(0)
fn tally(k) {
  mutex_add(m, k)
  return k
}
hs = []
for i in range(1, 9) {
  push(hs, async_spawn(work, i))
  push(hs, async_spawn(tally, i))
}
out = []
for h in hs {
  push(out, async_await(h))
}
println(out)
println(mutex_get(m))
"#;
    let first = run(src);
    assert_eq!(first, "[19900, 1, 39800, 2, 59700, 3, 79600, 4, 99500, 5, 119400, 6, 139300, 7, 159200, 8]\n36");
    for _ in 0..19 {
        assert_eq!(run(src), first);
    }
}

#[test]
fn a_task_that_never_finishes_does_not_block_exit_or_other_tasks() {
    let src = r#"
fn stuck(c) {
  return chan_recv(c)
}
c = chan_new()
w = async_spawn(stuck, c)
fn fine(x) {
  return x + 1
}
println(async_await(async_spawn(fine, 1)))
println(async_timeout(w, 50))
println("done")
"#;
    assert_eq!(run(src), "2\ntimeout\ndone");
}

#[test]
fn big_integers_and_functions_cross_threads_safely() {
    // BigInt and Function values hold Rc's; each thread must get its own, or the non-atomic
    // reference counts are updated concurrently (crashed with a freed-Rc assertion)
    let src = r#"
fn digits(n, f) {
  return len(str(f(n)))
}
fn ident(x) { return x }
big = factorial(40)
hs = []
for round in range(10) {
  for i in range(40) {
    push(hs, async_spawn(digits, big, ident))
  }
  for h in hs {
    async_await(h)
  }
  hs = []
}
ch = chan_new()
fn echo(c, v) {
  chan_send(c, v)
  chan_send(c, v + 1n)
  return v * 2
}
workers = []
for i in range(20) {
  push(workers, async_spawn(echo, ch, big))
}
t = 0
for w in workers {
  t = t + len(str(async_await(w)))
}
n = 0
for i in range(40) {
  chan_recv(ch)
  n = n + 1
}
println(t)
println(n)
println(async_await(async_spawn(ident, big)) == big)
"#;
    assert_eq!(run(src), "980\n40\ntrue");
}
