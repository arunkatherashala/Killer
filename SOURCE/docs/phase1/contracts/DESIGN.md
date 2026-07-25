# Killer Contract Programming Design
## Phase 1 Weeks 16-18

### Overview
Contract programming (Design by Contract) enables:
- **Preconditions**: What must be true before calling a function
- **Postconditions**: What the function guarantees after execution
- **Invariants**: Properties that always hold in a loop/struct
- **Formal verification**: Prove properties about code before running

**Power Example:**
```killer
// Without contracts: caller responsible for validation
fn divide(a: i32, b: i32) -> i32 {
    if b == 0 {
        panic("Division by zero")
    }
    a / b
}

// With contracts: compiler & runtime ensure correctness
fn divide(a: i32, b: i32) -> i32
    requires b != 0;  // Precondition: b must not be zero
    ensures result == a / b;  // Postcondition: result matches division
{
    a / b
}

// Compiler error: violates precondition
divide(10, 0);  // ERROR: requires b != 0

// OK: precondition satisfied
if user_input != 0 {
    divide(10, user_input);  // OK
}
```

---

## Core Concepts

### Contract Types

#### 1. Preconditions
What must be true **before** calling the function:
```killer
fn safe_divide(a: i32, b: i32) -> i32
    requires b != 0;
{
    a / b
}

// Compiler error if called with b = 0
let result = safe_divide(10, 0);  // VIOLATION

// OK: precondition checked
if x != 0 {
    safe_divide(10, x);  // OK
}
```

#### 2. Postconditions
What the function **guarantees** after execution:
```killer
fn sqrt(x: f64) -> f64
    requires x >= 0.0;
    ensures result >= 0.0;
    ensures result * result ~= x;  // Approximate equality
{
    if x == 0.0 { 0.0 } else { math::sqrt(x) }
}

// Compiler verifies result meets postcondition
let y = sqrt(4.0);
// Proven: y >= 0.0 and y*y ~= 4.0
```

#### 3. Loop Invariants
What remains **true** throughout loop execution:
```killer
fn sum(arr: [i32]) -> i32 {
    let mut total = 0;
    for i in 0..arr.len() {
        invariant i >= 0 && i <= arr.len();  // Always true
        invariant total == sum(arr[0..i]);   // Sum of seen elements
        
        total = total + arr[i];
    }
    total
}
```

#### 4. Struct Invariants
Properties that always hold for an object:
```killer
struct Stack[T] {
    items: [T],
    top: nat,
    
    invariant top >= 0 && top <= items.len();
    invariant items[top..].all_null();
}

// Constructor must establish invariant
fn new[T]() -> Stack[T] {
    Stack {
        items: [],
        top: 0,  // Doesn't violate invariant
    }
}

// All methods must preserve invariant
fn push[T](mut self, item: T) -> Stack[T] {
    self.items.push(item);
    self.top = self.top + 1;
    // Compiler verifies: invariant still holds
    self
}
```

---

## Implementation Roadmap

### Week 16: Parser & AST
**Goal**: Parse contract syntax

Tasks:
- [ ] Add `requires` keyword to parser
- [ ] Add `ensures` keyword to parser
- [ ] Add `invariant` keyword to parser
- [ ] Store contracts in AST
- [ ] Parse boolean expressions in contracts

**Example syntax to parse:**
```killer
fn foo(x: i32) -> i32
    requires x > 0;
    ensures result > 0;
{
    x * 2
}
```

### Week 17: Type Checking & Verification
**Goal**: Verify contracts at compile-time where possible

Tasks:
- [ ] Type check contract expressions
- [ ] Propagate contract information through call graph
- [ ] Prove simple contracts automatically
  - `requires x > 0` with `x = 5` ✓
  - `requires x > 0` with `x = y` (conditional)
- [ ] Generate error for unprovable contracts
- [ ] Contract violation error messages

