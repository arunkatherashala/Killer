//! `killer wasm`: every compiled function is executed by a small WebAssembly interpreter written here from the
//! specification (no wasm runtime is needed) and its results are compared with the Killer VM running the same source.
use killer_native::wasm_compile::compile;
use std::process::Command;

// ---------------------------------------------------------------------------------------------
// a minimal WebAssembly decoder + interpreter (f64 / i32 subset used by the compiler)

#[derive(Clone, Copy, Debug)]
enum V {
    I(i32),
    F(f64),
}

#[derive(Clone, Debug)]
enum Op {
    Block(usize), // index of the matching end
    Loop,
    If(usize, Option<usize>), // end, else
    Else,
    End,
    Br(u32),
    BrIf(u32),
    Return,
    Call(u32),
    Drop,
    Get(u32),
    Set(u32),
    I32Const(i32),
    F64Const(f64),
    Simple(u8),
}

struct Func {
    nparams: usize,
    nlocals: usize,
    ops: Vec<Op>,
}

struct Module {
    funcs: Vec<Func>,
    exports: Vec<(String, usize)>,
}

struct R<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> R<'a> {
    fn u8(&mut self) -> u8 {
        let v = self.b[self.p];
        self.p += 1;
        v
    }
    fn uleb(&mut self) -> u64 {
        let (mut r, mut shift) = (0u64, 0);
        loop {
            let b = self.u8();
            r |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                return r;
            }
            shift += 7;
        }
    }
    fn sleb(&mut self) -> i64 {
        let (mut r, mut shift) = (0i64, 0);
        loop {
            let b = self.u8();
            r |= ((b & 0x7f) as i64) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                if shift < 64 && b & 0x40 != 0 {
                    r |= -1i64 << shift;
                }
                return r;
            }
        }
    }
}

fn decode(bytes: &[u8]) -> Module {
    assert_eq!(&bytes[..8], &[0, 0x61, 0x73, 0x6d, 1, 0, 0, 0], "bad header");
    let mut r = R { b: bytes, p: 8 };
    let mut type_params: Vec<usize> = Vec::new();
    let mut func_types: Vec<usize> = Vec::new();
    let mut exports = Vec::new();
    let mut funcs = Vec::new();
    while r.p < bytes.len() {
        let id = r.u8();
        let size = r.uleb() as usize;
        let end = r.p + size;
        match id {
            1 => {
                for _ in 0..r.uleb() {
                    assert_eq!(r.u8(), 0x60);
                    let n = r.uleb() as usize;
                    for _ in 0..n {
                        assert_eq!(r.u8(), 0x7c);
                    }
                    assert_eq!(r.uleb(), 1);
                    assert_eq!(r.u8(), 0x7c);
                    type_params.push(n);
                }
            }
            3 => {
                for _ in 0..r.uleb() {
                    func_types.push(r.uleb() as usize);
                }
            }
            7 => {
                for _ in 0..r.uleb() {
                    let n = r.uleb() as usize;
                    let name = String::from_utf8(bytes[r.p..r.p + n].to_vec()).unwrap();
                    r.p += n;
                    assert_eq!(r.u8(), 0);
                    exports.push((name, r.uleb() as usize));
                }
            }
            10 => {
                let count = r.uleb() as usize;
                assert_eq!(count, func_types.len());
                for k in 0..count {
                    let size = r.uleb() as usize;
                    let body_end = r.p + size;
                    let mut nlocals = 0usize;
                    for _ in 0..r.uleb() {
                        nlocals += r.uleb() as usize;
                        assert_eq!(r.u8(), 0x7c);
                    }
                    let mut ops = Vec::new();
                    let mut open: Vec<usize> = Vec::new();
                    while r.p < body_end {
                        let at = ops.len();
                        let op = r.u8();
                        let decoded = match op {
                            0x02 => {
                                assert_eq!(r.u8(), 0x40);
                                open.push(at);
                                Op::Block(0)
                            }
                            0x03 => {
                                assert_eq!(r.u8(), 0x40);
                                open.push(at);
                                Op::Loop
                            }
                            0x04 => {
                                let t = r.u8();
                                assert!(t == 0x40 || t == 0x7f);
                                open.push(at);
                                Op::If(0, None)
                            }
                            0x05 => {
                                let o = *open.last().unwrap();
                                if let Op::If(_, e) = &mut ops[o] {
                                    *e = Some(at);
                                } else {
                                    panic!("else without if");
                                }
                                Op::Else
                            }
                            0x0b => {
                                if let Some(o) = open.pop() {
                                    match &mut ops[o] {
                                        Op::Block(e) => *e = at,
                                        Op::If(e, _) => *e = at,
                                        Op::Loop => {}
                                        _ => unreachable!(),
                                    }
                                    // loops need their end too: store it in a side table through Block-like semantics
                                }
                                Op::End
                            }
                            0x0c => Op::Br(r.uleb() as u32),
                            0x0d => Op::BrIf(r.uleb() as u32),
                            0x0f => Op::Return,
                            0x10 => Op::Call(r.uleb() as u32),
                            0x1a => Op::Drop,
                            0x20 => Op::Get(r.uleb() as u32),
                            0x21 => Op::Set(r.uleb() as u32),
                            0x41 => Op::I32Const(r.sleb() as i32),
                            0x44 => {
                                let mut a = [0u8; 8];
                                a.copy_from_slice(&bytes[r.p..r.p + 8]);
                                r.p += 8;
                                Op::F64Const(f64::from_le_bytes(a))
                            }
                            0x45 | 0x61..=0x66 | 0x71 | 0x72 | 0x99..=0x9f | 0xa0..=0xa5 | 0xb8 => Op::Simple(op),
                            other => panic!("unsupported opcode 0x{:02x}", other),
                        };
                        ops.push(decoded);
                    }
                    assert!(open.is_empty(), "unbalanced blocks");
                    assert_eq!(r.p, body_end);
                    funcs.push(Func { nparams: type_params[func_types[k]], nlocals, ops });
                }
            }
            _ => panic!("unexpected section {}", id),
        }
        assert_eq!(r.p, end, "section {} size mismatch", id);
    }
    Module { funcs, exports }
}

