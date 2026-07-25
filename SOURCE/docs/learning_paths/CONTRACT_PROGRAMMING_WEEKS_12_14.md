# Contract Programming (Weeks 12-14)
## Formal Methods, Design-by-Contract, and Automated Verification
**Target: 400+ problems | ~1200 hours | Expert Level**

---

## Module Overview

### Week 12: Preconditions and Postconditions (120 problems)
**Goal:** Master formal assertions at function boundaries

#### Core Concepts
1. **Preconditions** - What must be true BEFORE execution
   - Input parameter validation
   - State requirements
   - Environmental conditions
   - Examples:
     - `x > 0` for square root
     - `array.len() > 0` for head()
     - `balance >= amount` for withdrawal

2. **Postconditions** - What must be true AFTER execution
   - Output guarantees
   - State changes
   - Side effects
   - Examples:
     - `return_value >= 0` for positive computation
     - `new_array.len() == old_array.len() - 1` for remove()
     - `balance == old_balance - amount` for withdrawal

3. **Assertion Contract Pattern**
   ```rust
   fn guaranteed_operation(x: i32) {
       // PRECONDITION: x > 0
       assert!(x > 0, "precondition failed: x must be positive");
       
       let result = x * 2;
       
       // POSTCONDITION: result >= 2
       assert!(result >= 2, "postcondition failed: result must be >= 2");
   }
   ```

#### Problem Categories (120 problems)
- **30 problems:** Basic precondition verification
  - Range validation (0-100 score, age 0-150)
  - Type validation (non-null, non-empty)
  - Relationship validation (x < y, width > 0)
  
- **30 problems:** Basic postcondition verification
  - Output property checking (sorted, unique)
  - Size guarantees (output.len() == input.len() + 1)
  - Value guarantees (output in range)
  
- **30 problems:** Combined pre+postconditions
  - Integer division (non-zero divisor, integer result)
  - Array operations (non-empty input, same-size output)
  - String manipulation (input validation, output property)
  
- **30 problems:** Contract violation detection
  - Identifying violated preconditions
  - Identifying violated postconditions
  - Creating meaningful error messages

#### Example Problems (Week 12)

**Problem 12.1: Square Root Precondition**
```
Given x, ensure x >= 0 before computing sqrt
Write a function that:
1. Checks precondition: x >= 0
2. Computes sqrt(x)
3. Returns result
```

**Problem 12.2: Array Reverse Postcondition**
```
Given array, reverse it
Postcondition: if original[i] == x, then result[n-1-i] == x
Verify postcondition for all elements
```

**Problem 12.3: Division Contract**
```
Implement safe division
Precondition: divisor != 0
Postcondition: quotient * divisor + remainder == dividend
Verify both conditions
```

**Problem 12.4: String Uppercase Postcondition**
```
Convert string to uppercase
Postcondition: all alphabetic characters are uppercase
Postcondition: non-alphabetic characters unchanged
Verify both postconditions
```

---

### Week 13: Class Invariants and Object Contracts (140 problems)
**Goal:** Enforce consistency throughout object lifetime

#### Core Concepts
1. **Class Invariants** - Assertions that hold before AND after every public method
   - Example: Bank account balance >= 0 (always)
   - Example: Stack size >= 0 and <= capacity (always)
   - Example: Tree left_height <= right_height + 1 (always)

2. **Invariant Pattern**
   ```rust
   struct BankAccount {
       balance: f64,
   }
   
   impl BankAccount {
       fn invariant_check(&self) -> bool {
           self.balance >= 0.0  // INVARIANT: balance never negative
       }
       
       pub fn deposit(&mut self, amount: f64) {
           assert!(self.invariant_check(), "invariant violated!");
           self.balance += amount;
           assert!(self.invariant_check(), "invariant violated!");
       }
       
       pub fn withdraw(&mut self, amount: f64) {
           assert!(self.invariant_check(), "invariant violated!");
           if amount <= self.balance {
               self.balance -= amount;
           }
           assert!(self.invariant_check(), "invariant violated!");
       }
   }
   ```

