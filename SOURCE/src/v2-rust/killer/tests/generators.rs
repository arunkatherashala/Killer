//! Lazy generators: `yield` suspends a function; `next(g)` and `for` resume it on demand.
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_gen_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn calling_a_generator_does_not_run_its_body() {
    let src = r#"
fn gen() {
    print("body started")
    yield 1
    print("after first")
    yield 2
}
g = gen()
print("created")
print(next(g))
print(next(g))
"#;
    assert_eq!(run(src), "created\nbody started\n1\nafter first\n2");
}

#[test]
fn infinite_generator_consumed_with_next() {
    let src = r#"
fn naturals() {
    n = 0
    while true {
        yield n
        n = n + 1
    }
}
g = naturals()
total = 0
for i in range(5) {
    total = total + next(g)
}
print(total)
print(next(g))
"#;
    assert_eq!(run(src), "10\n5");
}

#[test]
fn infinite_generator_with_for_and_break() {
    let src = r#"
fn naturals() {
    n = 1
    while true {
        yield n
        n = n + 1
    }
}
for v in naturals() {
    if v > 4 {
        break
    }
    print(v)
}
"#;
    assert_eq!(run(src), "1\n2\n3\n4");
}

#[test]
fn generator_continues_after_a_for_loop_breaks() {
    let src = r#"
fn naturals() {
    n = 0
    while true {
        yield n
        n = n + 1
    }
}
g = naturals()
for v in g {
    if v == 2 {
        break
    }
}
print(next(g))
print(next(g))
"#;
    assert_eq!(run(src), "3\n4");
}

#[test]
fn fibonacci_generator() {
    let src = r#"
fn fib() {
    a = 0
    b = 1
    while true {
        yield a
        t = a + b
        a = b
        b = t
    }
}
f = fib()
out = []
for i in range(12) {
    push(out, next(f))
}
print(out)
"#;
    assert_eq!(run(src), "[0, 1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]");
}

#[test]
fn finite_generator_in_for_loop() {
    let src = r#"
fn upto(n) {
    for i in range(n) {
        yield i * i
    }
}
for x in upto(5) {
    print(x)
}
"#;
    assert_eq!(run(src), "0\n1\n4\n9\n16");
}

#[test]
fn next_after_exhaustion_gives_default_or_null() {
    let src = r#"
fn two() {
    yield "a"
    yield "b"
}
g = two()
print(next(g))
print(next(g))
print(next(g, "done"))
print(next(g))
print(next(g, 0))
"#;
    assert_eq!(run(src), "a\nb\ndone\nnull\n0");
}

#[test]
fn for_over_an_exhausted_generator_runs_zero_times() {
    let src = r#"
fn one() {
    yield 1
}
g = one()
print(next(g))
for x in g {
    print("never")
}
print("end")
"#;
    assert_eq!(run(src), "1\nend");
}

#[test]
fn has_next_peeks_without_losing_the_value() {
    let src = r#"
fn two() {
    yield 10
    yield 20
}
g = two()
print(has_next(g))
print(next(g))
print(has_next(g))
print(next(g))
print(has_next(g))
"#;
    assert_eq!(run(src), "true\n10\ntrue\n20\nfalse");
}

#[test]
fn generator_with_locals_loops_and_conditions() {
    let src = r#"
fn evens_squared(limit) {
    count = 0
    i = 0
    while i < limit {
        if i % 2 == 0 {
            count = count + 1
            yield "even#" + str(count) + "=" + str(i * i)
        }
        i = i + 1
    }
}
for s in evens_squared(7) {
    print(s)
}
"#;
    assert_eq!(run(src), "even#1=0\neven#2=4\neven#3=16\neven#4=36");
}

#[test]
fn try_catch_finally_inside_a_generator() {
    let src = r#"
fn safe() {
    try {
        yield 1
        throw "boom"
    } catch e {
        yield "caught " + str(e)
    } finally {
        print("cleanup")
    }
    yield 3
}
for s in safe() {
    print(s)
}
"#;
    assert_eq!(run(src), "1\ncaught boom\ncleanup\n3");
}

