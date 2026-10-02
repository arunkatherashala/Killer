#![allow(unsafe_code)]
//! Function-level JIT for *pure numeric* Killer functions (x86-64).
//!
//! A function qualifies when every reachable instruction is one of: number constants, `argN`
//! loads, local slots, `+ - * / %`, comparisons, boolean ops, jumps, calls to other qualifying
//! functions, `len(array)`, and indexing of array *arguments* (arrays of numbers only).
//! Such code has no side effects beyond writing into its array arguments, so on any condition
//! native code cannot handle (division by zero, bad index, deep recursion, runaway loop) it sets
//! a bail flag and the interpreter simply re-runs the call from scratch.
//!
//! Native convention: arguments stay on the machine stack (first argument deepest), result in
//! xmm0, caller pops. Number = f64 bits, array = pointer to `[len, f64...]`. The operand stack
//! lives on the machine stack; booleans are SSE compare masks (all-ones / zero). Array arguments
//! are copied into flat buffers on entry and copied back after a successful native run.

use crate::bytecode::Instruction;
use crate::value::{SharedArray, Value};
use std::collections::{HashMap, HashSet};

#[cfg(target_arch = "x86_64")]
use crate::jit_x86::ExecPage;

const CALL_THRESHOLD: u32 = 30;
const MAX_ARGS: usize = 8;
const MAX_DEPTH: i32 = 3000;
const MAX_FN_INSTRS: usize = 4096;
const MAX_SLOTS: usize = 64;
const LOOP_BUDGET: i64 = 4_000_000_000;
const LEN_BUILTIN_ID: u16 = 0;
const B_SQRT: u16 = 16;
const B_ABS: u16 = 18;
const B_FLOOR: u16 = 19;
const B_CEIL: u16 = 20;
const B_MIN: u16 = 22;
const B_MAX: u16 = 23;
const B_RANGE: u16 = 36;

fn has_sse41() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("sse4.1")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// (function entry ip, bit i set = argument i is an array)
type Key = (usize, u8);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ty {
    Num,
    Bool,
    Arr,
    /// Virtual `range(a, b)`: one word, `(start << 32) | len`, never materialised.
    Rng,
}

pub struct FnJit {
    counts: HashMap<usize, u32>,
    failed: HashSet<Key>,
    loopy: HashMap<Key, bool>,
    #[cfg(target_arch = "x86_64")]
    compiled: HashMap<Key, (usize, bool, Vec<String>)>, // key -> (thunk, writes arrays, named locals)
    #[cfg(target_arch = "x86_64")]
    pages: Vec<ExecPage>,
    // [0] = bail flag, [1] = recursion depth, [2] = remaining loop iterations.
    state: Box<[i64; 3]>,
    pub native_calls: u64,
}

impl FnJit {
    pub fn new() -> Self {
        FnJit {
            counts: HashMap::new(),
            failed: HashSet::new(),
            loopy: HashMap::new(),
            #[cfg(target_arch = "x86_64")]
            compiled: HashMap::new(),
            #[cfg(target_arch = "x86_64")]
            pages: Vec::new(),
            state: Box::new([0; 3]),
            native_calls: 0,
        }
    }

    pub fn reset(&mut self) {
        self.counts.clear();
        self.failed.clear();
        self.loopy.clear();
        #[cfg(target_arch = "x86_64")]
        {
            self.compiled.clear();
            self.pages.clear();
        }
    }

