//! `killer wasm`: every compiled function is executed by a small WebAssembly interpreter written here from the
//! specification (no wasm runtime is needed) and its results are compared with the Killer VM running the same source.
//! The interpreter has typed values (i32 / f64), linear memory with loads and stores, a global, `memory.grow`,
//! `select`, traps and data segments, so the string and list support (a bump allocator in linear memory) is tested too.
use killer_native::wasm_compile::{compile, compile_full};
use std::process::Command;

// ---------------------------------------------------------------------------------------------
// a minimal WebAssembly decoder + interpreter

#[derive(Clone, Copy, Debug, PartialEq)]
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
    Select,
    Get(u32),
    Set(u32),
    GGet(u32),
    GSet(u32),
    I32Const(i32),
    F64Const(f64),
    Load(u8, u32),
    Store(u8, u32),
    MemSize,
    MemGrow,
    Unreachable,
    Simple(u8),
}

struct Func {
    params: Vec<u8>,
    result: Option<u8>,
    locals: Vec<u8>,
    ops: Vec<Op>,
}

struct Module {
    funcs: Vec<Func>,
    exports: Vec<(String, u8, usize)>, // name, kind, index
    mem_pages: Option<u32>,
    globals: Vec<i32>,
    data: Vec<(u32, Vec<u8>)>,
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

fn valtype(b: u8) -> u8 {
    assert!(b == 0x7c || b == 0x7f, "unsupported value type 0x{:02x}", b);
    b
}

fn decode(bytes: &[u8]) -> Module {
    assert_eq!(&bytes[..8], &[0, 0x61, 0x73, 0x6d, 1, 0, 0, 0], "bad header");
    let mut r = R { b: bytes, p: 8 };
    let mut types: Vec<(Vec<u8>, Option<u8>)> = Vec::new();
    let mut func_types: Vec<usize> = Vec::new();
    let mut m = Module { funcs: Vec::new(), exports: Vec::new(), mem_pages: None, globals: Vec::new(), data: Vec::new() };
    let mut last_id = 0u8;
    while r.p < bytes.len() {
        let id = r.u8();
        assert!(id > last_id, "sections out of order: {} after {}", id, last_id);
        last_id = id;
        let size = r.uleb() as usize;
        let end = r.p + size;
        match id {
            1 => {
                for _ in 0..r.uleb() {
                    assert_eq!(r.u8(), 0x60);
                    let n = r.uleb() as usize;
                    let params: Vec<u8> = (0..n).map(|_| valtype(r.u8())).collect();
                    let nres = r.uleb();
                    assert!(nres <= 1);
                    let result = if nres == 1 { Some(valtype(r.u8())) } else { None };
                    types.push((params, result));
                }
            }
            3 => {
                for _ in 0..r.uleb() {
                    func_types.push(r.uleb() as usize);
                }
            }
            5 => {
                assert_eq!(r.uleb(), 1);
                assert_eq!(r.u8(), 0, "only a minimum size is expected");
                m.mem_pages = Some(r.uleb() as u32);
            }
            6 => {
                for _ in 0..r.uleb() {
                    assert_eq!(r.u8(), 0x7f);
                    assert_eq!(r.u8(), 0x01, "the heap pointer is mutable");
                    assert_eq!(r.u8(), 0x41);
                    m.globals.push(r.sleb() as i32);
                    assert_eq!(r.u8(), 0x0b);
                }
            }
            7 => {
                for _ in 0..r.uleb() {
                    let n = r.uleb() as usize;
                    let name = String::from_utf8(bytes[r.p..r.p + n].to_vec()).unwrap();
                    r.p += n;
                    let kind = r.u8();
                    m.exports.push((name, kind, r.uleb() as usize));
                }
            }
            10 => {
                let count = r.uleb() as usize;
                assert_eq!(count, func_types.len());
                for k in 0..count {
                    let size = r.uleb() as usize;
                    let body_end = r.p + size;
                    let mut locals = Vec::new();
                    for _ in 0..r.uleb() {
                        let n = r.uleb() as usize;
                        let t = valtype(r.u8());
                        locals.extend(std::iter::repeat(t).take(n));
                    }
                    let mut ops = Vec::new();
                    let mut open: Vec<usize> = Vec::new();
                    while r.p < body_end {
                        let at = ops.len();
                        let op = r.u8();
                        let decoded = match op {
                            0x00 => Op::Unreachable,
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
                                }
                                Op::End
                            }
                            0x0c => Op::Br(r.uleb() as u32),
                            0x0d => Op::BrIf(r.uleb() as u32),
                            0x0f => Op::Return,
                            0x10 => Op::Call(r.uleb() as u32),
                            0x1a => Op::Drop,
                            0x1b => Op::Select,
                            0x20 => Op::Get(r.uleb() as u32),
                            0x21 => Op::Set(r.uleb() as u32),
                            0x23 => Op::GGet(r.uleb() as u32),
                            0x24 => Op::GSet(r.uleb() as u32),
                            0x28 | 0x2b | 0x2d => {
                                let _align = r.uleb();
                                Op::Load(op, r.uleb() as u32)
                            }
                            0x36 | 0x39 | 0x3a => {
                                let _align = r.uleb();
                                Op::Store(op, r.uleb() as u32)
                            }
                            0x3f => {
                                assert_eq!(r.u8(), 0);
                                Op::MemSize
                            }
                            0x40 => {
                                assert_eq!(r.u8(), 0);
                                Op::MemGrow
                            }
                            0x41 => Op::I32Const(r.sleb() as i32),
                            0x44 => {
                                let mut a = [0u8; 8];
                                a.copy_from_slice(&bytes[r.p..r.p + 8]);
                                r.p += 8;
                                Op::F64Const(f64::from_le_bytes(a))
                            }
                            0x45..=0x4f | 0x61..=0x66 | 0x6a | 0x6b | 0x6c | 0x71 | 0x72 | 0x74..=0x76 | 0x99..=0x9f | 0xa0..=0xa5 | 0xaa | 0xb7 | 0xb8 => Op::Simple(op),
                            other => panic!("unsupported opcode 0x{:02x}", other),
                        };
                        ops.push(decoded);
                    }
                    assert!(open.is_empty(), "unbalanced blocks");
                    assert_eq!(r.p, body_end);
                    let (params, result) = types[func_types[k]].clone();
                    m.funcs.push(Func { params, result, locals, ops });
                }
            }
            11 => {
                for _ in 0..r.uleb() {
                    assert_eq!(r.uleb(), 0, "active segment of memory 0");
                    assert_eq!(r.u8(), 0x41);
                    let off = r.sleb() as u32;
                    assert_eq!(r.u8(), 0x0b);
                    let n = r.uleb() as usize;
                    m.data.push((off, bytes[r.p..r.p + n].to_vec()));
                    r.p += n;
                }
            }
            _ => panic!("unexpected section {}", id),
        }
        assert_eq!(r.p, end, "section {} size mismatch", id);
    }
    m
}

