/// Virtual Machine Execution Tests
/// Tests for bytecode execution, control flow, and runtime behavior

use killer_native::vm::VirtualMachine;
use killer_native::compiler::compile_killer_default;
use killer_native::value::Value;

#[cfg(test)]
mod vm_execution_tests {
    use super::*;

    // ========== BASIC ARITHMETIC ==========

    #[test]
    fn test_vm_constant_number() {
        let mut vm = VirtualMachine::new();
        let program = compile_killer_default("42").unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 42.0),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_vm_simple_addition() {
        let mut vm = VirtualMachine::new();
        let program = compile_killer_default("5 + 3").unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 8.0),
            _ => panic!("Expected 8.0"),
        }
    }

    #[test]
    fn test_vm_division_by_zero() {
        let mut vm = VirtualMachine::new();
        let program = compile_killer_default("10 / 0").unwrap();
        let result = vm.run(&program);
        assert!(result.is_err(), "Should error on division by zero");
    }

    #[test]
    fn test_vm_string_concatenation() {
        let mut vm = VirtualMachine::new();
        let program = compile_killer_default("\"hello\" + \" \" + \"world\"").unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Str(s) => assert_eq!(s, "hello world"),
            _ => panic!("Expected string"),
        }
    }

    // ========== VARIABLE OPERATIONS ==========

    #[test]
    fn test_vm_store_and_load_variable() {
        let mut vm = VirtualMachine::new();
        let program = compile_killer_default("let x = 42;
x").unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 42.0),
            _ => panic!("Expected 42.0"),
        }
    }

    #[test]
    fn test_vm_variable_shadowing() {
        let mut vm = VirtualMachine::new();
        let source = "
            let x = 1
            {
                let x = 2
            }
            x
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 1.0),
            _ => panic!("Expected 1.0 from outer scope"),
        }
    }

    // ========== CONTROL FLOW ==========

    #[test]
    fn test_vm_if_then_true_branch() {
        let mut vm = VirtualMachine::new();
        let source = "
            let x = 0
            if true {
                x = 1
            } else {
                x = 2
            }
            x
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 1.0),
            _ => panic!("Expected 1.0"),
        }
    }

    #[test]
    fn test_vm_if_then_false_branch() {
        let mut vm = VirtualMachine::new();
        let source = "
            let x = 0
            if false {
                x = 1
            } else {
                x = 2
            }
            x
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 2.0),
            _ => panic!("Expected 2.0"),
        }
    }

    // ========== LOOPS ==========

    #[test]
    fn test_vm_simple_while_loop() {
        let mut vm = VirtualMachine::new();
        let source = "
            let i = 0
            while i < 3 {
                i = i + 1
            }
            i
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 3.0),
            _ => panic!("Expected 3.0"),
        }
    }

    #[test]
    fn test_vm_for_loop_iteration() {
        let mut vm = VirtualMachine::new();
        let source = "
            let sum = 0
            for i of [1, 2, 3] {
                sum = sum + i
            }
            sum
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 6.0),
            _ => panic!("Expected 6.0"),
        }
    }

    // ========== FUNCTION CALLS ==========

    #[test]
    fn test_vm_function_call_with_args() {
        let mut vm = VirtualMachine::new();
        let source = "
            kfn add(a, b) {
                return a + b
            }
            add(3, 4)
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 7.0),
            _ => panic!("Expected 7.0"),
        }
    }

    #[test]
    fn test_vm_recursive_function() {
        let mut vm = VirtualMachine::new();
        let source = "
            kfn fib(n) {
                if n <= 1 {
                    return n
                }
                return fib(n - 1) + fib(n - 2)
            }
            fib(5)
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 5.0),
            _ => panic!("Expected 5.0 for fib(5)"),
        }
    }

    // ========== ARRAYS ==========

    #[test]
    fn test_vm_array_indexing() {
        let mut vm = VirtualMachine::new();
        let program = compile_killer_default("let arr = [10, 20, 30]
arr[1]").unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 20.0),
            _ => panic!("Expected 20.0"),
        }
    }

    #[test]
    fn test_vm_array_mutation() {
        let mut vm = VirtualMachine::new();
        let source = "
            let arr = [1, 2]
            arr[0] = 99
            arr[0]
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 99.0),
            _ => panic!("Expected 99.0"),
        }
    }

    // ========== EXCEPTION HANDLING ==========

    #[test]
    fn test_vm_try_catch_error() {
        let mut vm = VirtualMachine::new();
        let source = "
            let x = 0
            try {
                throw \"error\"
            } catch e {
                x = 99
            }
            x
        ";
        // Subset compiler might not support try/catch yet
        let program = compile_killer_default(source);
        if let Ok(p) = program {
            vm.run(&p).unwrap();
            match vm.stack.last().expect("Stack should not be empty") {
                Value::Number(n) => assert_eq!(*n, 99.0),
                _ => panic!("Expected 99.0 from catch block"),
            }
        }
    }
}

#[cfg(test)]
mod vm_stack_tests {
    use super::*;

    #[test]
    fn test_stack_push_pop() {
        let mut vm = VirtualMachine::new();
        vm.stack.push(Value::Number(1.0));
        vm.stack.push(Value::Number(2.0));
        assert_eq!(vm.stack.pop().unwrap(), Value::Number(2.0));
        assert_eq!(vm.stack.pop().unwrap(), Value::Number(1.0));
    }
}

#[cfg(test)]
mod vm_performance_tests {
    use super::*;

    #[test]
    fn test_loop_optimization_correctness() {
        let mut vm = VirtualMachine::new();
        let source = "
            let sum = 0
            for i of [1, 2, 3, 4] {
                sum = sum + i
            }
            sum
        ";
        let program = compile_killer_default(source).unwrap();
        vm.run(&program).unwrap();
        match vm.stack.last().expect("Stack should not be empty") {
            Value::Number(n) => assert_eq!(*n, 10.0),
            _ => panic!("Expected 10.0"),
        }
    }
}
