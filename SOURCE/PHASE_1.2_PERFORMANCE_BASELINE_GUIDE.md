/// Phase 1.2: Pre-Deployment Validation - Performance Baseline Execution Guide
/// March 24-25, 2026 - 2 Day Performance Baseline Establishment
/// Status: READY FOR EXECUTION

# Phase 1.2: Performance Baseline - Execution Guide
## March 24-25, 2026 | 8 Hour Comprehensive Baseline

---

## Overview

**Objective**: Establish performance baseline for v4.2 (current) before deploying v4.3

**Success Criteria**: 
- ✅ Baseline metrics documented for 10+ benchmarks
- ✅ Regression testing CI/CD integrated
- ✅ Alert thresholds defined
- ✅ Historical trend baseline captured

**Timeline**:
- Tuesday 3/24: 8 AM - 12 PM (Micro-benchmarks)
- Wednesday 3/25: 9 AM - 5 PM (End-to-end & analysis)

**Deliverable**: `PERFORMANCE_BASELINE_v4.3.md` (baseline metrics document)

---

## Day 1: Tuesday, March 24 (Micro-benchmarks)

### Pre-Day Setup (Monday evening)

**Setup Tasks** (30 min):
- [ ] Fresh environment for testing (clean machine or container)
- [ ] No background processes running
- [ ] Killer runtime compiled with `--release`
- [ ] Test scripts copied to `/tmp/benchmark/`
- [ ] Criterion.rs configured (`Cargo.toml` updated)

**Compiler Flags** (Release Mode):
```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
```

**System Notes**:
- Machine: [CPU model, RAM, OS]
- Network: [no network during tests]
- Load: [isolated, no other processes]
- Time: Record exact start time

---

### Session 1: Stack Operations (2 hours, 8 AM - 10 AM)

**Purpose**: Baseline stack-based execution performance

**Benchmark 1: Push/Pop Operations**

```rust
// File: benchmarks/stack_benchmark.killer

// Test 1: Simple push/pop loop
fn bench_push_pop_1000() {
    for i in range(0, 1000) {
        x = i + 1
        y = x * 2
    }
}

// Test 2: Variable stack depth
fn bench_push_pop_10000() {
    for i in range(0, 10000) {
        a = i
        b = a + 1
        c = b * 2
    }
}

// Test 3: Realistic mix
fn bench_push_pop_mixed() {
    result = 0
    for i in range(0, 5000) {
        temp = i * 2
        result = result + temp
        if (i % 100 == 0) {
            temp = 0
        }
    }
    return result
}
```

**Run Instructions**:
```bash
# Compile benchmark
killer --emit-executable benchmarks/stack_benchmark.killer

# Run each 3 times, record time
/usr/bin/time -v ./stack_benchmark

# Record memory, CPU, time
# Expected times (baseline):
#   Test 1: < 5ms
#   Test 2: < 50ms
#   Test 3: < 30ms
```

**Metrics to Record**:
- [ ] Execution time (user time)
- [ ] Memory peak resident set
- [ ] Instructions retired
- [ ] Cache misses (if available)

**Results Spreadsheet**:
| Benchmark | Run 1 | Run 2 | Run 3 | Avg | Std Dev |
|-----------|-------|-------|-------|-----|---------|
| push_pop_1000 | [ms] | [ms] | [ms] | | |
| push_pop_10000 | [ms] | [ms] | [ms] | | |
| push_pop_mixed | [ms] | [ms] | [ms] | | |

---

**Benchmark 2: Variable Lookup Performance**

```rust
// File: benchmarks/variable_benchmark.killer

fn bench_local_variable_lookup() {
    // Local scope variable lookup
    x = 0
    for i in range(0, 10000) {
        x = x + 1
    }
}

fn bench_multiple_variables() {
    // Multiple variable lookups
    a = 0
    b = 0
    c = 0
    for i in range(0, 5000) {
        a = a + 1
        b = a + 2
        c = b + 3
    }
}

fn bench_nested_scope_lookup() {
    // Nested scope lookups
    x = 100
    for i in range(0, 1000) {
        if (i % 2 == 0) {
            y = x + i
            z = y * 2
        }
    }
}
```

**Run Instructions**:
```bash
# Similar to stack benchmarks
/usr/bin/time -v ./variable_benchmark
```

**Metrics to Record**:
- [ ] Execution time per test
- [ ] Memory usage pattern
- [ ] Scope-specific latency

---

### Session 2: Parser Performance (2 hours, 10 AM - 12 PM)

**Purpose**: Baseline parsing speed vs file complexity

**Benchmark 3: File Size Scaling**

