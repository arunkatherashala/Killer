# Killer Core + Ecosystem Checklist

This checklist is based on your requested capability list and is organized for execution.

Status keys:
- ✅ Implemented
- 🟡 Partial / needs expansion
- 📋 Planned

## Phase 1 (High Priority: Core Language Completeness)

| Area | Status | Notes |
|---|---|---|
| Basic Syntax, Keywords | ✅ | Core syntax is available and tested in phase suites. |
| Variables & Data Types | ✅ | Core primitives and containers are supported. |
| Input/Output | ✅ | `print(...)` is supported; broader I/O APIs can expand. |
| Strings (formatting/methods/slicing) | 🟡 | Basic operations exist; full parity can be expanded. |
| Control Flow (if/loops) | ✅ | `if/else`, loop forms are available. |
| Functions (params/returns) | ✅ | Regular and arrow function support exists. |
| Lambda Functions | ✅ | Arrow/lambda style supported. |
| Modules & Packages | 🟡 | Import/export exists; package-manager workflow is planned. |
| Basic Data Structures (List/Tuple/Dict/Set) | ✅ | All major containers are supported. |
| Comprehensions (list/set/dict) | ✅ | Implemented in parser/runtime. |
| Exception Handling (try/except/finally, custom errors) | 🟡 | Try/catch is present; custom error ergonomics can expand. |
| OOP: Class/Object/Inheritance | ✅ | Core OOP and inheritance/super support available. |
| OOP: Polymorphism/Encapsulation/Abstraction | 🟡 | Patterns are possible; dedicated language-level polish pending. |
| Magic Methods (`__init__`, `__str__`, `__repr__`) | 🟡 | Magic-method support exists in part; broaden test/docs coverage. |
| Operators (arithmetic/logical/etc.) | 🟡 | Core operators work; complete parity matrix should be formalized. |

## Phase 2 (Medium Priority: Runtime Power)

| Area | Status | Notes |
|---|---|---|
| File Handling | 🟡 | Baseline Flash Functions implemented (`readFile`, `writeFile`, `appendFile`, `exists`); richer APIs still planned. |
| Iterators | 🟡 | Baseline Flash Functions implemented (`iter`, `next`); advanced iterator protocol still planned. |
| Generators | ✅ | Native `yield` syntax implemented with generator function execution + `next(...)` consumption. |
| Decorators | 🟡 | Baseline decorators plus ergonomics (`@logCalls`, `@time`) implemented; advanced tooling still planned. |
| Context Managers (`with`) | ✅ | Native `with <expr> as <name> { ... }` syntax implemented; `withFile(...)` helper also available. |
| Logging | 🟡 | Baseline Flash Functions implemented (`logInfo`, `logWarn`, `logError`); richer logger configuration still planned. |
| Debugging | 🟡 | `explain`, `debug(...)`, `trace(...)`, `debugOn()/debugOff()` implemented; full debugger tooling still planned. |

## Phase 3 (Ecosystem / Tooling Integration)

| Area | Status | Notes |
|---|---|---|
| Installation & Environment (`pyenv`, `venv`, `pip`) | ✅ | Integration guide plus automated setup scripts added (`setup-dev-env.bat`, `setup-dev-env.sh`). |
| Virtual Environments (`venv`/`conda`) | ✅ | Script-driven workflows available for both `venv` (default) and `conda` (`--conda`). |

## Recommended Execution Order

1. Complete Phase 1 parity gaps (`operators`, advanced strings, exception + OOP polish).
2. Add tests per topic (`tests/killer/` files grouped by feature).
3. Build Phase 2 runtime APIs (file/iterator/generator/decorator/context manager/logging).
4. Publish Phase 3 tooling guides and installer automation for environment workflows. ✅

## Definition of Done (Per Feature)

- Parser support (if syntax-level)
- Runtime/interpreter support
- At least one focused `tests/killer/*.killer` scenario
- One short user-facing example in docs
- Included in roadmap + release notes
