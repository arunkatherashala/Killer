use crate::error::VmError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Instruction {
    ConstStr(String),
    ConstNum(f64),
    ConstBool(bool),
    ConstNull,
    ConstFunc { params: Vec<String>, bytecode_start: usize, captured_names: Vec<String> },
    EnterScope,
    ExitScope,
    Store(String),
    StoreLocal(String),
    Load(String),
    // Fast-path variable access using compile-time integer slot index.
    // Avoids string hashing on every Load/Store in hot loops.
    StoreSlot(u16),
    LoadSlot(u16),
    // Fused OPTIMIZED instructions for common patterns:
    //   AddSlotConst(slot, n)  = LoadSlot + ConstNum + Add + StoreSlot
    //   SubSlotConst(slot, n)  = LoadSlot + ConstNum + Sub + StoreSlot
    //   LtSlotConst(slot, n)   = LoadSlot + ConstNum + Lt
    //   GtSlotConst(slot, n)   = LoadSlot + ConstNum + Gt
    //   GeSlotConst(slot, n)   = LoadSlot + ConstNum + Ge
    //   LeSlotConst(slot, n)   = LoadSlot + ConstNum + Le
    //   EqSlotConst(slot, n)   = LoadSlot + ConstNum + Eq
    // Each saves 3-4 instructions per iteration — critical for hot loops.
    AddSlotConst(u16, f64),
    SubSlotConst(u16, f64),
    LtSlotConst(u16, f64),
    GtSlotConst(u16, f64),
    GeSlotConst(u16, f64),
    LeSlotConst(u16, f64),
    EqSlotConst(u16, f64),
    // Fused K-STRING opcodes — eliminate intermediate str() + Add allocations.
    // Replaces the common K"prefix{var}" pattern with a single instruction.
    //
    //   PrefixStrSlot  = ConstStr(pre)  + LoadSlot + CallBuiltin("str",1) + Add   (4→1)
    //   SlotStrSuffix  = LoadSlot + CallBuiltin("str",1) + ConstStr(suf) + Add    (4→1)
    //   PrefixSlotSuffix = ConstStr(pre) + LoadSlot + CallBuiltin("str",1)        (6→1)
    //                      + Add + ConstStr(suf) + Add
    //
    // Each reduces allocations from 3 (clone pre, str(val), concat) to 1.
    PrefixStrSlot   { slot: u16, pre: String },
    SlotStrSuffix   { slot: u16, suf: String },
    PrefixSlotSuffix{ slot: u16, pre: String, suf: String },
    Add,
    Sub,
    Mul,
    Div,
    /// Floor division (`//`): pop b, pop a, push floor(a / b) for numbers.
    IntDiv,
    Mod,
    And,
    Or,
    Not,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Jump(usize),
    JumpIfFalse(usize),
    // -- Nova Galaxy: Phase A — Trit-native jump opcodes -------------------------------
    // Direct ternary branching — no is_truthy() bridge, no string compare
    JumpIfTNeg(usize),   // jump if top-of-stack is T_NEG  (-1)
    JumpIfTZero(usize),  // jump if top-of-stack is T_ZERO ( 0)
    JumpIfTPos(usize),   // jump if top-of-stack is T_POS  (+1)
    // Native trit ALU ops — faster than CallBuiltin("trit_and", 2)
    TritAnd,   // pop b, pop a, push trit_and(a,b)
    TritOr,    // pop b, pop a, push trit_or(a,b)
    TritNot,   // pop a, push trit_not(a)
    // Inline trit constants — no CallBuiltin for T_POS()/T_NEG()/T_ZERO()
    ConstTrit(i8),                                    // push Value::Trit(v)
    // Balanced ternary arithmetic
    TritAdd,                                          // pop b, pop a, push clamp(a+b, -1, 1)
    TritMul,                                          // pop b, pop a, push a*b  (±1 or 0)
    /// pop x, push `int_to_trit(x)` / `trit_from_int(x)` — no CallBuiltin
    IntToTrit,
    /// pop trit-ish value, push `trit_to_int` as Number
    TritToInt,
    // Slotted trit: LoadSlot+op+StoreSlot fused into one instruction (4→1 or 3→1)
    TritAndSlots { dst: u16, s1: u16, s2: u16 },    // frame[dst] = frame[s1].min(frame[s2])
    TritOrSlots  { dst: u16, s1: u16, s2: u16 },    // frame[dst] = frame[s1].max(frame[s2])
    TritNotSlot  { dst: u16, src: u16 },             // frame[dst] = -frame[src]
    TritAddSlots { dst: u16, s1: u16, s2: u16 },    // frame[dst] = clamp(s1+s2, -1, 1)
    TritMulSlots { dst: u16, s1: u16, s2: u16 },    // frame[dst] = clamp(s1*s2, -1, 1)
    // -- Nova Galaxy: Phase B — Signal fast-path opcodes -------------------------
    // Direct Signal operations — no string roundtrip
    SignalGetValue,       // pop signal, push its trit value
    SignalGetConfidence,  // pop signal, push its confidence (Number)
    SignalGetReason,      // pop signal, push its reason (Str)
    JumpIfSignalConfident(usize, f64), // jump if signal.confidence >= threshold
    JumpIfSignalUncertain(usize, f64), // jump if signal.confidence < threshold
    // -- Nova Galaxy: Phase C — Qubit probabilistic branching ------------------
    // Non-deterministic VM branching — Killer becomes probabilistic
    JumpIfQubitMeasure(usize), // measure qubit top-of-stack: jump if 1, fallthrough if 0
    QubitHadamard,  // pop qubit, push hadamard(qubit) — native gate
    QubitPauliX,    // pop qubit, push pauli_x(qubit)
    QubitMeasure,   // pop qubit, push measured bit (0 or 1) as Number
    // -- Nova Galaxy: Phase D — Fuzzy float native opcodes ---------------------
    // Float logic in one opcode — no CallBuiltin, no dispatch
    FuzzyAnd,               // pop b, pop a, push min(a,b) as Number
    FuzzyOr,                // pop b, pop a, push max(a,b) as Number
    FuzzyNot,               // pop a, push 1.0 - a as Number
    JumpIfFuzzyHigh(usize, f64), // jump if top-of-stack Number >= threshold
    JumpIfFuzzyLow(usize, f64),  // jump if top-of-stack Number < threshold
    // Slotted fuzzy: LoadSlot+op+StoreSlot fused (4→1 or 3→1)
    FuzzyAndSlots { dst: u16, s1: u16, s2: u16 },   // frame[dst] = min(frame[s1], frame[s2])
    FuzzyOrSlots  { dst: u16, s1: u16, s2: u16 },   // frame[dst] = max(frame[s1], frame[s2])
    FuzzyNotSlot  { dst: u16, src: u16 },            // frame[dst] = 1.0 - frame[src]
    // -- Nova Galaxy: Phase E — Tryte (6-trit word) native ALU opcodes ---------
    // 6-trit SIMD-style ops — all 6 trits in one instruction
    TryteAnd,   // pop b, pop a, push element-wise min Tryte
    TryteOr,    // pop b, pop a, push element-wise max Tryte
    TryteNot,   // pop a, push element-wise negated Tryte
    TryteAdd,   // pop b, pop a, push balanced-ternary sum Tryte
    Call { target: usize, arg_count: usize },
    /// Tail-call (same stack frame): pop args, reuse current `call_stack` entry, jump to `target`.
    TailCall { target: usize, arg_count: usize },
    CallDynamic { arg_count: usize },
    Ret,
    Pop,
    Print,
    PrintMultiple(usize),
    BuildArray(usize),
    BuildDict(usize),
    IndexRead,
    IndexWrite(String),
    /// Like [`Instruction::IndexWrite`], but the container is `locals_stack.last()[slot]` (slotted locals).
    IndexWriteSlot(u16),
    CallBuiltin(String, usize),
    /// Fast numeric dispatch for known builtins (id from `BUILTIN_ID_TABLE`).
    CallBuiltinId(u16, usize),
    DefineClass { name: String, parent: Option<String> },
    NewObject(String),
    CallMethod { object_name: String, method_name: String, arg_count: usize },
    CallMethodDynamic { method_name: String, arg_count: usize },
    NewQuality,
    TryEnter { catch_target: usize, finally_target: usize },
    TryExit,
    Throw,
    Yield,
    CatchEnter { var_name: Option<String> },
    FinallyEnter,
    Halt,
    // ── v2.2: Async / concurrency / packages ────────────────────────────────
    /// Spawn a closure/function in a background OS thread.
    /// Pops a Value::Function (captured closure) from stack.
    /// Pushes a Value::Future handle (Arc<Mutex<Option<Value>>>) onto stack.
    SpawnTask,
    /// True parallel OS-thread spawn — function ref is popped from stack.
    /// Stack layout (bottom→top): func, arg0, …, argN-1.
    SpawnCall { arg_count: usize },
    /// True parallel OS-thread spawn — function address baked at compile time (mirrors Call).
    /// Stack: arg0, …, argN-1 (no func on stack — target is the bytecode start).
    SpawnCallDirect { target: usize, arg_count: usize },
    /// Block current thread until the Future on top of stack resolves.
    /// Pops Value::Future, pushes resolved Value.
    AwaitTask,
    /// Load and execute a Killer package file.
    /// All top-level definitions from that file are merged into current scope.
    ImportPkg(String),
}

