# KILLER PROGRAMMING LANGUAGE

## About the Project

**Killer** is a modern, self-hosted programming language designed to be:
- Independent (zero external dependencies)
- User-friendly (JavaScript-like syntax)
- Powerful (full OOP, functions, classes)
- Transparent (open source, written in itself)

### Founded By
**Katherashala Sai Arun Kumar**

### Project Information
- **Version**: 3.0 (Self-Hosted Compiler)
- **Release Date**: March 8, 2026
- **Status**: Production Ready ✅
- **License**: Open Source
- **GitHub**: https://github.com/arunaug2008-ai/Killer

---

## Key Milestones

### Phase 1: Self-Hosted Interpreter ✅
Built a complete Killer interpreter IN Killer itself
- 1650+ lines of pure Killer code
- Full language support (variables, functions, classes, arrays, objects)
- All 16 example programs pass tests

### Phase 2: Bootstrap Compiler ✅
Created compilation pipeline from Killer → C → Native Binary
- Killer code compiles to C automatically
- C runtime library (14KB)
- Zero Python dependency for end users

### Phase 3: Module System (In Progress)
- Import/export system
- Package manager
- Standard library modules
- Decorators and advanced OOP

---

## Vision

Make programming accessible, transparent, and independent. 

By writing Killer in Killer, we prove that:
- Language interpreters can be understandable
- Programs can be compiled to native code
- Users can audit their tools completely
- Self-hosting is achievable and practical

---

## What Makes Killer Different

### Self-Hosting
The Killer interpreter is written in Killer itself, not in C or Python. This means:
- Users can read and understand how it works
- Developers can modify and extend it
- The language proves its own capabilities

### Zero Dependencies
- No Python dependency for running compiled code
- No external tools required
- Single installer, single binary
- Works offline after installation

### Transparency
- Full source code available
- Every operation explicit
- No hidden complexity
- Community-auditable

### Developer Experience
```killer
# Simple, readable syntax
print("Hello, World!");

fn greet(name) {
    return "Hello, " + name;
}

arr = [1, 2, 3];
obj = {name: "Killer", version: 3.0};

class Person {
    constructor(name) {
        this.name = name;
    }
}

if (version >= 3) {
    print("Latest version!");
}
```

---

## Current Capabilities

✅ Variables and constants  
✅ All data types (numbers, strings, booleans, arrays, objects)  
✅ Functions (regular and arrow)  
✅ Classes with inheritance  
✅ Arrays with methods  
✅ Objects with properties  
✅ Control flow (if/else, while, for, switch)  
✅ String operations  
✅ Error handling (try/catch)  
✅ Regular expressions  
✅ Math operations  

---

## Files & Components

### Core Interpreter
```
self-hosted/
├── lexer.killer          (350 lines - Tokenization)
├── parser.killer         (700 lines - AST Building)
├── interpreter.killer    (600 lines - Execution)
└── runtime.c            (400 lines - C Runtime)
```

### Bootstrap Compiler
```
root/
├── killer_bootstrap.py   (360 lines - Compilation Pipeline)
├── killer.killer         (2182 lines - Complete Interpreter)
├── killer.bat            (Windows Wrapper)
└── killer.sh             (Unix Wrapper)
```

### Distribution
```
├── killer-standalone-installer.bat    (Windows)
├── killer-standalone-installer.sh     (macOS/Linux)
├── examples/                          (16+ examples)
└── Documentation/                     (guides, checklists)
```

---

## Installation & Usage

### Install
```bash
# Windows
.\killer-standalone-installer.bat

# macOS/Linux  
sudo bash killer-standalone-installer.sh
```

### Run
```bash
killer hello.killer
killer examples/05_functions.killer
```

### Create
```killer
# Save as: myprogram.killer
print("My First Killer Program");

fn add(a, b) {
    return a + b;
}

result = add(10, 20);
print(result);  # Output: 30
```

---

## Technical Architecture

### Compilation Pipeline
```
killer.killer (Killer source)
    ↓ [Lexer]
Tokens
    ↓ [Parser]
Abstract Syntax Tree (AST)
    ↓ [Code Generator]
C Code (generated.c)
    ↓ [gcc/clang]
Native Binary (killer.exe / killer)
    ↓
User runs: killer myprogram.killer
    ↓ [Killer Interpreter]
Output
```

### Three-Layer Architecture

**Layer 1: Python v2.0**
- Initial Killer implementation
- 48/48 test cases pass
- Used to bootstrap everything

**Layer 2: Killer v2.5 (Self-Hosted)**
- Killer written in Killer
- Lexer, Parser, Interpreter
- 1650+ lines of pure code
- 16/16 examples pass

**Layer 3: Native Compiled**
- Killer compiles to C code
- C compiles to native binary
- Zero Python dependency
- Can run standalone

---

## Command Reference

### Basic
```bash
killer script.killer              # Run a script
killer --version                  # Show version
killer --help                     # Show help
```

### Language Examples
```killer
# Variables
x = 10;
name = "Killer";
ready = true;

# Arrays
arr = [1, 2, 3];
arr.push(4);
arr[0];  # Access

# Functions
fn multiply(a, b) {
    return a * b;
}

# Classes
class Animal {
    constructor(name) {
        this.name = name;
    }
    
    speak() {
        print(this.name);
    }
}

dog = new Animal("Buddy");
dog.speak();

# Control Flow
if (x > 5) {
    print("Greater");
} else {
    print("Less");
}

while (i < 10) {
    i = i + 1;
}
```

---

## Roadmap

### v3.0 ✅ (Current)
- Self-hosted interpreter
- Bootstrap compiler
- Zero dependencies
- 16 example programs

### v3.1 (Coming)
- Module system (import/export)
- Package manager (killer-pkg)
- REPL (interactive shell)
- Standard library

### v4.0 (Future)
- Abstract classes
- Interfaces & mixins
- Generics
- Decorators
- Advanced OOP

### v5.0 (Future)
- Async/await
- Pattern matching
- Reflection
- Advanced collections

---

## Development Team

**Founder & Creator**: Katherashala Sai Arun Kumar

**Contributors**: Open to contributions!

---

## Getting Involved

### Testing
- Run the test suite
- Test on different platforms
- Report bugs & issues

### Development
- Fork the repository
- Submit pull requests
- Help with Phase 3+ features

### Documentation
- Improve examples
- Translate documentation
- Create tutorials

---

## FAQ

**Q: Is Killer production-ready?**  
A: Yes! v3.0 is ready for use. We recommend testing additional use cases.

**Q: Can I use Killer for real projects?**  
A: Yes, but we recommend monitoring GitHub for updates and improvements.

**Q: Will Killer have package management?**  
A: Yes, in v3.1 with the built-in package manager.

**Q: Can I contribute to Killer?**  
A: Absolutely! We welcome contributions. See GitHub for details.

**Q: Is the source code available?**  
A: Yes, completely open source on GitHub.

**Q: How is it different from JavaScript/Python?**  
A: Simpler, self-hosted, zero dependencies, fully transparent.

---

## Resources

- **GitHub**: https://github.com/arunaug2008-ai/Killer
- **Examples**: See `examples/` directory
- **Guides**: See documentation files
- **Testing**: See TESTING_GUIDE.md

---

## License & Attribution

Written by: **Katherashala Sai Arun Kumar**

Open Source Project - Community Driven

---

*Killer Programming Language v3.0*  
*Self-Hosted, Zero Dependencies, Fully Transparent*  
*"Because code should be understandable."*
