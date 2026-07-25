# Week 5 Plan: Follow C's Approach for Native-Level Performance

## The Challenge: 234x Performance Gap

**Pure Rust arithmetic**: 250M ops/sec  
**Killer V2 Interpreter**: 1.01M ops/sec  
**C (baseline)**: 2.45M ops/sec  
**Goal**: Match or beat C

## Why We're Losing to C

### 1. Interpreter Overhead (10-15 cycles per operation)
**C**: Compile once, execute CPU instructions directly  
**Killer V2**: For EACH operation, decode instruction, dispatch to handler, execute

**Example arithmetic loop**:
```
C (direct):
    mov rax, 0          ; sum = 0
.loop:
    add rax, rcx        ; sum += i
    sub rax, rdx        ; sum -= i/2
    inc rcx              ; i++
    cmp rcx, iterations
    jl .loop
```

**Killer V2 (interpreted)**:
```
For each iteration (20M times):
  1. Decode LoadVar("i") instruction
  2. Dispatch to LoadVar handler
  3. HashMap lookup in scope
  4. Return Value::Number
  5. Decode LoadVar("sum") instruction
  6. Dispatch to LoadVar handler
  7. HashMap lookup in scope
  8. ... (repeats for every single operation)
```

### 2. Type Checking Overhead (2-3 cycles per operation)
**C**: Types resolved at compile time - no runtime checking  
**Killer V2**: Pattern matching on `Value` enum for every operation

```rust
// Every Add instruction does this:
match (&lhs, &rhs) {
    (Value::Number(l), Value::Number(r)) => { ... }
    (Value::Str(l), Value::Str(r)) => { ... }
    (Value::Str(l), r) => { ... }
    (l, Value::Str(r)) => { ... }
    _ => error
}
```

Cost: ~2-3 CPU cycles per operation × 50M operations = ~100-150M cycles

### 3. Stack Operations (2-3 cycles per operation)
**C**: Uses CPU registers (instant access)  
**Killer V2**: Vector push/pop for stack (memory access)

```rust
self.stack.push(Value::Number(...))  // Memory write
let value = self.pop_value()?         // Memory read
```

### 4. No Compiler Optimization Benefits
**C**: -O3 automatically applies:
- Loop unrolling (reduce branch overhead)
- Inlining (eliminate function calls)
- Vectorization (SIMD operations)
- Dead code elimination

**Killer V2**: Generic interpreter - can't apply these without knowing operation patterns

### 5. GC Overhead
**C**: Manual memory - zero GC pauses  
**Killer V2**: Even with concurrent GC, there's overhead for allocation tracking

---

## Week 5 Strategy: Follow C's Exact Approach

### Phase 1: Selective Native Compilation (Days 1-2)

**Goal**: For hot arithmetic loops, generate and execute native x86-64 code

**Implementation**:
```
1. Detect hot arithmetic loop (already done ✓)
2. Analyze bytecode of loop body
3. Generate x86-64 assembly:
   - For Loop (i = 0; i < N; i++)
   - For Arithmetic operations (Add/Sub/Mul/Div)
   - Unroll loop 4x to reduce branch overhead
4. Execute native code directly
5. Return result to interpreter
```

**Expected Speedup**: 3-5x (eliminates interpreter overhead + gets compiler optimizations)

### Phase 2: Type Specialization (Days 2-3)

**Goal**: For arithmetic-only loops, eliminate type checking

**Implementation**:
```
For loops detected as "arithmetic only":
- Generate specialized bytecode with no type checking
- Or: Generate native code that assumes all values are f64
- Cost: Single type pattern match at loop start
- Benefit: No per-operation type checking
```

**Expected Speedup**: 1.5-2x combined with Phase 1

### Phase 3: Register-based Stack (Days 3-4)

**Goal**: Reduce memory access to stack

**Implementation**:
```
For arithmetic loops:
- Pre-load loop variables into "registers" (Rust variables)
- Cache `i`, `sum`, `iterations` in stack frame
- Only use interpreter stack for operations
- Write back to scope after loop
```

**Expected Speedup**: 1.2-1.5x

---

## Detailed Implementation: Native Code Generation

### Step 1: Detect Arithmetic Loop Pattern

