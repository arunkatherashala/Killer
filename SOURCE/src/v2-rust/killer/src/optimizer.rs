// Bytecode Optimizer: Common subexpression elimination and dead code removal
use crate::bytecode::Instruction;
use std::collections::HashSet;

/// Optimize bytecode by removing dead code and redundant operations
/// This is a conservative optimization that doesn't change semantics
pub fn optimize_bytecode(instructions: &[Instruction]) -> Vec<Instruction> {
    optimize_bytecode_with_map(instructions).0
}

/// Like optimize_bytecode, but also returns the old→new index mapping so
/// callers can remap function_arities or other index-keyed tables.
pub fn optimize_bytecode_with_map(instructions: &[Instruction]) -> (Vec<Instruction>, Vec<usize>) {
    // Phase 1: Remove unreachable code after unconditional returns/halts
    let instructions = remove_unreachable_code(instructions);

    // Phase 2: Eliminate redundant operations (conservative pass)
    let instructions = eliminate_redundant_operations(&instructions);

    // Phase 3: Peephole fusion — combine common slot+const+op patterns
    let (instructions, map) = fuse_slot_patterns(&instructions);

    // Phase 4: Convert CallBuiltin(name) → CallBuiltinId(id) for known builtins
    let instructions = lower_builtin_calls(&instructions);

    (instructions, map)
}

fn lower_builtin_calls(instructions: &[Instruction]) -> Vec<Instruction> {
    instructions
        .iter()
        .map(|instr| match instr {
            Instruction::CallBuiltin(name, arity) => {
                if let Some(id) = crate::builtin::builtin_name_to_id(name) {
                    Instruction::CallBuiltinId(id, *arity)
                } else {
                    instr.clone()
                }
            }
            other => other.clone(),
        })
        .collect()
}

/// Remove code that is unreachable after unconditional jumps or halt
/// NOTE: Does NOT remove code after Ret — function bodies appear before the
/// main-code entry point which is reached via a Jump, so code after Ret is NOT
/// unconditionally unreachable.
fn remove_unreachable_code(instructions: &[Instruction]) -> Vec<Instruction> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < instructions.len() {
        result.push(instructions[i].clone());

        // Only skip truly unreachable code: after Halt (program end)
        // We do NOT skip after Ret because function bodies appear before the
        // main-code block and are reached by Jump instructions.
        if let Instruction::Halt = &instructions[i] {
            break; // everything after Halt is unreachable
        }

        i += 1;
    }

    result
}

/// Conservative **dead store** removal for slot locals: if a slot is never read anywhere
/// in this bytecode chunk (`LoadSlot` / fused slot reads), then `StoreSlot(s)` only discards
/// a stack value — replace with `Pop` to preserve stack semantics.
///
/// Does not yet do full CFG liveness (loops / merges); sound because we only remove stores
/// to slots that have **zero** reads in the whole program fragment.
fn eliminate_redundant_operations(instructions: &[Instruction]) -> Vec<Instruction> {
    let used = slot_read_set(instructions);
    instructions
        .iter()
        .map(|ins| {
            if let Instruction::StoreSlot(s) = ins {
                if !used.contains(s) {
                    return Instruction::Pop;
                }
            }
            ins.clone()
        })
        .collect()
}

fn slot_read_set(instructions: &[Instruction]) -> HashSet<u16> {
    let mut s = HashSet::new();
    for ins in instructions {
        match ins {
            Instruction::LoadSlot(i) => {
                s.insert(*i);
            }
            Instruction::AddSlotConst(i, _)
            | Instruction::SubSlotConst(i, _)
            | Instruction::LtSlotConst(i, _)
            | Instruction::GtSlotConst(i, _)
            | Instruction::GeSlotConst(i, _)
            | Instruction::LeSlotConst(i, _)
            | Instruction::EqSlotConst(i, _) => {
                s.insert(*i);
            }
            Instruction::PrefixStrSlot { slot, .. }
            | Instruction::SlotStrSuffix { slot, .. }
            | Instruction::PrefixSlotSuffix { slot, .. } => {
                s.insert(*slot);
            }
            Instruction::TritAndSlots { s1, s2, .. }
            | Instruction::TritOrSlots { s1, s2, .. }
            | Instruction::TritAddSlots { s1, s2, .. }
            | Instruction::TritMulSlots { s1, s2, .. }
            | Instruction::FuzzyAndSlots { s1, s2, .. }
            | Instruction::FuzzyOrSlots { s1, s2, .. } => {
                s.insert(*s1);
                s.insert(*s2);
            }
            Instruction::TritNotSlot { src, .. } | Instruction::FuzzyNotSlot { src, .. } => {
                s.insert(*src);
            }
            Instruction::IndexWriteSlot(i) => {
                s.insert(*i);
            }
            _ => {}
        }
    }
    s
}

