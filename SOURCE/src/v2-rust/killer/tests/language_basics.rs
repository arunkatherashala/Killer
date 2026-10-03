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

// ---------------------------------------------------------------- operators and literals

#[test]
fn augmented_assignment_and_increment() {
    let src = "x = 10\nx += 5\nx -= 3\nx *= 2\nx /= 4\nprintln(x)\nn = 1\nn++\nn++\nprintln(n)\n";
    assert_eq!(run(src), "6\n3");
}

#[test]
fn augmented_assignment_on_indexes_fields_and_inside_functions() {
    let src = "a = [1, 2, 3]\na[1] += 10\nprintln(a)\nclass C {\n  fn init() {\n    this.n = 0\n  }\n  fn bump() {\n    this.n += 2\n    this.n++\n  }\n}\nc = new C()\nc.bump()\nc.bump()\nprintln(c.n)\nfn total(xs) {\n  t = 0\n  for x in xs {\n    t += x\n  }\n  return t\n}\nprintln(total([1, 2, 3, 4]))\n";
    assert_eq!(run(src), "[1, 12, 3]\n6\n10");
}

#[test]
fn string_concatenation_with_plus_equals() {
    let src = "s = \"a\"\ns += \"b\"\ns += \"c\"\nprintln(s)\n";
    assert_eq!(run(src), "abc");
}

#[test]
fn multiple_assignment_swap_and_unpacking() {
    let src = "a, b = 1, 2\nprintln(a + b)\na, b = b, a\nprintln(a)\nprintln(b)\nfn pair() {\n  return [10, 20]\n}\nx, y = pair()\nprintln(x + y)\nfn f() {\n  p, q = 3, 4\n  return p * q\n}\nprintln(f())\n";
    assert_eq!(run(src), "3\n2\n1\n30\n12");
}

#[test]
fn word_operators_and_elif() {
    let src = "x = 7\nif x < 5 {\n  println(\"low\")\n} elif x < 10 and x > 6 {\n  println(\"mid\")\n} else {\n  println(\"high\")\n}\nprintln(true and false)\nprintln(false or true)\nprintln(not false)\nprintln(not x == 5)\n";
    assert_eq!(run(src), "mid\nfalse\ntrue\ntrue\ntrue");
}

#[test]
fn not_and_bang_bind_tighter_than_and_or() {
    let src = "println(!true && false)\nprintln(!false && true)\nprintln(!true || true)\nprintln(not true and false)\nprintln(not true or true)\n";
    assert_eq!(run(src), "false\ntrue\ntrue\nfalse\ntrue");
}

#[test]
fn string_literal_forms() {
    let src = "println('single')\nprintln(\"it's\")\nprintln('say \"hi\"')\nn = 5\nprintln(f\"n is {n}\")\nprintln(f\"n+1 is {n + 1}\")\ns = \"\"\"a\nb\"\"\"\nprintln(len(s))\n";
    assert_eq!(run(src), "single\nit's\nsay \"hi\"\nn is 5\nn+1 is 6\n3");
}

#[test]
fn number_literal_forms() {
    let src = "println(0xFF)\nprintln(0b101)\nprintln(0o17)\nprintln(1_000_000)\nprintln(0xFF + 1_000)\n";
    assert_eq!(run(src), "255\n5\n15\n1000000\n1255");
}

#[test]
fn comments_and_quotes_inside_strings_are_left_alone() {
    let src = "x = 1  # it's fine\ns = \"a # b\"\nprintln(s)\nt = 'a#b'\nprintln(t)\nprintln(x)\n";
    assert_eq!(run(src), "a # b\na#b\n1");
}

// ---------------------------------------------------------------- expression grammar

#[test]
fn unary_minus_binds_tighter_than_binary_operators() {
    let src = "x = 5\nprintln(-1 + 5)\nprintln(-2 * 3)\nprintln(2 - -3)\nprintln(-(1 + 2) * 2)\nprintln(-x + 10)\nprintln(-3 > -4)\nprintln(-2 ** 2)\nprintln(2 ** 3 ** 2)\n";
    assert_eq!(run(src), "4\n-6\n5\n-6\n5\ntrue\n-4\n512");
}

#[test]
fn arithmetic_precedence_and_associativity() {
    let src = "println(10 - 2 - 3)\nprintln(2 + 3 * 4)\nprintln(100 / 10 / 5)\nprintln(8 / 2 * 2)\nprintln(7 % 4 + 1)\nprintln((1 + 2) * (3 + 4))\n";
    assert_eq!(run(src), "5\n14\n2\n8\n4\n21");
}