```rust
fn is_arithmetic_loop(bytecode: &[Instruction]) -> bool {
    // Check if loop contains ONLY:
    // - LoadVar
    // - Add/Sub/Mul/Div/Mod
    // - StoreVar
    // - Lt/Gt/etc comparison
    // - Jump/JumpIfFalse
    
    for instr in bytecode {
        match instr {
            Instruction::Add | Instruction::Sub | Instruction::Mul |
            Instruction::Div | Instruction::Mod | 
            Instruction::LoadVar | Instruction::StoreVar |
            Instruction::Lt | Instruction::Gt | Instruction::Le |
            Instruction::Ge | Instruction::Eq | Instruction::Ne |
            Instruction::Jump | Instruction::JumpIfFalse => {},
            _ => return false,  // Non-arithmetic instruction found
        }
    }
    true
}
```

### Step 2: Generate x86-64 Code

```rust
fn generate_arithmetic_loop_code(&self, loop_id: usize, iterations: u64) -> Vec<u8> {
    // Pseudo-code of what to generate:
    // 
    // Function prologue:
    //   push rbp
    //   mov rbp, rsp
    //   sub rsp, 0x40  ; Allocate space for variables
    //
    // Initialize variables:
    //   xor rax, rax   ; sum = 0
    //   xor rcx, rcx   ; i = 0
    //
    // Generate loop body (unrolled 4x):
    // .loop:
    //   add rax, rcx   ; sum += i
    //   sub rax, rdx   ; sum -= i/2
    //   inc rcx        ; i++
    //   
    //   add rax, rcx   ; (unrolled)
    //   sub rax, rdx   ; (unrolled)
    //   inc rcx        ; (unrolled)
    //   
    //   ... (unroll 2 more times)
    //   
    //   cmp rcx, 20000000
    //   jl .loop
    //
    // Function epilogue:
    //   mov rax, sum    ; Return result
    //   leave
    //   ret
    
    let mut code = Vec::new();
    
    // Function prologue
    code.extend_from_slice(&[0x55]);                    // push rbp
    code.extend_from_slice(&[0x48, 0x89, 0xe5]);       // mov rbp, rsp
    code.extend_from_slice(&[0x48, 0x83, 0xec, 0x40]); // sub rsp, 0x40
    
    // Initialize registers: sum (rax) = 0, i (rcx) = 0
    code.extend_from_slice(&[0x48, 0x31, 0xc0]);       // xor rax, rax
    code.extend_from_slice(&[0x48, 0x31, 0xc9]);       // xor rcx, rcx
    
    // Loop body (simplified, unroll 4x in real version)
    let loop_label = code.len();
    
    // sum += i
    code.extend_from_slice(&[0x48, 0x01, 0xc8]);       // add rax, rcx
    
    // sum -= i/2  (implemented as sum -= (i >> 1))
    code.extend_from_slice(&[0x48, 0x89, 0xca]);       // mov rdx, rcx
    code.extend_from_slice(&[0x48, 0xd1, 0xea]);       // shr rdx, 1
    code.extend_from_slice(&[0x48, 0x29, 0xd0]);       // sub rax, rdx
    
    // i++
    code.extend_from_slice(&[0x48, 0xff, 0xc1]);       // inc rcx
    
    // Loop condition: cmp rcx, iterations; jl loop_start
    // (simplified - real version would include iterations as immediate)
    code.extend_from_slice(&[0x48, 0xb9]);             // movabs rcx, iterations
    code.extend_from_slice(&iterations.to_le_bytes()); // (iterations as 8 bytes)
    code.extend_from_slice(&[0x48, 0x39, 0xc1]);       // cmp rax, rcx
    code.extend_from_slice(&[0x7d, 0x05]);             // jnl end_loop (skip 5 bytes)
    code.extend_from_slice(&[0xeb, 0xf0]);             // jmp loop_start (back ~16 bytes)
    
    // Function epilogue
    code.extend_from_slice(&[0xc9]);                    // leave
    code.extend_from_slice(&[0xc3]);                    // ret
    
    code
}
```

### Step 3: Execute Native Code

