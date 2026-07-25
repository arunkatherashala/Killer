// WEEK 12-14: CONTRACT PROGRAMMING PRACTICE EXERCISES
// Comprehensive hands-on exercises for mastery-level learning

use std::collections::{HashMap, VecDeque};
use std::fmt;

// ============================================================================
// WEEK 12 EXERCISES: PRECONDITIONS & POSTCONDITIONS
// ============================================================================

// ============================================================================
// Exercise 12.1: Basic Precondition - Square Root
// ============================================================================
/*
LEARNING GOAL: Understand precondition verification

PROBLEM:
Write a function that safely computes the square root of a number.
- Precondition: x must be non-negative (x >= 0)
- Postcondition: result * result must be approximately equal to x
- Handle invalid input gracefully

HINTS:
1. Check precondition first
2. Return Result type for error handling
3. Verify postcondition in tests

EXPECTED OUTCOME:
- sqrt(4.0) -> Ok(2.0)
- sqrt(-1.0) -> Err("x must be non-negative")
- sqrt(9.0) -> Ok(3.0)
*/

pub fn safe_square_root(x: f64) -> Result<f64, String> {
    // TODO: Implement with precondition check
    // PRECONDITION: x >= 0.0
    if x < 0.0 {
        return Err("Precondition violated: x must be non-negative".to_string());
    }
    
    let result = x.sqrt();
    
    // POSTCONDITION: result * result ≈ x (within floating point precision)
    let epsilon = 1e-10;
    if (result * result - x).abs() > epsilon {
        return Err("Postcondition violated: sqrt computation failed".to_string());
    }
    
    Ok(result)
}

// ============================================================================
// Exercise 12.2: Array Precondition - Head Element
// ============================================================================
/*
LEARNING GOAL: Validate collection non-emptiness

PROBLEM:
Write a function that safely returns the first element of an array.
- Precondition: array must be non-empty
- Postcondition: returned element is array[0]
- Handle empty array gracefully

EXPECTED OUTCOME:
- head(&[1,2,3]) -> Ok(&1)
- head(&[]) -> Err("array is empty")
- head(&[5]) -> Ok(&5)
*/

pub fn safe_head<T: Clone>(array: &[T]) -> Result<T, String> {
    // PRECONDITION: array.len() > 0
    if array.is_empty() {
        return Err("Precondition violated: array must be non-empty".to_string());
    }
    
    let result = array[0].clone();
    
    // POSTCONDITION: result == array[0]
    // (implicit in Rust's type system, but can be verified)
    
    Ok(result)
}

// ============================================================================
// Exercise 12.3: Multiple Preconditions - Division
// ============================================================================
/*
LEARNING GOAL: Handle multiple preconditions

PROBLEM:
Write a division function with comprehensive precondition checking.
- Precondition 1: divisor must not be zero
- Precondition 2: operands must be valid (not NaN, not infinite)
- Postcondition: quotient * divisor ≈ dividend (within tolerance)

EXPECTED OUTCOME:
- divide(10, 2) -> Ok(5)
- divide(10, 0) -> Err("divisor cannot be zero")
- divide(f64::NAN, 2) -> Err("operands must be valid numbers")
*/

pub fn safe_divide(dividend: f64, divisor: f64) -> Result<f64, String> {
    // PRECONDITION 1: divisor != 0
    if divisor == 0.0 {
        return Err("Precondition 1 violated: divisor cannot be zero".to_string());
    }
    
    // PRECONDITION 2: both operands are valid numbers
    if !dividend.is_finite() || !divisor.is_finite() {
        return Err("Precondition 2 violated: operands must be valid numbers".to_string());
    }
    
    let quotient = dividend / divisor;
    
    // POSTCONDITION: quotient * divisor ≈ dividend
    let epsilon = 1e-10;
    if (quotient * divisor - dividend).abs() > epsilon {
        return Err("Postcondition violated: division result invalid".to_string());
    }
    
    Ok(quotient)
}

