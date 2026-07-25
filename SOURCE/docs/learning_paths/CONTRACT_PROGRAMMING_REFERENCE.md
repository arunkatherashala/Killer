# Contract Programming: Comprehensive Reference & Quick Guide
## For Weeks 12-14 Learning Module

---

## TABLE OF CONTENTS
1. Core Concepts
2. Precondition Patterns
3. Postcondition Patterns
4. Class Invariant Patterns
5. Common Mistakes & Fixes
6. Real-World Applications
7. Performance Considerations
8. Integration with Other Patterns
9. Tools & Frameworks
10. Further Reading

---

## 1. CORE CONCEPTS

### 1.1 Design by Contract (DbC)

**Definition:** A formal approach to software design where functions/classes define explicit contracts with their callers.

**Three Components:**
1. **Preconditions** - What client must ensure before calling
2. **Postconditions** - What provider guarantees after execution
3. **Invariants** - What must always be true

**Philosophical Principle:**
```
If precondition violated -> caller's fault
If precondition satisfied but postcondition violated -> provider's fault
```

### 1.2 Why Contract Programming?

**Benefits:**
- Clear specifications
- Early error detection
- Better testing
- Self-documenting code
- Reduced debugging time
- Formalized requirements

**Costs:**
- Overhead of checking
- More code to write
- Learning curve
- False sense of security

---

## 2. PRECONDITION PATTERNS

### Pattern 2.1: Simple Validation

**Pattern:**
```rust
fn operation(x: i32) -> Result<Output, Error> {
    // PRECONDITION: x > 0
    if x <= 0 {
        return Err("x must be positive");
    }
    // ... operation ...
}
```

**When to use:**
- Single condition
- Easy to check
- Caller can easily satisfy

**Examples:**
- Square root: x >= 0
- Array access: 0 <= index < len
- Division: divisor != 0

### Pattern 2.2: Complex Validation

**Pattern:**
```rust
fn operation(x: i32, y: i32) -> Result<Output, Error> {
    // PRECONDITION 1: x >= 0
    if x < 0 { return Err("x must be non-negative"); }
    
    // PRECONDITION 2: y >= 0
    if y < 0 { return Err("y must be non-negative"); }
    
    // PRECONDITION 3: x + y < 1000
    if x + y >= 1000 { return Err("sum too large"); }
    
    // ... operation ...
}
```

**When to use:**
- Multiple independent conditions
- Failures have different meanings
- Each condition important

**Examples:**
- Rectangle creation: width > 0, height > 0, area < MAX
- Date validation: month 1-12, day 1-31, year > 0
- Credit rating: score 0-1000, account active

### Pattern 2.3: Dependent Conditions

**Pattern:**
```rust
fn operation(a: i32, b: i32) -> Result<Output, Error> {
    // PRECONDITION: if a > 0 then b must be >= a
    if a > 0 && b < a {
        return Err("if a > 0, b must be >= a");
    }
    // ... operation ...
}
```

**When to use:**
- Conditions depend on each other
- Complex boolean logic
- Business rule constraints

**Examples:**
- Time range: if start_time set, end_time must be > start_time
- Discount: if rate > 0%, maximum_discount must be >= rate
- Access: if user == admin, permissions must include DELETE

### Pattern 2.4: State-Based Preconditions

**Pattern:**
```rust
struct Resource {
    is_open: bool,
    data: Vec<u8>,
}

impl Resource {
    fn read(&self) -> Result<Vec<u8>, Error> {
        // PRECONDITION: resource must be open
        if !self.is_open {
            return Err("resource not open");
        }
        Ok(self.data.clone())
    }
}
```

**When to use:**
- Precondition about object state
- Resource management (open/closed, locked/unlocked)
- State machines

**Examples:**
- File operations: file must be open
- Database transactions: transaction must be active
- Thread synchronization: lock must be held

### Pattern 2.5: Defensive Preconditions