#[test]
fn member_access_on_the_right_of_an_operator() {
    let src = "class C {\n  fn init() {\n    this.n = 3\n    this.flag = true\n  }\n  fn get() {\n    return 10\n  }\n}\nc = new C()\nprintln(1 + c.n)\nprintln(1 + c.get())\nprintln(true && c.flag)\nprintln(len([1, 2]) + c.n)\nprintln(c.get() + c.get())\nprintln(c.n > 2 && c.flag)\n";
    assert_eq!(run(src), "4\n11\ntrue\n5\n20\ntrue");
}

#[test]
fn logical_operators_follow_the_usual_precedence() {
    let src = "println(true && false || true)\nprintln(false && true || true)\nprintln(true || false && false)\nprintln(false || false && true)\n";
    assert_eq!(run(src), "true\ntrue\ntrue\nfalse");
}

#[test]
fn ternary_expressions() {
    let src = "x = 5\nprintln(x > 2 ? \"big\" : \"small\")\nprintln(x < 2 ? \"big\" : \"small\")\nprintln(\"big\" if x > 2 else \"small\")\nprintln(x > 9 ? 1 : x > 3 ? 2 : 3)\nprintln(\"a\" if x > 9 else \"b\" if x > 3 else \"c\")\nfn sign(n) {\n  return n > 0 ? 1 : n < 0 ? -1 : 0\n}\nprintln(sign(5))\nprintln(sign(-5))\nprintln(sign(0))\n";
    assert_eq!(run(src), "big\nsmall\nbig\n2\nb\n1\n-1\n0");
}

#[test]
fn ternary_does_not_confuse_other_question_mark_operators() {
    let src = "a = null\nprintln(a ?? 7)\nprintln(a?.x)\nb = 3\nprintln(b ?? 7)\n";
    assert_eq!(run(src), "7\nnull\n3");
}

#[test]
fn chained_comparisons() {
    let src = "println(1 < 2 < 3)\nprintln(1 < 3 < 2)\nx = 5\nprintln(1 < x <= 5)\nprintln(5 > x > 1)\nprintln(1 == 1 == 1)\n";
    assert_eq!(run(src), "true\nfalse\ntrue\nfalse\ntrue");
}

#[test]
fn membership_operators() {
    let src = "xs = [1, 2, 3]\nd = {\"a\": 1}\nprintln(2 in xs)\nprintln(5 in xs)\nprintln(5 not in xs)\nprintln(\"a\" in d)\nprintln(\"z\" in d)\nprintln(\"ell\" in \"hello\")\nprintln(\"x\" not in \"hello\")\nprintln([y for y in xs if y in [1, 3]])\n";
    assert_eq!(run(src), "true\nfalse\ntrue\ntrue\nfalse\ntrue\ntrue\n[1, 3]");
}

#[test]
fn bitwise_operators() {
    let src = "println(6 & 3)\nprintln(6 | 3)\nprintln(6 ^ 3)\nprintln(1 << 4)\nprintln(32 >> 2)\nprintln(~5)\nprintln(8 | 1 ^ 3 & 2)\nprintln(1 << 2 + 1)\nprintln((6 & 3) == 2)\nprintln(true || false)\n";
    assert_eq!(run(src), "2\n7\n5\n16\n8\n-6\n11\n8\ntrue\ntrue");
}

#[test]
fn tuple_literals() {
    let src = "t = (1, 2, 3)\nprintln(t[1])\nprintln(len(t))\nprintln((1 + 2) * 3)\nfn two() {\n  return (10, 20)\n}\na, b = two()\nprintln(a + b)\n";
    assert_eq!(run(src), "2\n3\n9\n30");
}

#[test]
fn not_binds_looser_than_comparison_but_tighter_than_and_or() {
    let src = "x = 5\nprintln(not x == 5)\nprintln(not x == 6)\nprintln(not x == 6 and true)\nprintln(not true or true)\nprintln(not (1 == 2))\nprintln(not contains([1, 2], 3))\n";
    assert_eq!(run(src), "false\ntrue\ntrue\ntrue\ntrue\ntrue");
}

#[test]
fn comprehensions_can_use_local_and_top_level_variables() {
    let src = "xs = [1, 2, 3, 4]\nprintln([y for y in xs])\nprintln([y * 2 for y in xs if y > 1])\nn = 10\nprintln([y + n for y in xs])\nfn evens(limit) {\n  base = 2\n  return [i * base for i in range(limit) if i % 2 == 0]\n}\nprintln(evens(10))\nwords = [\"ab\", \"c\", \"def\"]\nprintln([len(w) for w in words])\nprintln(len([w for w in words if len(w) > 1]))\n";
    assert_eq!(run(src), "[1, 2, 3, 4]\n[4, 6, 8]\n[11, 12, 13, 14]\n[0, 4, 8, 12, 16]\n[2, 1, 3]\n2");
}