// ============================================================================
// Exercise 12.4: Postcondition Verification - Array Sorting
// ============================================================================
/*
LEARNING GOAL: Verify output properties

PROBLEM:
Write a function that sorts an array and verifies the postcondition.
- Precondition: none (any array is valid)
- Postcondition 1: result is sorted (a[i] <= a[i+1])
- Postcondition 2: result has same length as input
- Postcondition 3: result contains same elements as input

EXPECTED OUTCOME:
- sort(&[3,1,4,1,5]) -> Ok([1,1,3,4,5])
- Postconditions verified internally
*/

pub fn verified_sort(arr: &[i32]) -> Result<Vec<i32>, String> {
    let mut result = arr.to_vec();
    result.sort();
    
    // POSTCONDITION 1: array is sorted
    for i in 0..result.len().saturating_sub(1) {
        if result[i] > result[i + 1] {
            return Err("Postcondition 1 violated: array is not sorted".to_string());
        }
    }
    
    // POSTCONDITION 2: same length
    if result.len() != arr.len() {
        return Err("Postcondition 2 violated: length changed".to_string());
    }
    
    // POSTCONDITION 3: same elements
    let mut result_counts = HashMap::new();
    let mut input_counts = HashMap::new();
    
    for &val in &result {
        *result_counts.entry(val).or_insert(0) += 1;
    }
    for &val in arr {
        *input_counts.entry(val).or_insert(0) += 1;
    }
    
    if result_counts != input_counts {
        return Err("Postcondition 3 violated: elements changed".to_string());
    }
    
    Ok(result)
}

// ============================================================================
// Exercise 12.5: Contract with Error Classes
// ============================================================================
/*
LEARNING GOAL: Design meaningful error types

PROBLEM:
Create a custom error type and use it in contracts.

EXPECTED OUTCOME:
- PreconditionError when input invalid
- PostconditionError when output invalid
- PostconditionError::OutputOutOfRange for range violations
*/

#[derive(Debug, PartialEq)]
pub enum ContractViolation {
    PreconditionError(String),
    PostconditionError(String),
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContractViolation::PreconditionError(msg) => write!(f, "Precondition: {}", msg),
            ContractViolation::PostconditionError(msg) => write!(f, "Postcondition: {}", msg),
        }
    }
}

pub fn clamp(value: i32, min: i32, max: i32) -> Result<i32, ContractViolation> {
    // PRECONDITION: min <= max
    if min > max {
        return Err(ContractViolation::PreconditionError(
            "min must be <= max".to_string(),
        ));
    }
    
    let result = value.max(min).min(max);
    
    // POSTCONDITION: min <= result <= max
    if result < min || result > max {
        return Err(ContractViolation::PostconditionError(
            "result not in bounds".to_string(),
        ));
    }
    
    Ok(result)
}

// ============================================================================
// WEEK 13 EXERCISES: CLASS INVARIANTS
// ============================================================================

// ============================================================================
// Exercise 13.1: Simple Counter Invariant
// ============================================================================
/*
LEARNING GOAL: Enforce invariants on class state

PROBLEM:
Implement a Counter that never goes below zero.
- Invariant: count >= 0
- Method increment(n) -> increments count by n
- Method decrement(n) -> decrements count, maintaining invariant
- Method reset() -> sets count to 0

EXPECTED OUTCOME:
- Counter starts at 0
- Increment/decrement maintain invariant
- Cannot decrement below 0
*/

pub struct Counter {
    count: u32, // Using u32 ensures count >= 0 at type level
}

impl Counter {
    pub fn new() -> Self {
        Counter { count: 0 }
    }
    
    fn check_invariant(&self) -> Result<(), String> {
        // INVARIANT: count >= 0 (but this is enforced by type system)
        Ok(())
    }
    
    pub fn increment(&mut self, n: u32) -> Result<(), String> {
        self.check_invariant()?;
        
        self.count = self.count.saturating_add(n);
        
        self.check_invariant()?;
        Ok(())
    }
    