3. **Method Contracts** - Contracts specific to methods
   - Preconditions for the method
   - Postconditions for the method
   - Invariants must hold before and after

4. **Inheritance Contracts** (Liskov Substitution Principle)
   - Derived class preconditions <= base class preconditions
   - Derived class postconditions >= base class postconditions
   - Both must maintain class invariants

#### Problem Categories (140 problems)

- **35 problems:** Simple invariants
  - Counter never negative
  - Queue size always valid
  - Stack pointer valid
  - List length consistent with element count
  
- **35 problems:** Invariants with preconditions
  - Withdraw only if balance sufficient
  - Pop only if queue non-empty
  - Access element only if index in range
  
- **35 problems:** Invariants with postconditions
  - After deposit, balance increases exactly by amount
  - After insert, size increases by 1
  - After delete, size decreases by 1
  
- **35 problems:** Complex invariants
  - Binary search tree property (left < parent < right)
  - Heap property (parent <= children)
  - Sorted list property
  - Balanced tree property

#### Example Problems (Week 13)

**Problem 13.1: Bank Account Invariant**
```
Invariant: balance >= 0
Methods: deposit(+), withdraw(-), transfer(-, +)
Each method must:
1. Verify invariant before execution
2. Execute operation
3. Verify invariant after execution
4. Return error if invariant violated
```

**Problem 13.2: Stack Invariant**
```
Invariants:
- size >= 0 and <= capacity
- top >= -1 and < capacity (or appropriate bounds)
Methods: push, pop, peek
All must maintain invariants
```

**Problem 13.3: Binary Search Tree Invariant**
```
Invariant: for all nodes, left < node <= right
Contract for insert(value):
- Precondition: tree in valid state
- Postcondition: value inserted correctly, invariant holds
Contract for delete(value):
- Precondition: tree in valid state
- Postcondition: value removed, invariant holds
```

**Problem 13.4: Sorted List Invariant**
```
Invariant: list always sorted in ascending order
Methods: insert, remove, replace
Each method must maintain sort order invariant
```

---

### Week 14: Automated Verification and Testing (140 problems)
**Goal:** Systematically verify contracts at runtime

#### Core Concepts
1. **Automated Contract Verification**
   - Test oracle generation
   - Property-based testing
   - Coverage analysis
   - Mutation testing

2. **Verification Framework**
   ```rust
   struct ContractVerifier {
       preconditions: Vec<Box<dyn Fn(&T) -> bool>>,
       postconditions: Vec<Box<dyn Fn(&R) -> bool>>,
       invariants: Vec<Box<dyn Fn(&T) -> bool>>,
   }
   
   impl ContractVerifier {
       fn verify(&self, input: &T, output: &R) -> Result<(), String> {
           // Check all preconditions
           for pre in &self.preconditions {
               if !pre(input) {
                   return Err("Precondition failed".into());
               }
           }
           
           // Check all postconditions
           for post in &self.postconditions {
               if !post(output) {
                   return Err("Postcondition failed".into());
               }
           }
           
           Ok(())
       }
   }
   ```

3. **Test Case Generation**
   - Boundary values
   - Random values
   - Edge cases
   - Invalid inputs

4. **Contracts as Specifications**
   - Direct method of converting requirements to code
   - Specifications become test oracles
   - Gap analysis: contract vs. implementation

#### Problem Categories (140 problems)

- **35 problems:** Test case generation
  - Identify boundary values for contracts
  - Generate test cases from preconditions
  - Generate test cases from postconditions
  - Create comprehensive test suites
  
- **35 problems:** Verification framework implementation
  - Implement precondition checker
  - Implement postcondition checker
  - Implement invariant checker
  - Implement combined verification
  
- **35 problems:** Property-based testing
  - Properties that hold for all valid inputs
  - Properties that relate input to output
  - Invariant properties that must always hold
  - Differential testing: two implementations
  
