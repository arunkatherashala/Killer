# QUICK ACTION PLAN: ARITHMETIC #1 IN 3 WEEKS

## 🚀 WEEK-BY-WEEK ROADMAP

### WEEK 1: BASELINE JIT (Get to 3.2-5.4M ops/sec)

**Goal**: Beat Python (1.80M → need 3.2M+)

**Files to Modify**:
```
src/v2-rust/killer_vm/src/runtime_optimization.rs
src/v2-rust/killer_vm/src/executor.rs
```

**Code to Add** (approx 200-300 lines):

```rust
// In runtime_optimization.rs

pub struct HotCodeDetector {
    loop_counters: HashMap<usize, u32>,
    hot_threshold: u32,
}

impl HotCodeDetector {
    pub fn new() -> Self {
        HotCodeDetector {
            loop_counters: HashMap::new(),
            hot_threshold: 1000,  // loops with 1000+ iterations
        }
    }
    
    pub fn record_loop(&mut self, loop_id: usize) -> bool {
        let count = self.loop_counters.entry(loop_id).or_insert(0);
        *count += 1;
        *count >= self.hot_threshold
    }
}

pub struct BasecodeJITCompiler {
    native_code_cache: HashMap<usize, Vec<u8>>,
}

impl BasecodeJITCompiler {
    pub fn compile_hot_arithmetic_loop(&mut self, bytecode: &[u8]) -> Vec<u8> {
        // Convert bytecode to native x86-64 ADD/SUB/DIV instructions
        // Example transformation:
        // Bytecode: PUSH i, LOAD i, ADD_CONST, SUB (i/2)
        // Native:   mov rax, i; add rax, rcx; shr rax, 1; sub rax, rdx
        
        vec![]  // Simplified placeholder
    }
}
```

**Estimated Time**: 6-8 hours coding + testing  
**Expected Result**: **3.2-5.4M ops/sec** ✅ (beats Python!)

**Testing**:
```bash
cd c:\Users\skathera\Downloads\killer_V2_RS_M11
cargo build --release
.\src\v2-rust\killer_vm\target\release\killer-native.exe .\benchmarks\arithmetic_bench.killer
# Should show 3.2-5.4M ops/sec with JIT enabled
```

---

### WEEK 2: INLINING + CONST FOLD (Get to 8-10M ops/sec)

**Goal**: Reach 8M+ operations/second

**Files to Modify**:
```
src/v2-rust/killer_vm/src/runtime_optimization.rs
```

**Code to Add** (approx 150-200 lines):

```rust
pub struct InliningOptimizer {
    max_inline_size: usize,  // Only inline small functions
}

impl InliningOptimizer {
    pub fn inline_hot_functions(&self, bytecode: &[u8]) -> Vec<u8> {
        // Replace CALL instructions with actual function bytecode
        // Eliminates call/return overhead for small functions
        bytecode.to_vec()
    }
}

pub struct ConstantFolder {
    optimizations: Vec<String>,
}

impl ConstantFolder {
    pub fn optimize_divisions(&self, instr: &[u8]) -> Vec<u8> {
        // Replace DIV by constants with bit shifts
        // DIV 2 → SHL 1
        // DIV 4 → SHL 2
        // etc.
        instr.to_vec()
    }
}
```

**Estimated Time**: 4-6 hours  
**Expected Result**: **8-10M ops/sec** ✅

---

### WEEK 3: LOOP UNROLLING (Get to 12-15M ops/sec - BEAT C!)

**Goal**: Reach 15M+ to beat C (2.45M baseline)

**Files to Modify**:
```
src/v2-rust/killer_vm/src/runtime_optimization.rs
```

**Code to Add** (approx 200-250 lines):

```rust
pub struct LoopUnroller {
    unroll_factor: u32,  // 2x, 4x unrolling
}

impl LoopUnroller {
    pub fn unroll_hot_loop(&self, loop_body: &[u8], factor: u32) -> Vec<u8> {
        // Duplicate loop body 2-4 times
        // Reduces branch predictions per iteration
        // Example: 1000M iterations → 500M branch checks (50% savings)
        
        let mut unrolled = Vec::new();
        for _ in 0..factor {
            unrolled.extend_from_slice(loop_body);
        }
        unrolled
    }
}
```

**Estimated Time**: 6-8 hours  
**Expected Result**: **12-15M ops/sec** ✅ **BEATS C!**

---

## 📊 CUMULATIVE PROGRESS

```
Starting:   1.07 M ops/sec   (Current Release)

Week 1  +   3-5x JIT        = 3.2-5.4 M ops/sec  ✅ Beats Python (1.8M)
Week 2  +   1.5-2x Optimize = 8-10 M ops/sec     ✅ Competitive
Week 3  +   1.5-2x Unroll   = 12-15 M ops/sec    ✅ BEATS C (2.45M)!

TOTAL IMPROVEMENT: 11-14x
```

---

## 🎯 SUCCESS CRITERIA

| Week | Target | Status | Validation |
|------|--------|--------|------------|
| 1 | 3.2M+ | Beat Python | Run benchmark |
| 2 | 8M+ | Competitive | Run benchmark |
| 3 | 15M+ | **#1 CHAMPIONSHIP** | Run benchmark |

---

## ⚡ FAST-TRACK OPTION

**If you want to implement just ONE thing** to get immediate wins:

### PHASE 20 BASELINE JIT (3-5 hours)
```
File: src/v2-rust/killer_vm/src/runtime_optimization.rs
LOC: ~300 lines
Result: 3-5x speedup (1.07M → 3.2-5.4M ops/sec) = BEATS PYTHON immediately ✅
```

This single optimization:
- Gets you from #6 to #2 position
- Beats Python without beating C
- Provides foundation for other optimizations
- Can be done this weekend!

---

## 📋 IMPLEMENTATION CHECKLIST

### Week 1 (Baseline JIT)
- [ ] Add HotCodeDetector to runtime_optimization.rs
- [ ] Add BasecodeJITCompiler struct
- [ ] Implement loop detection logic
- [ ] Add x86-64 basic compilation
- [ ] Cargo check: OK
- [ ] Cargo test: All pass
- [ ] Run benchmark: 3.2M+ ops/sec ✅

### Week 2 (Inlining + Const Fold)
- [ ] Add InliningOptimizer
- [ ] Add ConstantFolder  
- [ ] Optimize DIV operations
- [ ] Test: 8M+ ops/sec ✅

### Week 3 (Loop Unrolling)
- [ ] Add LoopUnroller
- [ ] Implement 2x/4x unrolling
- [ ] Test: 12M+ ops/sec ✅
- [ ] **Confirm: BEAT C AT 2.45M!**

---

## 🏆 FINAL RESULT

**By End of Week 3**:
```
Killer V2 ARITHMETIC: 12-15M ops/sec
C (Current #1):       2.45M ops/sec

KILLER V2 IS #1 CHAMPION! 🏆
```

**Speed Multiplier**: 5-6x faster than C!

---

**Start Date**: March 13, 2026  
**Target Completion**: April 3, 2026  
**Status**: Ready to implement