**Pattern:**
```rust
fn operation(obj: &Option<T>, value: i32) -> Result<Output, Error> {
    // PRECONDITION: obj must contain Some value
    let obj = obj.as_ref()
        .ok_or("object is None")?;
    
    // PRECONDITION: value in valid range
    if value < 0 || value > 100 {
        return Err("value must be 0-100");
    }
    
    // ... operation ...
}
```

**When to use:**
- Nullable/optional values
- Multiple conditions to check
- Defensive programming approach

---

## 3. POSTCONDITION PATTERNS

### Pattern 3.1: Simple Output Verification

**Pattern:**
```rust
fn sqrt(x: f64) -> Result<f64, Error> {
    if x < 0.0 { return Err("precondition"); }
    
    let result = x.sqrt();
    
    // POSTCONDITION: result >= 0
    if result < 0.0 {
        return Err("postcondition: result negative");
    }
    
    Ok(result)
}
```

**When to use:**
- Output property verification
- Type-level guarantees insufficient
- Important properties to verify

### Pattern 3.2: Transformation Verification

**Pattern:**
```rust
fn reverse(arr: &[i32]) -> Result<Vec<i32>, Error> {
    let result = arr.iter().rev().cloned().collect::<Vec<_>>();
    
    // POSTCONDITION: reversed correctly
    for i in 0..arr.len() {
        if result[i] != arr[arr.len() - 1 - i] {
            return Err("postcondition: reversal failed");
        }
    }
    
    // POSTCONDITION: same length
    if result.len() != arr.len() {
        return Err("postcondition: length changed");
    }
    
    Ok(result)
}
```

**When to use:**
- Verify transformation properties
- Element-by-element verification
- Relationship between input and output

### Pattern 3.3: State Change Verification

**Pattern:**
```rust
fn append(mut list: Vec<i32>, item: i32) -> Result<Vec<i32>, Error> {
    let old_len = list.len();
    list.push(item);
    
    // POSTCONDITION 1: length increased by 1
    if list.len() != old_len + 1 {
        return Err("postcondition 1: length not increased");
    }
    
    // POSTCONDITION 2: last element is item
    if list[list.len() - 1] != item {
        return Err("postcondition 2: item not appended");
    }
    
    Ok(list)
}
```

**When to use:**
- Mutations and state changes
- Collection operations
- Resource management

### Pattern 3.4: Relationship Verification

**Pattern:**
```rust
fn find_max(arr: &[i32]) -> Result<i32, Error> {
    if arr.is_empty() { return Err("precondition"); }
    
    let result = *arr.iter().max().unwrap();
    
    // POSTCONDITION 1: result is in the array
    if !arr.contains(&result) {
        return Err("postcondition 1: result not in array");
    }
    
    // POSTCONDITION 2: result >= all elements
    for &elem in arr {
        if result < elem {
            return Err("postcondition 2: result not maximum");
        }
    }
    
    Ok(result)
}
```

**When to use:**
- Relationships between values
- Ordering properties
- Comparisons

### Pattern 3.5: Compositional Postcondition

**Pattern:**
```rust
fn parse_and_validate(input: &str) -> Result<ParsedData, Error> {
    let parsed = parse(input)?;
    
    // POSTCONDITION: parsed can be re-serialized to original
    let reserialized = serialize(&parsed)?;
    
    if reserialized != input {
        return Err("postcondition: round-trip failed");
    }
    
    Ok(parsed)
}
```

**When to use:**
- Round-trip verification
- Bidirectional operations
- Format validation

---

## 4. CLASS INVARIANT PATTERNS

### Pattern 4.1: Simple Invariant

**Pattern:**
```rust
struct Counter {
    count: u32,  // Type enforces count >= 0
}

impl Counter {
    fn check_invariants(&self) -> Result<(), Error> {
        // INVARIANT: count >= 0 (enforced by type)
        Ok(())
    }
    
    pub fn increment(&mut self, n: u32) -> Result<(), Error> {
        self.check_invariants()?;
        self.count = self.count.saturating_add(n);
        self.check_invariants()?;
        Ok(())
    }
}
```

**When to use:**
- Simple value constraints
- Can be enforced by types
- Straightforward to verify

### Pattern 4.2: Structural Invariant