    /// Try to execute `target(args)` natively. `None` means: use the interpreter.
    pub fn try_call(
        &mut self,
        instrs: &[Instruction],
        arities: &HashMap<usize, usize>,
        target: usize,
        args: &[Value],
        conflict: &dyn Fn(&str) -> bool,
    ) -> Option<f64> {
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (instrs, arities, target, args, conflict);
            return None;
        }
        #[cfg(target_arch = "x86_64")]
        {
            if args.len() > MAX_ARGS {
                return None;
            }
            let mut mask = 0u8;
            for (i, a) in args.iter().enumerate() {
                match a {
                    Value::Number(_) => {}
                    Value::Array(_) => mask |= 1 << i,
                    _ => return None,
                }
            }
            let key: Key = (target, mask);
            if self.failed.contains(&key) {
                return None;
            }
            if !self.compiled.contains_key(&key) {
                if arities.get(&target).copied() != Some(args.len()) {
                    self.failed.insert(key);
                    return None;
                }
                // A function with a loop is worth compiling on its first call (it may only ever be
                // called once); loop-free functions wait until they are hot.
                let loopy = match self.loopy.get(&key) {
                    Some(l) => *l,
                    None => match analyze(instrs, arities, key) {
                        Some(an) => {
                            let l = an.at.keys().any(|&i| {
                                matches!(&instrs[i], Instruction::Jump(t) | Instruction::JumpIfFalse(t) if *t <= i)
                            });
                            self.loopy.insert(key, l);
                            l
                        }
                        None => {
                            self.failed.insert(key);
                            return None;
                        }
                    },
                };
                let c = self.counts.entry(target).or_insert(0);
                *c += 1;
                if !loopy && *c < CALL_THRESHOLD {
                    return None;
                }
                if !self.compile(instrs, arities, key) {
                    self.failed.insert(key);
                    return None;
                }
            }
            let (thunk, writes, names) = self.compiled.get(&key)?.clone();
            // A plain `Store(name)` updates a same-named outer variable in place in the interpreter
            // (`StoreLocal`, used by `for`, never does); native code keeps its own locals, so run
            // natively only when no such outer variable exists.
            if names.iter().any(|n| conflict(n)) {
                return None;
            }

            // Marshal: numbers as f64 bits, arrays as flat [len, f64...] buffers.
            let mut words: Vec<u64> = Vec::with_capacity(args.len());
            let mut buffers: Vec<(usize, Vec<u64>)> = Vec::new();
            let mut seen_ptrs: Vec<usize> = Vec::new();
            for (i, a) in args.iter().enumerate() {
                match a {
                    Value::Number(x) => words.push(x.to_bits()),
                    Value::Array(arr) => {
                        let p = arr.rc_ptr();
                        if seen_ptrs.contains(&p) {
                            return None; // aliased array arguments: keep exact semantics
                        }
                        seen_ptrs.push(p);
                        let elems = arr.clone_elements();
                        let mut buf: Vec<u64> = Vec::with_capacity(elems.len() + 1);
                        buf.push(elems.len() as u64);
                        for e in &elems {
                            match e {
                                Value::Number(x) => buf.push(x.to_bits()),
                                _ => return None,
                            }
                        }
                        buffers.push((i, buf));
                        words.push(0); // patched below once buffers stop moving
                    }
                    _ => return None,
                }
            }
            for (i, buf) in &buffers {
                words[*i] = buf.as_ptr() as u64;
            }

            self.state[0] = 0;
            self.state[1] = 0;
            self.state[2] = LOOP_BUDGET;
            let r = unsafe { std::mem::transmute::<usize, extern "C" fn(*const u64) -> f64>(thunk)(words.as_ptr()) };
            if self.state[0] != 0 {
                self.state[0] = 0;
                self.state[1] = 0;
                return None;
            }
            if writes {
                for (i, buf) in &buffers {
                    if let Value::Array(arr) = &args[*i] {
                        let vals: Vec<Value> = buf[1..].iter().map(|b| Value::Number(f64::from_bits(*b))).collect();
                        arr.replace_all(vals);
                    }
                }
            }
            self.native_calls += 1;
            Some(r)
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn compile(&mut self, instrs: &[Instruction], arities: &HashMap<usize, usize>, root: Key) -> bool {
        let state_ptr = self.state.as_ptr() as usize;
        let Some((code, thunk_off, writes, names)) = compile_group(instrs, arities, root, state_ptr) else { return false };
        let size = (code.len() + 4095) & !4095;
        let Some(mut page) = ExecPage::alloc(size) else { return false };
        unsafe { page.write(&code) };
        let base = page.base() as usize;
        self.compiled.insert(root, (base + thunk_off, writes, names));
        self.pages.push(page);
        true
    }
}

impl Default for FnJit {
    fn default() -> Self {
        Self::new()
    }
}

fn arg_index(name: &str, arity: usize) -> Option<usize> {
    let i: usize = name.strip_prefix("arg")?.parse().ok()?;
    if i < arity && i < MAX_ARGS { Some(i) } else { None }
}

#[derive(Clone, PartialEq, Eq)]
struct State {
    stack: Vec<Ty>,
    slots: Vec<Option<Ty>>,    // numbered slot -> type when definitely assigned
    named: Vec<Option<Ty>>,    // named local -> type when definitely assigned
    scopes: Vec<Vec<usize>>,   // named locals introduced in each open scope
}

struct Analysis {
    at: HashMap<usize, State>,
    callees: Vec<Key>,
    nslots: usize,
    writes: bool,
    names: Vec<String>,
    /// names written with a plain `Store` (may update an outer variable in the interpreter)
    plain_stores: Vec<String>,
}

/// Abstract-interpret one function specialised for `mask`; `None` if anything is unsupported.
fn analyze(instrs: &[Instruction], arities: &HashMap<usize, usize>, key: Key) -> Option<Analysis> {
    let (entry, mask) = key;
    let arity = *arities.get(&entry)?;
    if arity > MAX_ARGS {
        return None;
    }
    let n = instrs.len();
    let mut at: HashMap<usize, State> = HashMap::new();
    let mut work: Vec<usize> = vec![entry];
    let mut callees: Vec<Key> = Vec::new();
    let mut nslots = 0usize;
    let mut writes = false;
    let mut names: Vec<String> = Vec::new();
    let mut plain_stores: Vec<String> = Vec::new();
    at.insert(entry, State { stack: Vec::new(), slots: Vec::new(), named: Vec::new(), scopes: Vec::new() });

    // Join `st` into the state recorded at `ip`: stacks must match; a slot stays "assigned" only
    // if it is assigned on every incoming path. Weakening re-queues `ip` so downstream is rechecked.
    fn flow(at: &mut HashMap<usize, State>, work: &mut Vec<usize>, ip: usize, st: &State, n: usize) -> bool {
        if ip >= n {
            return false;
        }
        match at.get_mut(&ip) {
            None => {
                at.insert(ip, st.clone());
                work.push(ip);
                true
            }
            Some(prev) => {
                if prev.stack != st.stack || prev.scopes.len() != st.scopes.len() {
                    return false;
                }
                // Variables dropped at scope exit: union over paths (over-dropping is always safe).
                let mut scopes_changed = false;
                for (ps, ss) in prev.scopes.iter_mut().zip(st.scopes.iter()) {
                    for id in ss {
                        if !ps.contains(id) {
                            ps.push(*id);
                            scopes_changed = true;
                        }
                    }
                }
                let len = prev.slots.len().max(st.slots.len());
                let mut merged: Vec<Option<Ty>> = Vec::with_capacity(len);
                for i in 0..len {
                    let a = prev.slots.get(i).copied().flatten();
                    let b = st.slots.get(i).copied().flatten();
                    merged.push(match (a, b) {
                        (Some(x), Some(y)) if x == y => Some(x),
                        _ => None, // unassigned on some path, or reused with a different type
                    });
                }
                let mut old = prev.slots.clone();
                old.resize(len, None);
                let nlen = prev.named.len().max(st.named.len());
                let mut nmerged: Vec<Option<Ty>> = Vec::with_capacity(nlen);
                for i in 0..nlen {
                    let a = prev.named.get(i).copied().flatten();
                    let b = st.named.get(i).copied().flatten();
                    nmerged.push(match (a, b) {
                        (Some(x), Some(y)) if x == y => Some(x),
                        (Some(_), Some(_)) => return false,
                        _ => None,
                    });
                }
                let mut nold = prev.named.clone();
                nold.resize(nlen, None);
                if merged != old || nmerged != nold || scopes_changed {
                    prev.slots = merged;
                    prev.named = nmerged;
                    work.push(ip);
                }
                true
            }
        }
    }
    fn slot_ty(st: &State, s: usize) -> Option<Ty> {
        if s < MAX_SLOTS { st.slots.get(s).copied().flatten() } else { None }
    }
    fn top2_num(st: &State) -> bool {
        let l = st.stack.len();
        l >= 2 && st.stack[l - 1] == Ty::Num && st.stack[l - 2] == Ty::Num
    }
    let arg_ty = |i: usize| if mask & (1 << i) != 0 { Ty::Arr } else { Ty::Num };

    while let Some(ip0) = work.pop() {
        let mut ip = ip0;
        let mut st = at.get(&ip)?.clone();
        loop {
            if at.len() > MAX_FN_INSTRS || ip >= n {
                return None;
            }
            match &instrs[ip] {
                Instruction::ConstNum(_) => st.stack.push(Ty::Num),
                Instruction::ConstBool(_) => st.stack.push(Ty::Bool),
                Instruction::Load(name) => {
                    if let Some(i) = arg_index(name, arity) {
                        st.stack.push(arg_ty(i));
                    } else {
                        let id = names.iter().position(|n| n == name)?;
                        let ty = st.named.get(id).copied().flatten()?;
                        st.stack.push(ty);
                    }
                }
                Instruction::Store(name) | Instruction::StoreLocal(name) => {
                    if name.strip_prefix("arg").map_or(false, |r| r.parse::<usize>().is_ok()) {
                        return None;
                    }
                    let ty = st.stack.pop()?;
                    if ty == Ty::Bool {
                        return None;
                    }
                    if matches!(&instrs[ip], Instruction::Store(_)) && !plain_stores.contains(name) {
                        plain_stores.push(name.clone());
                    }
                    let id = match names.iter().position(|n| n == name) {
                        Some(i) => i,
                        None => {
                            names.push(name.clone());
                            names.len() - 1
                        }
                    };
                    if st.named.len() <= id {
                        st.named.resize(id + 1, None);
                    }
                    match st.named[id] {
                        Some(prev) if prev != ty => return None,
                        Some(_) => {}
                        None => {
                            st.named[id] = Some(ty);
                            if let Some(sc) = st.scopes.last_mut() {
                                sc.push(id);
                            }
                        }
                    }
                }
                Instruction::LoadSlot(s) => {
                    let ty = slot_ty(&st, *s as usize)?;
                    st.stack.push(ty);
                }
                Instruction::StoreSlot(s) => {
                    let s = *s as usize;
                    let ty = st.stack.pop()?;
                    if s >= MAX_SLOTS || ty == Ty::Bool {
                        return None;
                    }
                    if st.slots.len() <= s {
                        st.slots.resize(s + 1, None);
                    }
                    st.slots[s] = Some(ty);
                    nslots = nslots.max(s + 1);
                }
                Instruction::AddSlotConst(s, _) | Instruction::SubSlotConst(s, _) => {
                    if slot_ty(&st, *s as usize) != Some(Ty::Num) {
                        return None;
                    }
                }
                Instruction::LtSlotConst(s, _) | Instruction::GtSlotConst(s, _)
                | Instruction::GeSlotConst(s, _) | Instruction::LeSlotConst(s, _)
                | Instruction::EqSlotConst(s, _) => {
                    if slot_ty(&st, *s as usize) != Some(Ty::Num) {
                        return None;
                    }
                    st.stack.push(Ty::Bool);
                }
                Instruction::Add | Instruction::Sub | Instruction::Mul | Instruction::Div | Instruction::Mod => {
                    if !top2_num(&st) {
                        return None;
                    }
                    st.stack.pop();
                }
                Instruction::Lt | Instruction::Le | Instruction::Gt | Instruction::Ge
                | Instruction::Eq | Instruction::Ne => {
                    if !top2_num(&st) {
                        return None;
                    }
                    st.stack.pop();
                    st.stack.pop();
                    st.stack.push(Ty::Bool);
                }
                Instruction::And | Instruction::Or => {
                    let l = st.stack.len();
                    if l < 2 || st.stack[l - 1] != Ty::Bool || st.stack[l - 2] != Ty::Bool {
                        return None;
                    }
                    st.stack.pop();
                }
                Instruction::Not => {
                    if st.stack.last() != Some(&Ty::Bool) {
                        return None;
                    }
                }
                Instruction::Pop => {
                    st.stack.pop()?;
                }
                Instruction::IndexRead => {
                    let l = st.stack.len();
                    if l < 2 || st.stack[l - 1] != Ty::Num || !matches!(st.stack[l - 2], Ty::Arr | Ty::Rng) {
                        return None;
                    }
                    st.stack.pop();
                    st.stack.pop();
                    st.stack.push(Ty::Num);
                }
                Instruction::IndexWrite(name) => {
                    let is_arr = match arg_index(name, arity) {
                        Some(i) => arg_ty(i) == Ty::Arr,
                        None => {
                            let id = names.iter().position(|n| n == name)?;
                            st.named.get(id).copied().flatten() == Some(Ty::Arr)
                        }
                    };
                    if !is_arr || !top2_num(&st) {
                        return None;
                    }
                    st.stack.pop();
                    st.stack.pop();
                    writes = true;
                }
                Instruction::CallBuiltinId(id, argc) => {
                    let l = st.stack.len();
                    match (*id, *argc) {
                        (LEN_BUILTIN_ID, 1) => {
                            if !matches!(st.stack.last(), Some(Ty::Arr | Ty::Rng)) {
                                return None;
                            }
                            st.stack.pop();
                            st.stack.push(Ty::Num);
                        }
                        (B_SQRT | B_ABS, 1) => {
                            if st.stack.last() != Some(&Ty::Num) {
                                return None;
                            }
                        }
                        (B_FLOOR | B_CEIL, 1) => {
                            if st.stack.last() != Some(&Ty::Num) || !has_sse41() {
                                return None;
                            }
                        }
                        (B_MIN | B_MAX, 2) => {
                            if !top2_num(&st) {
                                return None;
                            }
                            st.stack.pop();
                        }
                        (B_RANGE, 1) => {
                            if st.stack.last() != Some(&Ty::Num) {
                                return None;
                            }
                            st.stack[l - 1] = Ty::Rng;
                        }
                        (B_RANGE, 2) => {
                            if !top2_num(&st) {
                                return None;
                            }
                            st.stack.pop();
                            let l2 = st.stack.len();
                            st.stack[l2 - 1] = Ty::Rng;
                        }
                        _ => return None,
                    }
                }
                Instruction::ForNext { iter, idx, var, body, exit } => {
                    let (is, xs) = (*iter as usize, *idx as usize);
                    if !matches!(slot_ty(&st, is), Some(Ty::Arr | Ty::Rng)) || slot_ty(&st, xs) != Some(Ty::Num) {
                        return None;
                    }
                    if var.strip_prefix("arg").map_or(false, |r| r.parse::<usize>().is_ok()) {
                        return None;
                    }
                    // exit path: unchanged state; body path: loop variable assigned a number
                    if !flow(&mut at, &mut work, *exit, &st, n) {
                        return None;
                    }
                    let id = match names.iter().position(|nm| nm == var) {
                        Some(i) => i,
                        None => {
                            names.push(var.clone());
                            names.len() - 1
                        }
                    };
                    let mut bst = st.clone();
                    if bst.named.len() <= id {
                        bst.named.resize(id + 1, None);
                    }
                    match bst.named[id] {
                        Some(prev) if prev != Ty::Num => return None,
                        Some(_) => {}
                        None => {
                            bst.named[id] = Some(Ty::Num);
                            if let Some(sc) = bst.scopes.last_mut() {
                                sc.push(id);
                            }
                        }
                    }
                    if !flow(&mut at, &mut work, *body, &bst, n) {
                        return None;
                    }
                    break; // never falls through: the generic path is not compiled natively
                }
                Instruction::EnterScope => st.scopes.push(Vec::new()),
                Instruction::ExitScope => {
                    let sc = st.scopes.pop()?;
                    for id in sc {
                        st.named[id] = None;
                    }
                }
                Instruction::Jump(t) => {
                    if !flow(&mut at, &mut work, *t, &st, n) {
                        return None;
                    }
                    break;
                }
                Instruction::JumpIfFalse(t) => {
                    if st.stack.pop()? != Ty::Bool {
                        return None;
                    }
                    if !flow(&mut at, &mut work, *t, &st, n) {
                        return None;
                    }
                }
                Instruction::Ret => {
                    if st.stack.as_slice() != [Ty::Num] {
                        return None;
                    }
                    break;
                }
                Instruction::Call { target, arg_count } => {
                    let argc = *arg_count;
                    if arities.get(target).copied() != Some(argc) || argc > MAX_ARGS {
                        return None;
                    }
                    let l = st.stack.len();
                    if l < argc {
                        return None;
                    }
                    let mut cmask = 0u8;
                    for (k, t) in st.stack[l - argc..].iter().enumerate() {
                        match t {
                            Ty::Num => {}
                            Ty::Arr => cmask |= 1 << k,
                            Ty::Bool | Ty::Rng => return None,
                        }
                    }
                    st.stack.truncate(l - argc);
                    st.stack.push(Ty::Num);
                    let ck = (*target, cmask);
                    if !callees.contains(&ck) {
                        callees.push(ck);
                    }
                }
                _ => return None,
            }
            ip += 1;
            if at.contains_key(&ip) {
                if !flow(&mut at, &mut work, ip, &st, n) {
                    return None;
                }
                break;
            }
            if ip >= n {
                return None;
            }
            at.insert(ip, st.clone());
        }
    }
    Some(Analysis { at, callees, nslots, writes, names, plain_stores })
}

#[cfg(target_arch = "x86_64")]
struct Asm {
    buf: Vec<u8>,
}

#[cfg(target_arch = "x86_64")]
impl Asm {
    fn b(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }
    fn rel32_placeholder(&mut self) -> usize {
        let p = self.buf.len();
        self.buf.extend_from_slice(&[0, 0, 0, 0]);
        p
    }
    fn patch(&mut self, at: usize, target: usize) {
        let rel = target as i64 - (at as i64 + 4);
        self.buf[at..at + 4].copy_from_slice(&(rel as i32).to_le_bytes());
    }
    fn state_to_rax(&mut self, state_ptr: usize) {
        self.b(&[0x48, 0xB8]);
        self.b(&(state_ptr as u64).to_le_bytes());
    }
}

/// Emit the integer-index + bounds check. Expects the index in xmm0 and the array pointer in rcx;
/// leaves the validated index in rax. Jumps to bail on non-integral / out-of-range indices.
#[cfg(target_arch = "x86_64")]
fn emit_index_check(asm: &mut Asm, bail_fixups: &mut Vec<usize>) {
    asm.b(&[0xF2, 0x48, 0x0F, 0x2C, 0xC0]); // cvttsd2si rax,xmm0
    asm.b(&[0xF2, 0x48, 0x0F, 0x2A, 0xC8]); // cvtsi2sd xmm1,rax
    asm.b(&[0x66, 0x0F, 0x2E, 0xC1]); // ucomisd xmm0,xmm1
    asm.b(&[0x0F, 0x85]); // jne bail (fractional)
    bail_fixups.push(asm.rel32_placeholder());
    asm.b(&[0x0F, 0x8A]); // jp bail (NaN)
    bail_fixups.push(asm.rel32_placeholder());
    asm.b(&[0x48, 0x3B, 0x01]); // cmp rax,[rcx]
    asm.b(&[0x0F, 0x83]); // jae bail (negative or >= len)
    bail_fixups.push(asm.rel32_placeholder());
}

/// Compile `root` and every function it calls. Returns (code, thunk offset, writes-arrays).
#[cfg(target_arch = "x86_64")]
fn compile_group(
    instrs: &[Instruction],
    arities: &HashMap<usize, usize>,
    root: Key,
    state_ptr: usize,
) -> Option<(Vec<u8>, usize, bool, Vec<String>)> {
    let mut asm = Asm { buf: Vec::new() };
    let mut all_names: Vec<String> = Vec::new();
    let mut entries: HashMap<Key, usize> = HashMap::new();
    let mut call_fixups: Vec<(usize, Key)> = Vec::new();
    let mut pending = vec![root];
    let mut seen: HashSet<Key> = HashSet::new();
    seen.insert(root);
    let mut any_writes = false;

    while let Some(fk) = pending.pop() {
        let an = analyze(instrs, arities, fk)?;
        any_writes |= an.writes;
        for n in &an.plain_stores {
            if !all_names.contains(n) {
                all_names.push(n.clone());
            }
        }
        for c in &an.callees {
            if seen.insert(*c) {
                pending.push(*c);
            }
        }
        let arity = arities[&fk.0];
        let types = an.at;
        entries.insert(fk, asm.buf.len());

        // prologue: push rbp; mov rbp,rsp; sub rsp,frame
        let frame = ((8 * (an.nslots + an.names.len()) as i32) + 15) & !15;
        asm.b(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x81, 0xEC]);
        asm.b(&frame.to_le_bytes());
        asm.state_to_rax(state_ptr);
        asm.b(&[0x48, 0xFF, 0x40, 0x08]); // inc qword [rax+8]
        asm.b(&[0x48, 0x81, 0x78, 0x08]); // cmp qword [rax+8], imm32
        asm.b(&MAX_DEPTH.to_le_bytes());
        asm.b(&[0x0F, 0x8F]); // jg bail
        let mut bail_fixups = vec![asm.rel32_placeholder()];

        let mut labels: HashMap<usize, usize> = HashMap::new();
        let mut jump_fixups: Vec<(usize, usize)> = Vec::new();
        let mut ips: Vec<usize> = types.keys().copied().collect();
        ips.sort_unstable();
        let headers: HashSet<usize> = ips
            .iter()
            .filter_map(|&i| match &instrs[i] {
                Instruction::Jump(t) | Instruction::JumpIfFalse(t) if *t <= i => Some(*t),
                _ => None,
            })
            .collect();
        let slot_disp = |s: u16| -> [u8; 4] { (-8 * (1 + s as i32)).to_le_bytes() };
        let arg_disp = |i: usize| -> [u8; 4] { (16 + 8 * (arity as i32 - 1 - i as i32)).to_le_bytes() };
        let nslots = an.nslots;
        let names = &an.names;
        let named_disp = |name: &str| -> Option<[u8; 4]> {
            let id = names.iter().position(|n| n == name)?;
            Some((-8 * (1 + (nslots + id) as i32)).to_le_bytes())
        };

        for ip in ips {
            labels.insert(ip, asm.buf.len());
            if headers.contains(&ip) {
                asm.state_to_rax(state_ptr);
                asm.b(&[0x48, 0xFF, 0x48, 0x10]); // dec qword [rax+16]
                asm.b(&[0x0F, 0x8E]); // jle bail
                bail_fixups.push(asm.rel32_placeholder());
            }
            match &instrs[ip] {
                Instruction::ConstNum(x) => {
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&x.to_bits().to_le_bytes());
                    asm.b(&[0x50]); // push rax
                }
                Instruction::ConstBool(b) => {
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&(if *b { u64::MAX } else { 0 }).to_le_bytes());
                    asm.b(&[0x50]);
                }
                Instruction::Load(name) => {
                    asm.b(&[0xFF, 0xB5]); // push qword [rbp+disp32]
                    match arg_index(name, arity) {
                        Some(i) => asm.b(&arg_disp(i)),
                        None => asm.b(&named_disp(name)?),
                    }
                }
                Instruction::Store(name) | Instruction::StoreLocal(name) => {
                    asm.b(&[0x8F, 0x85]); // pop qword [rbp+disp32]
                    asm.b(&named_disp(name)?);
                }
                Instruction::LoadSlot(s) => {
                    asm.b(&[0xFF, 0xB5]);
                    asm.b(&slot_disp(*s));
                }
                Instruction::StoreSlot(s) => {
                    asm.b(&[0x8F, 0x85]); // pop qword [rbp+disp32]
                    asm.b(&slot_disp(*s));
                }
                Instruction::AddSlotConst(s, c) | Instruction::SubSlotConst(s, c) => {
                    asm.b(&[0xF2, 0x0F, 0x10, 0x85]); // movsd xmm0,[slot]
                    asm.b(&slot_disp(*s));
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&c.to_bits().to_le_bytes());
                    asm.b(&[0x66, 0x48, 0x0F, 0x6E, 0xC8]); // movq xmm1,rax
                    let op = if matches!(&instrs[ip], Instruction::AddSlotConst(..)) { 0x58 } else { 0x5C };
                    asm.b(&[0xF2, 0x0F, op, 0xC1]);
                    asm.b(&[0xF2, 0x0F, 0x11, 0x85]); // movsd [slot],xmm0
                    asm.b(&slot_disp(*s));
                }
                Instruction::LtSlotConst(s, c) | Instruction::LeSlotConst(s, c)
                | Instruction::GtSlotConst(s, c) | Instruction::GeSlotConst(s, c) => {
                    let slot_first = matches!(&instrs[ip], Instruction::LtSlotConst(..) | Instruction::LeSlotConst(..));
                    let pred: u8 = if matches!(&instrs[ip], Instruction::LtSlotConst(..) | Instruction::GtSlotConst(..)) { 1 } else { 2 };
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&c.to_bits().to_le_bytes());
                    if slot_first {
                        asm.b(&[0x66, 0x48, 0x0F, 0x6E, 0xC8]); // movq xmm1,rax (const)
                        asm.b(&[0xF2, 0x0F, 0x10, 0x85]); // movsd xmm0,[slot]
                        asm.b(&slot_disp(*s));
                    } else {
                        asm.b(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // movq xmm0,rax (const)
                        asm.b(&[0xF2, 0x0F, 0x10, 0x8D]); // movsd xmm1,[slot]
                        asm.b(&slot_disp(*s));
                    }
                    asm.b(&[0xF2, 0x0F, 0xC2, 0xC1, pred]); // cmpsd xmm0,xmm1
                    asm.b(&[0x48, 0x83, 0xEC, 0x08]);
                    asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]);
                }
                Instruction::EqSlotConst(s, c) => {
                    // |slot - c| < EPSILON (matches the interpreter)
                    asm.b(&[0xF2, 0x0F, 0x10, 0x85]);
                    asm.b(&slot_disp(*s));
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&c.to_bits().to_le_bytes());
                    asm.b(&[0x66, 0x48, 0x0F, 0x6E, 0xC8]);
                    asm.b(&[0xF2, 0x0F, 0x5C, 0xC1]); // subsd xmm0,xmm1
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&0x7FFF_FFFF_FFFF_FFFFu64.to_le_bytes());
                    asm.b(&[0x66, 0x48, 0x0F, 0x6E, 0xC8]);
                    asm.b(&[0x66, 0x0F, 0x54, 0xC1]); // andpd xmm0,xmm1
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&f64::EPSILON.to_bits().to_le_bytes());
                    asm.b(&[0x66, 0x48, 0x0F, 0x6E, 0xC8]);
                    asm.b(&[0xF2, 0x0F, 0xC2, 0xC1, 1]); // cmpltsd
                    asm.b(&[0x48, 0x83, 0xEC, 0x08]);
                    asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]);
                }
                Instruction::Add | Instruction::Sub | Instruction::Mul | Instruction::Div => {
                    asm.b(&[0xF2, 0x0F, 0x10, 0x0C, 0x24]); // movsd xmm1,[rsp]
                    asm.b(&[0xF2, 0x0F, 0x10, 0x44, 0x24, 0x08]); // movsd xmm0,[rsp+8]
                    let op = match &instrs[ip] {
                        Instruction::Add => 0x58,
                        Instruction::Sub => 0x5C,
                        Instruction::Mul => 0x59,
                        _ => {
                            asm.b(&[0x66, 0x0F, 0x57, 0xD2]); // xorpd xmm2,xmm2
                            asm.b(&[0x66, 0x0F, 0x2E, 0xCA]); // ucomisd xmm1,xmm2
                            asm.b(&[0x0F, 0x84]); // je bail
                            bail_fixups.push(asm.rel32_placeholder());
                            0x5E
                        }
                    };
                    asm.b(&[0xF2, 0x0F, op, 0xC1]);
                    asm.b(&[0x48, 0x83, 0xC4, 0x08]); // add rsp,8
                    asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]); // movsd [rsp],xmm0
                }
                Instruction::Mod => {
                    asm.b(&[0xF2, 0x0F, 0x10, 0x0C, 0x24]); // xmm1 = rhs
                    asm.b(&[0x66, 0x0F, 0x57, 0xD2, 0x66, 0x0F, 0x2E, 0xCA]); // xorpd; ucomisd
                    asm.b(&[0x0F, 0x84]); // je bail (modulo by zero -> interpreter error)
                    bail_fixups.push(asm.rel32_placeholder());
                    asm.b(&[0xDD, 0x04, 0x24]); // fld qword [rsp]      (rhs)
                    asm.b(&[0xDD, 0x44, 0x24, 0x08]); // fld qword [rsp+8] (lhs)
                    asm.b(&[0xD9, 0xF8, 0xDF, 0xE0, 0xF6, 0xC4, 0x04, 0x75, 0xF7]); // fprem loop
                    asm.b(&[0xDD, 0xD9]); // fstp st(1)
                    asm.b(&[0xDD, 0x5C, 0x24, 0x08]); // fstp qword [rsp+8]
                    asm.b(&[0x48, 0x83, 0xC4, 0x08]); // add rsp,8
                }
                Instruction::Lt | Instruction::Le | Instruction::Eq | Instruction::Ne => {
                    asm.b(&[0xF2, 0x0F, 0x10, 0x0C, 0x24]); // xmm1 = rhs
                    asm.b(&[0xF2, 0x0F, 0x10, 0x44, 0x24, 0x08]); // xmm0 = lhs
                    let pred = match &instrs[ip] {
                        Instruction::Eq => 0,
                        Instruction::Lt => 1,
                        Instruction::Le => 2,
                        _ => 4,
                    };
                    asm.b(&[0xF2, 0x0F, 0xC2, 0xC1, pred]);
                    asm.b(&[0x48, 0x83, 0xC4, 0x08]);
                    asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]);
                }
                Instruction::Gt | Instruction::Ge => {
                    asm.b(&[0xF2, 0x0F, 0x10, 0x04, 0x24]); // xmm0 = rhs
                    asm.b(&[0xF2, 0x0F, 0x10, 0x4C, 0x24, 0x08]); // xmm1 = lhs
                    let pred = if matches!(&instrs[ip], Instruction::Gt) { 1 } else { 2 };
                    asm.b(&[0xF2, 0x0F, 0xC2, 0xC1, pred]); // rhs < lhs  /  rhs <= lhs
                    asm.b(&[0x48, 0x83, 0xC4, 0x08]);
                    asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]);
                }
                Instruction::And => asm.b(&[0x58, 0x48, 0x21, 0x04, 0x24]), // pop rax; and [rsp],rax
                Instruction::Or => asm.b(&[0x58, 0x48, 0x09, 0x04, 0x24]),  // pop rax; or [rsp],rax
                Instruction::Not => asm.b(&[0x48, 0x83, 0x34, 0x24, 0xFF]), // xor qword [rsp],-1
                Instruction::Pop => asm.b(&[0x48, 0x83, 0xC4, 0x08]),
                Instruction::ForNext { iter, idx, var, body, exit } => {
                    let st = types.get(&ip)?;
                    let is_rng = st.slots.get(*iter as usize).copied().flatten()? == Ty::Rng;
                    asm.b(&[0xF2, 0x0F, 0x10, 0x85]); // movsd xmm0,[idx]
                    asm.b(&slot_disp(*idx));
                    asm.b(&[0xF2, 0x48, 0x0F, 0x2C, 0xC0]); // cvttsd2si rax,xmm0
                    if is_rng {
                        asm.b(&[0x48, 0x8B, 0x95]); // mov rdx,[iter]  (packed range)
                        asm.b(&slot_disp(*iter));
                        asm.b(&[0x89, 0xD1]); // mov ecx,edx  (len, zero-extended)
                        asm.b(&[0x48, 0x39, 0xC8]); // cmp rax,rcx
                    } else {
                        asm.b(&[0x48, 0x8B, 0x8D]); // mov rcx,[iter]  (array pointer)
                        asm.b(&slot_disp(*iter));
                        asm.b(&[0x48, 0x8B, 0x11]); // mov rdx,[rcx]   (length)
                        asm.b(&[0x48, 0x39, 0xD0]); // cmp rax,rdx
                    }
                    asm.b(&[0x0F, 0x83]); // jae exit
                    jump_fixups.push((asm.rel32_placeholder(), *exit));
                    if is_rng {
                        asm.b(&[0x48, 0xC1, 0xFA, 0x20]); // sar rdx,32      (start)
                        asm.b(&[0x48, 0x01, 0xC2]); // add rdx,rax
                        asm.b(&[0xF2, 0x48, 0x0F, 0x2A, 0xC2]); // cvtsi2sd xmm0,rdx
                    } else {
                        asm.b(&[0xF2, 0x0F, 0x10, 0x44, 0xC1, 0x08]); // movsd xmm0,[rcx+rax*8+8]
                    }
                    asm.b(&[0xF2, 0x0F, 0x11, 0x85]); // movsd [var],xmm0
                    asm.b(&named_disp(var)?);
                    asm.b(&[0x48, 0xFF, 0xC0]); // inc rax
                    asm.b(&[0xF2, 0x48, 0x0F, 0x2A, 0xC8]); // cvtsi2sd xmm1,rax
                    asm.b(&[0xF2, 0x0F, 0x11, 0x8D]); // movsd [idx],xmm1
                    asm.b(&slot_disp(*idx));
                    asm.b(&[0xE9]); // jmp body
                    jump_fixups.push((asm.rel32_placeholder(), *body));
                }
                Instruction::IndexRead => {
                    let st = types.get(&ip)?;
                    let is_rng = st.stack[st.stack.len() - 2] == Ty::Rng;
                    asm.b(&[0xF2, 0x0F, 0x10, 0x04, 0x24]); // movsd xmm0,[rsp]   (index)
                    if is_rng {
                        asm.b(&[0xF2, 0x48, 0x0F, 0x2C, 0xC0]); // cvttsd2si rax,xmm0
                        asm.b(&[0xF2, 0x48, 0x0F, 0x2A, 0xC8]); // cvtsi2sd xmm1,rax
                        asm.b(&[0x66, 0x0F, 0x2E, 0xC1]); // ucomisd xmm0,xmm1
                        asm.b(&[0x0F, 0x85]);
                        bail_fixups.push(asm.rel32_placeholder());
                        asm.b(&[0x0F, 0x8A]);
                        bail_fixups.push(asm.rel32_placeholder());
                        asm.b(&[0x8B, 0x4C, 0x24, 0x08]); // mov ecx,[rsp+8]   (len, zero-extended)
                        asm.b(&[0x48, 0x39, 0xC8]); // cmp rax,rcx
                        asm.b(&[0x0F, 0x83]); // jae bail
                        bail_fixups.push(asm.rel32_placeholder());
                        asm.b(&[0x48, 0x8B, 0x54, 0x24, 0x08]); // mov rdx,[rsp+8]
                        asm.b(&[0x48, 0xC1, 0xFA, 0x20]); // sar rdx,32        (start)
                        asm.b(&[0x48, 0x01, 0xD0]); // add rax,rdx
                        asm.b(&[0xF2, 0x48, 0x0F, 0x2A, 0xC0]); // cvtsi2sd xmm0,rax
                    } else {
                        asm.b(&[0x48, 0x8B, 0x4C, 0x24, 0x08]); // mov rcx,[rsp+8]    (array)
                        emit_index_check(&mut asm, &mut bail_fixups);
                        asm.b(&[0xF2, 0x0F, 0x10, 0x44, 0xC1, 0x08]); // movsd xmm0,[rcx+rax*8+8]
                    }
                    asm.b(&[0x48, 0x83, 0xC4, 0x08]); // add rsp,8
                    asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]); // movsd [rsp],xmm0
                }
                Instruction::IndexWrite(name) => {
                    asm.b(&[0xF2, 0x0F, 0x10, 0x14, 0x24]); // movsd xmm2,[rsp]    (value)
                    asm.b(&[0xF2, 0x0F, 0x10, 0x44, 0x24, 0x08]); // movsd xmm0,[rsp+8] (index)
                    asm.b(&[0x48, 0x8B, 0x8D]); // mov rcx,[rbp+disp32] (array)
                    match arg_index(name, arity) {
                        Some(i) => asm.b(&arg_disp(i)),
                        None => asm.b(&named_disp(name)?),
                    }
                    emit_index_check(&mut asm, &mut bail_fixups);
                    asm.b(&[0xF2, 0x0F, 0x11, 0x54, 0xC1, 0x08]); // movsd [rcx+rax*8+8],xmm2
                    asm.b(&[0x48, 0x83, 0xC4, 0x10]); // add rsp,16
                }
                Instruction::CallBuiltinId(id, argc) => {
                    let id = *id;
                    if id == LEN_BUILTIN_ID {
                        let st = types.get(&ip)?;
                        if *st.stack.last()? == Ty::Rng {
                            asm.b(&[0x8B, 0x04, 0x24]); // mov eax,[rsp]   (len, zero-extended)
                        } else {
                            asm.b(&[0x48, 0x8B, 0x04, 0x24]); // mov rax,[rsp]
                            asm.b(&[0x48, 0x8B, 0x00]); // mov rax,[rax]  (length)
                        }
                        asm.b(&[0xF2, 0x48, 0x0F, 0x2A, 0xC0]); // cvtsi2sd xmm0,rax
                        asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]); // movsd [rsp],xmm0
                    } else if id == B_SQRT {
                        asm.b(&[0xF2, 0x0F, 0x51, 0x04, 0x24]); // sqrtsd xmm0,[rsp]
                        asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]);
                    } else if id == B_ABS {
                        asm.b(&[0x48, 0xB8]);
                        asm.b(&0x7FFF_FFFF_FFFF_FFFFu64.to_le_bytes());
                        asm.b(&[0x48, 0x21, 0x04, 0x24]); // and [rsp],rax
                    } else if id == B_FLOOR || id == B_CEIL {
                        let mode = if id == B_FLOOR { 0x09 } else { 0x0A };
                        asm.b(&[0x66, 0x0F, 0x3A, 0x0B, 0x04, 0x24, mode]); // roundsd xmm0,[rsp],mode
                        asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]);
                    } else if id == B_MIN || id == B_MAX {
                        // res = +/-inf; for each operand: take it only if strictly better (NaN never wins)
                        let start = if id == B_MIN { f64::INFINITY } else { f64::NEG_INFINITY };
                        asm.b(&[0x48, 0xB8]);
                        asm.b(&start.to_bits().to_le_bytes());
                        asm.b(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]); // movq xmm0,rax   (res)
                        for off in [8u8, 0u8] {
                            if off == 8 {
                                asm.b(&[0xF2, 0x0F, 0x10, 0x4C, 0x24, 0x08]); // movsd xmm1,[rsp+8]
                            } else {
                                asm.b(&[0xF2, 0x0F, 0x10, 0x0C, 0x24]); // movsd xmm1,[rsp]
                            }
                            if id == B_MIN {
                                asm.b(&[0x66, 0x0F, 0x28, 0xD1]); // movapd xmm2,xmm1
                                asm.b(&[0xF2, 0x0F, 0xC2, 0xD0, 0x01]); // cmpltsd xmm2,xmm0   (t < res)
                            } else {
                                asm.b(&[0x66, 0x0F, 0x28, 0xD0]); // movapd xmm2,xmm0
                                asm.b(&[0xF2, 0x0F, 0xC2, 0xD1, 0x01]); // cmpltsd xmm2,xmm1   (res < t)
                            }
                            asm.b(&[0x66, 0x0F, 0x54, 0xCA]); // andpd xmm1,xmm2
                            asm.b(&[0x66, 0x0F, 0x55, 0xD0]); // andnpd xmm2,xmm0
                            asm.b(&[0x66, 0x0F, 0x56, 0xCA]); // orpd xmm1,xmm2
                            asm.b(&[0x66, 0x0F, 0x28, 0xC1]); // movapd xmm0,xmm1
                        }
                        asm.b(&[0x48, 0x83, 0xC4, 0x08]); // add rsp,8
                        asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]); // movsd [rsp],xmm0
                    } else if id == B_RANGE {
                        let two = *argc == 2;
                        asm.b(&[0xF2, 0x0F, 0x10, 0x0C, 0x24]); // movsd xmm1,[rsp]     (end / n)
                        if two {
                            asm.b(&[0xF2, 0x0F, 0x10, 0x44, 0x24, 0x08]); // movsd xmm0,[rsp+8] (start)
                            asm.b(&[0xF2, 0x48, 0x0F, 0x2C, 0xC0]); // cvttsd2si rax,xmm0
                        } else {
                            asm.b(&[0x31, 0xC0]); // xor eax,eax            (start = 0)
                        }
                        asm.b(&[0xF2, 0x48, 0x0F, 0x2C, 0xD1]); // cvttsd2si rdx,xmm1
                        asm.b(&[0x48, 0x63, 0xC8, 0x48, 0x39, 0xC1]); // movsxd rcx,eax; cmp rcx,rax
                        asm.b(&[0x0F, 0x85]); // jne bail (start not an i32)
                        bail_fixups.push(asm.rel32_placeholder());
                        asm.b(&[0x48, 0x63, 0xCA, 0x48, 0x39, 0xD1]); // movsxd rcx,edx; cmp rcx,rdx
                        asm.b(&[0x0F, 0x85]); // jne bail (end not an i32)
                        bail_fixups.push(asm.rel32_placeholder());
                        asm.b(&[0x48, 0x89, 0xD1, 0x48, 0x29, 0xC1]); // mov rcx,rdx; sub rcx,rax   (len)
                        asm.b(&[0x31, 0xD2, 0x48, 0x85, 0xC9, 0x48, 0x0F, 0x48, 0xCA]); // xor edx,edx; test rcx,rcx; cmovs rcx,rdx
                        asm.b(&[0x48, 0xBA, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00]); // mov rdx,0xFFFFFFFF
                        asm.b(&[0x48, 0x39, 0xD1]); // cmp rcx,rdx
                        asm.b(&[0x0F, 0x87]); // ja bail (too long to pack)
                        bail_fixups.push(asm.rel32_placeholder());
                        asm.b(&[0x48, 0xC1, 0xE0, 0x20, 0x48, 0x09, 0xC8]); // shl rax,32; or rax,rcx
                        if two {
                            asm.b(&[0x48, 0x83, 0xC4, 0x08]); // add rsp,8
                        }
                        asm.b(&[0x48, 0x89, 0x04, 0x24]); // mov [rsp],rax
                    } else {
                        return None;
                    }
                }

                Instruction::EnterScope | Instruction::ExitScope => {}
                Instruction::Jump(t) => {
                    asm.b(&[0xE9]);
                    jump_fixups.push((asm.rel32_placeholder(), *t));
                }
                Instruction::JumpIfFalse(t) => {
                    asm.b(&[0x58, 0x48, 0x85, 0xC0]); // pop rax; test rax,rax
                    asm.b(&[0x0F, 0x84]); // jz
                    jump_fixups.push((asm.rel32_placeholder(), *t));
                }
                Instruction::Ret => {
                    asm.b(&[0xF2, 0x0F, 0x10, 0x04, 0x24]); // movsd xmm0,[rsp]
                    asm.state_to_rax(state_ptr);
                    asm.b(&[0x48, 0xFF, 0x48, 0x08]); // dec qword [rax+8]
                    asm.b(&[0xC9, 0xC3]); // leave; ret
                }
                Instruction::Call { target, arg_count } => {
                    let mut cmask = 0u8;
                    // callee key: recompute from analysis-time stack types
                    let st = types.get(&ip)?;
                    let l = st.stack.len();
                    for (k, t) in st.stack[l - arg_count..].iter().enumerate() {
                        if *t == Ty::Arr {
                            cmask |= 1 << k;
                        }
                    }
                    asm.b(&[0xE8]);
                    call_fixups.push((asm.rel32_placeholder(), (*target, cmask)));
                    asm.b(&[0x48, 0x83, 0xC4, (8 * *arg_count) as u8]); // add rsp,8*argc (pop args)
                    asm.state_to_rax(state_ptr);
                    asm.b(&[0x48, 0x83, 0x38, 0x00]); // cmp qword [rax],0
                    asm.b(&[0x0F, 0x85]); // jne bail
                    bail_fixups.push(asm.rel32_placeholder());
                    asm.b(&[0x48, 0x83, 0xEC, 0x08]); // sub rsp,8
                    asm.b(&[0xF2, 0x0F, 0x11, 0x04, 0x24]); // movsd [rsp],xmm0
                }
                _ => return None,
            }
        }

        for (pos, tip) in jump_fixups {
            let off = *labels.get(&tip)?;
            asm.patch(pos, off);
        }

        // bail stub: flag = 1; leave; ret
        let bail = asm.buf.len();
        asm.state_to_rax(state_ptr);
        asm.b(&[0x48, 0xC7, 0x00, 0x01, 0x00, 0x00, 0x00]); // mov qword [rax],1
        asm.b(&[0xC9, 0xC3]);
        for pos in bail_fixups {
            asm.patch(pos, bail);
        }
    }

    // Entry thunk: extern "C" fn(*const u64) -> f64. Pushes the args block, calls root, returns.
    let thunk = asm.buf.len();
    let root_arity = arities[&root.0];
    asm.b(&[0x55, 0x48, 0x89, 0xE5]); // push rbp; mov rbp,rsp
    for i in 0..root_arity {
        #[cfg(windows)]
        asm.b(&[0xFF, 0x71, (8 * i) as u8]); // push qword [rcx+8*i]
        #[cfg(not(windows))]
        asm.b(&[0xFF, 0x77, (8 * i) as u8]); // push qword [rdi+8*i]
    }
    asm.b(&[0xE8]);
    call_fixups.push((asm.rel32_placeholder(), root));
    asm.b(&[0xC9, 0xC3]); // leave; ret

    for (pos, key) in call_fixups {
        let off = *entries.get(&key)?;
        asm.patch(pos, off);
    }
    Some((asm.buf, thunk, any_writes, all_names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile_killer_default;

    fn nums(v: &[f64]) -> Vec<Value> {
        v.iter().map(|x| Value::Number(*x)).collect()
    }

    fn arr(v: &[f64]) -> Value {
        Value::Array(SharedArray::new(nums(v)))
    }

    fn arr_vals(a: &Value) -> Vec<f64> {
        match a {
            Value::Array(s) => s.to_vec().iter().map(|v| if let Value::Number(n) = v { *n } else { f64::NAN }).collect(),
            _ => vec![],
        }
    }

    /// Warm up past the threshold with `warm` args, then run once with `args`.
    fn run_native(src: &str, target: usize, warm: &[Value], args: &[Value]) -> Option<f64> {
        let p = compile_killer_default(src).unwrap();
        let mut j = FnJit::new();
        for _ in 0..CALL_THRESHOLD {
            let w: Vec<Value> = warm.iter().map(|v| match v { Value::Array(a) => Value::Array(SharedArray::new(a.to_vec())), o => o.clone() }).collect();
            j.try_call(&p.instructions, &p.function_arities, target, &w, &|_| false);
        }
        j.try_call(&p.instructions, &p.function_arities, target, args, &|_| false)
    }

    fn run_nums(src: &str, target: usize, args: &[f64]) -> Option<f64> {
        run_native(src, target, &nums(args), &nums(args))
    }

    const FIB: &str = "fn fib(n) {\n  if n <= 1 {\n    return n\n  }\n  return fib(n - 1) + fib(n - 2)\n}\nprintln(fib(1))\n";

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn fib_matches_known_values() {
        for (n, want) in [(0.0, 0.0), (1.0, 1.0), (10.0, 55.0), (20.0, 6765.0), (25.0, 75025.0)] {
            assert_eq!(run_nums(FIB, 1, &[n]), Some(want), "fib({})", n);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn arithmetic_and_comparisons() {
        let src = "fn f(a, b) {\n  if a > b {\n    return a - b\n  }\n  if a >= 5 {\n    return a * b\n  }\n  return a / b + 0.5\n}\nprintln(f(1, 2))\n";
        assert_eq!(run_nums(src, 1, &[9.0, 4.0]), Some(5.0));
        assert_eq!(run_nums(src, 1, &[5.0, 5.0]), Some(25.0));
        assert_eq!(run_nums(src, 1, &[1.0, 2.0]), Some(1.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn division_by_zero_bails_to_interpreter() {
        let src = "fn d(a, b) {\n  return a / b\n}\nprintln(d(1, 2))\n";
        assert_eq!(run_nums(src, 1, &[1.0, 0.0]), None);
        assert_eq!(run_nums(src, 1, &[1.0, 4.0]), Some(0.25));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn runaway_recursion_bails() {
        let src = "fn r(n) {\n  return r(n + 1)\n}\nprintln(1)\n";
        assert_eq!(run_nums(src, 1, &[0.0]), None);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn loops_and_locals() {
        let src = "fn sumsq(n) {\n  total = 0\n  i = 1\n  while i <= n {\n    total = total + i * i\n    i = i + 1\n  }\n  return total\n}\nprintln(sumsq(10))\n";
        assert_eq!(run_nums(src, 1, &[10.0]), Some(385.0));
        assert_eq!(run_nums(src, 1, &[0.0]), Some(0.0));
        assert_eq!(run_nums(src, 1, &[1000.0]), Some(333833500.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn mod_matches_rust_fmod() {
        let src = "fn m(a, b) {\n  return a % b\n}\nprintln(m(7, 3))\n";
        for (a, b) in [(7.0, 3.0), (-7.0, 3.0), (7.0, -3.0), (5.5, 2.0), (1e18, 7.0), (0.3, 0.1), (3.0, f64::INFINITY), (-0.0, 5.0)] {
            let got = run_nums(src, 1, &[a, b]).unwrap();
            let want = a % b;
            assert!(got == want || (got.is_nan() && want.is_nan()), "{} % {} = {} want {}", a, b, got, want);
        }
        assert!(run_nums(src, 1, &[f64::INFINITY, 3.0]).unwrap().is_nan());
        assert_eq!(run_nums(src, 1, &[1.0, 0.0]), None);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn nested_loops_and_logic() {
        let src = "fn collatz(n) {\n  steps = 0\n  x = n\n  while x > 1 {\n    if x % 2 == 0 {\n      x = x / 2\n    } else {\n      x = 3 * x + 1\n    }\n    steps = steps + 1\n  }\n  return steps\n}\nprintln(collatz(6))\n";
        assert_eq!(run_nums(src, 1, &[6.0]), Some(8.0));
        assert_eq!(run_nums(src, 1, &[27.0]), Some(111.0));
    }

    const BSORT: &str = "fn bsort(a) {\n  n = len(a)\n  i = 0\n  while i < n {\n    j = 0\n    while j < n - i - 1 {\n      if a[j] > a[j + 1] {\n        t = a[j]\n        a[j] = a[j + 1]\n        a[j + 1] = t\n      }\n      j = j + 1\n    }\n    i = i + 1\n  }\n  return n\n}\nprintln(1)\n";

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn array_sort_writes_back() {
        let warm = arr(&[3.0, 1.0, 2.0]);
        let data = arr(&[5.0, -2.0, 9.0, 1.0, 1.0, 0.5, 7.0]);
        let r = run_native(BSORT, 1, &[warm], &[data.clone()]);
        assert_eq!(r, Some(7.0));
        assert_eq!(arr_vals(&data), vec![-2.0, 0.5, 1.0, 1.0, 5.0, 7.0, 9.0]);
        let empty = arr(&[]);
        assert_eq!(run_native(BSORT, 1, &[arr(&[1.0])], &[empty]), Some(0.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn array_sum_with_helper_calls() {
        let src = "fn at(a, i) {\n  return a[i]\n}\nfn total(a) {\n  s = 0\n  i = 0\n  while i < len(a) {\n    s = s + at(a, i)\n    i = i + 1\n  }\n  return s\n}\nprintln(1)\n";
        // `total` is the second function in the program
        let p = compile_killer_default(src).unwrap();
        let total = *p.function_arities.keys().max().unwrap();
        let data = arr(&[1.5, 2.5, 3.0, 4.0]);
        assert_eq!(run_native(src, total, &[arr(&[1.0])], &[data]), Some(11.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn bad_index_and_odd_arrays_bail() {
        let src = "fn g(a, i) {\n  return a[i]\n}\nprintln(1)\n";
        let warm = [arr(&[1.0, 2.0]), Value::Number(0.0)];
        let ok = [arr(&[1.0, 2.0, 3.0]), Value::Number(2.0)];
        assert_eq!(run_native(src, 1, &warm, &ok), Some(3.0));
        for bad in [3.0, -1.0, 0.5, f64::NAN, 1e300] {
            assert_eq!(run_native(src, 1, &warm, &[arr(&[1.0, 2.0, 3.0]), Value::Number(bad)]), None, "index {}", bad);
        }
        let mixed = Value::Array(SharedArray::new(vec![Value::Number(1.0), Value::Str("x".into())]));
        assert_eq!(run_native(src, 1, &warm, &[mixed, Value::Number(0.0)]), None);
        let a = arr(&[1.0, 2.0]);
        let src2 = "fn h(a, b) {\n  return a[0] + b[0]\n}\nprintln(1)\n";
        assert_eq!(run_native(src2, 1, &[a.clone(), arr(&[9.0])], &[a.clone(), a.clone()]), None);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn conditionally_assigned_slot_is_never_read_natively() {
        // `t` is only assigned when n > 0, so reading it afterwards must stay in the interpreter.
        let src = "fn c(n) {
  if n > 0 {
    t = 5
  }
  return t
}
println(1)
";
        assert_eq!(run_nums(src, 1, &[1.0]), None);
        assert_eq!(run_nums(src, 1, &[-1.0]), None);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn loop_function_compiles_on_first_call() {
        let src = "fn s(n) {
  t = 0
  i = 0
  while i < n {
    t = t + i
    i = i + 1
  }
  return t
}
println(1)
";
        let p = compile_killer_default(src).unwrap();
        let mut j = FnJit::new();
        assert_eq!(j.try_call(&p.instructions, &p.function_arities, 1, &nums(&[100.0]), &|_| false), Some(4950.0));
        assert_eq!(j.native_calls, 1);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn for_loops_over_arrays_and_ranges() {
        let sum_arr = "fn s(a) {\n  t = 0\n  for x in a {\n    t = t + x\n  }\n  return t\n}\nprintln(1)\n";
        assert_eq!(run_native(sum_arr, 1, &[arr(&[1.0])], &[arr(&[1.5, 2.5, 3.0])]), Some(7.0));
        assert_eq!(run_native(sum_arr, 1, &[arr(&[1.0])], &[arr(&[])]), Some(0.0));

        let sum_rng = "fn r(n) {\n  t = 0\n  for i in range(n) {\n    t = t + i\n  }\n  return t\n}\nprintln(1)\n";
        assert_eq!(run_nums(sum_rng, 1, &[10.0]), Some(45.0));
        assert_eq!(run_nums(sum_rng, 1, &[0.0]), Some(0.0));
        assert_eq!(run_nums(sum_rng, 1, &[-4.0]), Some(0.0));
        assert_eq!(run_nums(sum_rng, 1, &[7.9]), Some(21.0)); // range truncates like the interpreter

        let two = "fn r(a, b) {\n  t = 0\n  for i in range(a, b) {\n    t = t + i\n  }\n  return t\n}\nprintln(1)\n";
        assert_eq!(run_nums(two, 1, &[2.0, 6.0]), Some(14.0));
        assert_eq!(run_nums(two, 1, &[-3.0, 3.0]), Some(-3.0));
        assert_eq!(run_nums(two, 1, &[5.0, 5.0]), Some(0.0));
        assert_eq!(run_nums(two, 1, &[9.0, 2.0]), Some(0.0));
        assert_eq!(run_nums(two, 1, &[0.0, 1e12]), None); // not packable: interpreter decides
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn nested_for_loops_with_array_writes() {
        let src = "fn fill(a) {\n  n = len(a)\n  for i in range(n) {\n    for j in range(i) {\n      a[i] = a[i] + j\n    }\n  }\n  return n\n}\nprintln(1)\n";
        let data = arr(&[1.0, 1.0, 1.0, 1.0, 1.0]);
        assert_eq!(run_native(src, 1, &[arr(&[1.0, 1.0])], &[data.clone()]), Some(5.0));
        assert_eq!(arr_vals(&data), vec![1.0, 1.0, 2.0, 4.0, 7.0]);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn math_builtins_match_interpreter_semantics() {
        fn interp_min(a: f64, b: f64) -> f64 { let mut m = f64::INFINITY; for n in [a, b] { if n < m { m = n; } } m }
        fn interp_max(a: f64, b: f64) -> f64 { let mut m = f64::NEG_INFINITY; for n in [a, b] { if n > m { m = n; } } m }
        let vals = [0.0, -0.0, 1.5, -2.5, 3.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e300, -7.25];
        let mn = "fn f(a, b) {\n  return min(a, b)\n}\nprintln(1)\n";
        let mx = "fn f(a, b) {\n  return max(a, b)\n}\nprintln(1)\n";
        let same = |got: f64, want: f64| got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan());
        for a in vals {
            for b in vals {
                let g = run_nums(mn, 1, &[a, b]).unwrap();
                assert!(same(g, interp_min(a, b)), "min({}, {}) = {}", a, b, g);
                let g = run_nums(mx, 1, &[a, b]).unwrap();
                assert!(same(g, interp_max(a, b)), "max({}, {}) = {}", a, b, g);
            }
        }
        for (src, f) in [
            ("fn f(a) {\n  return sqrt(a)\n}\nprintln(1)\n", f64::sqrt as fn(f64) -> f64),
            ("fn f(a) {\n  return abs(a)\n}\nprintln(1)\n", f64::abs as fn(f64) -> f64),
            ("fn f(a) {\n  return floor(a)\n}\nprintln(1)\n", f64::floor as fn(f64) -> f64),
            ("fn f(a) {\n  return ceil(a)\n}\nprintln(1)\n", f64::ceil as fn(f64) -> f64),
        ] {
            for a in vals {
                let g = run_nums(src, 1, &[a]).unwrap();
                assert!(same(g, f(a)), "{} -> {} want {}", a, g, f(a));
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn for_variable_is_local_even_if_outer_variable_exists() {
        let src = "fn r(n) {
  t = 0
  for i in range(n) {
    t = t + i
  }
  return t
}
println(1)
";
        let p = compile_killer_default(src).unwrap();
        let mut j = FnJit::new();
        // `for` binds its variable with StoreLocal, so a global `i` does not matter
        assert_eq!(j.try_call(&p.instructions, &p.function_arities, 1, &nums(&[5.0]), &|n| n == "i"), Some(10.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn plain_store_to_an_existing_outer_variable_stays_in_interpreter() {
        // g = 1 via plain Store: in the interpreter this would update an existing outer `g`
        let instrs = vec![
            Instruction::ConstNum(1.0),
            Instruction::Store("g".to_string()),
            Instruction::Load("g".to_string()),
            Instruction::Ret,
        ];
        let arities: HashMap<usize, usize> = [(0usize, 0usize)].into_iter().collect();
        let mut j = FnJit::new();
        for _ in 0..(CALL_THRESHOLD + 5) {
            assert_eq!(j.try_call(&instrs, &arities, 0, &[], &|n| n == "g"), None);
        }
        let mut j2 = FnJit::new();
        let mut last = None;
        for _ in 0..(CALL_THRESHOLD + 5) {
            last = j2.try_call(&instrs, &arities, 0, &[], &|_| false);
        }
        assert_eq!(last, Some(1.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn variable_scoped_to_a_block_is_not_readable_after_it() {
        // `y` only exists inside the if-block scope; reading it afterwards is an interpreter error
        let src = "fn c(n) {\n  if n > 0 {\n    y = 5\n  }\n  return y\n}\nprintln(1)\n";
        assert_eq!(run_nums(src, 1, &[1.0]), None);
    }

    #[test]
    fn impure_functions_are_rejected() {
        let src = "fn p(n) {\n  println(n)\n  return n\n}\np(1)\n";
        let p = compile_killer_default(src).unwrap();
        let mut j = FnJit::new();
        for _ in 0..(CALL_THRESHOLD + 2) {
            assert_eq!(j.try_call(&p.instructions, &p.function_arities, 1, &nums(&[1.0]), &|_| false), None);
        }
    }
}
