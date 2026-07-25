# Track B Feature Split (Phase 1 → Phase 2)

This document divides all major Killer language/runtime features into Track B implementation phases.

Status keys:
- ✅ Implemented in Track B
- 🟡 Partial in Track B
- 📋 Planned for Track B

## Phase 1 (Core Language Parity)

Goal: native Track B can run common Killer programs with core control flow and function logic.

| Feature Area | Track A | Track B | Phase | Notes |
|---|---|---|---|---|
| Variables (`let`, assignment) | ✅ | ✅ | Phase 1 | Native subset supports declaration and reassignment. |
| Primitive values (number/string/bool) | ✅ | ✅ | Phase 1 | Supported in native subset compiler and VM. |
| Output (`print`) | ✅ | ✅ | Phase 1 | Native subset supports `print(expr)`. |
| Arithmetic (`+ - * /`) | ✅ | ✅ | Phase 1 | End-to-end supported in native subset. |
| Comparisons (`== != > < >= <=`) | ✅ | ✅ | Phase 1 | Supported in native subset conditions. |
| Logical ops (`&& ||`) | ✅ | ✅ | Phase 1 | Supported in native subset conditions. |
| `if / else` | ✅ | ✅ | Phase 1 | Supported with block syntax. |
| `while` loops | ✅ | ✅ | Phase 1 | Supported with native jump compilation. |
| `break` / `continue` | ✅ | ✅ | Phase 1 | Supported inside `while` loop context. |
| Functions (`fn`, params, call) | ✅ | ✅ | Phase 1 | Supported with arg binding and arity checks. |
| `return` | ✅ | ✅ | Phase 1 | Supported inside functions. |
| Forward function calls | ✅ | ✅ | Phase 1 | Pending-call patching added. |
| Recursion | ✅ | 🟡 | Phase 1 | Engine supports call stack; dedicated factorial demo/test still pending. |
| Native CLI run (`--killer`) | N/A | ✅ | Phase 1 | Direct source path added for subset. |

## Phase 2 (Extended Language + Runtime Parity)

Goal: close major parity gaps with full Killer feature set and advanced runtime behavior.

| Feature Area | Track A | Track B | Phase | Notes |
|---|---|---|---|---|
| `for` loops | ✅ | 📋 | Phase 2 | Add parser + bytecode lowerings for range/iter loops. |
| Arrays/list operations | ✅ | ✅ | Phase 2 | Literals `[1,2,3]`, indexing `arr[i]`, mutation `arr[i]=v`. Length/iteration pending. |
| Dictionaries/maps | ✅ | ✅ | Phase 2 | Literals `{key: val}`, key access `dict[key]`, updates `dict[key]=v`. Iteration pending. |
| Classes/objects/inheritance | ✅ | 📋 | Phase 2 | Core OOP model parity target. |
| Exceptions (`try/catch/finally`) | ✅ | 📋 | Phase 2 | Runtime error model + structured handlers. |
| Modules/imports | ✅ | 📋 | Phase 2 | Module loader and import resolution. |
| Lambdas/arrow functions | ✅ | 📋 | Phase 2 | Function literal support in native compiler path. |
| Generators / `yield` | ✅ | 📋 | Phase 2 | VM suspension/resume model. |
| Context manager (`with`) | ✅ | 📋 | Phase 2 | Resource-safe block semantics. |
| Decorators (`@...`) | ✅ | 📋 | Phase 2 | Function wrapping metadata and call transformation. |
| Debug/explain hooks | ✅ | 📋 | Phase 2 | Native debug tracing + explain parity strategy. |
| Logging APIs | ✅ | 📋 | Phase 2 | Native host logger APIs parity. |
| File handling APIs | ✅ | 📋 | Phase 2 | `readFile/writeFile/appendFile/exists` parity path. |
| Iterators/protocol helpers | ✅ | 📋 | Phase 2 | `iter/next` style parity and protocol behavior. |

## Exit Criteria

### Phase 1 done when:
- All Phase 1 rows are `✅` in Track B.
- Core sample suite (conditionals, loops, functions) passes in native mode.
- Output parity is validated against Track A for selected examples.

### Phase 2 done when:
- Phase 2 rows required for v1 parity are `✅`.
- Native mode can be enabled by default with Track A fallback.

## Suggested Work Order (Track B)

1. Finish Phase 1 recursion verification + parity checks.
2. Implement Phase 2 data structures (`array`, `dict`) first.
3. Add modules and exception model.
4. Implement OOP model.
5. Add advanced runtime features (`yield`, `with`, decorators, tooling hooks).