**Pattern:**
```rust
struct BinarySearchTree {
    value: i32,
    left: Option<Box<BST>>,
    right: Option<Box<BST>>,
}

impl BST {
    fn check_invariants(&self) -> Result<(), Error> {
        // INVARIANT: left < value and right > value
        if let Some(left) = &self.left {
            if left.value >= self.value {
                return Err("invariant: left >= parent");
            }
            left.check_invariants()?; // Recursive check
        }
        
        if let Some(right) = &self.right {
            if right.value <= self.value {
                return Err("invariant: right <= parent");
            }
            right.check_invariants()?; // Recursive check
        }
        
        Ok(())
    }
}
```

**When to use:**
- Data structure properties
- Recursive structures
- Complex relationships

### Pattern 4.3: Aggregate Invariant

**Pattern:**
```rust
struct Portfolio {
    positions: Vec<Position>,
    total_value: f64,
}

impl Portfolio {
    fn check_invariants(&self) -> Result<(), Error> {
        // INVARIANT: total_value == sum of position values
        let computed_total: f64 = self.positions
            .iter()
            .map(|p| p.value)
            .sum();
        
        if (computed_total - self.total_value).abs() > 0.01 {
            return Err("invariant: cached total incorrect");
        }
        
        Ok(())
    }
}
```

**When to use:**
- Cached values must match computed
- Aggregate properties
- Summary fields

### Pattern 4.4: Consistency Invariant

**Pattern:**
```rust
struct UserAccount {
    id: i32,
    email: String,
    verified: bool,
    verification_code: String,
}

impl UserAccount {
    fn check_invariants(&self) -> Result<(), Error> {
        // INVARIANT: if verified then verification_code is empty
        if self.verified && !self.verification_code.is_empty() {
            return Err("invariant: verified but has code");
        }
        
        // INVARIANT: if not verified then verification_code not empty
        if !self.verified && self.verification_code.is_empty() {
            return Err("invariant: not verified and no code");
        }
        
        Ok(())
    }
}
```

**When to use:**
- Related fields must be consistent
- State consistency checks
- Interdependent fields

### Pattern 4.5: Quantified Invariant

**Pattern:**
```rust
struct SortedList {
    items: Vec<i32>,
}

impl SortedList {
    fn check_invariants(&self) -> Result<(), Error> {
        // INVARIANT: for all i, items[i] <= items[i+1]
        for i in 0..self.items.len().saturating_sub(1) {
            if self.items[i] > self.items[i + 1] {
                return Err("invariant: not sorted");
            }
        }
        
        Ok(())
    }
}
```

**When to use:**
- Universal quantification (for all)
- Sequence properties
- Collection invariants

---

## 5. COMMON MISTAKES & FIXES

### Mistake 5.1: Missing Precondition Check

**❌ Wrong:**
```rust
fn head(arr: &[i32]) -> i32 {
    arr[0]  // Panics if arr is empty!
}
```

**✓ Correct:**
```rust
fn head(arr: &[i32]) -> Result<i32, Error> {
    if arr.is_empty() {
        return Err("array empty");
    }
    Ok(arr[0])
}
```

### Mistake 5.2: Incomplete Postcondition Verification

**❌ Wrong:**
```rust
fn sort(arr: &[i32]) -> Vec<i32> {
    let mut result = arr.to_vec();
    result.sort();
    result  // What if sort didn't work?
}
```

**✓ Correct:**
```rust
fn sort(arr: &[i32]) -> Result<Vec<i32>, Error> {
    let mut result = arr.to_vec();
    result.sort();
    
    // Verify postcondition
    for i in 0..result.len().saturating_sub(1) {
        if result[i] > result[i+1] {
            return Err("sort failed");
        }
    }
    
    Ok(result)
}
```

### Mistake 5.3: Invariant Check Only at Entry

**❌ Wrong:**
```rust
impl Stack {
    pub fn push(&mut self, item: T) {
        self.check_invariants(); // Only at start
        self.data.push(item);
        // Forgot to check again!
    }
}
```

