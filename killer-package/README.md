# 🔥 Killer Language - Distribution Package

**Version**: v1.1  
**Release Date**: March 2026  
**Status**: Production Ready

---

## What is Killer?

Killer is a **standalone compiled language** built on Rust for:
- ✅ Real-time systems (p50/p99 latencies < 100ms)
- ✅ High concurrency (1000+ simultaneous tasks)
- ✅ Network services & microservices
- ✅ Systems programming
- ✅ Data processing & stream operations

**Key Features:**
- Actor model for concurrency (no GIL like Python)
- Strong type system with pattern matching
- Simple, elegant syntax (Python-like readability)
- Cross-platform (Windows, Mac, Linux)
- Compiles to native executables

---

## Quick Start

### Installation

1. **Extract this package** to your desired location
2. **No additional dependencies** - killer.exe is all you need!

### Hello World

Create `hello.killer`:
```killer
kfn main() {
  println("Hello from Killer!")
}

main()
```

Run it:
```bash
killer.exe hello.killer
```

---

## Folder Structure

```
killer-package/
├── killer.exe              ← The compiler (run your .killer files)
├── README.md              ← This file
├── QUICK_START.md         ← More examples
├── examples/              ← Sample programs
│   ├── hello.killer
│   ├── fibonacci.killer
│   ├── server.killer
│   └── actor_example.killer
└── docs/                  ← Documentation
    ├── SYNTAX.md
    ├── ACTORS.md
    └── PERFORMANCE.md
```

---

## Common Commands

```bash
# Compile and run a Killer program
killer.exe myprogram.killer

# Run multiple programs
killer.exe program1.killer program2.killer

# Check version
killer.exe --version
```

---

## Documentation

- **QUICK_START.md** - 5 minute tutorial
- **docs/SYNTAX.md** - Language syntax reference
- **docs/ACTORS.md** - Concurrency patterns
- **docs/PERFORMANCE.md** - Performance tuning

---

## Examples

See `examples/` folder for:
- `hello.killer` - Basic output
- `fibonacci.killer` - Functions & recursion
- `server.killer` - HTTP server example
- `actor_example.killer` - Concurrent actors

---

## Support

For issues or questions:
1. Check QUICK_START.md
2. Review examples/
3. See docs/ for detailed reference

---

## License

This is YOUR product. Use, modify, and distribute as needed.

**Version**: v1.1 | **March 2026**
