# Week 4 Action Plan: Interpreter Optimization (1.5-2x Speedup Target)

## Updated Objective

**Previous Week 4 Goal**: Skip loop execution via fast-path (complex, state management issues)  
**Revised Week 4 Goal**: Optimize interpreter's hot path for arithmetic operations  
**Performance Target**: 15-17s for 20M ops (1.1-1.3x improvement minimally, 2-3x ideally)  
**Key Insight**: Bottleneck is arithmetic dispatch overhead, not loop control flow  

## Critical Discovery

Our fast arithmetic executor achieves **0.08s for 20M operations** = **250M ops/second**!
Current interpreter: **1.07M ops/second** (19.5s baseline)
**Gap: 234x slower due to interpreter overhead**

This reveals the TRUE bottleneck:
- ❌ NOT the while loop control (Jump/JumpIfFalse)
- ✅ YES the arithmetic operations (Add/Sub/Div/Mod)
- ✅ YES the Value type wrapping/unwrapping
- ✅ YES the instruction dispatch per-operation

## New Strategy: Optimize the Interpreter

Instead of trying to replace the loop (risky, breaks state management), optimize each instruction:

### Target Optimizations (in priority order)

1. **Fast-path arithmetic** (Add, Sub, Mul, Div when operands are Numbers)
   - Check type without unwrapping
   - Direct arithmetic  
   - ~1.3x speedup

2.** Reduce scope lookups for loop variables**
   - Cache i, sum in fast variables
   - Avoid HashMap lookups each iteration
   - ~1.2x speedup

3. **Inline hot operations**
   - `Add` operation happens millions of times
   - Make it inline-friendly
   - ~1.1x speedup

4. **Eliminate bounds checking in hot loops**
   - For loops with known safe bounds
   - Skip safety checks
   - ~1.1x speedup

**Cumulative Expected Effect**: 1.3 x 1.2 x 1.1 x 1.1 = **1.9x speedup** → 10.3s for 20M ops!

## Detailed Implementation Plan

### Step 1: Create Specialized Add Instruction (1 hour)

**File**: `src/v2-rust/killer_vm/src/vm.rs`

Current Add implementation:
```rust
Instruction::Add => {
    let rhs = self.pop_number()?;  // Type check + unwrap
    let lhs = self.pop_number()?;  // Type check + unwrap
    self.stack.push(Value::Number(lhs + rhs));  // Re-wrap
}
```

Optimized version:
```rust
Instruction::Add => {
    // Fast path for numbers
    if let (Value::Number(rhs), Value::Number(lhs)) = (
        self.stack.pop(),
        self.stack.pop()
    ) {
        self.stack.push(Value::Number(lhs + rhs));
        continue;
    }
    
    // Fallback to original implementation
    let rhs = self.pop_number()?;
    let lhs = self.pop_number()?;
    self.stack.push(Value::Number(lhs + rhs));
}
```

**Speedup**: ~1.3x for arithmetic-heavy workloads

### Step 2: Optimize Sub/Div Instructions (45 min)

Apply same pattern to Subtract and Divide:

```rust
Instruction::Sub => {
    // Fast path
    if let Some(Value::Number(rhs)) = self.stack.pop() {
        if let Some(Value::Number(lhs)) = self.stack.pop() {
            self.stack.push(Value::Number(lhs - rhs));
            continue;
        } else {
            // Put rhs back
            self.stack.push(Value::Number(rhs));
        }
    }
    // Fallback
    let rhs = self.pop_number()?;
    let lhs = self.pop_number()?;
    self.stack.push(Value::Number(lhs - rhs));
}
```

Similar for Divide (with zero-check optimization).

**Cumulative Speedup**: ~1.5x

### Step 3: Benchmark After Each Change (15 min per change)

After implementing Add fast-path:
- Build release
- Run benchmark 3x
- Record timing
- Expected: 19.5s → ~18.2s

After implementing Sub fast-path:
- Run benchmark again  
- Expected: 18.2s → ~17.0s

After implementing Div fast-path:
- Run benchmark again
- Expected: 17.0s → ~16.0s

### Step 4: Variable Caching Optimization (1 hour, optional if speedup is sufficient)

If Step 1-3 don't reach 15.0s:

**Idea**: For hot loops, detect repeated variable accesses and cache them

```rust
// Detect: while (i < N) { sum = sum + i; i = i + 1; }
// Cache i and sum in fast locals during loop execution
let mut cached_i = variables.get("i")?;
let mut cached_sum = variables.get("sum")?;

// Hot loop uses cached values
for _ in 0..1000 {  // When hot_detector triggers
    cached_sum = cached_sum + cached_i;
    cached_i = cached_i + 1;
}

// Write back to scope
variables.set("i", cached_i);
variables.set("sum", cached_sum);
```

**Speedup**:  ~1.2x by reducing HashMap lookups

---

## Success Criteria

| Target | Metric | Current | Week 4 Goal | Status |
|--------|--------|---------|-------------|--------|
| Time for 20M ops | Seconds | 19.5s | 15.0s | 1.3x |
| Operations/sec | Millions | 1.07M | 1.33M | +24% |
| All tests pass | Count | 555/555 | 555/555 | ✓ |
| Regression | None | - | Zero | ✓ |

**Minimum Success**: 17.0s (1.15x speedup)  
**Expected Success**: 15.0s (1.3x speedup)  
**Stretch Goal**: 12.0s (1.6x speedup, if variable caching works)

---

## Weekly Timeline

| Time | Task | Est. Time | Cum. Time |
|------|------|-----------|-----------|
| Mon 09:00 | Implement Add fast-path | 1h | 1h |
| Mon 10:00 | Implement Sub fast-path | 45m | 1h45m |
| Mon 11:00 | Implement Div fast-path | 45m | 2h30m |
| Mon 12:00 | Benchmark and validate | 30m | 3h |
| Mon 13:00 | If needed: Variable caching | 1h | 4h |
| Mon 14:00 | Final benchmark & report | 30m | 4h30m |

---

## Risk Assessment

**Risk: Low** - All changes are in hot path optimization, not core logic changes
- If individual optimizations don't help, can revert easily
- No state management issues
- No loop execution skipping

**Risk Mitigation**:
- Test after each optimization
- Keep baseline builds
- Incremental validation

---

## If Week 4 Doesn't Achieve Target

**Fallback Plan A**: Implement instruction specialization
- Create specialized `add_number` bytecode for arithmetic loops
- Faster than generic `Add`
- More invasive but proven approach

**Fallback Plan B**: Enable loop pattern detection
- Detect arithmetic loop patterns at compile time
- Generate optimized bytecode
- More complex but very effective

**Fallback Plan C**: Accept current performance
- 1.07M ops/sec is respectable
- Better than Python in some workloads
- Plan for future optimization

---

## What We'll Learn This Week

1. Which instruction contributes most to slowdown
2. If optimization is interpreter design or just unoptimized code
3. Whether caching variable lookups helps
4. If we can reach 2-3x speedup through interpreter optimization alone
5. Path forward for next week

---

## References

- Current baseline: `docs/WEEK3_COMPLETION_REPORT.md` (19.5s)
- Fast executor performance: `src/v2-rust/killer_vm/src/vm.rs` (execute_hot_arithmetic_loop = 0.08s!)
- Benchmark: `benchmarks/arithmetic_bench.killer`
- Next level plan: `WEEK5_PATTERN_DETECTION.md` (if needed)