**✓ Correct:**
```rust
impl Stack {
    pub fn push(&mut self, item: T) -> Result<(), Error> {
        self.check_invariants()?;  // Before
        self.data.push(item);
        self.check_invariants()?;  // After
        Ok(())
    }
}
```

### Mistake 5.4: Overly Complex Contracts

**❌ Wrong:**
```rust
fn operation(a: i32, b: i32, c: i32, d: i32) -> Result<Output, Error> {
    // 10 different preconditions, all mixed together
    if a < 0 || b > 100 || (c == 0 && d != 0) || ... {
        return Err("failed");
    }
}
```

**✓ Correct:**
```rust
fn operation(a: i32, b: i32, c: i32, d: i32) -> Result<Output, Error> {
    // Separate concerns
    validate_a(a)?;
    validate_b(b)?;
    validate_c_d(c, d)?;
    // Continue
}

fn validate_a(a: i32) -> Result<(), Error> {
    if a < 0 { Err("a must be non-negative") }
    else { Ok(()) }
}
```

### Mistake 5.5: Breaking Invariants in Intermediate States

**❌ Wrong:**
```rust
impl BankAccount {
    pub fn transfer(&mut self, amount: f64, to: &mut BankAccount) {
        self.balance -= amount;  // Breaks invariant temporarily!
        to.balance += amount;     // Invariant restored here
    }
}
```

**✓ Correct:**
```rust
impl BankAccount {
    pub fn transfer(&mut self, amount: f64, to: &mut BankAccount) -> Result<(), Error> {
        self.check_invariants()?;
        
        // Atomic operation
        if self.balance >= amount {
            self.balance -= amount;
            to.balance += amount;
            
            self.check_invariants()?;
            to.check_invariants()?;
            Ok(())
        } else {
            Err("insufficient funds")
        }
    }
}
```

---

## 6. REAL-WORLD APPLICATIONS

### Application 6.1: Financial Systems

**Contracts:**
```rust
// Precondition: amount >= 0.01 (minimum transaction)
// Precondition: account has sufficient balance
// Precondition: not already in transaction
// Postcondition: balance decreased exactly by amount
// Invariant: balance >= 0
```

**Benefits:**
- Prevents negative balances
- Catches underflow errors
- Auditable transactions
- Regulatory compliance

### Application 6.2: Medical Software

**Contracts:**
```rust
// Precondition: medication_id is valid
// Precondition: dose >= minimum_dose AND dose <= maximum_dose
// Precondition: patient not allergic
// Postcondition: medication logged
// Postcondition: alert generated if interaction
// Invariant: patient has valid record
```

**Benefits:**
- Safety-critical verification
- Prevents harmful interactions
- Audit trail
- Liability reduction

### Application 6.3: Operating Systems

**Contracts:**
```rust
// Precondition: memory address valid
// Precondition: page table set up
// Postcondition: page loaded into cache
// Postcondition: TLB updated
// Invariant: memory consistency
// Invariant: no data corruption
```

**Benefits:**
- System stability
- Performance verification
- Debugging
- Formal verification

### Application 6.4: Distributed Systems

**Contracts:**
```rust
// Precondition: message well-formed
// Precondition: sender authenticated
// Postcondition: delivered exactly once
// Postcondition: order preserved
// Invariant: consistency maintained
// Invariant: quorum reachable
```

**Benefits:**
- Reliability guarantees
- Failure detection
- Consensus verification
- Network health monitoring

---

## 7. PERFORMANCE CONSIDERATIONS

### 7.1 Contract Checking Overhead

**Levels of Verification:**

| Level | Overhead | When to Use |
|-------|----------|------------|
| None | 0% | Production (code you trust) |
| Preconditions | 1-5% | Production (input validation) |
| Pre+Post | 5-15% | Development/Testing |
| Full | 10-30% | Development/Safety-critical |

### 7.2 Optimization Strategies

**Strategy 1: Selective Verification**
```rust
fn operation(x: i32) -> Result<Output, Error> {
    #[cfg(debug_assertions)]
    if x < 0 {
        return Err("precondition");
    }
    
    // Implementation, unchecked
    let result = expensive_computation(x);
    
    #[cfg(debug_assertions)]
    if result < 0 {
        return Err("postcondition");
    }
    
    Ok(result)
}
```