struct Label {
    end: usize,
    loop_start: Option<usize>,
}

fn run_func(m: &Module, idx: usize, args: &[f64], fuel: &mut u64) -> f64 {
    let f = &m.funcs[idx];
    assert_eq!(args.len(), f.nparams);
    let mut locals: Vec<f64> = args.to_vec();
    locals.extend(std::iter::repeat(0.0).take(f.nlocals));
    // end positions of loops (the compiler never branches past a loop's end, but `end` pops labels uniformly)
    let mut stack: Vec<V> = Vec::new();
    let mut labels: Vec<Label> = Vec::new();
    let mut pc = 0usize;
    let f64v = |v: V| match v {
        V::F(x) => x,
        V::I(_) => panic!("expected f64"),
    };
    let i32v = |v: V| match v {
        V::I(x) => x,
        V::F(_) => panic!("expected i32"),
    };
    // precompute matching end for loops
    let mut loop_end: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    {
        let mut open: Vec<usize> = Vec::new();
        for (i, op) in f.ops.iter().enumerate() {
            match op {
                Op::Block(_) | Op::Loop | Op::If(..) => open.push(i),
                Op::End => {
                    if let Some(o) = open.pop() {
                        loop_end.insert(o, i);
                    }
                }
                _ => {}
            }
        }
    }
    loop {
        *fuel -= 1;
        assert!(*fuel > 0, "out of fuel (endless loop?)");
        if pc >= f.ops.len() {
            break;
        }
        match f.ops[pc].clone() {
            Op::Block(end) => labels.push(Label { end, loop_start: None }),
            Op::Loop => labels.push(Label { end: loop_end[&pc], loop_start: Some(pc + 1) }),
            Op::If(end, els) => {
                let c = i32v(stack.pop().unwrap());
                labels.push(Label { end, loop_start: None });
                if c == 0 {
                    match els {
                        Some(e) => pc = e, // continue after the else
                        None => {
                            labels.pop();
                            pc = end;
                        }
                    }
                }
            }
            Op::Else => {
                // reached the end of the then-branch: jump to the end of the if
                let l = labels.pop().unwrap();
                pc = l.end;
            }
            Op::End => {
                if labels.pop().is_none() {
                    break; // the function's own end
                }
            }
            Op::Br(n) | Op::BrIf(n) => {
                let take = match f.ops[pc] {
                    Op::BrIf(_) => i32v(stack.pop().unwrap()) != 0,
                    _ => true,
                };
                if take {
                    let at = labels.len() - 1 - n as usize;
                    match labels[at].loop_start {
                        Some(start) => {
                            labels.truncate(at + 1);
                            pc = start;
                            continue;
                        }
                        None => {
                            let end = labels[at].end;
                            labels.truncate(at);
                            pc = end;
                        }
                    }
                }
            }
            Op::Return => break,
            Op::Call(i) => {
                let n = m.funcs[i as usize].nparams;
                let at = stack.len() - n;
                let a: Vec<f64> = stack.drain(at..).map(f64v).collect();
                let r = run_func(m, i as usize, &a, fuel);
                stack.push(V::F(r));
            }
            Op::Drop => {
                stack.pop();
            }
            Op::Get(i) => stack.push(V::F(locals[i as usize])),
            Op::Set(i) => locals[i as usize] = f64v(stack.pop().unwrap()),
            Op::I32Const(v) => stack.push(V::I(v)),
            Op::F64Const(v) => stack.push(V::F(v)),
            Op::Simple(op) => match op {
                0x45 => {
                    let a = i32v(stack.pop().unwrap());
                    stack.push(V::I((a == 0) as i32));
                }
                0x71 | 0x72 => {
                    let b = i32v(stack.pop().unwrap());
                    let a = i32v(stack.pop().unwrap());
                    stack.push(V::I(if op == 0x71 { a & b } else { a | b }));
                }
                0x61..=0x66 => {
                    let b = f64v(stack.pop().unwrap());
                    let a = f64v(stack.pop().unwrap());
                    stack.push(V::I(match op {
                        0x61 => a == b,
                        0x62 => a != b,
                        0x63 => a < b,
                        0x64 => a > b,
                        0x65 => a <= b,
                        _ => a >= b,
                    } as i32));
                }
                0xb8 => {
                    let a = i32v(stack.pop().unwrap());
                    stack.push(V::F(a as u32 as f64));
                }
                0x99..=0x9f => {
                    let a = f64v(stack.pop().unwrap());
                    stack.push(V::F(match op {
                        0x99 => a.abs(),
                        0x9a => -a,
                        0x9b => a.ceil(),
                        0x9c => a.floor(),
                        0x9d => a.trunc(),
                        0x9e => a.round(),
                        _ => a.sqrt(),
                    }));
                }
                _ => {
                    let b = f64v(stack.pop().unwrap());
                    let a = f64v(stack.pop().unwrap());
                    stack.push(V::F(match op {
                        0xa0 => a + b,
                        0xa1 => a - b,
                        0xa2 => a * b,
                        0xa3 => a / b,
                        0xa4 => a.min(b),
                        _ => a.max(b),
                    }));
                }
            },
        }
        pc += 1;
    }
    f64v(stack.pop().expect("function left no result"))
}

