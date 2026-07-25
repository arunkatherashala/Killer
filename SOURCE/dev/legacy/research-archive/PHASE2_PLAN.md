# Phase 2: Bootstrap & Compilation Plan
## Converting Self-Hosted Interpreter to Standalone Executable

**Objective**: Create `killer.exe` / `killer` binary with ZERO Python dependency

### Architecture Overview

```
Step 1: Self-Hosted Interpreter (Already complete ✅)
   ├─ lexer.killer (tokenization)
   ├─ parser.killer (AST building)
   └─ interpreter.killer (execution)

Step 2: Bootstrap Compiler (phase 2 goal)
   ├─ codegen.killer (AST → C code)
   ├─ runtime.c (C runtime library)
   ├─ killer_bootstrap.py (Python→Killer once, then outputs C)
   └─ compiler.killer (full compiler pipeline)

Step 3: Compilation Pipeline
   ├─ killer source → lexer → tokens
   ├─ tokens → parser → AST
   ├─ AST → codegen → C code
   └─ C code → gcc/clang → native executable

Step 4: Standalone Distribution
   ├─ killer.exe (Windows)
   ├─ killer (macOS/Linux)
   └─ killer as CLI tool
```

### Implementation Tasks (5 Components)

#### 1. ✅ Code Generator (codegen.killer) - STARTED
   - Convert AST nodes to C code
   - Handle all expression types
   - Handle all statement types
   - Function and class generation
   - Memory management

#### 2. 📋 Runtime Library (runtime.c)
   - Value type system (number, string, bool, array, object)
   - Memory allocation/deallocation
   - String operations
   - Array operations
   - Object/dictionary operations
   - Math functions
   - I/O functions (print, etc)

#### 3. 📋 Compiler Pipeline (compiler.killer)
   - Orchestrate lexer → parser → codegen
   - Output C code to file
   - Handle compilation errors
   - Generate proper C structure

#### 4. 📋 Bootstrap Script (killer_bootstrap.py)
   - Use Python to load self-hosted components
   - Run a test example through the pipeline
   - Generate C code
   - Invoke C compiler (gcc/clang)
   - Create executable

#### 5. 📋 Build System (Makefile)
   - Compile Killer → executable
   - Link with runtime library
   - Strip dependencies
   - Handle Windows/Mac/Linux differences

### Success Criteria

✅ **Phase 2 Complete When:**
1. Code generator fully implemented
2. Runtime C library complete
3. Compiler pipeline orchestrates lexer → parser → codegen
4. Bootstrap script successfully compiles Killer code to C
5. `killer.exe` / `killer` binary created and tested
6. All 16 examples run on standalone executable
7. Zero Python dependency in final executable

### Timeline Estimate
- Days 1-2: Code generator (expression, statement, function, class)
- Days 3-4: Runtime C library implementation
- Days 5: Compiler orchestration
- Days 6: Bootstrap script and testing
- Days 7-8: Optimization and final executable release

### Key Files Created This Phase

```
self-hosted/
  ├─ codegen.killer (AST → C code generator)
  ├─ compiler.killer (full pipeline orchestrator)
  ├─ runtime.c (C runtime support)
  ├─ killer_bootstrap.py (Python entry point)
  └─ PHASE2_PROGRESS.md (tracking document)

output/
  ├─ killer.c (generated from Killer code)
  ├─ killer.exe / killer (compiled executable)
  └─ examples/ (test outputs)
```

### What Makes Phase 2 Different

**Phase 1**: Interpreter (AST → evaluation in memory)
- Input: Source code
- Process: Parse → Interpret
- Output: Console output

**Phase 2**: Compiler (AST → C → executable)
- Input: Source code
- Process: Parse → Generate C → Compile
- Output: Native executable file
- No Python required!

### Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| C generation complexity | Start simple with basic expressions, expand gradually |
| Memory management in C | Use reference counting or arena allocator |
| Type system mismatch | Value struct handles all Killer types |
| Cross-platform issues | Test on Windows, Mac, Linux separately |

### Next Immediate Steps

1. ✅ Complete codegen.killer (expression and statement generation)
2. Create runtime.c with C value type system
3. Build compiler.killer orchestrator
4. Test code generation with simple examples
5. Integrate C compilation step
6. Create bootstrap script
7. Test full pipeline: Killer → C → executable

### Dependencies for Phase 2

**Required**:
- gcc or clang (C compiler) - user must have installed
- Standard C library (included with gcc/clang)

**Optional**:
- Make (for Makefile build automation)

### Success Metrics

- [ ] Killer code compiles to standalone executable
- [ ] All 16 examples run on standalone binary
- [ ] No Python in final executable
- [ ] Execution faster than Python interpreter version
- [ ] File size < 5MB (after optimization)
- [ ] Works on Windows, macOS, Linux

---

**Status**: Phase 2 INITIATED
**Progress**: Code generator skeleton complete
**Next**: Expand codegen to handle all AST node types