#[derive(Debug, Clone)]
enum RawInstruction {
    Concrete(Instruction),
    JumpLabel(String),
    JumpIfFalseLabel(String),
    CallLabel { label: String, arg_count: usize },
}

#[derive(Debug, Clone)]
pub struct Program {
    pub instructions: Vec<Instruction>,
    pub function_arities: HashMap<usize, usize>,
    pub function_names: HashMap<usize, String>,
    pub method_bytecode: HashMap<(String, String), usize>, // (class_name, method_name) -> bytecode_start
    pub classes: HashMap<String, (Option<String>, Vec<(String, Vec<String>, Vec<crate::ast::Stmt>)>)>, // class_name -> (parent, methods)
}

impl Program {
    pub fn parse(source: &str) -> Result<Self, VmError> {
        let mut raw_instructions: Vec<RawInstruction> = Vec::new();
        let mut labels: HashMap<String, usize> = HashMap::new();
        let mut function_arities_by_name: HashMap<String, usize> = HashMap::new();

        for (index, raw_line) in source.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("LABEL ") {
                let name = rest.trim();
                if name.is_empty() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: LABEL requires name",
                        index + 1
                    )));
                }
                if labels.contains_key(name) {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: duplicate label `{name}`",
                        index + 1
                    )));
                }
                labels.insert(name.to_string(), raw_instructions.len());
                continue;
            }

            if let Some(rest) = line.strip_prefix("FUNC ") {
                let mut parts = rest.split_whitespace();
                let name = parts.next().unwrap_or("").trim();
                if name.is_empty() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: FUNC requires name",
                        index + 1
                    )));
                }

                let arity = if let Some(arity_text) = parts.next() {
                    arity_text.parse::<usize>().map_err(|_| {
                        VmError::parse_error_simple(format!(
                            "Line {}: FUNC arity must be a non-negative integer",
                            index + 1
                        ))
                    })?
                } else {
                    0
                };

                if parts.next().is_some() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: FUNC format is `FUNC <name> [arity]`",
                        index + 1
                    )));
                }

                if labels.contains_key(name) {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: duplicate function `{name}`",
                        index + 1
                    )));
                }
                labels.insert(name.to_string(), raw_instructions.len());
                function_arities_by_name.insert(name.to_string(), arity);
                continue;
            }

            if let Some(rest) = line.strip_prefix("CONST_STR ") {
                let text = rest.trim();
                let cleaned = text
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .ok_or_else(|| VmError::parse_error_simple(format!("Line {}: CONST_STR requires quoted string", index + 1)))?;
                raw_instructions.push(RawInstruction::Concrete(Instruction::ConstStr(
                    cleaned.to_string(),
                )));
                continue;
            }

            if let Some(rest) = line.strip_prefix("CONST_NUM ") {
                let value = rest.trim().parse::<f64>().map_err(|_| {
                    VmError::parse_error_simple(format!("Line {}: invalid number for CONST_NUM", index + 1))
                })?;
                raw_instructions.push(RawInstruction::Concrete(Instruction::ConstNum(value)));
                continue;
            }

            if let Some(rest) = line.strip_prefix("CONST_BOOL ") {
                let bool_text = rest.trim().to_ascii_lowercase();
                let value = match bool_text.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => {
                        return Err(VmError::parse_error_simple(format!(
                            "Line {}: CONST_BOOL must be true or false",
                            index + 1
                        )))
                    }
                };
                raw_instructions.push(RawInstruction::Concrete(Instruction::ConstBool(value)));
                continue;
            }

            if let Some(rest) = line.strip_prefix("STORE ") {
                let name = rest.trim();
                if name.is_empty() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: STORE requires variable name",
                        index + 1
                    )));
                }
                raw_instructions.push(RawInstruction::Concrete(Instruction::Store(
                    name.to_string(),
                )));
                continue;
            }

            if let Some(rest) = line.strip_prefix("LOAD ") {
                let name = rest.trim();
                if name.is_empty() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: LOAD requires variable name",
                        index + 1
                    )));
                }
                raw_instructions.push(RawInstruction::Concrete(Instruction::Load(
                    name.to_string(),
                )));
                continue;
            }

            if let Some(rest) = line.strip_prefix("JUMP_IF_FALSE ") {
                let target_text = rest.trim();
                if target_text.is_empty() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: JUMP_IF_FALSE requires target",
                        index + 1
                    )));
                }
                if let Ok(target) = target_text.parse::<usize>() {
                    raw_instructions.push(RawInstruction::Concrete(Instruction::JumpIfFalse(target)));
                } else {
                    raw_instructions.push(RawInstruction::JumpIfFalseLabel(target_text.to_string()));
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix("JUMP ") {
                let target_text = rest.trim();
                if target_text.is_empty() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: JUMP requires target",
                        index + 1
                    )));
                }
                if let Ok(target) = target_text.parse::<usize>() {
                    raw_instructions.push(RawInstruction::Concrete(Instruction::Jump(target)));
                } else {
                    raw_instructions.push(RawInstruction::JumpLabel(target_text.to_string()));
                }
                continue;
            }

            if let Some(rest) = line.strip_prefix("CALL ") {
                let mut parts = rest.split_whitespace();
                let target_text = parts.next().unwrap_or("").trim();
                if target_text.is_empty() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: CALL requires target",
                        index + 1
                    )));
                }

                let arg_count = if let Some(arg_text) = parts.next() {
                    arg_text.parse::<usize>().map_err(|_| {
                        VmError::parse_error_simple(format!(
                            "Line {}: CALL arg_count must be a non-negative integer",
                            index + 1
                        ))
                    })?
                } else {
                    0
                };

                if parts.next().is_some() {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: CALL format is `CALL <target> [arg_count]`",
                        index + 1
                    )));
                }

                if let Ok(target) = target_text.parse::<usize>() {
                    raw_instructions.push(RawInstruction::Concrete(Instruction::Call {
                        target,
                        arg_count,
                    }));
                } else {
                    raw_instructions.push(RawInstruction::CallLabel {
                        label: target_text.to_string(),
                        arg_count,
                    });
                }
                continue;
            }

            match line {
                "ENTER_SCOPE" => raw_instructions.push(RawInstruction::Concrete(Instruction::EnterScope)),
                "EXIT_SCOPE" => raw_instructions.push(RawInstruction::Concrete(Instruction::ExitScope)),
                "ADD" => raw_instructions.push(RawInstruction::Concrete(Instruction::Add)),
                "SUB" => raw_instructions.push(RawInstruction::Concrete(Instruction::Sub)),
                "MUL" => raw_instructions.push(RawInstruction::Concrete(Instruction::Mul)),
                "DIV" => raw_instructions.push(RawInstruction::Concrete(Instruction::Div)),
                "IDIV" | "INTDIV" | "FLOOR_DIV" => {
                    raw_instructions.push(RawInstruction::Concrete(Instruction::IntDiv))
                }
                "AND" => raw_instructions.push(RawInstruction::Concrete(Instruction::And)),
                "OR" => raw_instructions.push(RawInstruction::Concrete(Instruction::Or)),
                "EQ" => raw_instructions.push(RawInstruction::Concrete(Instruction::Eq)),
                "NE" => raw_instructions.push(RawInstruction::Concrete(Instruction::Ne)),
                "GT" => raw_instructions.push(RawInstruction::Concrete(Instruction::Gt)),
                "GE" => raw_instructions.push(RawInstruction::Concrete(Instruction::Ge)),
                "LT" => raw_instructions.push(RawInstruction::Concrete(Instruction::Lt)),
                "LE" => raw_instructions.push(RawInstruction::Concrete(Instruction::Le)),
                "RET" => raw_instructions.push(RawInstruction::Concrete(Instruction::Ret)),
                "POP" => raw_instructions.push(RawInstruction::Concrete(Instruction::Pop)),
                "PRINT" => raw_instructions.push(RawInstruction::Concrete(Instruction::Print)),
                "HALT" => raw_instructions.push(RawInstruction::Concrete(Instruction::Halt)),
                _ => {
                    return Err(VmError::parse_error_simple(format!(
                        "Line {}: unknown instruction `{}`",
                        index + 1,
                        line
                    )))
                }
            }
        }

        let mut instructions = Vec::new();
        for raw in raw_instructions {
            let instruction = match raw {
                RawInstruction::Concrete(value) => value,
                RawInstruction::JumpLabel(label) => {
                    let target = labels.get(&label).ok_or_else(|| {
                        VmError::parse_error_simple(format!("Unknown label `{label}` for JUMP"))
                    })?;
                    Instruction::Jump(*target)
                }
                RawInstruction::JumpIfFalseLabel(label) => {
                    let target = labels.get(&label).ok_or_else(|| {
                        VmError::parse_error_simple(format!("Unknown label `{label}` for JUMP_IF_FALSE"))
                    })?;
                    Instruction::JumpIfFalse(*target)
                }
                RawInstruction::CallLabel { label, arg_count } => {
                    let target = labels.get(&label).ok_or_else(|| {
                        VmError::parse_error_simple(format!("Unknown label `{label}` for CALL"))
                    })?;
                    Instruction::Call {
                        target: *target,
                        arg_count,
                    }
                }
            };
            instructions.push(instruction);
        }

        let mut function_arities: HashMap<usize, usize> = HashMap::new();
        for (name, arity) in &function_arities_by_name {
            if let Some(target) = labels.get(name) {
                function_arities.insert(*target, *arity);
            }
        }

        let mut function_names = HashMap::new();
        for (name, target) in labels.iter() {
            if let Some(_arity) = function_arities_by_name.get(name) {
                function_names.insert(*target, name.clone());
            }
        }

        Ok(Self {
            instructions,
            function_arities,
            function_names,
            method_bytecode: HashMap::new(),
            classes: HashMap::new(),
        })
    }
}