```bash
# Create test files of varying sizes
echo "# Parser benchmark - small file" > /tmp/parser_small.killer
# Add 100 function definitions
for i in {1..100}; do
    echo "kfn func_$i(x) { x + 1 }" >> /tmp/parser_small.killer
done

# Create medium file (1000 lines)
# Create large file (5000 lines)
# Create very large file (10000 lines)
```

**Run Parser Benchmarks**:
```bash
# Parse each file, measure time
time killer --emit-bytecode /tmp/parser_small.killer > /tmp/parser_small.bc
time killer --emit-bytecode /tmp/parser_medium.killer > /tmp/parser_medium.bc
time killer --emit-bytecode /tmp/parser_large.killer > /tmp/parser_large.bc
time killer --emit-bytecode /tmp/parser_xlarge.killer > /tmp/parser_xlarge.bc

# Record times, memory usage
```

**Results Spreadsheet**:
| Lines | Time (ms) | Memory (MB) | Lines/sec |
|-------|-----------|-------------|-----------|
| 100   | | | |
| 1,000 | | | |
| 5,000 | | | |
| 10,000 | | | |

**Acceptance Criteria**:
- [ ] Linear scaling (not exponential)
- [ ] < 1ms per 100 lines
- [ ] Memory growth proportional to file size

---

**Benchmark 4: Error Recovery**

```bash
# Test parser error handling speed
# Create file with parse error at different points

# Error at 10%:
head -n $(( LINES / 10 )) script.killer > /tmp/error_early.killer
echo "SYNTAX ERROR HERE" >> /tmp/error_early.killer

# Error at 50%:
head -n $(( LINES / 2 )) script.killer > /tmp/error_mid.killer
echo "SYNTAX ERROR HERE" >> /tmp/error_mid.killer

# Error at 90%:
head -n $(( LINES * 9 / 10 )) script.killer > /tmp/error_late.killer
echo "SYNTAX ERROR HERE" >> /tmp/error_late.killer

# Run and measure error time
time killer --lint /tmp/error_early.killer
time killer --lint /tmp/error_mid.killer
time killer --lint /tmp/error_late.killer
```

---

## Day 2: Wednesday, March 25 (End-to-End & Analysis)

### Session 3: End-to-End Script Performance (4 hours, 9 AM - 1 PM)

**Purpose**: Real-world script performance characteristics

**Benchmark 5: Arithmetic Heavy Script**

```rust
// File: benchmarks/arithmetic.killer

kfn fibonacci(n) {
    if (n <= 1) {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

result = 0
for i in range(1, 30) {
    result = result + fibonacci(i)
}

print(K"Result: {result}")
```

**Run & Measure**:
```bash
time killer benchmarks/arithmetic.killer
# Expected: ~50-100ms for fib(30)
```

**Metrics**:
- [ ] Total execution time
- [ ] Recursion depth handling
- [ ] Memory usage at peak

---

**Benchmark 6: Variable-Heavy Script**

```rust
// File: benchmarks/variable_heavy.killer

kfn process_variables(count) {
    vars = {}
    for i in range(0, count) {
        vars[K"var_{i}"] = i * 2
    }
    
    total = 0
    for key in vars.keys() {
        total = total + vars[key]
    }
    
    return total
}

result = process_variables(500)
print(K"Total: {result}")
```

**Run & Measure**:
```bash
time killer benchmarks/variable_heavy.killer
# Expected: < 10ms
```

**Metrics**:
- [ ] Dictionary operations speed
- [ ] Iteration performance
- [ ] Memory allocation patterns

---

**Benchmark 7: Method Call Heavy Script**

```rust
// File: benchmarks/method_heavy.killer

class Calculator {
    handle add(a, b) -> i64 {
        a + b
    }
    
    handle multiply(a, b) -> i64 {
        a * b
    }
    
    handle compute(x, y) -> i64 {
        result = this.add(x, y)
        return this.multiply(result, 2)
    }
}

calc = Calculator::spawn()
total = 0
for i in range(0, 1000) {
    total = total + calc.compute(i, i+1).await
}

print(K"Total: {total}")
```

**Run & Measure**:
```bash
time killer benchmarks/method_heavy.killer
# Expected: < 50ms for 1000 calls
```

**Metrics**:
- [ ] Method call overhead
- [ ] Actor spawn/message time
- [ ] Cache hit rates (if instrumented)

---

### Session 4: Analysis & Documentation (2 hours, 1 PM - 3 PM)

**Activity 1: Compile Results** (30 min)

Create `PERFORMANCE_BASELINE_v4.3.md`:

