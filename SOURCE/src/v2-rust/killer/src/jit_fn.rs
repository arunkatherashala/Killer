#![allow(unsafe_code)]
//! Function-level JIT for *pure numeric* Killer functions (x86-64).
//!
//! A function qualifies when every reachable instruction is one of: number constants, `argN`
//! loads, `+ - * /`, comparisons, `Jump`/`JumpIfFalse`, scope no-ops, `Ret`, and calls to other
//! qualifying functions. Such code has no side effects, so on any condition native code cannot
//! handle (division by zero, recursion too deep) it sets a bail flag and the interpreter simply
//! re-runs the call.
//!
//! Native convention: up to 4 `f64` args in xmm0..xmm3, result in xmm0. The operand stack lives
//! on the machine stack; booleans are SSE compare masks (all-ones / zero).

use crate::bytecode::Instruction;
use std::collections::{HashMap, HashSet};

#[cfg(target_arch = "x86_64")]
use crate::jit_x86::ExecPage;

const CALL_THRESHOLD: u32 = 30;
const MAX_ARGS: usize = 4;
const MAX_DEPTH: i32 = 3000;
const MAX_FN_INSTRS: usize = 4096;
const MAX_SLOTS: usize = 64;
const LOOP_BUDGET: i64 = 4_000_000_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ty {
    Num,
    Bool,
}

pub struct FnJit {
    counts: HashMap<usize, u32>,
    failed: HashSet<usize>,
    #[cfg(target_arch = "x86_64")]
    compiled: HashMap<usize, (usize, usize)>, // target -> (entry address, arity)
    #[cfg(target_arch = "x86_64")]
    pages: Vec<ExecPage>,
    // [0] = bail flag, [1] = recursion depth, [2] = remaining loop iterations. Heap-allocated so native code can hold the address.
    state: Box<[i64; 3]>,
    pub native_calls: u64,
}