/// Peephole fusion: combines common 4-instruction sequences into fused ops.
///
/// Pattern A — counter increment:
///   LoadSlot(s), ConstNum(n), Add, StoreSlot(s)
///   → AddSlotConst(s, n)
///
/// Pattern B — slot-vs-const comparison:
///   LoadSlot(s), ConstNum(n), Lt
///   → LtSlotConst(s, n)
///
/// IMPORTANT: After fusion the instruction array is shorter, so all jump targets
/// must be remapped from old indices to new indices.
/// Returns (optimized_instructions, old_to_new_map).
fn fuse_slot_patterns(instructions: &[Instruction]) -> (Vec<Instruction>, Vec<usize>) {
    // Build a map: old_index → new_index (accounting for fused instructions).
    let n = instructions.len();
    let mut old_to_new = vec![0usize; n + 1]; // +1 for "one past end" sentinel
    let mut out: Vec<Instruction> = Vec::with_capacity(n);
    let mut new_idx = 0usize;
    let mut i = 0;

    while i < n {
        old_to_new[i] = new_idx;

        // ── Constant folding (numeric / bool / compare) — 3→1, jump-remapped like other fusions ──
        if i + 2 < n {
            if let (Instruction::ConstNum(a), Instruction::ConstNum(b), op) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
            ) {
                let folded: Option<Instruction> = match op {
                    Instruction::Add => Some(Instruction::ConstNum(a + b)),
                    Instruction::Sub => Some(Instruction::ConstNum(a - b)),
                    Instruction::Mul => Some(Instruction::ConstNum(a * b)),
                    Instruction::Div => {
                        if *b != 0.0 {
                            Some(Instruction::ConstNum(a / b))
                        } else {
                            None
                        }
                    }
                    Instruction::IntDiv => {
                        if *b != 0.0 {
                            Some(Instruction::ConstNum((a / b).floor()))
                        } else {
                            None
                        }
                    }
                    Instruction::Mod => Some(Instruction::ConstNum(a % b)),
                    Instruction::Eq => Some(Instruction::ConstBool(a == b)),
                    Instruction::Ne => Some(Instruction::ConstBool(a != b)),
                    Instruction::Lt => Some(Instruction::ConstBool(a < b)),
                    Instruction::Gt => Some(Instruction::ConstBool(a > b)),
                    Instruction::Le => Some(Instruction::ConstBool(a <= b)),
                    Instruction::Ge => Some(Instruction::ConstBool(a >= b)),
                    _ => None,
                };
                if let Some(instr) = folded {
                    let ok = match &instr {
                        Instruction::ConstNum(v) => v.is_finite(),
                        _ => true,
                    };
                    if ok {
                        old_to_new[i + 1] = new_idx;
                        old_to_new[i + 2] = new_idx;
                        out.push(instr);
                        i += 3;
                        new_idx += 1;
                        continue;
                    }
                }
            }
        }
        if i + 2 < n {
            if let (Instruction::ConstBool(a), Instruction::ConstBool(b), op) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
            ) {
                let folded: Option<Instruction> = match op {
                    Instruction::And => Some(Instruction::ConstBool(*a && *b)),
                    Instruction::Or => Some(Instruction::ConstBool(*a || *b)),
                    _ => None,
                };
                if let Some(instr) = folded {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    out.push(instr);
                    i += 3;
                    new_idx += 1;
                    continue;
                }
            }
        }

        // Pattern TR0 (2→1): ConstNum(n), IntToTrit → ConstTrit — matches VM clamp (int_to_trit / trit_from_int)
        if i + 1 < n {
            if let (Instruction::ConstNum(nv), Instruction::IntToTrit) = (
                &instructions[i],
                &instructions[i + 1],
            ) {
                let t = (*nv as i64).clamp(-1, 1) as i8;
                old_to_new[i + 1] = new_idx;
                out.push(Instruction::ConstTrit(t));
                i += 2;
                new_idx += 1;
                continue;
            }
        }
        // Pattern TR1 (2→1): ConstTrit(t), TritToInt → ConstNum — stack-neutral trit→scalar for literals
        if i + 1 < n {
            if let (Instruction::ConstTrit(t), Instruction::TritToInt) = (
                &instructions[i],
                &instructions[i + 1],
            ) {
                old_to_new[i + 1] = new_idx;
                out.push(Instruction::ConstNum(*t as f64));
                i += 2;
                new_idx += 1;
                continue;
            }
        }

        // Pattern A: LoadSlot(s), ConstNum(n), Add, StoreSlot(s) → AddSlotConst(s, n)
        if i + 3 < n {
            if let (
                Instruction::LoadSlot(s1),
                Instruction::ConstNum(nv),
                Instruction::Add,
                Instruction::StoreSlot(s2),
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
                &instructions[i + 3],
            ) {
                if s1 == s2 {
                    // Mark skipped slots as mapping to same new_idx
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    old_to_new[i + 3] = new_idx;
                    out.push(Instruction::AddSlotConst(*s1, *nv));
                    i += 4;
                    new_idx += 1;
                    continue;
                }
            }
        }
        // Pattern A2: LoadSlot(s), ConstNum(n), Sub, StoreSlot(s) -> SubSlotConst(s, n)
        if i + 3 < n {
            if let (
                Instruction::LoadSlot(s1),
                Instruction::ConstNum(nv),
                Instruction::Sub,
                Instruction::StoreSlot(s2),
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
                &instructions[i + 3],
            ) {
                if s1 == s2 {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    old_to_new[i + 3] = new_idx;
                    out.push(Instruction::SubSlotConst(*s1, *nv));
                    i += 4;
                    new_idx += 1;
                    continue;
                }
            }
        }
        // Pattern K1 (6→1): ConstStr(pre) + LoadSlot + CallBuiltin("str",1) + Add + ConstStr(suf) + Add
        // → PrefixSlotSuffix  (K"prefix{slot}suffix")
        if i + 5 < n {
            if let (
                Instruction::ConstStr(pre),
                Instruction::LoadSlot(s),
                Instruction::CallBuiltin(cb1, 1),
                Instruction::Add,
                Instruction::ConstStr(suf),
                Instruction::Add,
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
                &instructions[i + 3],
                &instructions[i + 4],
                &instructions[i + 5],
            ) {
                if cb1 == "str" {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    old_to_new[i + 3] = new_idx;
                    old_to_new[i + 4] = new_idx;
                    old_to_new[i + 5] = new_idx;
                    out.push(Instruction::PrefixSlotSuffix { slot: *s, pre: pre.clone(), suf: suf.clone() });
                    i += 6; new_idx += 1; continue;
                }
            }
        }
        // Pattern K2 (4→1): ConstStr(pre) + LoadSlot + CallBuiltin("str",1) + Add
        // → PrefixStrSlot  (K"prefix{slot}")
        if i + 3 < n {
            if let (
                Instruction::ConstStr(pre),
                Instruction::LoadSlot(s),
                Instruction::CallBuiltin(cb1, 1),
                Instruction::Add,
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
                &instructions[i + 3],
            ) {
                if cb1 == "str" {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    old_to_new[i + 3] = new_idx;
                    out.push(Instruction::PrefixStrSlot { slot: *s, pre: pre.clone() });
                    i += 4; new_idx += 1; continue;
                }
            }
        }
        // Pattern K3 (4→1): LoadSlot + CallBuiltin("str",1) + ConstStr(suf) + Add
        // → SlotStrSuffix  (K"{slot}suffix")
        if i + 3 < n {
            if let (
                Instruction::LoadSlot(s),
                Instruction::CallBuiltin(cb1, 1),
                Instruction::ConstStr(suf),
                Instruction::Add,
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
                &instructions[i + 3],
            ) {
                if cb1 == "str" {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    old_to_new[i + 3] = new_idx;
                    out.push(Instruction::SlotStrSuffix { slot: *s, suf: suf.clone() });
                    i += 4; new_idx += 1; continue;
                }
            }
        }
        // Pattern B family: LoadSlot(s), ConstNum(n), CMP -> CmpSlotConst(s, n)
        if i + 2 < n {
            if let (
                Instruction::LoadSlot(s),
                Instruction::ConstNum(nv),
                cmp_instr,
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
            ) {
                let fused = match cmp_instr {
                    Instruction::Lt => Some(Instruction::LtSlotConst(*s, *nv)),
                    Instruction::Gt => Some(Instruction::GtSlotConst(*s, *nv)),
                    Instruction::Ge => Some(Instruction::GeSlotConst(*s, *nv)),
                    Instruction::Le => Some(Instruction::LeSlotConst(*s, *nv)),
                    Instruction::Eq => Some(Instruction::EqSlotConst(*s, *nv)),
                    _ => None,
                };
                if let Some(instr) = fused {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    out.push(instr);
                    i += 3;
                    new_idx += 1;
                    continue;
                }
            }
        }
        // Pattern T (Trit/Fuzzy 4→1): LoadSlot(s1), LoadSlot(s2), TritAnd/TritOr/FuzzyAnd/FuzzyOr, StoreSlot(dst)
        if i + 3 < n {
            if let (
                Instruction::LoadSlot(s1),
                Instruction::LoadSlot(s2),
                trit_op,
                Instruction::StoreSlot(dst),
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
                &instructions[i + 3],
            ) {
                let fused: Option<Instruction> = match trit_op {
                    Instruction::TritAnd => Some(Instruction::TritAndSlots { dst: *dst, s1: *s1, s2: *s2 }),
                    Instruction::TritOr  => Some(Instruction::TritOrSlots  { dst: *dst, s1: *s1, s2: *s2 }),
                    Instruction::TritAdd => Some(Instruction::TritAddSlots { dst: *dst, s1: *s1, s2: *s2 }),
                    Instruction::TritMul => Some(Instruction::TritMulSlots { dst: *dst, s1: *s1, s2: *s2 }),
                    Instruction::FuzzyAnd => Some(Instruction::FuzzyAndSlots { dst: *dst, s1: *s1, s2: *s2 }),
                    Instruction::FuzzyOr  => Some(Instruction::FuzzyOrSlots  { dst: *dst, s1: *s1, s2: *s2 }),
                    _ => None,
                };
                if let Some(instr) = fused {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    old_to_new[i + 3] = new_idx;
                    out.push(instr);
                    i += 4;
                    new_idx += 1;
                    continue;
                }
            }
        }
        // Pattern T3 (Trit/Fuzzy 3→1): LoadSlot(s), TritNot/FuzzyNot, StoreSlot(dst)
        if i + 2 < n {
            if let (
                Instruction::LoadSlot(src),
                not_op,
                Instruction::StoreSlot(dst),
            ) = (
                &instructions[i],
                &instructions[i + 1],
                &instructions[i + 2],
            ) {
                let fused: Option<Instruction> = match not_op {
                    Instruction::TritNot  => Some(Instruction::TritNotSlot  { dst: *dst, src: *src }),
                    Instruction::FuzzyNot => Some(Instruction::FuzzyNotSlot { dst: *dst, src: *src }),
                    _ => None,
                };
                if let Some(instr) = fused {
                    old_to_new[i + 1] = new_idx;
                    old_to_new[i + 2] = new_idx;
                    out.push(instr);
                    i += 3;
                    new_idx += 1;
                    continue;
                }
            }
        }
        out.push(instructions[i].clone());
        i += 1;
        new_idx += 1;
    }
    // Sentinel: one-past-end maps to current new_idx
    old_to_new[n] = new_idx;

    // Now remap all jump targets in `out` using old_to_new.
    for instr in &mut out {
        match instr {
            Instruction::Jump(target) => {
                *target = old_to_new[(*target).min(n)];
            }
            Instruction::JumpIfFalse(target) => {
                *target = old_to_new[(*target).min(n)];
            }
            Instruction::Call { target, .. } => {
                // Function start indices shift when earlier instructions are fused
                *target = old_to_new[(*target).min(n)];
            }
            Instruction::TailCall { target, .. } => {
                *target = old_to_new[(*target).min(n)];
            }
            Instruction::SpawnCallDirect { target, .. } => {
                // Spawn targets also shift when earlier instructions are fused
                *target = old_to_new[(*target).min(n)];
            }
            Instruction::TryEnter { catch_target, finally_target } => {
                *catch_target = old_to_new[(*catch_target).min(n)];
                if *finally_target != usize::MAX {
                    *finally_target = old_to_new[(*finally_target).min(n)];
                }
            }
            _ => {}
        }
    }

    (out, old_to_new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_fold_add_and_compare() {
        let input = vec![
            Instruction::ConstNum(3.0),
            Instruction::ConstNum(4.0),
            Instruction::Add,
            Instruction::ConstNum(7.0),
            Instruction::ConstNum(8.0),
            Instruction::Lt,
            Instruction::Halt,
        ];
        let (out, _) = optimize_bytecode_with_map(&input);
        assert!(
            matches!(
                out.as_slice(),
                [Instruction::ConstNum(x), Instruction::ConstBool(b), Instruction::Halt]
                    if *x == 7.0 && *b
            ),
            "got {:?}",
            out
        );
    }

    #[test]
    fn dead_store_unused_slot_becomes_pop() {
        let input = vec![
            Instruction::ConstNum(1.0),
            Instruction::StoreSlot(7),
            Instruction::Halt,
        ];
        let (out, _) = optimize_bytecode_with_map(&input);
        assert!(
            matches!(out.as_slice(), [Instruction::ConstNum(x), Instruction::Pop, Instruction::Halt] if *x == 1.0),
            "got {:?}",
            out
        );
    }

    #[test]
    fn constant_fold_bool_and() {
        let input = vec![
            Instruction::ConstBool(true),
            Instruction::ConstBool(false),
            Instruction::And,
            Instruction::Halt,
        ];
        let (out, _) = optimize_bytecode_with_map(&input);
        assert!(
            matches!(out.as_slice(), [Instruction::ConstBool(b), Instruction::Halt] if !*b),
            "got {:?}",
            out
        );
    }
}