#[test]
fn comprehension_variable_does_not_leak_over_an_outer_variable() {
    let src = "x = 100\nxs = [1, 2]\nprintln([x * 2 for x in xs])\nprintln(x)\nfn f(a) {\n  return [a * 2 for a in [5, 6]]\n}\nprintln(f(1))\n";
    assert_eq!(run(src), "[2, 4]\n100\n[10, 12]");
}

#[test]
fn comprehensions_support_destructuring_and_several_clauses() {
    let src = "pairs = [[1, 2], [3, 4]]\nprintln([a + b for a, b in pairs])\nprintln([[a, b] for a in range(2) for b in range(3) if a < b])\nm = [[1, 2], [3, 4], [5, 6]]\nprintln([c for row in m for c in row])\nprintln([c * 2 for row in m if len(row) == 2 for c in row if c > 2])\nprintln([i for i in range(10) if i % 2 == 0 if i > 3])\nprintln([[c * 2 for c in row] for row in m])\n";
    assert_eq!(run(src), "[3, 7]\n[[0, 1], [0, 2], [1, 2]]\n[1, 2, 3, 4, 5, 6]\n[6, 8, 10, 12]\n[4, 6, 8]\n[[2, 4], [6, 8], [10, 12]]");
}

#[test]
fn everyday_builtins_are_callable_from_scripts() {
    let src = "println(ord(\"a\"))\nprintln(chr(66))\nprintln(pad_left(\"7\", 3, \"0\"))\nprintln(gcd(12, 18))\nprintln(lcm(4, 6))\nprintln(mean([1, 2, 3, 4]))\nprintln(median([3, 1, 2]))\nprintln(log2(8))\nprintln(bool(\"\"))\nprintln(base64_encode(\"foobar\"))\nprintln(base64_decode(\"Zm9vYmFy\"))\nprintln(len(set([1, 2, 2, 3])))\nprintln(float(\"2.5\"))\nxs = [1, 2, 3]\ninsert(xs, 0, 9)\nprintln(xs)\n";
    assert_eq!(run(src), "97\nB\n007\n6\n12\n2.5\n2\n3\nfalse\nZm9vYmFy\nfoobar\n3\n2.5\n[9, 1, 2, 3]");
}

#[test]
fn strings_index_slice_reverse_and_compare() {
    let src = "s = \"héllo\"\nprintln(s[1])\nprintln(s[-1])\nprintln(len(s))\nprintln(s[1:3])\nprintln(s[:2])\nprintln(s[3:])\nprintln(reverse(s))\nprintln(\"ab\" < \"b\")\nprintln(\"b\" <= \"a\")\nprintln(\"ab\" * 3)\nprintln(s[9])\n";
    assert_eq!(run(src), "é\no\n5\n\u{e9}l\nhé\nlo\nolléh\ntrue\nfalse\nababab\nnull");
}

#[test]
fn lists_slice_concat_repeat_and_sets_contain() {
    let src = "a = [1, 2, 3, 4, 5]\nprintln(a[1:3])\nprintln(a[-2:])\nprintln(a[:])\nprintln(a[1 > 0 ? 1 : 2])\nprintln([1, 2] + [3])\nprintln([0] * 3)\nb = a[:2]\nb[0] = 99\nprintln(a[0])\nst = set([1, 2, 2])\nprintln(2 in st)\nprintln(contains(st, 7))\nprintln(type(gc_stats()))\n";
    assert_eq!(run(src), "[2, 3]\n[4, 5]\n[1, 2, 3, 4, 5]\n2\n[1, 2, 3]\n[0, 0, 0]\n1\ntrue\nfalse\ndict");
}

#[test]
fn try_catch_throw_with_any_value() {
    let src = "try {\n  throw \"boom\"\n} catch e {\n  println(\"caught \" + str(e))\n}\ntry {\n  throw {\"code\": 42}\n} catch e {\n  println(e[\"code\"])\n}\ntry {\n  x = 1 / 0\n} catch e {\n  println(e)\n}\nprintln(\"after\")\n";
    assert_eq!(run(src), "caught boom\n42\nDivision by zero\nafter");
}

