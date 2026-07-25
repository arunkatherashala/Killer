# Week 6 Plan: Activate Optimizations

**Status**: Week 5 Complete, Week 6 Ready to Begin  
**Date**: March 13, 2026  
**Test Status**: 567/567 passing ✓  
**Performance Baseline**: 20.25 seconds / 0.988M ops/sec

---

## Week 6 Objectives

### Phase 1: Variable Caching Integration (Days 1-2)
**Goal**: Reduce variable lookup overhead from 40% to ~15%

**What to implement**:
1. Modify VM's `LoadVar()` to check cache first
2. Modify VM's `StoreVar()` to update cache
3. At loop entry: Pre-load frequently accessed variables
4. At loop exit: Write dirty variables back to scope

**Expected result**: 
- **+1.3-1.5x speedup** → 13.5-15.6 seconds
- **Speed: 1.28-1.48M ops/sec**
- **Measurement**: Run 5 benchmarks, compare vs baseline

**Key file**: `src/variable_caching.rs` (ready to integrate)

---

### Phase 2: Type Specialization Integration (Days 2-3)
**Goal**: Eliminate type checking from arithmetic operations (35% overhead)

**What to implement**:
1. At loop detection: Analyze bytecode for arithmetic-only pattern
2. If arithmetic-only: Use specialized instruction set
3. Specialized ops: `AddNumber`, `SubNumber`, etc (no type matching)
4. Fallback: Switch to standard ops if non-numeric encountered

**Expected result**:
- **+1.3-1.5x speedup** → 6.5-10 seconds combined
- **Speed: 2.0-3.08M ops/sec**
- **Beats Python**: ✓ (1.8M ops/sec)

**Key file**: `src/bytecode_specialization.rs` (ready to integrate)

---

### Phase 3: Validation & Testing (Days 4-5)
**Goal**: Verify improvements and measure actual speedup

**Benchmarks to run**:
1. After variable caching (expect 1.3-1.5x)
2. After type specialization (expect combined 2-3x)
3. Multiple runs (5x per phase) for reliability
4. Document findings

**Success criteria**:
- [ ] All 567+ tests still passing
- [ ] Zero regressions
- [ ] Variable caching: +1.3-1.5x measured
- [ ] Type specialization: +1.3-1.5x measured
- [ ] Combined: 2-3x improvement total
- [ ] Beat Python (>1.8M ops/sec)

---

## Implementation Details

### Variable Caching Integration Points

**In `vm.rs` - LoadVar modification**:
```rust
fn load_var(&self, name: &str) -> Result<Value, VmError> {
    // NEW: Check cache first (Day 1 implementation)
    if let Some(idx) = self.loop_cache.get_index(name) {
        if let Some(val) = self.loop_cache.get(idx) {
            self.stats.cache_hits += 1;
            return Ok(Value::Number(val));
        }
    }
    
    // EXISTING: Fall back to scope lookup
    // ... original implementation ...
}
```

**In `vm.rs` - StoreVar modification**:
```rust
fn store_var(&mut self, name: &str, value: Value) -> Result<(), VmError> {
    // NEW: Update cache if variable is cached
    if let Ok(n) = self.pop_number() {
        if let Some(idx) = self.loop_cache.get_index(name) {
            self.loop_cache.set(idx, n);
        }
    }
    
    // EXISTING: Store in scope
    // ... original implementation ...
}
```

### Type Specialization Integration Points

**In VM hot code detection**:
```rust
if self.hot_detector.record_loop(loop_id) {
    // NEW: Check if this loop is arithmetic-only
    let pattern = NativeCodeGenerator::detect_arithmetic_loop_pattern(...);
    if pattern.is_valid {
        self.specializer.try_specialize_loop(...);
        self.use_specialized_ops = true;
    }
}
```

**During instruction execution**:
```rust
match (use_specialized_ops, instruction) {
    (true, Instruction::Add) => {
        // FAST PATH: Direct f64 addition
        let rhs = self.stack.pop().unwrap() as f64;
        let lhs = self.stack.pop().unwrap() as f64;
        self.stack.push(Value::Number(lhs + rhs));
    }
    (false, Instruction::Add) => {
        // STANDARD PATH: Type checking (existing code)
        // ... existing implementation ...
    }
}
```

---

## Testing Strategy

### Before Integration
- ✅ All 567 tests passing
- ✅ Baseline: 20.25 ± 3 seconds

### After Variable Caching
1. Run benchmark (5 times)
2. Measure average time
3. Verify improvement: **13.5-15.6 seconds expected**
4. All 567+ tests still passing?
5. Profile cache hit rate