fn wasm_call(m: &Module, name: &str, args: &[f64]) -> f64 {
    let idx = m.exports.iter().find(|(n, _)| n == name).unwrap_or_else(|| panic!("no export {name}")).1;
    let mut fuel = 50_000_000u64;
    run_func(m, idx, args, &mut fuel)
}

// ---------------------------------------------------------------------------------------------

fn vm_values(src: &str, calls: &[(&str, Vec<f64>)]) -> Vec<f64> {
    let mut script = src.to_string();
    script.push('\n');
    for (f, a) in calls {
        let args: Vec<String> = a.iter().map(|x| format!("{:?}", x)).collect();
        script.push_str(&format!("println({}({}))\n", f, args.join(", ")));
    }
    let dir = std::env::temp_dir().join(format!("killer_wasm_{}_{}", std::process::id(), calls.len()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("t{}.killer", std::process::id()));
    std::fs::write(&file, script).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success(), "VM failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| match l.trim() {
            "true" => 1.0,
            "false" => 0.0,
            "null" => 0.0,
            t => t.parse::<f64>().unwrap_or_else(|_| panic!("not a number: {t}")),
        })
        .collect()
}

fn check(src: &str, calls: Vec<(&str, Vec<f64>)>) {
    let (bytes, _) = compile(src).unwrap_or_else(|e| panic!("compile failed: {e}\n{src}"));
    let m = decode(&bytes);
    let want = vm_values(src, &calls);
    assert_eq!(want.len(), calls.len());
    for ((f, a), w) in calls.iter().zip(want) {
        let got = wasm_call(&m, f, a);
        let same = (got - w).abs() <= 1e-9 * w.abs().max(1.0) || (got.is_nan() && w.is_nan());
        assert!(same, "{f}({a:?}): wasm gave {got}, the VM gave {w}\n{src}");
    }
}