impl FnJit {
    pub fn new() -> Self {
        FnJit {
            counts: HashMap::new(),
            failed: HashSet::new(),
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
        args: &[f64],
    ) -> Option<f64> {
        #[cfg(not(target_arch = "x86_64"))]
        {
            let _ = (instrs, arities, target, args);
            return None;
        }
        #[cfg(target_arch = "x86_64")]
        {
            if args.len() > MAX_ARGS || self.failed.contains(&target) {
                return None;
            }
            if !self.compiled.contains_key(&target) {
                let c = self.counts.entry(target).or_insert(0);
                *c += 1;
                if *c < CALL_THRESHOLD {
                    return None;
                }
                if arities.get(&target).copied() != Some(args.len()) || !self.compile(instrs, arities, target) {
                    self.failed.insert(target);
                    return None;
                }
            }
            let (entry, arity) = *self.compiled.get(&target)?;
            if arity != args.len() {
                return None;
            }
            self.state[0] = 0;
            self.state[1] = 0;
            self.state[2] = LOOP_BUDGET;
            let r = unsafe {
                let mut a = [0f64; MAX_ARGS];
                a[..args.len()].copy_from_slice(args);
                match arity {
                    0 => std::mem::transmute::<usize, extern "C" fn() -> f64>(entry)(),
                    1 => std::mem::transmute::<usize, extern "C" fn(f64) -> f64>(entry)(a[0]),
                    2 => std::mem::transmute::<usize, extern "C" fn(f64, f64) -> f64>(entry)(a[0], a[1]),
                    3 => std::mem::transmute::<usize, extern "C" fn(f64, f64, f64) -> f64>(entry)(a[0], a[1], a[2]),
                    _ => std::mem::transmute::<usize, extern "C" fn(f64, f64, f64, f64) -> f64>(entry)(a[0], a[1], a[2], a[3]),
                }
            };
            if self.state[0] != 0 {
                self.state[0] = 0;
                self.state[1] = 0;
                return None;
            }
            self.native_calls += 1;
            Some(r)
        }
    }

    #[cfg(target_arch = "x86_64")]
    fn compile(&mut self, instrs: &[Instruction], arities: &HashMap<usize, usize>, root: usize) -> bool {
        let state_ptr = self.state.as_ptr() as usize;
        let Some((code, entries)) = compile_group(instrs, arities, root, state_ptr) else { return false };
        let size = (code.len() + 4095) & !4095;
        let Some(mut page) = ExecPage::alloc(size) else { return false };
        unsafe { page.write(&code) };
        let base = page.base() as usize;
        for (t, off) in &entries {
            let arity = arities.get(t).copied().unwrap_or(0);
            self.compiled.insert(*t, (base + off, arity));
        }
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
    slots: Vec<bool>, // definitely assigned a number
}

struct Analysis {
    at: HashMap<usize, State>,
    callees: Vec<usize>,
    nslots: usize,
}

/// Abstract-interpret one function; returns the stack type at every reachable instruction, plus
/// the set of callee entry points. `None` if anything is unsupported.
fn analyze(
    instrs: &[Instruction],
    arities: &HashMap<usize, usize>,
    entry: usize,
) -> Option<Analysis> {
    let arity = *arities.get(&entry)?;
    if arity > MAX_ARGS {
        return None;
    }
    let n = instrs.len();
    let mut at: HashMap<usize, State> = HashMap::new();
    let mut work: Vec<usize> = vec![entry];
    let mut callees = Vec::new();
    let mut nslots = 0usize;
    at.insert(entry, State { stack: Vec::new(), slots: Vec::new() });

    fn flow(at: &mut HashMap<usize, State>, work: &mut Vec<usize>, ip: usize, st: &State, n: usize) -> bool {
        if ip >= n {
            return false;
        }
        match at.get(&ip) {
            Some(prev) => prev == st,
            None => {
                at.insert(ip, st.clone());
                work.push(ip);
                true
            }
        }
    }
    fn slot_ok(st: &State, s: usize) -> bool {
        s < MAX_SLOTS && st.slots.get(s).copied().unwrap_or(false)
    }
    fn top2_num(st: &State) -> bool {
        let l = st.stack.len();
        l >= 2 && st.stack[l - 1] == Ty::Num && st.stack[l - 2] == Ty::Num
    }

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
                    arg_index(name, arity)?;
                    st.stack.push(Ty::Num);
                }
                Instruction::LoadSlot(s) => {
                    if !slot_ok(&st, *s as usize) {
                        return None;
                    }
                    st.stack.push(Ty::Num);
                }
                Instruction::StoreSlot(s) => {
                    let s = *s as usize;
                    if s >= MAX_SLOTS || st.stack.pop()? != Ty::Num {
                        return None;
                    }
                    if st.slots.len() <= s {
                        st.slots.resize(s + 1, false);
                    }
                    st.slots[s] = true;
                    nslots = nslots.max(s + 1);
                }
                Instruction::AddSlotConst(s, _) | Instruction::SubSlotConst(s, _) => {
                    if !slot_ok(&st, *s as usize) {
                        return None;
                    }
                }
                Instruction::LtSlotConst(s, _) | Instruction::GtSlotConst(s, _)
                | Instruction::GeSlotConst(s, _) | Instruction::LeSlotConst(s, _)
                | Instruction::EqSlotConst(s, _) => {
                    if !slot_ok(&st, *s as usize) {
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
                Instruction::EnterScope | Instruction::ExitScope => {}
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
                    if arities.get(target).copied() != Some(*arg_count) || *arg_count > MAX_ARGS {
                        return None;
                    }
                    let l = st.stack.len();
                    if l < *arg_count || st.stack[l - arg_count..].iter().any(|t| *t != Ty::Num) {
                        return None;
                    }
                    st.stack.truncate(l - arg_count);
                    st.stack.push(Ty::Num);
                    if !callees.contains(target) {
                        callees.push(*target);
                    }
                }
                _ => return None,
            }
            ip += 1;
            match at.get(&ip) {
                Some(prev) => {
                    if *prev != st {
                        return None;
                    }
                    break;
                }
                None => {
                    if ip >= n {
                        return None;
                    }
                    at.insert(ip, st.clone());
                }
            }
        }
    }
    Some(Analysis { at, callees, nslots })
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
}