**Strategy 2: Cached Invariant Checks**
```rust
impl Container {
    fn check_invariants_cached(&mut self) -> Result<(), Error> {
        if self.needs_check {
            self.perform_check()?;
            self.needs_check = false;
        }
        Ok(())
    }
}
```

**Strategy 3: Batch Verification**
```rust
fn verify_all(items: &[Item]) -> Result<(), Error> {
    // Single pass through all items instead of per-item
    for item in items {
        if item.is_invalid() {
            return Err("invariant");
        }
    }
    Ok(())
}
```

### 7.3 Profiling and Analysis

```rust
fn measure_overhead() {
    let start = Instant::now();
    let result = operation_with_verification(x);
    let with_checks = start.elapsed();
    
    let start = Instant::now();
    let result = operation_without_verification(x);
    let without_checks = start.elapsed();
    
    let overhead_percent = 100.0 * (with_checks - without_checks) / without_checks;
    println!("Overhead: {}%", overhead_percent);
}
```

---

## 8. INTEGRATION WITH OTHER PATTERNS

### Integration 8.1: With Error Handling

**DbC + Custom Errors:**
```rust
#[derive(Debug)]
enum DomainError {
    PreconditionViolated(String),
    PostconditionViolated(String),
    InvariantViolated(String),
    OperationFailed(String),
}

fn operation(x: i32) -> Result<Output, DomainError> {
    if x < 0 {
        return Err(DomainError::PreconditionViolated(
            "x must be non-negative".into()
        ));
    }
    // ...
}
```

### Integration 8.2: With Type System

**Leveraging Types:**
```rust
// Type system enforces >= 0
struct NonNegative(u32);

// Type system enforces sorted
struct SortedList(Vec<i32>);

// Type system enforces non-empty
struct NonEmptyVec<T>(Vec<T>); // Could use custom constructor

fn operation(x: NonNegative) -> Result<Output, Error> {
    // No precondition check needed!
    // Type guarantees x >= 0
}
```

### Integration 8.3: With Testing

**Contract-Driven Testing:**
```rust
#[test]
fn test_precondition() {
    // Test that precondition is enforced
    assert!(operation(-1).is_err());
}

#[test]
fn test_postcondition() {
    // Test that postcondition is verified
    let result = operation(5).unwrap();
    assert!(result > 0);
}

#[test]
fn test_invariant() {
    // Test that invariant is maintained
    let mut obj = create_object();
    obj.modify();
    assert!(obj.check_invariants().is_ok());
}
```

### Integration 8.4: With Concurrency

**Thread-Safe Contracts:**
```rust
impl<T: Send + Sync> ThreadSafeQueue<T> {
    fn check_invariants(&self) -> Result<(), Error> {
        // INVARIANT: internal structure consistent
        let lock = self.lock.lock().unwrap();
        // Verify under lock
    }
    
    pub fn enqueue(&mut self, item: T) -> Result<(), Error> {
        let mut lock = self.lock.lock().unwrap();
        self.data.push(item);
        // Invariant checked while holding lock
    }
}
```

---

## 9. TOOLS & FRAMEWORKS

### Tool 9.1: Assertion Macros

**Rust Built-in:**
```rust
assert!(condition);           // Panic on failure
debug_assert!(condition);     // Only in debug
assert_eq!(a, b);            // Equality check
```

**Custom Wrapper:**
```rust
macro_rules! precondition {
    ($cond:expr, $msg:expr) => {
        if !$cond {
            return Err(format!("Precondition: {}", $msg));
        }
    };
}

macro_rules! postcondition { ... }
macro_rules! invariant { ... }
```

### Tool 9.2: Documentation Tools