#[test]
fn try_region_spans_several_resumes() {
    let src = r#"
fn guarded() {
    try {
        yield 1
        yield 2
        x = 10 / 0
        yield 3
    } catch e {
        yield "recovered"
    }
}
g = guarded()
print(next(g))
print(next(g))
print(next(g))
print(next(g, "end"))
"#;
    let out = run(src);
    assert!(out.starts_with("1\n2\n"), "{out}");
    assert!(out.ends_with("end"), "{out}");
}

#[test]
fn generator_calls_other_functions() {
    let src = r#"
fn square(x) {
    return x * x
}
fn helper(n) {
    s = 0
    for i in range(n) {
        s = s + i
    }
    return s
}
fn squares(n) {
    for i in range(n) {
        yield square(i) + helper(i)
    }
}
for v in squares(5) {
    print(v)
}
"#;
    // i*i + (0+..+i-1): 0, 1, 5, 12, 22
    assert_eq!(run(src), "0\n1\n5\n12\n22");
}

#[test]
fn generator_passed_to_a_function_and_consumed_partially() {
    let src = r#"
fn naturals() {
    n = 0
    while true {
        yield n
        n = n + 1
    }
}
fn take(g, k) {
    out = []
    for i in range(k) {
        push(out, next(g))
    }
    return out
}
g = naturals()
print(take(g, 3))
print(take(g, 3))
print(next(g))
"#;
    assert_eq!(run(src), "[0, 1, 2]\n[3, 4, 5]\n6");
}

#[test]
fn two_generators_from_one_function_are_independent() {
    let src = r#"
fn counter(start) {
    n = start
    while true {
        yield n
        n = n + 1
    }
}
a = counter(0)
b = counter(100)
print(next(a))
print(next(b))
print(next(a))
print(next(b))
print(next(a))
print(next(b))
"#;
    assert_eq!(run(src), "0\n100\n1\n101\n2\n102");
}

#[test]
fn generators_delegating_to_generators() {
    let src = r#"
fn inner(a, b) {
    for i in range(a, b) {
        yield i
    }
}
fn outer() {
    for x in inner(0, 2) {
        yield x
    }
    for y in inner(10, 12) {
        yield y
    }
}
for v in outer() {
    print(v)
}
"#;
    assert_eq!(run(src), "0\n1\n10\n11");
}

#[test]
fn recursive_generator() {
    let src = r#"
fn walk(n) {
    if n > 0 {
        for x in walk(n - 1) {
            yield x
        }
        yield n
    }
}
for v in walk(4) {
    print(v)
}
"#;
    assert_eq!(run(src), "1\n2\n3\n4");
}

#[test]
fn generator_methods_in_classes() {
    let src = r#"
class Bag {
    fn init(items) {
        this.items = items
    }
    fn each() {
        for it in this.items {
            yield it
        }
    }
    fn doubled() {
        for it in this.items {
            yield it * 2
        }
    }
}
b = new Bag([1, 2, 3])
for x in b.each() {
    print(x)
}
d = b.doubled()
print(next(d))
print(next(d))
print(next(d))
print(next(d, "end"))
"#;
    assert_eq!(run(src), "1\n2\n3\n2\n4\n6\nend");
}

#[test]
fn generator_captures_variables_like_a_closure() {
    let src = r#"
fn make(prefix) {
    fn labels(n) {
        for i in range(n) {
            yield prefix + str(i)
        }
    }
    return labels(3)
}
g = make("item")
print(next(g))
print(next(g))
print(next(g))
print(next(g, "end"))
"#;
    assert_eq!(run(src), "item0\nitem1\nitem2\nend");
}

#[test]
fn error_in_generator_propagates_to_the_consumer() {
    let src = r#"
fn bad() {
    yield 1
    throw "oops"
}
b = bad()
print(next(b))
try {
    next(b)
} catch e {
    print("consumer got " + str(e))
}
print(next(b, "finished"))
"#;
    assert_eq!(run(src), "1\nconsumer got oops\nfinished");
}

