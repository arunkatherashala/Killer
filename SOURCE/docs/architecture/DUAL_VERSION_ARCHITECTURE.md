# Killer Language - Dual Version Architecture

## Overview

Killer is available in **two independent implementations**:

### Version 2 (Rust) - PRIMARY PRODUCTION RELEASE ✅
- **Binary:** `killer-native.exe` (438 KB)
- **Location:** `v2-rust/killer_vm/target/release/killer-native.exe`
- **Approach:** Compiled Rust VM with bytecode interpreter
- **Performance:** Fast, memory-safe, single executable
- **Requirements:** None (fully standalone)
- **Status:** Production-ready ✅

**Usage:**
```bash
# Execute a Killer program
.\killer-native.exe program.killer

# Or from anywhere in PATH
killer-native.exe program.killer
```

---

### Version 1 (Python) - REFERENCE IMPLEMENTATION
- **Source:** `/v1-python/` directory
- **Entry:** `main.py`
- **Approach:** Python interpreter with transpilers
- **Features:** Can transpile to Python or JavaScript
- **Requirements:** Python 3.7+ installed
- **Status:** Legacy reference implementation

**Usage:**
```bash
# Run with Python
python main.py program.killer

# Or directly
python .\main.py program.killer
```

---

## Architecture Comparison

| Feature | V2 (Rust) | V1 (Python) |
|---------|-----------|------------|
| **Executable Size** | 438 KB | N/A (source) |
| **Startup Time** | <100ms | ~500ms |
| **Memory Usage** | Minimal | ~50MB |
| **Distribution** | Single file | Requires Python |
| **Performance** | Optimized (release build) | Interpreted |
| **Dependencies** | None | Python 3.7+ |
| **Production Ready** | ✅ Yes | ⚠️ Reference only |

---

## Recommendation for Users

**New users:** Use `killer-native.exe` (V2 Rust)
- ✅ Fastest execution
- ✅ No dependencies
- ✅ Single standalone executable
- ✅ Official production version

**Developers/Contributors:** Can inspect both:
- `killer-native.exe` for production use
- `python main.py` for understanding the interpreter logic
- `v2-rust/killer_vm/src/` for Rust VM implementation

---

## Building Both Versions

### Build Rust V2:
```bash
cd v2-rust/killer_vm
cargo build --release
# Output: target/release/killer-native.exe
```

### Run Python V1:
```bash
python main.py your_program.killer
```

---

## Distribution Strategy

### For Users:
- **Primary Release:** Distribute `killer-native.exe` only
- **Installation:** Drag & drop, add to PATH, or run directly
- **No setup required** - truly standalone

### For Developers:
- **Full Source:** Include entire repo (both V1 + V2)
- **Instructions:** Build from source with: `cargo build --release`
- **Reference:** Python V1 available for learning interpreter design

---

## Feature Parity

Both versions support:
- ✅ Dual syntax (Python indentation + Go braces)
- ✅ 25 standard library functions
- ✅ Variables, functions, loops, conditionals
- ✅ Custom user-defined functions
- ✅ Arrays and dictionaries

**V2 (Rust) Advantages:**
- Bytecode compilation
- Better error messages
- Memory safety (no segfaults)
- Faster execution
- Single executable

**V1 (Python) Advantages:**
- Easier to modify/extend
- Transpilation to other languages
- Educational - see interpreter code directly

---

## Migration from V1 to V2

Code written for V1 works directly in V2:
```killer
// Works in both V1 and V2
kfn greet(name) {
    print("Hello, " + name)
}

greet("World")
```

No rewriting needed - same language, different runtime!

---

## Future Roadmap

- **V2 Focus:** Performance optimization, IDE support, package manager
- **V1 Role:** Educational reference, research & history
- **Long-term:** V3 (Self-hosted in Killer) - compiler written in Killer

---

## Version Selection

```
Need fast execution?           → killer-native.exe (V2)
Learning interpreter design?   → python main.py (V1)
Contributing to core?          → Both (understand each)
Deploying production app?       → killer-native.exe (V2)
```

---

**Summary:** Two production-ready versions, same language, different implementation. Users get V2's speed; developers get V1's transparency.