```rust
unsafe fn execute_native_code(&self, code: &[u8]) -> f64 {
    // Convert code bytes to executable function pointer
    let func: extern "C" fn() -> f64 = std::mem::transmute(code.as_ptr());
    
    // Call native function
    func()
}
```

---

## Expected Performance Progression

| Week | Approach | Target | Estimated |
|---|---|---|---|
| 3 | Hot detection + compilation (infrastructure) | - | 19.56s |
| 4 | Bottleneck analysis (discovered issue) | - | 19.74s |
| 5 | Native code generation for hot loops | 3-5x | 6-7s ⚡ |
| 6 | Type specialization + register caching | +1.5-2x | 3-4s 🚀 |
| 7 | Fine-tuning + SIMD if applicable | +1.2x | 2.5-3s 💪 |

**End Goal**: Killer V2 matching or beating C's 2.45M ops/sec (8.16s for 20M operations)

---

## Why This Follows C's Approach

| C Advantage | How We Match It | Implementation |
|---|---|---|
| Compiles to machine code | Generate native x86-64 for hot loops | Code generation + execution |
| No type checking | Specialize for arithmetic (assume f64) | Pattern detection at loop start |
| Uses registers | Cache loop variables in stack frame | Pre-load before loop, write-back after |
| Compiler optimizations | Apply loop unrolling manually | Generate 4x unrolled code |
| No GC overhead | Skip GC for hot loop execution | Execute in isolated native context |

---

## Risk Assessment and Mitigation

### Risk 1: Unsafe Code Complexity
**Solution**: Start with well-tested code generation, extensive validation

### Risk 2: Code Size Explosion
**Solution**: Only generate code for detected hot loops (usually 1-2 per program)

### Risk 3: Platform Portability
**Solution**: Start with x86-64, document platform limitations

### Risk 4: Debugging Difficulty
**Solution**: Log generated code, validate against interpreter output

---

## Checkpoint Plan

**Day 1 - Detection & Analysis**:
- [ ] Function to identify arithmetic-only loops
- [ ] Bytecode parser for loop instructions
- [ ] Test with arithmetic_bench.killer

**Day 2 - Code Generation**:
- [ ] Basic x86-64 code generator
- [ ] Generate test binary for simple loop
- [ ] Validate output matches interpreter

**Day 3 - Integration**:
- [ ] Hook code generation into hot detection
- [ ] Execute native code when hot loop detected
- [ ] Benchmark: Target 6-7 seconds

**Day 4 - Optimization**:
- [ ] Loop unrolling (4x)
- [ ] Register usage optimization
- [ ] Final benchmarks and tuning

**Day 5 - Documentation & Fallback**:
- [ ] Document native code generation
- [ ] Fallback to interpreter if code gen fails
- [ ] Comprehensive testing

---

## Success Criteria

| Metric | Target | Success |
|---|---|---|
| Arithmetic benchmark time | 6-7s | > 3x speedup |
| Operations/second | 2.8-3.3M | Beat Python (1.8M) |
| Beats C | Yes | 2.45M ops/sec |
| All tests pass | 555/555 | Zero regressions |
| Code quality | Excellent | Safe with fallback |

---

## If This Week Doesn't Full Succeed

**Checkpoint 1** (Day 2): If code generation is too complex
→ Fall back to **aggressive loop unrolling in interpreter** (1.5-2x speedup)

**Checkpoint 2** (Day 3): If native execution fails  
→ Fall back to **type specialization** + register caching (1.5x speedup)

**Minimum Success**: Any of these achieves >1.2x speedup  
**Excellent Success**: Native compilation achieves 3x speedup

---

## The Path Forward

This is the "follow C's approach" strategy:
1. **Compile hot code to native** - Like C compiles to machine code
2. **Specialize types** - Like C knows types at compile time
3. **Use registers** - Like C avoids stack operations
4. **Apply optimizations** - Like C gets loop unrolling, inlining
5. **Skip GC** - Like C's manual memory management

By the end of Week 5, Killer V2 should be executing hot arithmetic loops in a way that's fundamentally similar to C - native code with no interpreter overhead, no runtime type checking, and compiler-optimized loop structure.

**Result of success**: Killer V2 at 3M+ ops/sec, beating Python and approaching C.