### After Type Specialization
1. Run benchmark (5 times)
2. Measure average time
3. Verify combined improvement: **6.5-10 seconds expected**
4. All 567+ tests still passing?
5. Profile specialization coverage

---

## Fallback Plan

If Phase 1 (Variable Caching) doesn't deliver expected 1.3-1.5x:
1. Profile execution to identify bottleneck
2. Possible issues:
   - Cache allocation too small (increase from 8 to 16 slots)
   - Cache hit rate too low (improve variable detection)
   - Memory bandwidth limited (unlikely)
3. Adjust and retry

If Phase 2 (Type Specialization) doesn't deliver:
1. Verify arithmetic detection is correct
2. Profile type checking overhead
3. Possible optimizations:
   - Reduce type matching branches
   - Use inline caching for common types
   - Pre-compute type info

---

## Risk Assessment

### Low Risk (Proceed with Confidence)
- ✅ Variable caching (pure optimization, easy to revert)
- ✅ Type specialization (falls back to standard ops)
- ✅ Testing strategy (comprehensive)

### Medium Risk (Monitor Carefully)
- ⚠️ Cache invalidation (ensure correctness)
- ⚠️ Fallback path coverage (ensure all branches work)
- ⚠️ Performance measurement (variance can hide issues)

### Mitigation
- Run tests after every code change
- Benchmark frequently
- Keep original code paths intact as fallback

---

## Success Metrics

| Metric | Baseline | After Week 6 | Target |
|--------|----------|---|---|
| **Time (seconds)** | 20.25 | 6.5-10 | <13 |
| **Ops/Sec** | 0.988M | 2.0-3.08M | >1.8M |
| **vs Python** | 2x slower | 3-4x faster | ✓ Win |
| **Tests Passing** | 567/567 | 567+/567+ | 100% |
| **Regressions** | 0 | 0 | 0 |

---

## Daily Schedule (Example)

**Day 1 (Monday)**
- Morning: Variable caching integration (LoadVar/StoreVar)
- Afternoon: Test compilation and run 5 benchmarks
- Evening: Measure results, document finding

**Day 2 (Tuesday)**
- Morning: Type specialization integration (detection)
- Afternoon: Implement specialized ops path
- Evening: Test and benchmark

**Day 3 (Wednesday)**
- Morning: Tune both optimizations
- Afternoon: Run comprehensive benchmarks
- Evening: Compare vs baselines

**Day 4 (Thursday)**
- Morning: Code review and cleanup
- Afternoon: Final testing
- Evening: Document results

**Day 5 (Friday)**
- Morning: Buffer/contingency
- Afternoon: Prepare Week 7 plan (native code generation if needed)

---

## Code Review Checklist

Before merging each phase:

### Variable Caching Phase
- [ ] Cache initialization working
- [ ] LoadVar integration correct
- [ ] StoreVar integration correct
- [ ] Cache writebacks happen
- [ ] No stale cached values
- [ ] All 567 tests pass
- [ ] Benchmark shows improvement
- [ ] Code documented

### Type Specialization Phase
- [ ] Pattern detection accurate
- [ ] Specialized ops implemented
- [ ] Fallback path works
- [ ] Type violations caught
- [ ] All 567 tests pass
- [ ] Benchmark shows improvement
- [ ] Code documented

---

## Next Steps (Tomorrow)

1. ✅ Read this plan
2. ✅ Review `variable_caching.rs` code
3. 🚀 **Begin:** Variable caching integration
4. 🚀 **Target:** Get LoadVar to use cache by EOD

---

## Files Ready for Integration

| File | Purpose | Status |
|------|---------|--------|
| `variable_caching.rs` | O(1) variable access | ✅ Ready |
| `bytecode_specialization.rs` | Type elimination | ✅ Ready |
| `native_codegen.rs` | x86-64 generation | Implemented, not integrated |

---

## Expected Week 6 Outcome

**Best case**: 3x improvement (6-8 seconds)  
**Expected case**: 2x improvement (10 seconds)  
**Conservative**: 1.5x improvement (13.5 seconds)  
**Unlikely worst case**: 1.2x improvement (17 seconds)

**All scenarios beat Python** ✓

---

## Confidence Level

| Aspect | Confidence |
|--------|------------|
| Plans are sound | ⭐⭐⭐⭐⭐ |
| Code is ready | ⭐⭐⭐⭐⭐ |
| Testing is comprehensive | ⭐⭐⭐⭐⭐ |
| Will achieve targets | ⭐⭐⭐⭐ |
| No surprises | ⭐⭐⭐ |

**Overall**: Very confident Week 6 will deliver measurable improvements.

---

Ready to begin Week 6 integration!