#[test]
fn error_in_generator_propagates_through_for() {
    let src = r#"
fn bad() {
    yield 1
    yield 2
    throw "late failure"
}
try {
    for x in bad() {
        print(x)
    }
} catch e {
    print("caught: " + str(e))
}
print("after")
"#;
    assert_eq!(run(src), "1\n2\ncaught: late failure\nafter");
}

#[test]
fn consumer_state_survives_a_generator_error() {
    let src = r#"
fn bad() {
    throw "immediately"
    yield 1
}
fn run_it() {
    total = 5
    try {
        for x in bad() {
            total = total + x
        }
    } catch e {
        total = total + 100
    }
    return total
}
print(run_it())
print(run_it())
"#;
    assert_eq!(run(src), "105\n105");
}

#[test]
fn default_arguments_work_for_generators() {
    let src = r#"
fn rep(word, times = 2) {
    for i in range(times) {
        yield word
    }
}
for w in rep("hi") {
    print(w)
}
for w in rep("yo", 1) {
    print(w)
}
"#;
    assert_eq!(run(src), "hi\nhi\nyo");
}

#[test]
fn yield_inside_nested_blocks_and_loops() {
    let src = r#"
fn grid(h, w) {
    for r in range(h) {
        for c in range(w) {
            if (r + c) % 2 == 0 {
                yield str(r) + "," + str(c)
            }
        }
    }
}
out = []
for cell in grid(2, 3) {
    push(out, cell)
}
print(out)
"#;
    assert_eq!(run(src), "[0,0, 0,2, 1,1]");
}

#[test]
fn ordinary_functions_are_unaffected() {
    let src = r#"
fn add(a, b) {
    return a + b
}
fn fact(n) {
    if n <= 1 {
        return 1
    }
    return n * fact(n - 1)
}
print(add(2, 3))
print(fact(5))
"#;
    assert_eq!(run(src), "5\n120");
}

#[test]
fn a_yield_in_a_nested_function_does_not_make_the_outer_one_a_generator() {
    let src = r#"
fn outer() {
    fn inner() {
        yield 1
    }
    return 42
}
print(outer())
"#;
    assert_eq!(run(src), "42");
}

#[test]
fn side_effects_run_on_demand() {
    let src = r#"
fn noisy() {
    for i in range(3) {
        print("producing " + str(i))
        yield i
    }
}
g = noisy()
print("start")
x = next(g)
print("got " + str(x))
y = next(g)
print("got " + str(y))
"#;
    assert_eq!(run(src), "start\nproducing 0\ngot 0\nproducing 1\ngot 1");
}

#[test]
fn many_generators_do_not_interfere_with_the_caller_frame() {
    let src = r#"
fn gen(k) {
    yield k
    yield k + 1
}
fn work() {
    keep = 7
    a = gen(10)
    b = gen(20)
    x = next(a) + next(b)
    y = next(a) + next(b)
    return keep + x + y
}
print(work())
"#;
    // 7 + (10+20) + (11+21)
    assert_eq!(run(src), "69");
}

#[test]
fn hot_generator_function_is_never_compiled_natively() {
    // called often enough to trigger the function JIT; must keep behaving like a generator
    let src = r#"
fn one(k) {
    yield k + 1
}
fn main() {
    s = 0
    for i in range(500) {
        g = one(i)
        s = s + next(g)
    }
    return s
}
print(main())
"#;
    assert_eq!(run(src), "125250");
}

#[test]
fn deeply_nested_generator_resumes() {
    let src = r#"
fn depth(n) {
    if n == 0 {
        yield "bottom"
    } else {
        for x in depth(n - 1) {
            yield x
        }
    }
}
for v in depth(40) {
    print(v)
}
"#;
    assert_eq!(run(src), "bottom");
}