struct Label {
    end: usize,
    loop_start: Option<usize>,
}

/// An instance: the module plus its memory and globals.
struct Inst<'m> {
    m: &'m Module,
    mem: Vec<u8>,
    globals: Vec<i32>,
    fuel: u64,
}

type Trap = String;

impl<'m> Inst<'m> {
    fn new(m: &'m Module) -> Inst<'m> {
        let mut mem = vec![0u8; m.mem_pages.unwrap_or(0) as usize * 65536];
        for (off, bytes) in &m.data {
            mem[*off as usize..*off as usize + bytes.len()].copy_from_slice(bytes);
        }
        Inst { m, mem, globals: m.globals.clone(), fuel: 200_000_000 }
    }

    fn export(&self, name: &str) -> usize {
        self.m.exports.iter().find(|(n, k, _)| n == name && *k == 0).unwrap_or_else(|| panic!("no export {name}")).2
    }

    fn call(&mut self, name: &str, args: &[V]) -> Result<Option<V>, Trap> {
        let idx = self.export(name);
        self.run(idx, args)
    }

    fn addr(&self, base: i32, off: u32, size: usize) -> Result<usize, Trap> {
        let a = (base as u32 as u64) + off as u64;
        if a + size as u64 > self.mem.len() as u64 {
            return Err(format!("trap: out of bounds memory access at {}", a));
        }
        Ok(a as usize)
    }

    fn run(&mut self, idx: usize, args: &[V]) -> Result<Option<V>, Trap> {
        let m = self.m;
        let f = &m.funcs[idx];
        assert_eq!(args.len(), f.params.len());
        for (a, t) in args.iter().zip(&f.params) {
            assert!(matches!((a, t), (V::F(_), 0x7c) | (V::I(_), 0x7f)), "argument type mismatch calling function {idx}");
        }
        let mut locals: Vec<V> = args.to_vec();
        locals.extend(f.locals.iter().map(|t| if *t == 0x7c { V::F(0.0) } else { V::I(0) }));
        let mut stack: Vec<V> = Vec::new();
        let mut labels: Vec<Label> = Vec::new();
        let mut pc = 0usize;
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
        macro_rules! pf {
            () => {
                match stack.pop().unwrap() {
                    V::F(x) => x,
                    V::I(_) => panic!("expected f64"),
                }
            };
        }
        macro_rules! pi {
            () => {
                match stack.pop().unwrap() {
                    V::I(x) => x,
                    V::F(_) => panic!("expected i32"),
                }
            };
        }
        loop {
            self.fuel -= 1;
            assert!(self.fuel > 0, "out of fuel (endless loop?)");
            if pc >= f.ops.len() {
                break;
            }
            match f.ops[pc].clone() {
                Op::Unreachable => return Err("trap: unreachable".to_string()),
                Op::Block(end) => labels.push(Label { end, loop_start: None }),
                Op::Loop => labels.push(Label { end: loop_end[&pc], loop_start: Some(pc + 1) }),
                Op::If(end, els) => {
                    let c = pi!();
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
                        Op::BrIf(_) => pi!() != 0,
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
                    let n = m.funcs[i as usize].params.len();
                    let at = stack.len() - n;
                    let a: Vec<V> = stack.drain(at..).collect();
                    if let Some(r) = self.run(i as usize, &a)? {
                        stack.push(r);
                    }
                }
                Op::Drop => {
                    stack.pop();
                }
                Op::Select => {
                    let c = pi!();
                    let b = stack.pop().unwrap();
                    let a = stack.pop().unwrap();
                    stack.push(if c != 0 { a } else { b });
                }
                Op::Get(i) => stack.push(locals[i as usize]),
                Op::Set(i) => {
                    let v = stack.pop().unwrap();
                    assert!(std::mem::discriminant(&v) == std::mem::discriminant(&locals[i as usize]), "local {i} gets the wrong type");
                    locals[i as usize] = v;
                }
                Op::GGet(i) => stack.push(V::I(self.globals[i as usize])),
                Op::GSet(i) => self.globals[i as usize] = pi!(),
                Op::I32Const(v) => stack.push(V::I(v)),
                Op::F64Const(v) => stack.push(V::F(v)),
                Op::Load(op, off) => {
                    let base = pi!();
                    match op {
                        0x28 => {
                            let a = self.addr(base, off, 4)?;
                            stack.push(V::I(i32::from_le_bytes(self.mem[a..a + 4].try_into().unwrap())));
                        }
                        0x2b => {
                            let a = self.addr(base, off, 8)?;
                            stack.push(V::F(f64::from_le_bytes(self.mem[a..a + 8].try_into().unwrap())));
                        }
                        _ => {
                            let a = self.addr(base, off, 1)?;
                            stack.push(V::I(self.mem[a] as i32));
                        }
                    }
                }
                Op::Store(op, off) => match op {
                    0x36 => {
                        let v = pi!();
                        let base = pi!();
                        let a = self.addr(base, off, 4)?;
                        self.mem[a..a + 4].copy_from_slice(&v.to_le_bytes());
                    }
                    0x39 => {
                        let v = pf!();
                        let base = pi!();
                        let a = self.addr(base, off, 8)?;
                        self.mem[a..a + 8].copy_from_slice(&v.to_le_bytes());
                    }
                    _ => {
                        let v = pi!();
                        let base = pi!();
                        let a = self.addr(base, off, 1)?;
                        self.mem[a] = v as u8;
                    }
                },
                Op::MemSize => stack.push(V::I((self.mem.len() / 65536) as i32)),
                Op::MemGrow => {
                    let n = pi!() as u32 as usize;
                    let old = self.mem.len() / 65536;
                    if old + n > 16384 {
                        stack.push(V::I(-1));
                    } else {
                        self.mem.resize((old + n) * 65536, 0);
                        stack.push(V::I(old as i32));
                    }
                }
                Op::Simple(op) => match op {
                    0x45 => {
                        let a = pi!();
                        stack.push(V::I((a == 0) as i32));
                    }
                    0x46..=0x4f => {
                        let b = pi!();
                        let a = pi!();
                        let (ua, ub) = (a as u32, b as u32);
                        stack.push(V::I(match op {
                            0x46 => a == b,
                            0x47 => a != b,
                            0x48 => a < b,
                            0x49 => ua < ub,
                            0x4a => a > b,
                            0x4b => ua > ub,
                            0x4c => a <= b,
                            0x4d => ua <= ub,
                            0x4e => a >= b,
                            _ => ua >= ub,
                        } as i32));
                    }
                    0x6a..=0x6c | 0x71 | 0x72 | 0x74..=0x76 => {
                        let b = pi!();
                        let a = pi!();
                        stack.push(V::I(match op {
                            0x6a => a.wrapping_add(b),
                            0x6b => a.wrapping_sub(b),
                            0x6c => a.wrapping_mul(b),
                            0x71 => a & b,
                            0x72 => a | b,
                            0x74 => a.wrapping_shl(b as u32 & 31),
                            0x75 => a.wrapping_shr(b as u32 & 31),
                            _ => ((a as u32).wrapping_shr(b as u32 & 31)) as i32,
                        }));
                    }
                    0x61..=0x66 => {
                        let b = pf!();
                        let a = pf!();
                        stack.push(V::I(match op {
                            0x61 => a == b,
                            0x62 => a != b,
                            0x63 => a < b,
                            0x64 => a > b,
                            0x65 => a <= b,
                            _ => a >= b,
                        } as i32));
                    }
                    0xaa => {
                        let a = pf!();
                        if a.is_nan() || a <= -2147483649.0 || a >= 2147483648.0 {
                            return Err("trap: invalid conversion to integer".to_string());
                        }
                        stack.push(V::I(a as i32));
                    }
                    0xb7 => {
                        let a = pi!();
                        stack.push(V::F(a as f64));
                    }
                    0xb8 => {
                        let a = pi!();
                        stack.push(V::F(a as u32 as f64));
                    }
                    0x99..=0x9f => {
                        let a = pf!();
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
                        let b = pf!();
                        let a = pf!();
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
        match f.result {
            Some(t) => {
                let v = stack.pop().expect("function left no result");
                assert!(matches!((v, t), (V::F(_), 0x7c) | (V::I(_), 0x7f)), "function result has the wrong type");
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    // ---- the host side of the documented API: strings and lists are built and read through the exported helpers
    fn new_str(&mut self, s: &str) -> i32 {
        let p = match self.call("killer_str_new", &[V::I(s.len() as i32)]).unwrap() {
            Some(V::I(p)) => p,
            other => panic!("killer_str_new gave {other:?}"),
        };
        self.mem[p as usize + 4..p as usize + 4 + s.len()].copy_from_slice(s.as_bytes());
        p
    }
    fn read_str(&self, p: i32) -> String {
        let p = p as usize;
        let n = u32::from_le_bytes(self.mem[p..p + 4].try_into().unwrap()) as usize;
        String::from_utf8(self.mem[p + 4..p + 4 + n].to_vec()).unwrap()
    }
    fn new_list(&mut self, xs: &[f64]) -> i32 {
        let p = match self.call("killer_list_new", &[]).unwrap() {
            Some(V::I(p)) => p,
            other => panic!("killer_list_new gave {other:?}"),
        };
        for x in xs {
            self.call("killer_list_push", &[V::I(p), V::F(*x)]).unwrap();
        }
        p
    }
    fn read_list(&mut self, p: i32) -> Vec<f64> {
        let n = match self.call("killer_list_len", &[V::I(p)]).unwrap() {
            Some(V::I(n)) => n,
            other => panic!("killer_list_len gave {other:?}"),
        };
        (0..n)
            .map(|i| match self.call("killer_list_get", &[V::I(p), V::F(i as f64)]).unwrap() {
                Some(V::F(x)) => x,
                other => panic!("killer_list_get gave {other:?}"),
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------------------------

/// An argument or result as the host sees it.
#[derive(Clone, Debug, PartialEq)]
enum Val {
    N(f64),
    S(String),
    L(Vec<f64>),
}

fn n(x: f64) -> Val {
    Val::N(x)
}
fn s(x: &str) -> Val {
    Val::S(x.to_string())
}
fn l(x: &[f64]) -> Val {
    Val::L(x.to_vec())
}

fn killer_literal(v: &Val) -> String {
    match v {
        Val::N(x) => format!("{:?}", x),
        Val::S(t) => format!("{:?}", t),
        Val::L(xs) => format!("[{}]", xs.iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>().join(", ")),
    }
}

/// The raw output lines of the Killer VM for `println(f(args))` of each call.
fn vm_lines(src: &str, calls: &[(&str, Vec<Val>)]) -> Vec<String> {
    let mut script = src.to_string();
    script.push('\n');
    for (f, a) in calls {
        let args: Vec<String> = a.iter().map(killer_literal).collect();
        script.push_str(&format!("println({}({}))\n", f, args.join(", ")));
    }
    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("killer_wasm_{}_{}", std::process::id(), COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("t{}.killer", std::process::id()));
    std::fs::write(&file, script).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_killer")).arg("run").arg(&file).output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert!(out.status.success(), "VM failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).lines().map(|l| l.to_string()).collect()
}

fn parse_num(t: &str) -> f64 {
    match t.trim() {
        "true" => 1.0,
        "false" => 0.0,
        "null" => 0.0,
        t => t.parse::<f64>().unwrap_or_else(|_| panic!("not a number: {t}")),
    }
}

fn parse_list(t: &str) -> Vec<f64> {
    let t = t.trim().trim_start_matches('[').trim_end_matches(']');
    if t.trim().is_empty() {
        return Vec::new();
    }
    t.split(',').map(parse_num).collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * b.abs().max(1.0) || (a.is_nan() && b.is_nan())
}

/// Calls `name` in the wasm instance with host values, returning the host view of the result.
fn wasm_call_val(inst: &mut Inst, name: &str, args: &[Val], ret: &str) -> Result<Val, Trap> {
    let a: Vec<V> = args
        .iter()
        .map(|v| match v {
            Val::N(x) => V::F(*x),
            Val::S(t) => V::I(inst.new_str(t)),
            Val::L(xs) => V::I(inst.new_list(xs)),
        })
        .collect();
    let r = inst.call(name, &a)?.expect("an exported function returns a value");
    Ok(match (ret, r) {
        ("number", V::F(x)) => Val::N(x),
        ("string", V::I(p)) => Val::S(inst.read_str(p)),
        ("list", V::I(p)) => Val::L(inst.read_list(p)),
        (t, v) => panic!("{name}: result {v:?} does not match the type {t}"),
    })
}

/// Compiles `src`, runs every call both in the interpreter and in the Killer VM and compares the results.
fn check_vals(src: &str, calls: Vec<(&str, Vec<Val>)>) {
    let (bytes, exports) = compile_full(src).unwrap_or_else(|e| panic!("compile failed: {e}\n{src}"));
    let m = decode(&bytes);
    let want = vm_lines(src, &calls);
    assert_eq!(want.len(), calls.len(), "VM output: {want:?}");
    for ((f, a), w) in calls.iter().zip(want) {
        let ret = exports.iter().find(|e| e.name == *f).unwrap().ret;
        let mut inst = Inst::new(&m);
        let got = wasm_call_val(&mut inst, f, a, ret).unwrap_or_else(|t| panic!("{f}({a:?}) trapped: {t}\n{src}"));
        let same = match &got {
            Val::N(g) => close(*g, parse_num(&w)),
            Val::S(g) => *g == w,
            Val::L(g) => {
                let wl = parse_list(&w);
                g.len() == wl.len() && g.iter().zip(&wl).all(|(x, y)| close(*x, *y))
            }
        };
        assert!(same, "{f}({a:?}): wasm gave {got:?}, the VM printed {w:?}\n{src}");
    }
}

/// The numbers-only form used by the original tests.
fn check(src: &str, calls: Vec<(&str, Vec<f64>)>) {
    check_vals(src, calls.into_iter().map(|(f, a)| (f, a.into_iter().map(Val::N).collect())).collect());
}

fn wasm_call(m: &Module, name: &str, args: &[f64]) -> f64 {
    let mut inst = Inst::new(m);
    let a: Vec<V> = args.iter().map(|x| V::F(*x)).collect();
    match inst.call(name, &a).unwrap() {
        Some(V::F(x)) => x,
        other => panic!("{name} gave {other:?}"),
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
    std::fs::write(&src, "fn ok(x) { return x }\nfn bad(x) { return {1: 2} }\n").unwrap();
    let r = Command::new(env!("CARGO_BIN_EXE_killer")).arg("wasm").arg(&src).arg("-o").arg(&out).output().unwrap();
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stderr).contains("Line 2"), "{}", String::from_utf8_lossy(&r.stderr));
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------------------------
// strings, lists and the host API

#[test]
fn strings_match_the_vm() {
    check_vals(
        r#"
fn greet(name) {
  return "Hello, " + name + "!"
}
fn size(s: string) { return len(s) }
fn same(a: string, b: string) { return a == b }
fn differ(a: string, b: string) { return a != b }
fn first(s: string) { return s[0] }
fn last(s: string) { return s[-1] }
fn at(s: string, i) { return s[i] }
fn shout(s: string) {
  out = ""
  for c in s {
    out = out + c + c
  }
  return out
}
fn rev(s: string) {
  out = ""
  i = len(s) - 1
  while i >= 0 {
    out += s[i]
    i = i - 1
  }
  return out
}
fn count_a(s: string) {
  n = 0
  for c in s {
    if c == "a" {
      n += 1
    }
  }
  return n
}
fn lit() { return 'it works' }
fn empty() { return "" }
fn is_pal(s) { return s == rev(s) }
fn repeat(s, n) {
  out = ""
  for i in range(n) { out = out + s }
  return out
}
"#,
        vec![
            ("greet", vec![s("Ada")]),
            ("greet", vec![s("")]),
            ("size", vec![s("hello")]),
            ("size", vec![s("")]),
            ("same", vec![s("abc"), s("abc")]),
            ("same", vec![s("abc"), s("abd")]),
            ("same", vec![s("abc"), s("ab")]),
            ("differ", vec![s("abc"), s("ab")]),
            ("first", vec![s("xyz")]),
            ("last", vec![s("xyz")]),
            ("at", vec![s("xyz"), n(1.0)]),
            ("shout", vec![s("abc")]),
            ("rev", vec![s("stressed")]),
            ("count_a", vec![s("banana")]),
            ("lit", vec![]),
            ("empty", vec![]),
            ("is_pal", vec![s("racecar")]),
            ("is_pal", vec![s("racecars")]),
            ("repeat", vec![s("ab"), n(3.0)]),
        ],
    );
}

#[test]
fn lists_match_the_vm() {
    check_vals(
        r#"
fn total(xs: array) {
  t = 0
  for x in xs {
    t += x
  }
  return t
}
fn count(xs: array) { return len(xs) }
fn nth(xs: array, i) { return xs[i] }
fn last(xs: array) { return xs[-1] }
fn range_list(n) {
  out = []
  for i in range(n) { push(out, i * i) }
  return out
}
fn lit() { return [1, 2.5, -3, 4 * 5] }
fn doubled(xs: array) {
  out = []
  for x in xs { push(out, x * 2) }
  return out
}
fn bump(xs) {
  for i in range(len(xs)) {
    xs[i] += 1
  }
  return xs
}
fn set_first(xs, v) {
  xs[0] = v
  return xs
}
fn maxof(xs: array) {
  m = xs[0]
  for x in xs { if x > m { m = x } }
  return m
}
fn sorted(xs) {
  n = len(xs)
  for i in range(n) {
    for j in range(n - 1 - i) {
      if xs[j] > xs[j + 1] {
        t = xs[j]
        xs[j] = xs[j + 1]
        xs[j + 1] = t
      }
    }
  }
  return xs
}
fn sieve(n) {
  flags = []
  for i in range(n + 1) { push(flags, 1) }
  flags[0] = 0
  if n >= 1 { flags[1] = 0 }
  for i in range(2, n + 1) {
    if flags[i] == 1 {
      j = i * i
      while j <= n {
        flags[j] = 0
        j += i
      }
    }
  }
  primes = []
  for i in range(n + 1) {
    if flags[i] == 1 { push(primes, i) }
  }
  return primes
}
fn many(n) {
  xs = []
  for i in range(n) { push(xs, i) }
  return len(xs) + xs[n - 1]
}
fn empty() { return [] }
fn dot(a, b) {
  t = 0
  for i in range(len(a)) {
    t += a[i] * b[i]
  }
  return t
}
"#,
        vec![
            ("total", vec![l(&[1.0, 2.0, 3.5])]),
            ("total", vec![l(&[])]),
            ("count", vec![l(&[5.0, 6.0])]),
            ("nth", vec![l(&[5.0, 6.0, 7.0]), n(2.0)]),
            ("last", vec![l(&[5.0, 6.0, 7.0])]),
            ("range_list", vec![n(6.0)]),
            ("lit", vec![]),
            ("doubled", vec![l(&[1.0, 2.0, 3.0])]),
            ("bump", vec![l(&[1.0, 2.0, 3.0])]),
            ("set_first", vec![l(&[1.0, 2.0]), n(9.0)]),
            ("maxof", vec![l(&[3.0, 9.0, 2.0])]),
            ("sorted", vec![l(&[5.0, 1.0, 4.0, 2.0, 3.0, 0.5])]),
            ("sieve", vec![n(50.0)]),
            ("many", vec![n(1000.0)]),
            ("empty", vec![]),
            ("dot", vec![l(&[1.0, 2.0, 3.0]), l(&[4.0, 5.0, 6.0])]),
        ],
    );
}

#[test]
fn annotations_and_mixed_programs_match_the_vm() {
    check_vals(
        r#"
fn describe(name: string, scores: array) -> string {
  total = 0
  for s in scores {
    total += s
  }
  return name + " scored " + "ok"
}
fn avg(xs: array) -> number {
  t = 0
  for x in xs {
    t += x
  }
  if len(xs) == 0 { return 0 }
  return t / len(xs)
}
fn join_len(a: string, b: string) { return len(a + b) }
fn words_total(n) {
  w = []
  for i in range(n) { push(w, len("abc") + i) }
  return w
}
fn pad(s, n) {
  while len(s) < n { s = s + "." }
  return s
}
fn caller() { return join_len("ab", "cde") + avg([2, 4, 6]) }
fn cmp_lit(s) {
  if s == "yes" { return 1 }
  return 0
}
"#,
        vec![
            ("describe", vec![s("Bo"), l(&[1.0, 2.0])]),
            ("avg", vec![l(&[2.0, 4.0, 9.0])]),
            ("avg", vec![l(&[])]),
            ("join_len", vec![s("ab"), s("cde")]),
            ("words_total", vec![n(4.0)]),
            ("pad", vec![s("ab"), n(6.0)]),
            ("pad", vec![s("abcdef"), n(3.0)]),
            ("caller", vec![]),
            ("cmp_lit", vec![s("yes")]),
            ("cmp_lit", vec![s("no")]),
        ],
    );
}

#[test]
fn the_host_api_reads_back_strings_and_lists() {
    let (bytes, ex) = compile_full(
        r#"fn up(s: string) { return s + "!" }
fn squares(n) {
  out = []
  for i in range(n) { push(out, i * i) }
  return out
}
"#,
    )
    .unwrap();
    assert_eq!(ex[0].params, vec![("s".to_string(), "string")]);
    assert_eq!(ex[0].ret, "string");
    assert_eq!(ex[1].ret, "list");
    let m = decode(&bytes);
    assert_eq!(m.mem_pages, Some(1));
    assert!(m.exports.iter().any(|(n, k, _)| n == "memory" && *k == 2));
    let mut inst = Inst::new(&m);
    let p = inst.new_str("hey");
    let r = match inst.call("up", &[V::I(p)]).unwrap() {
        Some(V::I(r)) => r,
        other => panic!("{other:?}"),
    };
    assert_eq!(inst.read_str(r), "hey!");
    // the memory is shared: the input string is untouched
    assert_eq!(inst.read_str(p), "hey");
    let r = match inst.call("squares", &[V::F(5.0)]).unwrap() {
        Some(V::I(r)) => r,
        other => panic!("{other:?}"),
    };
    assert_eq!(inst.read_list(r), vec![0.0, 1.0, 4.0, 9.0, 16.0]);
}

#[test]
fn memory_grows_for_big_allocations() {
    let (bytes, _) = compile_full(
        r#"fn big(n) {
  xs = []
  for i in range(n) { push(xs, i) }
  return xs[n - 1] + len(xs)
}
"#,
    )
    .unwrap();
    let m = decode(&bytes);
    let mut inst = Inst::new(&m);
    // 100000 numbers are 800 KB plus the doubling copies: far more than the first 64 KB page
    assert_eq!(inst.call("big", &[V::F(100000.0)]).unwrap(), Some(V::F(199999.0)));
    assert!(inst.mem.len() > 65536 * 8);
}

#[test]
fn out_of_range_indexes_trap() {
    let (bytes, _) = compile_full("fn get(xs: array, i) { return xs[i] }\nfn ch(s: string, i) { return s[i] }\n").unwrap();
    let m = decode(&bytes);
    let mut inst = Inst::new(&m);
    let p = inst.new_list(&[1.0, 2.0]);
    assert_eq!(inst.call("get", &[V::I(p), V::F(1.0)]).unwrap(), Some(V::F(2.0)));
    assert_eq!(inst.call("get", &[V::I(p), V::F(-2.0)]).unwrap(), Some(V::F(1.0)));
    assert!(inst.call("get", &[V::I(p), V::F(2.0)]).unwrap_err().contains("unreachable"));
    assert!(inst.call("get", &[V::I(p), V::F(-3.0)]).is_err());
    let q = inst.new_str("ab");
    assert!(inst.call("ch", &[V::I(q), V::F(2.0)]).is_err());
}

#[test]
fn a_numbers_only_module_still_has_no_memory() {
    let (bytes, _) = compile_full("fn f(a, b) { return a * b + 1 }\nfn g(x) { return f(x, x) }\n").unwrap();
    let m = decode(&bytes);
    assert_eq!(m.mem_pages, None);
    assert!(m.exports.iter().all(|(_, k, _)| *k == 0));
    assert_eq!(m.funcs.len(), 2);
}

#[test]
fn unsupported_string_and_list_uses_name_the_line() {
    let cases: Vec<(&str, &str)> = vec![
        ("fn f(x) {\n  return {1: 2}\n}\n", "Line 2"),
        ("fn f(s) {\n  y = s + 1\n  return y\n}\nfn g() { return f(\"a\") }\n", "Line 5"),
        ("fn f(xs: array) {\n  return xs + xs\n}\n", "Line 2"),
        ("fn f(s: string) {\n  x = 1\n  x = s\n  return x\n}\n", "Line 3"),
        ("fn f(xs: array) {\n  return [xs]\n}\n", "Line 2"),
        ("fn f(x) {\n  return len(x)\n}\n", "Line 2"),
        ("fn f(s: string) {\n  s[0] = \"a\"\n  return s\n}\n", "Line 2"),
        ("fn f() {\n  return upper(\"a\")\n}\n", "Line 2"),
        ("fn f() {\n  return \"caf\u{e9}\"\n}\n", "Line 2"),
        ("fn f() {\n  return \"abc\n}\n", "Line 2"),
        ("fn f(a: string, b: array) {\n  return a == b\n}\n", "Line 2"),
        ("fn f(xs: array) {\n  xs[foo()] += 1\n}\nfn foo() { return 0 }\n", "Line 2"),
    ];
    for (src, line) in cases {
        let e = compile(src).err().unwrap_or_else(|| panic!("should not compile:\n{src}"));
        assert!(e.starts_with(line), "expected {line} in {e:?} for\n{src}");
    }
}
