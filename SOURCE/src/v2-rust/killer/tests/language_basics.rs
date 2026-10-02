//! Regression tests for core-language behaviour found by the parity audit (parity/).
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn run(src: &str) -> String {
    let path = std::env::temp_dir().join(format!("killer_lb_{}_{}.killer", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super")).arg(&path).arg("--run").output().unwrap();
    let _ = std::fs::remove_file(&path);
    format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).replace("\r\n", "\n").trim().to_string()
}

#[test]
fn negative_literal_before_a_function_does_not_break_jumps() {
    // `-5` compiles to `0 - 5`; folding it used to leave every later jump pointing at the wrong place
    let src = "temp = -5\nfn f(a) {\n  return a + 1\n}\nprintln(f(temp))\nif temp < 0 {\n  println(\"neg\")\n}\n";
    assert_eq!(run(src), "-4\nneg");
}

#[test]
fn negative_literals_in_loops_and_nested_functions() {
    let src = "fn g(a) {\n  return a * -2\n}\nfn h(a) {\n  t = 0\n  for i in range(a) {\n    t = t + g(i) - 1\n  }\n  return t\n}\nprintln(h(4))\n";
    assert_eq!(run(src), "-16");
}

#[test]
fn negative_array_indices_count_from_the_end() {
    let src = "a = [10, 20, 30]\nprintln(a[-1])\nprintln(a[-3])\nprintln(a[-4])\nprintln(a[3])\na[-1] = 99\nprintln(a)\nfn last(xs) {\n  return xs[-1]\n}\nprintln(last([1, 2, 3]))\n";
    assert_eq!(run(src), "30\n10\nnull\nnull\n[10, 20, 99]\n3");
}

#[test]
fn negative_index_in_a_hot_function_matches_the_interpreter() {
    // called often enough to be compiled natively; out-of-range reads fall back and still agree
    let src = "fn last(xs) {\n  return xs[-1]\n}\nfn main() {\n  s = 0\n  for i in range(200) {\n    s = s + last([i, i + 1, i + 2])\n  }\n  return s\n}\nprintln(main())\n";
    assert_eq!(run(src), "20300");
}

#[test]
fn default_arguments_fill_in_omitted_parameters() {
    let src = "fn greet(name, greeting = \"Hi\") {\n  return greeting + \" \" + name\n}\nprintln(greet(\"Sai\"))\nprintln(greet(\"Sai\", \"Yo\"))\n";
    assert_eq!(run(src), "Hi Sai\nYo Sai");
}

#[test]
fn defaults_can_be_expressions_use_earlier_parameters_and_contain_commas() {
    let src = "fn f(a, b = a * 2, c = [1, 2, 3], d = \"x, y: z\") {\n  return str(a) + \"/\" + str(b) + \"/\" + str(len(c)) + \"/\" + d\n}\nprintln(f(5))\nprintln(f(5, 1))\nprintln(f(5, 1, [9]))\nprintln(f(5, 1, [9], \"q\"))\n";
    assert_eq!(run(src), "5/10/3/x, y: z\n5/1/3/x, y: z\n5/1/1/x, y: z\n5/1/1/q");
}

#[test]
fn defaults_work_with_type_annotations() {
    let src = "fn area(w: number, h: number = 10) -> number {\n  return w * h\n}\nprintln(area(3))\nprintln(area(3, 4))\n";
    assert_eq!(run(src), "30\n12");
}

#[test]
fn a_required_parameter_after_a_default_is_rejected() {
    let out = run("fn f(a = 1, b) {\n  return a\n}\nprintln(f(1, 2))\n");
    assert!(out.contains("without a default follows"), "{}", out);
}

#[test]
fn methods_support_default_arguments_and_omitted_arguments_are_null() {
    let src = "class C {\n  fn init(n = 7) {\n    this.n = n\n  }\n  fn add(x, y = 10) {\n    return x + y\n  }\n  fn maybe(a) {\n    return a == null\n  }\n}\nc = new C()\nprintln(c.n)\nprintln(new C(3).n)\nprintln(c.add(1))\nprintln(c.add(1, 2))\nprintln(c.maybe())\n";
    assert_eq!(run(src), "7\n3\n11\n3\ntrue");
}

#[test]
fn defaults_work_in_a_hot_function_called_many_times() {
    let src = "fn scale(x, k = 3) {\n  return x * k\n}\nfn main() {\n  t = 0\n  for i in range(300) {\n    t = t + scale(i) + scale(i, 2)\n  }\n  return t\n}\nprintln(main())\n";
    assert_eq!(run(src), "224250");
}

const ANIMAL: &str = "class Animal {\n  fn init(name) {\n    this.name = name\n  }\n  fn speak() {\n    return this.name + \" makes a sound\"\n  }\n}\n";

#[test]
fn subclass_without_init_inherits_the_parents_constructor() {
    let src = format!("{}class Dog extends Animal {{\n  fn fetch() {{\n    return this.name + \" fetches\"\n  }}\n}}\nd = new Dog(\"Rex\")\nprintln(d.speak())\nprintln(d.fetch())\n", ANIMAL);
    assert_eq!(run(&src), "Rex makes a sound\nRex fetches");
}

#[test]
fn constructor_is_inherited_through_several_levels() {
    let src = format!("{}class Dog extends Animal {{\n}}\nclass Puppy extends Dog {{\n}}\np = new Puppy(\"Bit\")\nprintln(p.name)\n", ANIMAL);
    assert_eq!(run(&src), "Bit");
}

#[test]
fn a_subclass_init_overrides_and_extra_arguments_are_ignored_without_init() {
    let src = format!("{}class Cat extends Animal {{\n  fn init(name, lives) {{\n    this.name = name\n    this.lives = lives\n  }}\n}}\nc = new Cat(\"Tom\", 9)\nprintln(c.name + str(c.lives))\nclass Plain {{\n}}\np = new Plain(1, 2, 3)\nprintln(type(p))\nprintln(\"stack ok\")\n", ANIMAL);
    assert_eq!(run(&src), "Tom9\nPlain\nstack ok");
}

#[test]
fn constructor_default_arguments_work_with_zero_arguments() {
    let src = "class C {\n  fn init(n = 7) {\n    this.n = n\n  }\n}\nprintln(new C().n)\nprintln(new C(3).n)\n";
    assert_eq!(run(src), "7\n3");
}

#[test]
fn polymorphism_through_inherited_constructors() {
    let src = format!("{}class Cat extends Animal {{\n  fn speak() {{\n    return \"meow\"\n  }}\n}}\nxs = [new Animal(\"a\"), new Cat(\"c\")]\nfor x in xs {{\n  println(x.speak())\n}}\nprintln(xs[1].name)\n", ANIMAL);
    assert_eq!(run(&src), "a makes a sound\nmeow\nc");
}

// ---------------------------------------------------------------- first-class functions

#[test]
fn functions_are_values() {
    let src = "fn double(a) {\n  return a * 2\n}\nf = double\nprintln(f(4))\nfn apply(g, x) {\n  return g(x)\n}\nprintln(apply(double, 21))\nfn make() {\n  return double\n}\nprintln(make()(5))\n";
    assert_eq!(run(src), "8\n42\n10");
}

#[test]
fn functions_in_lists_and_dicts_can_be_called() {
    let src = "fn a() {\n  return 1\n}\nfn b(x) {\n  return x + 2\n}\nfs = [a, b]\nprintln(fs[0]())\nprintln(fs[1](40))\nd = {\"go\": b}\nprintln(d[\"go\"](5))\n";
    assert_eq!(run(src), "1\n42\n7");
}

#[test]
fn a_variable_shadows_a_function_of_the_same_name() {
    let src = "fn f(x) {\n  return x + 1\n}\nfn g(f) {\n  return f(10)\n}\nfn h(x) {\n  return x * 100\n}\nprintln(f(1))\nprintln(g(h))\n";
    assert_eq!(run(src), "2\n1000");
}

#[test]
fn map_filter_reduce_accept_user_functions() {
    let src = "fn d(a) {\n  return a * 2\n}\nfn even(x) {\n  return x % 2 == 0\n}\nfn add(a, b) {\n  return a + b\n}\nprintln(map([1, 2, 3], d))\nprintln(filter([1, 2, 3, 4], even))\nprintln(reduce([1, 2, 3, 4], add, 0))\nprintln(reduce([1, 2, 3, 4], add))\n";
    assert_eq!(run(src), "[2, 4, 6]\n[2, 4]\n10\n10");
}

#[test]
fn callbacks_run_the_full_interpreter_not_a_subset() {
    // loops, local variables, nested calls and early returns inside a callback
    let src = "fn tri(n) {\n  t = 0\n  for i in range(n + 1) {\n    t = t + i\n  }\n  return t\n}\nfn classify(n) {\n  if n > 2 {\n    return \"big\"\n  }\n  return tri(n)\n}\nprintln(map([1, 2, 3, 4], tri))\nprintln(map([1, 2, 3], classify))\n";
    assert_eq!(run(src), "[1, 3, 6, 10]\n[1, 3, big]");
}

#[test]
fn sorted_with_a_key_function_is_stable_and_supports_reverse() {
    let src = "fn neg(x) {\n  return 0 - x\n}\nfn size(s) {\n  return len(s)\n}\nprintln(sorted([1, 3, 2], neg))\nprintln(sorted([\"ccc\", \"a\", \"bb\", \"dd\"], size))\nprintln(sorted([\"ccc\", \"a\", \"bb\"], size, true))\n";
    assert_eq!(run(src), "[3, 2, 1]\n[a, bb, dd, ccc]\n[ccc, bb, a]");
}

#[test]
fn builtins_still_accept_builtin_names_as_strings() {
    let src = "println(map([\"a\", \"b\"], \"upper\"))\nprintln(sorted([3, 1, 2]))\nprintln(sorted([3, 1, 2], true))\n";
    assert_eq!(run(src), "[A, B]\n[1, 2, 3]\n[3, 2, 1]");
}

// ---------------------------------------------------------------- return values and globals

#[test]
fn a_function_without_return_yields_null_and_does_not_underflow() {
    let src = "fn f() {\n  y = 1\n}\nprintln(f())\nx = f()\nprintln(x == null)\n";
    assert_eq!(run(src), "null\ntrue");
}

#[test]
fn falling_off_the_end_of_an_if_does_not_run_on_into_later_code() {
    // the last instruction of f is the Ret inside the if; the false branch used to continue into main code
    let src = "fn f(x) {\n  if x > 0 {\n    return \"pos\"\n  }\n}\nprintln(f(1))\nprintln(f(-1))\nprintln(\"after\")\n";
    assert_eq!(run(src), "pos\nnull\nafter");
}

#[test]
fn the_last_expression_of_a_function_is_its_value() {
    let src = "fn add(a, b) {\n  a + b\n}\nfn twice(a) {\n  add(a, a)\n}\nprintln(add(2, 3))\nprintln(twice(4))\n";
    assert_eq!(run(src), "5\n8");
}

#[test]
fn expression_statements_in_the_middle_of_a_function_are_discarded() {
    let src = "fn noisy() {\n  return 5\n}\nfn f() {\n  noisy()\n  noisy()\n  return 1\n}\nt = 0\nfor i in range(1000) {\n  t = t + f()\n  noisy()\n}\nprintln(t)\n";
    assert_eq!(run(src), "1000");
}

#[test]
fn functions_can_read_top_level_variables() {
    let src = "g = 5\nfn f() {\n  return g + 1\n}\nprintln(f())\ng = 10\nprintln(f())\n";
    assert_eq!(run(src), "6\n11");
}

#[test]
fn global_declaration_lets_a_function_write_a_top_level_variable() {
    let src = "total = 0\nfn add(n) {\n  global total\n  total = total + n\n}\nadd(5)\nadd(6)\nprintln(total)\n";
    assert_eq!(run(src), "11");
}

#[test]
fn assignment_inside_a_function_is_local_unless_declared_global() {
    let src = "x = 1\nfn f() {\n  x = 99\n  return x\n}\nprintln(f())\nprintln(x)\n";
    assert_eq!(run(src), "99\n1");
}

#[test]
fn a_global_holding_a_function_can_be_called_from_another_function() {
    let src = "fn double(a) {\n  return a * 2\n}\nop = double\nfn run(x) {\n  return op(x)\n}\nprintln(run(21))\n";
    assert_eq!(run(src), "42");
}

#[test]
fn recursion_and_mutual_recursion_still_work() {
    let src = "fn even(n) {\n  if n == 0 {\n    return true\n  }\n  return odd(n - 1)\n}\nfn odd(n) {\n  if n == 0 {\n    return false\n  }\n  return even(n - 1)\n}\nprintln(even(10))\nprintln(odd(7))\n";
    assert_eq!(run(src), "true\ntrue");
}