- **35 problems:** Contract-driven development
  - Write contracts first
  - Generate test suite from contracts
  - Implement to satisfy contracts
  - Verify implementation against contracts

#### Example Problems (Week 14)

**Problem 14.1: Contract-Based Test Generation**
```
Given contract:
- Precondition: x >= 0
- Precondition: x <= 100
- Postcondition: 0 <= result <= 10

Generate:
1. Valid test cases
2. Invalid test cases (violating preconditions)
3. Test cases to verify postconditions
4. Edge cases: 0, 100, 50
```

**Problem 14.2: Invariant-Based Testing**
```
Component: Queue
Invariant: size >= 0 and size <= capacity
Methods: enqueue, dequeue, peek

Generate tests that:
1. Create operations that violate invariant
2. Verify invariant after each operation
3. Test boundary conditions (empty, full)
4. Test operation sequences
```

**Problem 14.3: Property-Based Testing**
```
Property: sort(sort(list)) == sort(list) (idempotence)
Property: len(sort(list)) == len(list) (length preservation)
Property: all elements in output are from input

Write generator that creates diverse test cases
and verifies properties hold
```

**Problem 14.4: Mutation Testing**
```
Implement mutation tester:
1. Mutate contract definitions
2. Verify test suite catches mutations
3. Identify under-tested contracts
4. Generate additional tests
```

---

## Learning Path Progression

### Week 12 Progression (Preconditions → Postconditions)
```
Day 1-2: Preconditions (15 problems)
  - Simple value ranges
  - Type validation
  - State requirements

Day 3-4: Postconditions (15 problems)
  - Output validation
  - State change verification
  - Side effect checking

Day 5: Mixed Pre+Post (10 problems)
  - Functions with both contracts
  - Multi-parameter validation
  - Complex assertions

Day 6-7: Advanced (10 problems)
  - Violation scenarios
  - Error messages
  - Multiple violations
```

### Week 13 Progression (Simple → Complex Invariants)
```
Day 1-2: Simple Invariants (20 problems)
  - Single property invariants
  - State constraints
  - Basic enforcement

Day 3-4: Method-Specific Contracts (20 problems)
  - Method preconditions
  - Method postconditions
  - Invariant preservation

Day 5: Advanced Invariants (20 problems)
  - Structural properties (BST, Heap)
  - Relationship invariants
  - Complex consistency rules

Day 6-7: Inheritance & LSP (10 problems)
  - Derived class contracts
  - Substitution principle
  - Contract compatibility
```

### Week 14 Progression (Manual → Automated)
```
Day 1-2: Basic Verification (20 problems)
  - Test case identification
  - Oracle generation
  - Result validation

Day 3-4: Framework Implementation (20 problems)
  - Build verification backends
  - Implement checkers
  - Report violations

Day 5: Property-Based Testing (20 problems)
  - Generic properties
  - Relationship verification
  - Statistical testing

Day 6-7: Advanced Automation (10 problems)
  - Enterprise verification
  - Performance considerations
  - Real-world patterns
```

---

## Implementation Deep Dives

### Deep Dive 1: Precondition Patterns
```
1. Simple checks: x > 0
2. Compound checks: (x > 0) && (x < 100)
3. Dependent checks: y > 0 => x > y
4. State-dependent: (list.len() > 0) only if list is non-null
5. Resource checks: file must be open, connection must exist
```

### Deep Dive 2: Contract Composition
```
1. Sequential contracts: pre1, pre2, pre3
2. Alternative contracts: (pre1 OR pre2)
3. Conditional contracts: if condition then pre
4. Exception contracts: specify what exceptions can be raised
5. Performance contracts: must complete in < time
```

### Deep Dive 3: Invariant Categories
```
1. Value invariants: x >= 0
2. Relationship invariants: y > x
3. Structural invariants: tree is balanced
4. Quantified invariants: for all i, a[i] >= 0
5. Aggregate invariants: sum of values matches total
```