**Automatic proving examples:**
```killer
fn double_positive(x: i32) -> i32
    requires x > 0;
    ensures result > 0;
{
    x * 2
}

// Compiler proves you can call this:
double_positive(5)  // ✓ 5 > 0 provable
double_positive(x)  //  ✗ x > 0 not obviously true

// OK with guard:
if x > 0 {
    double_positive(x)  // ✓ Provable in this scope
}
```

### Week 18: Runtime Assertions & Integration
**Goal**: Runtime contract checking + full feature integration

Tasks:
- [ ] Generate runtime assertions for contracts
- [ ] `#[checked]` attribute for runtime contracts
- [ ] Integrate with exception/error system
- [ ] Performance: contract optimization (remove if proven)
- [ ] Integrate with other Phase 1 features

**Runtime checking:**
```killer
fn divide(a: i32, b: i32) -> i32 [checked] {
    requires b != 0;
    a / b  // Runtime check: panic if b == 0
}

// Compiler inserts runtime checks
divide(10, 0)  // RUNTIME ERROR: precondition violated
```

---

## Design Decisions

### Why Contracts?
1. **Formal verification**: Prove properties before running code
2. **Contract inheritance**: Subclasses can strengthen postconditions
3. **Debugging**: Clear what module/function violated contract
4. **Documentation**: Contracts are executable documentation
5. **Optimization**: Compiler can remove checks on proven contracts

### Contract Checking Modes

**Mode 1: Compile-time (default)**
```killer
fn foo(x: i32)
    requires x > 0;
{
    // Compiler verifies precondition at call sites
    // Zero runtime overhead
}

foo(5)    // ✓ Proven safe
foo(x)    // ✗ Not provable (unless x is known > 0)
```

**Mode 2: Runtime (opt-in)**
```killer
fn foo(x: i32) [checked] {
    requires x > 0;  // RUNTIME CHECK
    // Inserts: if !(x > 0) { panic(...) }
}

foo(-5)   // RUNTIME: precondition violation!
```

**Mode 3: Debug-only**
```killer
fn foo(x: i32) [debug_checked] {
    requires x > 0;  // Only checked in debug builds
}

// Release: removed for performance
// Debug: runtime check
```

### Integration with Type System
Contracts become part of the type:
```killer
type PositiveInt = i32 with (value > 0)

fn factorial(n: PositiveInt) -> PositiveInt pure {
    // Parameter type guarantees n > 0
    // No need for explicit precondition
}
```

---

## Examples

### Example 1: Simple Contract
```killer
fn safe_divide(a: i32, b: i32) -> i32
    requires b != 0;
    ensures result == a / b;
{
    a / b
}

// Usage:
if divisor != 0 {
    let q = safe_divide(10, divisor);
    // Proven: q == 10 / divisor
}
```

### Example 2: List Bounds
```killer
fn safe_access[T, n: nat](
    arr: Vector[T, n],
    idx: i32
) -> T
    requires idx >= 0 && idx < n;
{
    arr[idx]
}

// Compiler error: idx not provably < n
safe_access(v, user_input)  // ERROR

// OK: proven in bounds
for i in 0..v.len() {
    safe_access(v, i)  // ✓ i < n proven
}
```

### Example 3: Search with Postcondition
```killer
fn binary_search[T, n: nat](
    arr: Sorted[T, n],
    target: T
) -> Option<Idx[n]>
    ensures match(result) {
        Some(i) => arr[i] == target,
        None => !arr.contains(target),
    }
{
    // Implementation
}

// Caller can trust postcondition
match binary_search(sorted_data, target) {
    Some(idx) => println("Found at: " + idx),  // arr[idx] == target
    None => println("Not found"),              // !arr.contains(target)
}
```

### Example 4: Loop Invariants
```killer
fn bubble_sort[T, n: nat](mut arr: Vector[T, n]) -> Vector[T, n] {
    for i in 0..n {
        invariant arr[0..i].is_sorted();  // First i elements sorted
        invariant arr[i..].contains_all_large_elements();
        
        for j in 0..(n - i - 1) {
            if arr[j] > arr[j+1] {
                swap(arr, j, j+1);
            }
        }
    }
    arr  // Proven: arr.is_sorted()
}
```

