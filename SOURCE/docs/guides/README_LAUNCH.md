# 🔫 Killer Language

> **The Flexible Language** — Write code YOUR way
>
> *The only modern language that supports both Python-style indentation AND Go-style braces in the same codebase*

[![GitHub stars](https://img.shields.io/github/stars/arunaug2008-ai/Killer)](https://github.com/arunaug2008-ai/Killer)
[![Discord](https://img.shields.io/discord/)](https://discord.gg/killer-language)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ✨ What Makes Killer Different?

While every language forces you to choose ONE style, Killer lets you have **BOTH**:

### Python Style ✨
```killer
fn fibonacci(n)
    if n <= 1
        n
    else
        fibonacci(n - 1) + fibonacci(n - 2)
```

### Go Style 💪
```killer
fn fibonacci(n) {
    if (n <= 1) {
        n
    } else {
        fibonacci(n - 1) + fibonacci(n - 2)
    }
}
```

### Mixed in Real Code 🎯
```killer
fn process_results(data) {
    results = []
    
    for item in data
        if item["score"] > 80
            results.push(item)
    
    results
}
```

**All the same language. You choose.**

---

## 🎯 Philosophy: Simple + Strong + Secure

```
         SIMPLE
           /\
          /  \
         /    \
    STRONG -- SECURE
```

Killer optimizes for all three:
- **SIMPLE:** No unnecessary complexity. Python-level readability.
- **STRONG:** Type safety and static analysis without verbosity.
- **SECURE:** Memory-safe runtime with predictable semantics.

---

## 🚀 Quick Start

### Your First Program
```killer
print("Hello, Killer!")
```

### Run It
```bash
killer hello.killer
```

### More Examples
- 📚 [Quick Start Guide](QUICK_START_GUIDE.md) - 5 minutes to productive
- 🎨 [Beautiful Examples](examples/killer_showcase_examples.killer) - Real-world patterns
- 📖 [Full Documentation](DUAL_SYNTAX_ARCHITECTURE.md) - Deep dive

---

## 💎 Key Features

### ✅ Dual Syntax
Choose indentation-based (Python) or brace-based (Go) syntax **per project**. Not per file. Not forced.

### ✅ Auto-Formatter
```bash
killer fmt mycode.killer  # One command, consistent style
```

### ✅ Comprehensive Standard Library  
**25+ built-in functions** including:
- `len()`, `range()`, `type()`, `str()`, `int()`
- String methods: `upper()`, `lower()`, `trim()`, `split()`, `replace()`, `contains()`
- Array methods: `push()`, `pop()`, `slice()`, `reverse()`, `join()`, `concat()`
- Dictionary operations: `keys()`, `values()`

### ✅ Smart Defaults
- Implicit returns (last expression is returned)
- Optional semicolons
- UTF-8 support out of the box
- Clear, helpful error messages

### ✅ Safe by Design
- Memory-safe (no buffer overflows, use-after-free)
- Bounds checking for arrays
- Type safe (catch errors early)

### ✅ Fast Execution
- Rust-based VM = speed + safety
- Bytecode compilation
- No garbage collection pauses
- Predictable performance

---

## 📊 Language Comparison

| Feature | Killer | Python | Go | Rust | JavaScript |
|---------|--------|--------|----|----|------------|
| **Dual Syntax** | ✅ | ❌ | ❌ | ❌ | ❌ |
| **Simple Syntax** | ✅ | ✅ | ❌ | ❌ | ❌ |
| **Implicit Returns** | ✅ | ❌ | ❌ | ✅ | ✅ |
| **Memory Safe** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Fast** | ✅ | ❌ | ✅ | ✅ | ❌ |
| **Learning Curve** | 🟢 Easy | 🟢 Easy | 🟡 Medium | 🔴 Hard | 🟡 Medium |

---

## 🏗️ Architecture

**Killer is built with a modern multi-layer architecture:**

```
┌─────────────────────────────────────┐
│    VS Code IDE Extension (TS)     │  Editor integration, LSP support
├─────────────────────────────────────┤
│      Parser & Compiler (Rust)      │  Lexer → Parser → AST → Bytecode
├─────────────────────────────────────┤
│     Virtual Machine Core (Rust)     │  Type-safe bytecode interpreter
├─────────────────────────────────────┤
│    Standard Library (25+ functions)  │  String/Array/Dict/Type operations
└─────────────────────────────────────┘
```

**Implementation Status:**
- ✅ Phase 1: Core language (variables, functions, control flow, recursion)
- ✅ Phase 2.1: Standard library & simplified syntax (7+ functions, implicit returns)
- 🔄 Phase 2.5: Extended stdlib (25+ functions, dual-syntax support, auto-formatter)
- 📋 Phase 3: OOP (classes, methods, inheritance)

---

## 📦 What's Included

```
killer/
├── src/v2-rust/killer_vm/         # Rust VM implementation
│   ├── src/
│   │   ├── lexer.rs          # Tokenizer (dual-syntax support)
│   │   ├── parser.rs         # Parser (flexible blocks)
│   │   ├── compiler.rs       # Bytecode compiler
│   │   ├── vm.rs             # Bytecode interpreter
│   │   └── ...
│   ├── killer_fmt.py         # Auto-formatter
│   └── Cargo.toml
├── examples/                  # Example code
│   ├── killer_showcase_examples.killer
│   └── ...
├── QUICK_START_GUIDE.md      # 10-minute introduction
├── DUAL_SYNTAX_ARCHITECTURE.md # Technical deep-dive
└── README.md                  # This file
```

---

## 🛠️ Installation

### From Source (Recommended for Development)
```bash
# Clone the repository
git clone https://github.com/arunaug2008-ai/Killer.git
cd Killer/src/v2-rust/killer_vm

# Build with Cargo
cargo build --release

# Run a Killer file
./target/release/killer_vm yourfile.killer
```

### Requirements
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Python 3.8+ (for development tools)
- VS Code (for IDE extension)

---

## 💻 Usage

### Run a File
```bash
killer myprogram.killer
```

### Format Your Code
```bash
# Auto-format in place
killer fmt myprogram.killer

# Check formatting without changes
killer fmt --check myprogram.killer
```

### Interactive Mode (Coming Soon)
```bash
# REPL for interactive exploration
killer --repl
```

---

## 📚 Learning Resources

### For New Programmers
- Start with [Quick Start Guide](QUICK_START_GUIDE.md)
- Run examples from `examples/` directory
- Experiment in the playground

### For Python Developers
- See: "How Killer Compares to Python"
- Familiar syntax, enhanced safety
- Type system without the verbosity

### For Go Developers
- Full brace-based syntax support
- Memory safety without the complexity
- Clean semantics with Python's readability option

### For Language Designers
- Read [DUAL_SYNTAX_ARCHITECTURE.md](DUAL_SYNTAX_ARCHITECTURE.md)
- Review Rust source code in `src/v2-rust/killer_vm/src/`
- See how dual-syntax is implemented

---

## 🎓 Code Examples

### Hello World (Traditional)
```killer
print("Hello, World!")
```

### Variables & Types
```killer
name = "Killer"
version = 1.0
features = ["dual-syntax", "type-safe", "fast"]
info = {"name": name, "version": version}

print(name)     // Killer
print(type(version))  // "number"
```

### Control Flow
```killer
fn grade_score(score)
    if score >= 90
        "A"
    else if score >= 80
        "B"
    else
        "C"

print(grade_score(95))  // A
```

### Arrays & Iteration
```killer
numbers = [1, 2, 3, 4, 5]

// Using standard library
doubled = []
for n in numbers
    doubled.push(n * 2)

print(join(doubled, ", "))  // "2, 4, 6, 8, 10"
```

### String Manipulation
```killer
text = "hello world"
uppercase = upper(text)           // "HELLO WORLD"
words = split(uppercase, " ")     // ["HELLO", "WORLD"]
result = join(words, "-")         // "HELLO-WORLD"

print(result)
```

### Functions & Arrow Syntax
```killer
// Traditional
fn add(a, b)
    a + b

// Arrow syntax
multiply(a, b) => a * b

print(add(3, 4))         // 7
print(multiply(3, 4))    // 12
```

---

## 🤝 Contributing

We welcome contributions! Here's how:

1. **Code:** Improve the VM, add features, fix bugs
2. **Documentation:** Clarify docs, write examples
3. **Community:** Help others, share your creations
4. **Ideas:** Suggest features via GitHub Issues

### Development Setup
```bash
# Clone and enter directory
git clone https://github.com/arunaug2008-ai/Killer.git
cd Killer

# Check prerequisites
pwsh src/v2-rust/killer_vm/scripts/check-prereqs.ps1

# Build and test
cd src/v2-rust/killer_vm
cargo build --release
cargo test
```

### Areas We Need Help
- [ ] Phase 3: OOP implementation (classes, inheritance)
- [ ] Phase 4: Standard library expansion
- [ ] Documentation improvements
- [ ] Example programs
- [ ] Performance optimizations
- [ ] Windows/Mac/Linux testing

---

## 📄 License

Killer is open-source and free to use under the [MIT License](LICENSE).

---

## 🌟 Roadmap

### ✅ Completed (Phase 1-2.1)
- [x] Core language features
- [x] Dual-syntax support
- [x] 25+ standard library functions
- [x] Auto-formatter
- [x] Implicit returns

### 🔄 In Progress (Phase 2.5)
- [x] Extended standard library
- [ ] IDE/LSP extension
- [ ] Performance optimization

### 📋 Coming Next (Phase 3)
- [ ] Object-oriented programming (classes, methods)
- [ ] Exception handling (try/catch)
- [ ] Module/package system
- [ ] Web framework integration

### 🎯 Future (Phase 4+)
- [ ] Killer self-hosting (compiler in Killer)
- [ ] Package manager
- [ ] Native compilation (via LLVM)
- [ ] Async/await support
- [ ] REPL/Interactive mode

---

## 💬 Community

- **Discord:** [Join our server](https://discord.gg/killer-language)
- **GitHub Discussions:** Ask questions, share ideas
- **Twitter:** [@KillerLanguage](https://twitter.com/killer-language)
- **Issue Tracker:** Report bugs, request features

---

## ✨ Why Killer?

**The name reflects our vision:**
- **Killer app:** Small, focused, does one thing (dual syntax) extremely well
- **Killer feature:** Unique competitive advantage (support both styles)
- **Killer community:** Bringing together Python and Go developers

**Most importantly:** A language that's actually simple to learn, strong enough to trust, and secure by default. A language that doesn't force you to compromise. A language that's... killer. 🔫

---

## 📞 Support

- **Questions?** Check [Quick Start Guide](QUICK_START_GUIDE.md) or [FAQ](#frequently-asked-questions)
- **Found a bug?** [Open an issue](https://github.com/arunaug2008-ai/Killer/issues)
- **Want to help?** [Start contributing](CONTRIBUTING.md)
- **Need something else?** Join our [Discord community](https://discord.gg/killer-language)

---

**Built with ❤️ in Rust, usable everywhere**

*Last Updated: March 2026*
