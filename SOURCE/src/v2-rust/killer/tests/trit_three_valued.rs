//! Why trits: three-valued composition without inventing your own encoding.
//! See `SOURCE/docs/POSITIONING.md`.
//!
//! Kleene-style AND = min(trit); OR = max(trit). Unknown (0) combines predictably.
//!
//! We assert via [`killer_native::builtin::BuiltinFunctions`] — the same dispatch the VM uses for `trit_*` / `int_to_trit` (top-level scripts often use `StoreSlot`, so `get_globals()` is not where those values live).

use killer_native::builtin::BuiltinFunctions;
use killer_native::value::Value;

fn int_trit(n: i32) -> Value {
    BuiltinFunctions::call("int_to_trit", &[Value::Number(n as f64)]).expect("int_to_trit")
}

#[test]
fn trit_and_unknown_with_yes_stays_unknown() {
    let r = BuiltinFunctions::call("trit_and", &[int_trit(0), int_trit(1)]).expect("trit_and");
    assert_eq!(r, Value::Trit(0));
}

#[test]
fn trit_or_unknown_with_yes_is_yes() {
    let r = BuiltinFunctions::call("trit_or", &[int_trit(0), int_trit(1)]).expect("trit_or");
    assert_eq!(r, Value::Trit(1));
}

#[test]
fn trit_not_inverts() {
    let r = BuiltinFunctions::call("trit_not", &[int_trit(1)]).expect("trit_not");
    assert_eq!(r, Value::Trit(-1));
}

#[test]
fn trit_word_maps_for_ui_layer() {
    let r = BuiltinFunctions::call("trit_word", &[int_trit(0)]).expect("trit_word");
    assert_eq!(r, Value::Str("maybe".to_string()));
}

#[test]
fn vm_executes_trit_script_smoke() {
    use killer_native::compiler::compile_killer_default;
    use killer_native::vm::VirtualMachine;

    let src = "a = int_to_trit(0)\n\
         b = int_to_trit(1)\n\
         r = trit_and(a, b)\n\
         print(trit_to_int(r))\n";
    let program = compile_killer_default(src).expect("compile");
    let mut vm = VirtualMachine::new();
    vm.run(&program).expect("vm runs trit script");
}

#[test]
fn trit_conversion_emits_native_opcodes_not_builtin() {
    use killer_native::bytecode::Instruction;
    use killer_native::compiler::compile_killer_default;

    let src = "x = int_to_trit(1)\n\
         y = trit_to_int(x)\n\
         print(y)\n";
    let program = compile_killer_default(src).expect("compile");
    let calls_int_to_trit = program.instructions.iter().any(|i| {
        matches!(i, Instruction::CallBuiltin(name, _) if name == "int_to_trit" || name == "trit_from_int")
    });
    let calls_trit_to_int = program
        .instructions
        .iter()
        .any(|i| matches!(i, Instruction::CallBuiltin(name, _) if name == "trit_to_int"));

    assert!(
        !calls_int_to_trit && !calls_trit_to_int,
        "conversion builtins should lower to native opcodes or constant folds"
    );
    // Literal int_to_trit(1) peephole-folds to ConstTrit(1); trit_to_int(x) still uses TritToInt.
    assert!(
        program
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::ConstTrit(1))),
        "int_to_trit(1) should fold to ConstTrit(1)"
    );
    assert!(
        program
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::TritToInt)),
        "trit_to_int(slot) should use TritToInt opcode"
    );
}

#[test]
fn trit_mul_slot_fusion_emits_trit_mul_slots() {
    use killer_native::bytecode::Instruction;
    use killer_native::compiler::compile_killer_default;

    let src = "a = T_POS()\n\
         b = T_ZERO()\n\
         c = trit_mul(a, b)\n\
         print(trit_to_int(c))\n";
    let program = compile_killer_default(src).expect("compile");
    assert!(
        program
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::TritMulSlots { .. })),
        "LoadSlot + trit_mul + StoreSlot should fuse to TritMulSlots"
    );
}

#[test]
fn literal_int_to_trit_folds_to_const_trit() {
    use killer_native::bytecode::Instruction;
    use killer_native::compiler::compile_killer_default;

    let src = "x = int_to_trit(1)\n\
         y = int_to_trit(42)\n";
    let program = compile_killer_default(src).expect("compile");
    let pos_trits = program
        .instructions
        .iter()
        .filter(|i| matches!(i, Instruction::ConstTrit(1)))
        .count();
    assert!(
        pos_trits >= 2,
        "int_to_trit(1) and int_to_trit(42) should each fold to ConstTrit(1), got {} ConstTrit(1)",
        pos_trits
    );
    assert!(
        !program
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::IntToTrit)),
        "literal int_to_trit should not leave IntToTrit after peephole"
    );
}

#[test]
fn int_to_trit_from_variable_still_uses_native_opcode() {
    use killer_native::bytecode::Instruction;
    use killer_native::compiler::compile_killer_default;

    let src = "x = 1\n\
         t = int_to_trit(x)\n";
    let program = compile_killer_default(src).expect("compile");
    assert!(
        program
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::IntToTrit)),
        "int_to_trit(non-literal) should emit IntToTrit, not only folds"
    );
}

#[test]
fn literal_trit_to_int_folds_after_const_trit() {
    use killer_native::bytecode::Instruction;
    use killer_native::compiler::compile_killer_default;

    let src = "x = trit_to_int(T_NEG())\n\
         print(x)\n";
    let program = compile_killer_default(src).expect("compile");
    assert!(
        program.instructions.iter().any(|i| matches!(i, Instruction::ConstNum(n) if *n == -1.0)),
        "trit_to_int(T_NEG()) should fold to ConstNum(-1)"
    );
    assert!(
        !program
            .instructions
            .iter()
            .any(|i| matches!(i, Instruction::TritToInt)),
        "literal trit_to_int of constant trit should not stay as TritToInt"
    );
}

#[test]
fn example_trinary_in_killer_vm_runs() {
    use killer_native::compiler::compile_killer_default;
    use killer_native::vm::VirtualMachine;

    const SRC: &str = include_str!("../examples/trinary_in_killer.killer");
    let program = compile_killer_default(SRC).expect("examples/trinary_in_killer.killer should compile");
    let mut vm = VirtualMachine::new();
    vm.run(&program).expect("VM runs trinary_in_killer.killer");
}

#[test]
fn example_trinary_kleene_table_vm_runs() {
    use killer_native::compiler::compile_killer_default;
    use killer_native::vm::VirtualMachine;

    const SRC: &str = include_str!("../examples/trinary_kleene_table.killer");
    let program = compile_killer_default(SRC).expect("examples/trinary_kleene_table.killer should compile");
    let mut vm = VirtualMachine::new();
    vm.run(&program).expect("VM runs trinary_kleene_table.killer");
}

#[test]
fn example_killer_language_ready_manifest_runs() {
    use killer_native::compiler::compile_killer_default;
    use killer_native::vm::VirtualMachine;

    const SRC: &str = include_str!("../examples/killer_language_ready.killer");
    let program = compile_killer_default(SRC).expect("examples/killer_language_ready.killer should compile");
    let mut vm = VirtualMachine::new();
    vm.run(&program).expect("VM runs killer_language_ready.killer");
}
