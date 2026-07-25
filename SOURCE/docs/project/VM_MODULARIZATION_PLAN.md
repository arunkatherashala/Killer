# VM Modularization Plan - TIER 2

## Objective
Split vm.rs (2,830 lines) into 7 focused modules to improve maintainability and code clarity.

## Module Breakdown

### 1. **stack.rs** (~100 lines) - Stack & Scope Management
**Responsibility**: Push/pop scope, load/store variables, stack operations
**Key functions**:
- `push_scope()`
- `pop_scope()`
- `load_var(name)`
- `store_var(name, value)`
- `pop_value()`
- `pop_number()`

### 2. **operations.rs** (~200 lines) - Arithmetic & Comparison
**Responsibility**: Basic mathematical and logical operations
**Key instructions**:
- Add, Sub, Mul, Div, Mod, Pow
- Eq, Ne, Gt, Lt, Ge, Le
- And, Or, Not
- Increment/Decrement operations

### 3. **builtin.rs** (~600 lines) - Builtin Functions
**Responsibility**: All builtin functions (len, type, map, filter, next, etc.)
**Key instructions**:
- CallBuiltin (all builtin functions)
- Math functions (sqrt, sin, cos, etc.)
- String functions (length, upper, lower, etc.)
- Array methods (map, filter, reduce, etc.)

### 4. **generator.rs** (~200 lines) - Generator State Management
**Responsibility**: Yield collection and generator lifecycle
**Key instructions**:
- Yield
- Generator creation and state tracking
- Next functionality

### 5. **exception.rs** (~150 lines) - Exception Handling
**Responsibility**: Try/catch/finally/throw operations
**Key instructions**:
- TryEnter, TryExit
- Throw
- Catch and Finally handling
- Error propagation logic

### 6. **objects.rs** (~250 lines) - OOP & Method Calls
**Responsibility**: Classes, instantiation, method calls, inheritance
**Key instructions**:
- NewObject (class instantiation)
- CallMethodDynamic (method calls)
- This binding
- Inheritance chain walking

### 7. **core.rs** (~330 lines) - Core Execution Engine
**Responsibility**: Main run loop and instruction dispatch
**Content**:
- VirtualMachine struct
- Main run() loop
- Instruction match statement
- Module coordination

## Implementation Strategy

### Phase 1: Foundational Modules (Low Risk)
1. Create `stack.rs` - Extract push_scope, pop_scope, load_var, store_var
2. Test all tier1 tests after extraction
3. Create `operations.rs` - Extract arithmetic operations
4. Test again

### Phase 2: Specialized Modules (Medium Risk)
5. Create `builtin.rs` - Extract CallBuiltin handler
6. Test extensively (largest module, most used)
7. Create `exception.rs` - Extract exception handling
8. Test try/catch/finally functionality

### Phase 3: Complex Modules (High Risk)
9. Create `objects.rs` - Extract OOP functionality
10. Test class instantiation and inheritance
11. Create `generator.rs` - Extract generator logic
12. Test generator functionality

### Phase 3: Final Assembly
13. Create `core.rs` - Keep main run loop
14. Update `mod.rs` to declare submodules
15. Update `lib.rs` to use new structure
16. Full test suite validation

## Testing Strategy
After each module creation:
1. Run all 56 tier1 tests
2. Check for compilation warnings
3. Run bug fix tests (dict filter, unicode, null access)
4. Verify no performance regression with stress tests

## Rollback Plan
Keep original vm.rs as `vm.rs.backup` until all modules are fully tested and working.

## Estimated Completion
- Foundational modules: 30 minutes
- Specialized modules: 1 hour
- Complex modules: 1 hour
- Final assembly & testing: 30 minutes
- **Total**: ~3 hours of focused work

## Current Status
- [x] Plan created
- [ ] Phase 1: Stack module
- [ ] Phase 1: Operations module
- [ ] Phase 2: Builtin module
- [ ] Phase 2: Exception module
- [ ] Phase 3: Objects module
- [ ] Phase 3: Generator module
- [ ] Phase 4: Core finalization
- [ ] Full test validation