**Doc Comment with Contract:**
```rust
/// Computes square root of x.
/// 
/// # Precondition
/// - `x >= 0.0`
///
/// # Postcondition
/// - Returns `Ok(result)` where `result >= 0.0`
/// - `result * result ≈ x` (within floating-point precision)
///
/// # Invariants
/// - No invariants
///
/// # Examples
/// ```
/// assert!(sqrt(4.0).ok() == Some(2.0));
/// assert!(sqrt(-1.0).is_err());
/// ```
pub fn sqrt(x: f64) -> Result<f64, String> { ... }
```

### Tool 9.3: Testing Frameworks

**Property-Based Testing:**
```
quickcheck: Generate random test cases
proptest: Guided property testing
hypothesis: Python-style property testing
```

**Contract Verification:**
```
covenant: Contract specification language
jml4: Java modeling language
dafny: Verification-aware programming language
```

---

## 10. FURTHER READING

### Foundational Papers
- "Designing Classes with Design by Contract" - Bertrand Meyer
- "Programming by Contract" - Meyer & Nerson
- "Behavioral Subtyping Using Invariants and Constraints" - Liskov & Wing

### Books
- "Code Complete" - Steve McConnell (Chapter on contracts)
- "Design by Contract" - Bertrand Meyer (comprehensive)
- "The Practice of Programming" - Kernighan & Pike

### Online Resources
- Eiffel language documentation (DbC pioneer)
- Microsoft Code Contracts documentation
- Design by Contract papers archive

### Modern Applications
- Blockchain smart contracts
- Formal verification research
- Safety-critical systems documentation
- Formal methods in aerospace

---

## SUMMARY: QUICK REFERENCE

### When to Use What

| Need | Use |
|------|-----|
| Catch invalid inputs | Precondition |
| Guarantee output quality | Postcondition |
| Ensure object consistency | Invariant |
| Method-specific guarantees | Method contract |
| Inheritance compatibility | Behavioral subtyping |
| Runtime verification | Assert + check_invariants |
| Compile-time verification | Type system |

### Contract Checklist

- [ ] Identified all preconditions
- [ ] Implemented precondition checks
- [ ] Identified all postconditions
- [ ] Implemented postcondition verification
- [ ] Identified all invariants
- [ ] Checked invariants in all methods
- [ ] Designed clear error messages
- [ ] Created comprehensive tests
- [ ] Documented all contracts
- [ ] Verified contracts work correctly
- [ ] Considered performance impact
- [ ] Integrated with error handling
- [ ] Considered type-level guarantees
- [ ] Reviewed for common mistakes

---

## GLOSSARY

**Precondition:** Obligation on the caller; what must be true before calling
**Postcondition:** Obligation on the provider; what must be true after execution
**Invariant:** Property that must always hold
**Contract:** Complete specification of preconditions, postconditions, and invariants
**Behavioral Subtyping:** Derived classes can be substituted for base classes
**Assertion:** Runtime check of a boolean condition
**Contract Violation:** Runtime failure of precondition, postcondition, or invariant

---

## INDEX

- Aggregate invariant → Section 4.3
- Assertion → Section 9.1
- Behavioral subtyping → Section 4, Pattern 4.5
- Class invariant → Section 4
- Common mistakes → Section 5
- Complex validation → Pattern 2.2
- Compositional postcondition → Pattern 3.5
- Consistency invariant → Pattern 4.4
- Contract violation → Section 5
- Dependent conditions → Pattern 2.3
- Design by contract → Section 1.1
- Defensive preconditions → Pattern 2.5
- Distributed systems → Section 6.4
- Error handling integration → Section 8.1
- Financial systems → Section 6.1
- Invariant check patterns → Pattern 4
- Medical software → Section 6.2
- Operating systems → Section 6.3
- Performance consideration → Section 7
- Postcondition verification → Section 3
- Precondition patterns → Section 2
- Property verification → Pattern 3.4
- Quantified invariant → Pattern 4.5
- Relationship verification → Pattern 3.4
- Simple invariant → Pattern 4.1
- Simple validation → Pattern 2.1
- State-based preconditions → Pattern 2.4
- State change verification → Pattern 3.3
- Structural invariant → Pattern 4.2
- Testing integration → Section 8.3
- Tools and frameworks → Section 9
- Transformation verification → Pattern 3.2
- Type system integration → Section 8.2