#[test]
fn arithmetic_comparisons_and_modulo_match_the_vm() {
    check(
        "fn mix(a, b) {\n  return (a + b) * 2 - a / 4 + a % 3\n}\nfn cmp(a, b) {\n  if a < b { return 1 }\n  else if a == b { return 2 }\n  return 3\n}\nfn neg(x) { return -x + abs(x) }\nfn mod(a, b) { return a % b }\n",
        vec![
            ("mix", vec![7.0, 5.0]),
            ("mix", vec![-7.5, 2.0]),
            ("cmp", vec![1.0, 2.0]),
            ("cmp", vec![2.0, 2.0]),
            ("cmp", vec![3.0, 2.0]),
            ("neg", vec![-4.0]),
            ("mod", vec![-7.0, 3.0]),
            ("mod", vec![7.0, -3.0]),
            ("mod", vec![7.5, 2.0]),
            ("mod", vec![-7.5, 2.0]),
        ],
    );
}

#[test]
fn recursion_and_loops_match_the_vm() {
    check(
        "fn fib(n) {\n  if n < 2 { return n }\n  return fib(n - 1) + fib(n - 2)\n}\nfn fact(n) {\n  r = 1\n  for i in range(1, n + 1) { r = r * i }\n  return r\n}\nfn gcd(a, b) {\n  while b != 0 {\n    t = a % b\n    a = b\n    b = t\n  }\n  return a\n}\nfn collatz(n) {\n  steps = 0\n  while n != 1 {\n    if n % 2 == 0 { n = n / 2 } else { n = 3 * n + 1 }\n    steps++\n  }\n  return steps\n}\n",
        vec![
            ("fib", vec![0.0]),
            ("fib", vec![10.0]),
            ("fib", vec![15.0]),
            ("fact", vec![0.0]),
            ("fact", vec![10.0]),
            ("gcd", vec![48.0, 18.0]),
            ("gcd", vec![17.0, 5.0]),
            ("collatz", vec![27.0]),
            ("collatz", vec![1.0]),
        ],
    );
}

#[test]
fn break_continue_logic_and_math_functions_match_the_vm() {
    check(
        "const LIMIT = 50\nfn is_prime(n) {\n  if n < 2 { return false }\n  for i in range(2, n) {\n    if i * i > n { break }\n    if n % i == 0 { return false }\n  }\n  return true\n}\nfn count_primes(n) {\n  c = 0\n  for k in range(n) {\n    if !is_prime(k) { continue }\n    c += 1\n  }\n  return c\n}\nfn guard(n) {\n  if n > 0 && guard(n - 1) >= 0 { return n }\n  return 0\n}\nfn either(a, b) { return a > 5 || b > 5 }\nfn hyp(a, b) { return sqrt(a * a + b * b) }\nfn clamp(x, lo, hi) { return max(lo, min(x, hi)) }\nfn rounds(x) { return floor(x) + ceil(x) + trunc(x) }\nfn lim() { return LIMIT * 2 }\nfn sumsq(n) {\n  s = 0\n  i = 0\n  while i < n {\n    i += 1\n    if i % 2 == 1 { continue }\n    s += i * i\n  }\n  return s\n}\n",
        vec![
            ("is_prime", vec![1.0]),
            ("is_prime", vec![2.0]),
            ("is_prime", vec![97.0]),
            ("is_prime", vec![91.0]),
            ("count_primes", vec![LIMIT_N]),
            ("guard", vec![3.0]),
            ("either", vec![1.0, 9.0]),
            ("either", vec![1.0, 2.0]),
            ("hyp", vec![3.0, 4.0]),
            ("clamp", vec![15.0, 0.0, 10.0]),
            ("clamp", vec![-5.0, 0.0, 10.0]),
            ("rounds", vec![2.5]),
            ("rounds", vec![-2.5]),
            ("lim", vec![]),
            ("sumsq", vec![10.0]),
        ],
    );
}

const LIMIT_N: f64 = 50.0;

#[test]
fn the_cli_writes_a_module_and_lists_the_exports() {
    let dir = std::env::temp_dir().join(format!("killer_wasm_cli_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("m.killer");
    let out = dir.join("m.wasm");
    std::fs::write(&src, "fn sq(x) { return x * x }\n").unwrap();
    let r = Command::new(env!("CARGO_BIN_EXE_killer")).arg("wasm").arg(&src).arg("-o").arg(&out).output().unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    assert!(String::from_utf8_lossy(&r.stdout).contains("export sq(1 number)"));
    let bytes = std::fs::read(&out).unwrap();
    assert_eq!(wasm_call(&decode(&bytes), "sq", &[7.0]), 49.0);
    // an unsupported script is an error that names the line
    std::fs::write(&src, "fn ok(x) { return x }\nfn bad(x) { return \"s\" }\n").unwrap();
    let r = Command::new(env!("CARGO_BIN_EXE_killer")).arg("wasm").arg(&src).arg("-o").arg(&out).output().unwrap();
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("Line 2"), "{}", String::from_utf8_lossy(&r.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}
