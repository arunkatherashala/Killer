// Week 12-14: Contract Programming - Preconditions, Postconditions, and Invariants
// Goals: Formal verification, design-by-contract, automated testing
// Coverage: +400 problems (formal methods, verification, safety)

use std::collections::HashMap;
use std::cell::RefCell;

thread_local! {
    static CONTRACT_RUNTIME: RefCell<ContractRuntime> = 
        RefCell::new(ContractRuntime::new());
}

// ============================================================================
// Week 12: Preconditions and Postconditions
// ============================================================================

/// Precondition - assertion that must be true before function execution
#[derive(Clone, Debug)]
pub struct Precondition {
    name: String,
    description: String,
    check_fn: fn(&str) -> bool,
}

impl Precondition {
    pub fn new(name: impl Into<String>, description: impl Into<String>, check_fn: fn(&str) -> bool) -> Self {
        Precondition {
            name: name.into(),
            description: description.into(),
            check_fn,
        }
    }

    pub fn check(&self, param: &str) -> bool {
        (self.check_fn)(param)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Postcondition - assertion that must be true after function execution
#[derive(Clone, Debug)]
pub struct Postcondition {
    name: String,
    description: String,
    check_fn: fn(&str) -> bool,
}

impl Postcondition {
    pub fn new(name: impl Into<String>, description: impl Into<String>, check_fn: fn(&str) -> bool) -> Self {
        Postcondition {
            name: name.into(),
            description: description.into(),
            check_fn,
        }
    }

    pub fn check(&self, result: &str) -> bool {
        (self.check_fn)(result)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Contract for a function
#[derive(Clone)]
pub struct FunctionContract {
    function_name: String,
    preconditions: Vec<Precondition>,
    postconditions: Vec<Postcondition>,
    invariants: Vec<String>,
}

impl FunctionContract {
    pub fn new(name: impl Into<String>) -> Self {
        FunctionContract {
            function_name: name.into(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            invariants: Vec::new(),
        }
    }

    pub fn add_precondition(mut self, pre: Precondition) -> Self {
        self.preconditions.push(pre);
        self
    }

    pub fn add_postcondition(mut self, post: Postcondition) -> Self {
        self.postconditions.push(post);
        self
    }

    pub fn add_invariant(mut self, inv: impl Into<String>) -> Self {
        self.invariants.push(inv.into());
        self
    }

    pub fn verify_preconditions(&self, param: &str) -> Result<(), Vec<String>> {
        let failures: Vec<String> = self
            .preconditions
            .iter()
            .filter(|pre| !pre.check(param))
            .map(|pre| format!("Precondition '{}' failed: {}", pre.name, pre.description))
            .collect();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
    }

    pub fn verify_postconditions(&self, result: &str) -> Result<(), Vec<String>> {
        let failures: Vec<String> = self
            .postconditions
            .iter()
            .filter(|post| !post.check(result))
            .map(|post| format!("Postcondition '{}' failed: {}", post.name, post.description))
            .collect();

        if failures.is_empty() {
            Ok(())
        } else {
            Err(failures)
        }
    }
}

// ============================================================================
// Week 13: Class Invariants and Object Contracts
// ============================================================================

/// Class invariant - assertion that must hold throughout object lifetime
#[derive(Clone, Debug)]
pub struct ClassInvariant {
    name: String,
    property: String,
    check_fn: fn(&str) -> bool,
}

impl ClassInvariant {
    pub fn new(name: impl Into<String>, property: impl Into<String>, check_fn: fn(&str) -> bool) -> Self {
        ClassInvariant {
            name: name.into(),
            property: property.into(),
            check_fn,
        }
    }

    pub fn check(&self, state: &str) -> bool {
        (self.check_fn)(state)
    }
}

/// Class contract - contracts for class operations
#[derive(Clone)]
pub struct ClassContract {
    class_name: String,
    invariants: Vec<ClassInvariant>,
    constructor_contract: Option<FunctionContract>,
    method_contracts: HashMap<String, FunctionContract>,
}

impl ClassContract {
    pub fn new(name: impl Into<String>) -> Self {
        ClassContract {
            class_name: name.into(),
            invariants: Vec::new(),
            constructor_contract: None,
            method_contracts: HashMap::new(),
        }
    }

    pub fn add_invariant(mut self, inv: ClassInvariant) -> Self {
        self.invariants.push(inv);
        self
    }

    pub fn set_constructor_contract(mut self, contract: FunctionContract) -> Self {
        self.constructor_contract = Some(contract);
        self
    }

    pub fn add_method_contract(mut self, method_name: impl Into<String>, contract: FunctionContract) -> Self {
        self.method_contracts.insert(method_name.into(), contract);
        self
    }

    pub fn check_invariants(&self, state: &str) -> bool {
        self.invariants.iter().all(|inv| inv.check(state))
    }

    pub fn get_method_contract(&self, method_name: &str) -> Option<&FunctionContract> {
        self.method_contracts.get(method_name)
    }
}

// ============================================================================
// Week 14: Automated Verification and Testing
// ============================================================================

/// Test case for contract verification
#[derive(Clone, Debug)]
pub struct TestCase {
    name: String,
    input: String,
    expected_output: String,
}

impl TestCase {
    pub fn new(name: impl Into<String>, input: impl Into<String>, expected: impl Into<String>) -> Self {
        TestCase {
            name: name.into(),
            input: input.into(),
            expected_output: expected.into(),
        }
    }
}

/// Verification engine for automated testing
pub struct VerificationEngine {
    contracts: HashMap<String, FunctionContract>,
    test_cases: Vec<TestCase>,
    passed_tests: u64,
    failed_tests: u64,
    violations: Vec<String>,
}

impl VerificationEngine {
    pub fn new() -> Self {
        VerificationEngine {
            contracts: HashMap::new(),
            test_cases: Vec::new(),
            passed_tests: 0,
            failed_tests: 0,
            violations: Vec::new(),
        }
    }

    pub fn register_contract(&mut self, contract: FunctionContract) {
        self.contracts.insert(contract.function_name.clone(), contract);
    }

    pub fn add_test_case(&mut self, test: TestCase) {
        self.test_cases.push(test);
    }

    pub fn verify_function(&mut self, func_name: &str, input: &str, output: &str) -> bool {
        if let Some(contract) = self.contracts.get(func_name) {
            // Check preconditions
            match contract.verify_preconditions(input) {
                Ok(_) => {
                    // Check postconditions
                    match contract.verify_postconditions(output) {
                        Ok(_) => {
                            self.passed_tests += 1;
                            true
                        }
                        Err(failures) => {
                            self.failed_tests += 1;
                            self.violations.extend(failures);
                            false
                        }
                    }
                }
                Err(failures) => {
                    self.failed_tests += 1;
                    self.violations.extend(failures);
                    false
                }
            }
        } else {
            self.failed_tests += 1;
            self.violations.push(format!("Function {} has no contract", func_name));
            false
        }
    }

    pub fn run_all_tests(&mut self) -> u64 {
        for test in self.test_cases.clone() {
            self.verify_function("test_function", &test.input, &test.expected_output);
        }
        self.passed_tests
    }

    pub fn get_stats(&self) -> (u64, u64, u64) {
        (self.passed_tests, self.failed_tests, self.violations.len() as u64)
    }

    pub fn violations(&self) -> &[String] {
        &self.violations
    }
}

impl Default for VerificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Contract Runtime
// ============================================================================

pub struct ContractRuntime {
    verification_engine: VerificationEngine,
    contract_count: u64,
}

impl ContractRuntime {
    pub fn new() -> Self {
        ContractRuntime {
            verification_engine: VerificationEngine::new(),
            contract_count: 0,
        }
    }

    pub fn register(&mut self, contract: FunctionContract) {
        self.contract_count += 1;
        self.verification_engine.register_contract(contract);
    }

    pub fn add_test(&mut self, test: TestCase) {
        self.verification_engine.add_test_case(test);
    }

    pub fn run_verification(&mut self) -> u64 {
        self.verification_engine.run_all_tests()
    }

    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        let (passed, failed, violations) = self.verification_engine.get_stats();
        (self.contract_count, passed, failed, violations)
    }
}

impl Default for ContractRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Public API
// ============================================================================

pub fn register_contract(contract: FunctionContract) {
    CONTRACT_RUNTIME.with(|rt| {
        rt.borrow_mut().register(contract);
    });
}

pub fn add_test_case(test: TestCase) {
    CONTRACT_RUNTIME.with(|rt| {
        rt.borrow_mut().add_test(test);
    });
}

pub fn run_contract_verification() -> u64 {
    CONTRACT_RUNTIME.with(|rt| {
        rt.borrow_mut().run_verification()
    })
}

pub fn get_contract_stats() -> (u64, u64, u64, u64) {
    CONTRACT_RUNTIME.with(|rt| {
        rt.borrow().get_stats()
    })
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precondition() {
        let pre = Precondition::new(
            "positive",
            "value must be positive",
            |s| !s.is_empty() && s.parse::<i32>().map(|n| n > 0).unwrap_or(false),
        );
        
        assert!(pre.check("42"));
        assert!(!pre.check("-5"));
    }

    #[test]
    fn test_postcondition() {
        let post = Postcondition::new(
            "non_empty",
            "result must not be empty",
            |s| !s.is_empty(),
        );
        
        assert!(post.check("result"));
        assert!(!post.check(""));
    }

    #[test]
    fn test_function_contract() {
        let pre = Precondition::new("input", "input valid", |_| true);
        let post = Postcondition::new("output", "output valid", |_| true);
        
        let contract = FunctionContract::new("test_fn")
            .add_precondition(pre)
            .add_postcondition(post);

        assert!(contract.verify_preconditions("any").is_ok());
        assert!(contract.verify_postconditions("any").is_ok());
    }

    #[test]
    fn test_class_invariant() {
        let inv = ClassInvariant::new(
            "valid_state",
            "state is consistent",
            |_| true,
        );
        
        assert!(inv.check("some_state"));
    }

    #[test]
    fn test_class_contract() {
        let inv = ClassInvariant::new("inv", "prop", |_| true);
        let contract = ClassContract::new("TestClass")
            .add_invariant(inv);

        assert!(contract.check_invariants("state"));
    }

    #[test]
    fn test_test_case() {
        let test = TestCase::new("test1", "input1", "expected1");
        assert_eq!(test.name, "test1");
    }

    #[test]
    fn test_verification_engine() {
        let mut engine = VerificationEngine::new();
        let contract = FunctionContract::new("fn");
        engine.register_contract(contract);

        assert_eq!(engine.contracts.len(), 1);
    }

    #[test]
    fn test_contract_runtime() {
        CONTRACT_RUNTIME.with(|rt| {
            let mut r = rt.borrow_mut();
            let contract = FunctionContract::new("test");
            r.register(contract);

            let (count, _, _, _) = r.get_stats();
            assert_eq!(count, 1);
        });
    }
}