### Example 5: Struct Invariants
```killer
struct LinkedList[T] {
    head: Option<Node[T]>,
    len: nat,
    
    invariant {
        if head.is_some() {
            count_nodes(head) == len
        } else {
            len == 0
        }
    }
}

fn push[T](mut self, item: T) -> LinkedList[T] {
    let new_node = Node::new(item, self.head);
    self.head = Some(new_node);
    self.len = self.len + 1;
    // Compiler verifies: invariant still holds
    self
}
```

### Example 6: With Other Phase 1 Features

**Contracts + Dependent Types:**
```killer
fn sorted_merge[m: nat, n: nat](
    a: Sorted[i32, m],
    b: Sorted[i32, n]
) -> Sorted[i32, m + n]
    requires true;
    ensures result.len() == m + n;
    ensures result.is_sorted();
{
    // Implementation
}
```

**Contracts + Effects:**
```killer
fn safe_read(path: String, max_size: nat) -> String uses io
    requires path != "" && max_size > 0;
    ensures result.len() <= max_size;
{
    read_file(path).truncate(max_size)
}
```

**Contracts + Async:**
```killer
async fn fetch_safe(url: String) -> String uses io
    requires url.starts_with("https://");
    ensures result.len() > 0;
{
    fetch(url).await
}
```

---

## Testing Strategy

### Compile-time Tests (20+ cases)
```
✓ Valid precondition satisfied
✗ Precondition violated (compiler error)
✓ Postcondition verified
✗ Function violates own postcondition (error)
✓ Loop invariant maintained
✗ Loop invariant broken (error)
```

### Runtime Tests (with [checked])
```
✓ Precondition checked at runtime
✓ Postcondition verified after execution
✓ Contract violation throws exception
✓ Error message identifies violated clause
```

### Integration Tests
```
✓ Contracts with dependent types
✓ Contracts with effects
✓ Contracts with async
✓ Inheritance respects Liskov substitution (contracts)
```

---

## Files to Create

```
docs/phase1/contracts/
  ├── DESIGN.md (this file)
  ├── VERIFICATION.md
  ├── ERROR_MESSAGES.md
  └── examples/
      ├── simple_contracts.killer
      ├── loop_invariants.killer
      ├── struct_contracts.killer
      ├── error_cases.killer
      └── with_other_features.killer

src/v2-rust/killer_vm/src/
  ├── contracts.rs
  ├── contract_checker.rs
  └── contract_verifier.rs
```

---

## Success Criteria

✓ Parser handles `requires`, `ensures`, `invariant`
✓ Compiler can prove simple contracts
✓ Runtime assertions work with `[checked]`
✓ Clear error messages for violations
✓ Loop invariants tracked correctly
✓ Struct invariants enforced
✓ Integration with all other Phase 1 features
✓ Tests pass: 20+ contract examples
✓ Zero overhead when not using contracts

---

## Performance Impact

| Mode | Compile Time | Runtime | Binary Size |
|------|--------------|---------|-------------|
| Off | 0% | 0% | 0% |
| Proven | +5% | 0% | +1% |
| [checked] | +10% | +5-15% | +2% |
| [debug_checked] | +10% | 0% (release) | +2% |

---

## Liskov Substitution Principle (LSP)

Contracts enable safe polymorphism:
```killer
class Shape {
    fn area() -> f64
        ensures result >= 0;
}

class Circle extends Shape {
    // Postcondition must be >= original (not weaker)
    fn area() -> f64
        ensures result >= 0;  // OK: same or stronger
}

// Safe to use Circle where Shape expected
fn draw(shape: Shape) {
    println("Area: " + shape.area());  // Result >= 0 guaranteed
}
```

---

## Next Steps After Week 18

Phase 2: Advanced Features  
- Dependent types library (generic vectors, matrices)
- Effect system for concurrency
- Async runtime optimization
- Contract proof automation