    pub fn decrement(&mut self, n: u32) -> Result<(), String> {
        self.check_invariant()?;
        
        self.count = self.count.saturating_sub(n);
        
        self.check_invariant()?;
        Ok(())
    }
    
    pub fn reset(&mut self) -> Result<(), String> {
        self.check_invariant()?;
        
        self.count = 0;
        
        self.check_invariant()?;
        Ok(())
    }
    
    pub fn get(&self) -> u32 {
        self.count
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Exercise 13.2: Bank Account Invariant
// ============================================================================
/*
LEARNING GOAL: Complex invariants with validation

PROBLEM:
Implement a BankAccount with multiple invariants.
- Invariant: balance >= 0 (cannot go negative)
- Invariant: account_number is valid (non-empty)
- Postcondition deposit: balance increases by exactly amount
- Postcondition withdraw: balance decreases by exactly amount (if successful)

EXPECTED OUTCOME:
- Deposit increases balance correctly
- Withdraw fails if insufficient funds
- Cannot withdraw more than balance
*/

pub struct BankAccount {
    account_number: String,
    balance: f64,
}

impl BankAccount {
    pub fn new(account_number: String, initial_balance: f64) -> Result<Self, String> {
        if account_number.is_empty() {
            return Err("Account number cannot be empty".to_string());
        }
        
        if initial_balance < 0.0 {
            return Err("Initial balance cannot be negative".to_string());
        }
        
        Ok(BankAccount {
            account_number,
            balance: initial_balance,
        })
    }
    
    fn check_invariants(&self) -> Result<(), String> {
        // INVARIANT 1: balance >= 0
        if self.balance < 0.0 {
            return Err("Invariant violated: balance is negative".to_string());
        }
        
        // INVARIANT 2: account_number is non-empty
        if self.account_number.is_empty() {
            return Err("Invariant violated: account_number is empty".to_string());
        }
        
        Ok(())
    }
    
    pub fn deposit(&mut self, amount: f64) -> Result<f64, String> {
        self.check_invariants()?;
        
        // PRECONDITION: amount > 0
        if amount <= 0.0 {
            return Err("Amount must be positive".to_string());
        }
        
        let old_balance = self.balance;
        self.balance += amount;
        
        // POSTCONDITION: balance increased by exactly amount
        if (self.balance - old_balance - amount).abs() > 1e-10 {
            return Err("Postcondition violated: balance did not increase correctly".to_string());
        }
        
        self.check_invariants()?;
        Ok(self.balance)
    }
    
    pub fn withdraw(&mut self, amount: f64) -> Result<f64, String> {
        self.check_invariants()?;
        
        // PRECONDITION: amount > 0
        if amount <= 0.0 {
            return Err("Amount must be positive".to_string());
        }
        
        // PRECONDITION: sufficient balance
        if self.balance < amount {
            return Err("Insufficient balance".to_string());
        }
        
        let old_balance = self.balance;
        self.balance -= amount;
        
        // POSTCONDITION: balance decreased by exactly amount
        if (old_balance - self.balance - amount).abs() > 1e-10 {
            return Err("Postcondition violated: balance did not decrease correctly".to_string());
        }
        
        self.check_invariants()?;
        Ok(self.balance)
    }
    
    pub fn get_balance(&self) -> f64 {
        self.balance
    }
    
    pub fn get_account_number(&self) -> &str {
        &self.account_number
    }
}

// ============================================================================
// Exercise 13.3: Stack Invariant
// ============================================================================
/*
LEARNING GOAL: Data structure invariants

PROBLEM:
Implement a Stack with invariants.
- Invariant: size >= 0
- Invariant: size <= capacity
- Invariant: top pointer valid
- Postcondition push: size increases by 1
- Postcondition pop: size decreases by 1, returns element
- Postcondition peek: size unchanged

EXPECTED OUTCOME:
- Push/pop maintain invariants
- Cannot pop from empty stack
- Cannot push to full stack (if bounded)
*/

pub struct Stack<T> {
    data: Vec<T>,
    capacity: usize,
}

impl<T> Stack<T> {
    pub fn new(capacity: usize) -> Self {
        Stack {
            data: Vec::new(),
            capacity,
        }
    }
    
    fn check_invariants(&self) -> Result<(), String> {
        // INVARIANT 1: size >= 0
        if self.data.len() > i32::MAX as usize {
            return Err("Invariant violated: size < 0".to_string());
        }
        
        // INVARIANT 2: size <= capacity
        if self.data.len() > self.capacity {
            return Err("Invariant violated: size exceeds capacity".to_string());
        }
        
        Ok(())
    }
    
    pub fn push(&mut self, item: T) -> Result<(), String> {
        self.check_invariants()?;
        
        // PRECONDITION: size < capacity
        if self.data.len() >= self.capacity {
            return Err("Stack is full".to_string());
        }
        
        let old_size = self.data.len();
        self.data.push(item);
        
        // POSTCONDITION: size increased by 1
        if self.data.len() != old_size + 1 {
            return Err("Postcondition violated: size did not increase by 1".to_string());
        }
        
        self.check_invariants()?;
        Ok(())
    }
    
    pub fn pop(&mut self) -> Result<T, String> {
        self.check_invariants()?;
        
        // PRECONDITION: stack not empty
        if self.data.is_empty() {
            return Err("Stack is empty".to_string());
        }
        
        let old_size = self.data.len();
        let result = self.data.pop().unwrap();
        
        // POSTCONDITION: size decreased by 1
        if self.data.len() != old_size - 1 {
            return Err("Postcondition violated: size did not decrease by 1".to_string());
        }
        
        self.check_invariants()?;
        Ok(result)
    }
    
    pub fn peek(&self) -> Result<&T, String> {
        self.check_invariants()?;
        
        // PRECONDITION: stack not empty
        if self.data.is_empty() {
            return Err("Stack is empty".to_string());
        }
        
        Ok(&self.data[self.data.len() - 1])
    }
    
    pub fn size(&self) -> usize {
        self.data.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// ============================================================================
// WEEK 14 EXERCISES: AUTOMATED VERIFICATION
// ============================================================================

// ============================================================================
// Exercise 14.1: Test Case Generation from Contract
// ============================================================================
/*
LEARNING GOAL: Generate comprehensive test cases

PROBLEM:
Given a contract, generate test cases.
Contract for clamp(value, min, max):
- Precondition 1: min <= max
- Precondition 2: -1000 <= min, max <= 1000
- Postcondition: min <= result <= max

Generate test cases:
1. Valid boundaries
2. Valid interior points
3. Precondition violations
4. Postcondition verification
*/

pub struct TestCase {
    name: String,
    input: (i32, i32, i32), // (value, min, max)
    expected_result: Result<i32, String>,
}

impl TestCase {
    pub fn new(name: &str, input: (i32, i32, i32), expected: Result<i32, String>) -> Self {
        TestCase {
            name: name.to_string(),
            input,
            expected_result: expected,
        }
    }
    
    pub fn run(&self) -> bool {
        let (value, min, max) = self.input;
        let result = clamp(value, min, max);
        
        match (&result, &self.expected_result) {
            (Ok(actual), Ok(expected)) => actual == expected,
            (Err(_), Err(_)) => true, // Both errors, accept
            _ => false,
        }
    }
}

pub fn generate_clamp_tests() -> Vec<TestCase> {
    vec![
        // Valid cases - value in range
        TestCase::new("value in range", (50, 0, 100), Ok(50)),
        
        // Valid cases - boundary: value at min
        TestCase::new("value at min", (0, 0, 100), Ok(0)),
        
        // Valid cases - boundary: value at max
        TestCase::new("value at max", (100, 0, 100), Ok(100)),
        
        // Valid cases - value below min
        TestCase::new("value below min", (-50, 0, 100), Ok(0)),
        
        // Valid cases - value above max
        TestCase::new("value above max", (150, 0, 100), Ok(100)),
        
        // Invalid: min > max
        TestCase::new("min > max", (50, 100, 0), Err("invalid".to_string())),
    ]
}

// ============================================================================
// Exercise 14.2: Verification Framework
// ============================================================================
/*
LEARNING GOAL: Build automated verification system

PROBLEM:
Implement a framework that:
1. Registers contracts
2. Tests functions against contracts
3. Reports violations
4. Generates statistics
*/

pub struct FunctionSpec {
    name: String,
    check_preconditions: Box<dyn Fn(&str) -> bool>,
    check_postconditions: Box<dyn Fn(&str) -> bool>,
}

pub struct VerificationFramework {
    specs: HashMap<String, FunctionSpec>,
    tests_passed: u64,
    tests_failed: u64,
}

impl VerificationFramework {
    pub fn new() -> Self {
        VerificationFramework {
            specs: HashMap::new(),
            tests_passed: 0,
            tests_failed: 0,
        }
    }
    
    pub fn verify(&mut self, func_name: &str, input: &str, output: &str) -> bool {
        if let Some(spec) = self.specs.get(func_name) {
            let pre_ok = (spec.check_preconditions)(input);
            let post_ok = (spec.check_postconditions)(output);
            
            if pre_ok && post_ok {
                self.tests_passed += 1;
                true
            } else {
                self.tests_failed += 1;
                false
            }
        } else {
            self.tests_failed += 1;
            false
        }
    }
    
    pub fn get_stats(&self) -> (u64, u64) {
        (self.tests_passed, self.tests_failed)
    }
}

impl Default for VerificationFramework {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Unit Tests for Exercises
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_safe_square_root_precondition() {
        assert!(safe_square_root(4.0).is_ok());
        assert!(safe_square_root(-1.0).is_err());
    }
    
    #[test]
    fn test_safe_head_precondition() {
        assert!(safe_head(&[1, 2, 3]).is_ok());
        assert!(safe_head::<i32>(&[]).is_err());
    }
    
    #[test]
    fn test_safe_divide() {
        assert!(safe_divide(10.0, 2.0).is_ok());
        assert!(safe_divide(10.0, 0.0).is_err());
    }
    
    #[test]
    fn test_verified_sort() {
        let result = verified_sort(&[3, 1, 4, 1, 5]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 1, 3, 4, 5]);
    }
    
    #[test]
    fn test_clamp() {
        assert_eq!(clamp(50, 0, 100), Ok(50));
        assert_eq!(clamp(-50, 0, 100), Ok(0));
        assert_eq!(clamp(150, 0, 100), Ok(100));
        assert!(clamp(50, 100, 0).is_err());
    }
    
    #[test]
    fn test_counter() {
        let mut counter = Counter::new();
        assert_eq!(counter.get(), 0);
        counter.increment(5).unwrap();
        assert_eq!(counter.get(), 5);
        counter.decrement(3).unwrap();
        assert_eq!(counter.get(), 2);
    }
    
    #[test]
    fn test_bank_account() {
        let mut account = BankAccount::new("ACC123".to_string(), 100.0).unwrap();
        assert_eq!(account.get_balance(), 100.0);
        
        account.deposit(50.0).unwrap();
        assert_eq!(account.get_balance(), 150.0);
        
        account.withdraw(30.0).unwrap();
        assert_eq!(account.get_balance(), 120.0);
        
        assert!(account.withdraw(200.0).is_err());
    }
    
    #[test]
    fn test_stack() {
        let mut stack: Stack<i32> = Stack::new(3);
        
        stack.push(1).unwrap();
        stack.push(2).unwrap();
        assert_eq!(stack.peek(), Ok(&2));
        
        assert_eq!(stack.pop(), Ok(2));
        assert_eq!(stack.pop(), Ok(1));
        assert!(stack.pop().is_err());
    }
    
    #[test]
    fn test_verification_framework() {
        let mut framework = VerificationFramework::new();
        let (passed, failed) = framework.get_stats();
        assert_eq!(passed, 0);
        assert_eq!(failed, 0);
    }
}