#[test]
fn errors_unwind_through_function_calls_and_returns() {
    let src = "fn risky(n) {\n  if n > 2 {\n    throw \"too big: \" + str(n)\n  }\n  return n * 10\n}\nfn safe(n) {\n  try {\n    return risky(n)\n  } catch e {\n    return \"caught \" + e\n  }\n}\nprintln(safe(1))\nprintln(safe(5))\nprintln(safe(2))\nfn deep() {\n  throw \"deep\"\n}\nfn mid() {\n  deep()\n}\ntry {\n  mid()\n} catch e {\n  println(\"got \" + e)\n}\n";
    assert_eq!(run(src), "10\ncaught too big: 5\n20\ngot deep");
}

#[test]
fn finally_runs_on_every_path_and_rethrows() {
    let src = "try {\n  println(\"a\")\n} finally {\n  println(\"b\")\n}\ntry {\n  throw \"x\"\n} catch e {\n  println(\"c\")\n} finally {\n  println(\"f\")\n}\ntry {\n  try {\n    throw \"y\"\n  } finally {\n    println(\"cleanup\")\n  }\n} catch e {\n  println(\"got \" + e)\n}\ntry {\n  try {\n    throw \"in\"\n  } catch e {\n    throw \"out from \" + e\n  }\n} catch e2 {\n  println(e2)\n}\n";
    assert_eq!(run(src), "a\nb\nc\nf\ncleanup\ngot y\nout from in");
}

#[test]
fn try_inside_loops_with_break_and_continue_stays_correct() {
    let src = "total = 0\nfor i in range(1000) {\n  try {\n    if i % 3 == 0 {\n      continue\n    }\n    if i > 990 {\n      break\n    }\n    total = total + 1\n  } catch e {\n    println(\"never\")\n  }\n}\nprintln(total)\ntry {\n  y = 1 / 0\n} catch e {\n  println(\"still catching\")\n}\ncaught = 0\nfor i in range(1000) {\n  try {\n    if i % 2 == 0 {\n      throw i\n    }\n  } catch e {\n    caught = caught + 1\n  }\n}\nprintln(caught)\n";
    assert_eq!(run(src), "660\nstill catching\n500");
}

#[test]
fn errors_inside_callbacks_are_catchable_and_uncaught_ones_fail() {
    let src = "fn boom(x) {\n  if x == 3 {\n    throw \"bad three\"\n  }\n  return x\n}\ntry {\n  println(map([1, 2, 3], boom))\n} catch e {\n  println(\"map failed: \" + e)\n}\nprintln(map([1, 2], boom))\nthrow \"final\"\nprintln(\"unreachable\")\n";
    let out = run(src);
    assert!(out.starts_with("map failed: bad three\n[1, 2]\n"), "{out}");
    assert!(out.contains("Uncaught exception: final"), "{out}");
    assert!(!out.contains("unreachable"), "{out}");
}

#[test]
fn lambdas_arrows_and_closures() {
    let src = "f = fn(a) {\n  return a * 3\n}\nprintln(f(4))\ng = (a, b) => a + b\nprintln(g(2, 3))\nprintln(map([1, 2, 3], x => x + 1))\nprintln(reduce([1, 2, 3, 4], (a, b) => a + b, 0))\nprintln(filter([1, 2, 3, 4], n => n % 2 == 0))\nfn scale(xs, k) {\n  return map(xs, v => v * k)\n}\nprintln(scale([1, 2], 7))\nfn adder(k) {\n  return fn(x) {\n    return x + k\n  }\n}\nadd5 = adder(5)\nprintln(add5(10))\nprintln(map([1, 2, 3], fn(x) {\n  return x * 10\n}))\n";
    assert_eq!(run(src), "12\n5\n[2, 3, 4]\n10\n[2, 4]\n[7, 14]\n15\n[10, 20, 30]");
}

#[test]
fn closures_keep_their_own_state_between_calls() {
    let src = "fn make() {\n  n = 0\n  fn inc() {\n    n = n + 1\n    return n\n  }\n  return inc\n}\nc = make()\nprintln(c())\nprintln(c())\nd = make()\nprintln(d())\nprintln(c())\n";
    assert_eq!(run(src), "1\n2\n1\n3");
}

#[test]
fn decorators_rebind_the_function() {
    let src = "fn loud(f) {\n  fn wrapper(x) {\n    return f(x) + 1\n  }\n  return wrapper\n}\n@loud\nfn base(x) {\n  return x * 2\n}\nprintln(base(5))\n";
    assert_eq!(run(src), "11");
}
