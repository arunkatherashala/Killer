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