```markdown
# Performance Baseline - v4.2 (Current) vs v4.3 (Target)

## Test Environment
- Machine: [model]
- CPU: [specs]
- RAM: [GB]
- OS: [Windows/Linux/Mac]
- Date: March 25, 2026

## Micro-benchmarks (Stack Operations)

### Push/Pop 1,000 ops
- v4.2 Baseline: [X.Xms]
- v4.3 Target: < [X.X ms] (0% regression)
- Acceptance: ✓ PASS

### Push/Pop 10,000 ops
- v4.2 Baseline: [XXms]
- v4.3 Target: < [XX ms] (0% regression)
- Acceptance: ✓ PASS

... [all benchmarks]

## End-to-End Scripts

### Arithmetic (Fibonacci)
- v4.2 Baseline: [XXms]
- v4.3 Target: < [XX ms] (0% regression per review)

[... continue for all 7 benchmarks]

## Summary Statistics

| Category | Count | Avg Time | Trend |
|----------|-------|----------|-------|
| Micro | 4 | [Xms] | Baseline |
| E2E | 3 | [Xms] | Baseline |

## Performance Budget for v4.3

Total allowed regression: 5% (per review recommendation)
- Stack ops: 0% (critical path)
- Variable ops: 0% (critical path)
- Parser: 2% (less critical)
- E2E scripts: 3% (user-facing, acceptable)

## Regression Testing Thresholds

Alert if ANY metric exceeds:
- Stack ops: > 105% baselines
- Variable ops: > 105% baseline
- Parser: > 102% baseline
- E2E: > 103% baseline
```

**Activity 2: CI/CD Integration** (30 min)

- [ ] Add benchmarks to CI/CD pipeline
- [ ] Configure regression detection
- [ ] Setup alerting thresholds
- [ ] Create dashboard for tracking

**Activity 3: Documentation** (30 min)

- [ ] Summarize findings
- [ ] List any surprises/anomalies
- [ ] Record environment configuration
- [ ] Save raw data for future comparison

---

## Acceptance Criteria (Wed 3/25, 3 PM)

- [ ] 10+ benchmarks executed
- [ ] All metrics documented
- [ ] Baseline spreadsheet complete
- [ ] Regression thresholds defined
- [ ] CI/CD integrated
- [ ] Documentation complete

**Gate Decision**: 
- ✅ PASS: All benchmarks complete & reasonable
  - Proceed to Phase 1.3 (Documentation)
  - Share baseline with architecture team

- ⚠️ ANOMALY: Unexpected results
  - Investigate further (may add 1-2 hours)
  - Repeat tests if system was not clean
  - Document anomalies for future

- ❌ FAIL: Major issues found
  - Examples: Regressions already visible, system contamination
  - Investigate root cause
  - Repeat with clean environment

---

## Sign-Off Template

```
PERFORMANCE BASELINE ESTABLISHED

Baseline Date: March 25, 2026
Environment: [machine spec]
Test Count: 10+ benchmarks
Status: ✅ COMPLETE

✓ All micro-benchmarks recorded
✓ All end-to-end benchmarks recorded  
✓ Regression thresholds defined
✓ CI/CD integration complete
✓ Documentation complete

Performance Engineer: _________________ Date: _________

Next Step: Phase 1.3 (Documentation Review)
```

---

## Tools & Commands Reference

**Compilation**:
```bash
cd SOURCE/src/v2-rust/killer
cargo build --release
```

**Timing on Unix**:
```bash
/usr/bin/time -v killer script.killer
# Shows: wall time, user time, system time, memory, cache stats
```

**Timing on Windows**:
```powershell
Measure-Command { & 'C:\path\to\killer.exe' script.killer }
# Shows: TotalMilliseconds
```

**Memory Profiling** (if needed):
```bash
valgrind --tool=massif killer script.killer
ms_print massif.out.XXXXX
```

---

## Troubleshooting

**Issue: Inconsistent timing between runs**
- Solution: Run 3-5 times, discard outliers, use median
- Ensure: No background processes, consistent environment

**Issue: Out of memory on large benchmarks**
- Solution: Reduce input size
- Check: Memory limits not causing swap

**Issue: Parser seems slower than expected**
- Likely cause: File I/O included in time measurement
- Solution: Profile separately CPU vs I/O
- Use: `strace` or `Process Monitor` to isolate I/O

---

## Next Phase

**Phase 1.3 (Thu-Fri 3/26-27)**: Documentation Review

Notify documentation team when baseline complete.

---

**Performance Baseline Execution Guide - Ready for Tue 3/24**

Owner: Performance Engineer  
Duration: Tuesday 3/24 (4h) + Wednesday 3/25 (6h) = 10h total  
Deliverable: PERFORMANCE_BASELINE_v4.3.md