### Deep Dive 4: Verification Strategies
```
1. Static verification (compile-time if possible)
2. Dynamic verification (runtime checks)
3. Hybrid verification (some static, some dynamic)
4. Assumption-based verification (assume inputs satisfy pre)
5. Defensive verification (check everything, assume nothing)
```

---

## Advanced Topics

### 1. Formal Specification Languages
```
- Z notation
- B method
- Alloy
- TLA+ (Temporal Logic of Actions)
```

### 2. Model Checking
```
- State exploration
- Invariant verification
- Temporal properties
- Tools: SPIN, LTL model checkers
```

### 3. Theorem Proving
```
- Interactive proofs
- Automated theorem provers
- Proof assistants (Coq, Agda, Lean)
```

### 4. Specialized Contract Extensions
```
- Depends clauses (dependencies between values)
- Ensures clauses (return value guarantees)
- Modifies clauses (what can be modified)
- Signals clauses (exception specifications)
```

---

## Integration with Other Modules

### With Week 11 (Error Handling):
- Contracts define what errors are possible
- Error handling implements contract recovery
- Examples: Division by zero, index out of bounds

### With Week 15 (Concurrency):
- Thread-safe invariants
- Synchronized method contracts
- Race condition detection through contract violation

### With Week 16+ (Systems):
- Large-scale contract frameworks
- Performance optimization of verification
- Enterprise deployment of contract systems

---

## Real-World Applications

### 1. Database Transactions
```
Contract:
- Precondition: transaction is open
- Postcondition: all-or-nothing (ACID properties)
- Invariant: consistency maintained
```

### 2. API Contracts
```
Contract:
- Precondition: valid authentication token
- Postcondition: response format matches spec
- Invariant: rate limits maintained
```

### 3. Concurrent Data Structures
```
Contract:
- Precondition: proper synchronization
- Postcondition: element added/removed correctly
- Invariant: internal consistency
```

### 4. Cryptographic Operations
```
Contract:
- Precondition: key is valid length
- Postcondition: encryption/decryption works
- Invariant: IV is never reused
```

---

## Assessment Criteria

### Problem Solving (60%)
- Correctly identify contracts
- Implement verification logic
- Generate comprehensive test cases
- Handle violations appropriately

### Code Quality (20%)
- Clear contract specifications
- Efficient verification
- Good error messages
- Proper test coverage

### Theory Understanding (20%)
- Explain contract principles
- Apply contracts to new domains
- Design contract hierarchies
- Optimize verification strategies

---

## Recommended Project: Contract-Driven Banking System

A comprehensive project covering all weeks:

**Contracts:**
- Account: balance >= 0
- Transfer: (from.balance >= amount) => (from.balance -= amount AND to.balance += amount)
- Audit: all transactions logged

**Implementation:**
1. Define all contracts
2. Implement with contract checks
3. Build verification framework
4. Generate test suite
5. Verify against contracts
6. Performance optimization

---

## Resources

### Books
- "Design by Contract" - Bertrand Meyer
- "Code Complete" - Steve McConnell (contracts chapter)
- "Working with Contract-Based Programming" - Jean-Claude Royer & others

### Tools
- Eiffel (original DbC language)
- Rust (assertions, debug_assert!)
- Python (assert statements)
- Java (AssertionError)
- C++ (static_assert, assert)

### Research
- original Meyer papers on DbC
- Liskov & Wing: "Behavioral Subtyping Using Invariants and Constraints"
- Contract frameworks literature

---

## Success Metrics

By end of Week 14, you should:
1. ✓ Write contracts for any function
2. ✓ Identify and fix contract violations
3. ✓ Design class invariants
4. ✓ Build verification frameworks
5. ✓ Generate test suites from contracts
6. ✓ Apply contracts to real problems
7. ✓ Explain contract benefits and tradeoffs
8. ✓ Optimize contract verification

**Mastery = 300+ problems solved + robust understanding of formal verification**