/// Compile `root` and every function it calls. Returns code and per-function entry offsets.
#[cfg(target_arch = "x86_64")]
fn compile_group(
    instrs: &[Instruction],
    arities: &HashMap<usize, usize>,
    root: usize,
    state_ptr: usize,
) -> Option<(Vec<u8>, HashMap<usize, usize>)> {
    let mut asm = Asm { buf: Vec::new() };
    let mut entries: HashMap<usize, usize> = HashMap::new();
    let mut call_fixups: Vec<(usize, usize)> = Vec::new(); // (patch pos, callee target)
    let mut pending = vec![root];
    let mut seen: HashSet<usize> = HashSet::new();
    seen.insert(root);

    while let Some(f) = pending.pop() {
        let an = analyze(instrs, arities, f)?;
        let types = an.at;
        for c in an.callees {
            if seen.insert(c) {
                pending.push(c);
            }
        }
        let arity = arities[&f];
        entries.insert(f, asm.buf.len());

        // prologue
        let frame = ((32 + 8 * an.nslots as i32) + 15) & !15;
        asm.b(&[0x55, 0x48, 0x89, 0xE5, 0x48, 0x81, 0xEC]); // push rbp; mov rbp,rsp; sub rsp,imm32
        asm.b(&frame.to_le_bytes());
        for i in 0..arity {
            asm.b(&[0xF2, 0x0F, 0x11, 0x45 | ((i as u8) << 3), (-8 * (i as i32 + 1)) as i8 as u8]);
        }
        asm.b(&[0x48, 0xB8]); // mov rax, state
        asm.b(&(state_ptr as u64).to_le_bytes());
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
        let slot_disp = |s: u16| -> [u8; 4] { (-8 * (MAX_ARGS as i32 + 1 + s as i32)).to_le_bytes() };

        for ip in ips {
            labels.insert(ip, asm.buf.len());
            if headers.contains(&ip) {
                asm.b(&[0x48, 0xB8]);
                asm.b(&(state_ptr as u64).to_le_bytes());
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
                Instruction::LoadSlot(s) => {
                    asm.b(&[0xFF, 0xB5]); // push qword [rbp+disp32]
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
                Instruction::And => {
                    asm.b(&[0x58, 0x48, 0x21, 0x04, 0x24]); // pop rax; and [rsp],rax
                }
                Instruction::Or => {
                    asm.b(&[0x58, 0x48, 0x09, 0x04, 0x24]); // pop rax; or [rsp],rax
                }
                Instruction::Not => {
                    asm.b(&[0x48, 0x83, 0x34, 0x24, 0xFF]); // xor qword [rsp],-1
                }
                Instruction::Pop => {
                    asm.b(&[0x48, 0x83, 0xC4, 0x08]);
                }

                Instruction::Load(name) => {
                    let i = arg_index(name, arity)?;
                    asm.b(&[0xFF, 0xB5]); // push qword [rbp+disp32]
                    asm.b(&(-8 * (i as i32 + 1)).to_le_bytes());
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
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&(state_ptr as u64).to_le_bytes());
                    asm.b(&[0x48, 0xFF, 0x48, 0x08]); // dec qword [rax+8]
                    asm.b(&[0xC9, 0xC3]); // leave; ret
                }
                Instruction::Call { target, arg_count } => {
                    for i in (0..*arg_count).rev() {
                        asm.b(&[0xF2, 0x0F, 0x10, 0x04 | ((i as u8) << 3), 0x24]); // movsd xmm_i,[rsp]
                        asm.b(&[0x48, 0x83, 0xC4, 0x08]);
                    }
                    asm.b(&[0xE8]);
                    call_fixups.push((asm.rel32_placeholder(), *target));
                    asm.b(&[0x48, 0xB8]);
                    asm.b(&(state_ptr as u64).to_le_bytes());
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
        asm.b(&[0x48, 0xB8]);
        asm.b(&(state_ptr as u64).to_le_bytes());
        asm.b(&[0x48, 0xC7, 0x00, 0x01, 0x00, 0x00, 0x00]); // mov qword [rax],1
        asm.b(&[0xC9, 0xC3]);
        for pos in bail_fixups {
            asm.patch(pos, bail);
        }
    }

    for (pos, target) in call_fixups {
        let off = *entries.get(&target)?;
        asm.patch(pos, off);
    }
    Some((asm.buf, entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::compile_killer_default;

    fn run_native(src: &str, target: usize, args: &[f64]) -> Option<f64> {
        let p = compile_killer_default(src).unwrap();
        let mut j = FnJit::new();
        for _ in 0..CALL_THRESHOLD {
            j.try_call(&p.instructions, &p.function_arities, target, args);
        }
        j.try_call(&p.instructions, &p.function_arities, target, args)
    }

    const FIB: &str = "fn fib(n) {\n  if n <= 1 {\n    return n\n  }\n  return fib(n - 1) + fib(n - 2)\n}\nprintln(fib(1))\n";

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn fib_matches_known_values() {
        for (n, want) in [(0.0, 0.0), (1.0, 1.0), (10.0, 55.0), (20.0, 6765.0), (25.0, 75025.0)] {
            assert_eq!(run_native(FIB, 1, &[n]), Some(want), "fib({})", n);
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn arithmetic_and_comparisons() {
        let src = "fn f(a, b) {\n  if a > b {\n    return a - b\n  }\n  if a >= 5 {\n    return a * b\n  }\n  return a / b + 0.5\n}\nprintln(f(1, 2))\n";
        assert_eq!(run_native(src, 1, &[9.0, 4.0]), Some(5.0));
        assert_eq!(run_native(src, 1, &[5.0, 5.0]), Some(25.0));
        assert_eq!(run_native(src, 1, &[1.0, 2.0]), Some(1.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn division_by_zero_bails_to_interpreter() {
        let src = "fn d(a, b) {\n  return a / b\n}\nprintln(d(1, 2))\n";
        assert_eq!(run_native(src, 1, &[1.0, 0.0]), None);
        assert_eq!(run_native(src, 1, &[1.0, 4.0]), Some(0.25));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn runaway_recursion_bails() {
        let src = "fn r(n) {\n  return r(n + 1)\n}\nprintln(1)\n";
        assert_eq!(run_native(src, 1, &[0.0]), None);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn loops_and_locals() {
        let src = "fn sumsq(n) {\n  total = 0\n  i = 1\n  while i <= n {\n    total = total + i * i\n    i = i + 1\n  }\n  return total\n}\nprintln(sumsq(10))\n";
        assert_eq!(run_native(src, 1, &[10.0]), Some(385.0));
        assert_eq!(run_native(src, 1, &[0.0]), Some(0.0));
        assert_eq!(run_native(src, 1, &[1000.0]), Some(333833500.0));
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn mod_matches_rust_fmod() {
        let src = "fn m(a, b) {\n  return a % b\n}\nprintln(m(7, 3))\n";
        for (a, b) in [(7.0, 3.0), (-7.0, 3.0), (7.0, -3.0), (5.5, 2.0), (1e18, 7.0), (0.3, 0.1), (3.0, f64::INFINITY), (-0.0, 5.0)] {
            let got = run_native(src, 1, &[a, b]).unwrap();
            let want = a % b;
            assert!(got == want || (got.is_nan() && want.is_nan()), "{} % {} = {} want {}", a, b, got, want);
        }
        assert!(run_native(src, 1, &[f64::INFINITY, 3.0]).unwrap().is_nan());
        assert_eq!(run_native(src, 1, &[1.0, 0.0]), None);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn nested_loops_and_logic() {
        let src = "fn collatz(n) {\n  steps = 0\n  x = n\n  while x > 1 {\n    if x % 2 == 0 {\n      x = x / 2\n    } else {\n      x = 3 * x + 1\n    }\n    steps = steps + 1\n  }\n  return steps\n}\nprintln(collatz(6))\n";
        assert_eq!(run_native(src, 1, &[6.0]), Some(8.0));
        assert_eq!(run_native(src, 1, &[27.0]), Some(111.0));
    }


    #[test]
    fn impure_functions_are_rejected() {
        let src = "fn p(n) {\n  println(n)\n  return n\n}\np(1)\n";
        let p = compile_killer_default(src).unwrap();
        let mut j = FnJit::new();
        for _ in 0..(CALL_THRESHOLD + 2) {
            assert_eq!(j.try_call(&p.instructions, &p.function_arities, 1, &[1.0]), None);
        }
    }
}
