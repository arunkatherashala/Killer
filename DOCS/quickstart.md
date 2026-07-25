# Killer Language Quickstart

Welcome to Killer! This guide covers:
- Building the compiler and VM
- Running your first program
- Using the package manager (KPM)
- Running tests

## Prerequisites

- **Rust** (1.70+): Install from https://rustup.rs/
- **Windows** (this guide assumes Windows; Linux/macOS similar):
  - PowerShell 5.1+
  - Git

## Step 1: Build the Compiler

```powershell
# Clone the repository
git clone https://github.com/arunkatherashala/Killer.git
cd Killer

# Build Killer compiler in release mode
cd SOURCE/src/v2-rust/killer
cargo build --release

# Compiler binary: target/release/killer.exe (Windows) or target/release/killer (Unix)
```

## Step 2: Run Your First Program

Create a file `hello.killer`:

```killer
x = 10
y = 20
print(x + y)
```

Then run it:

```powershell
./target/release/killer hello.killer
# Output: 30
```

## Step 3: Test Recursion and Scoping

Create `fib.killer`:

```killer
def fib(n) {
  if n <= 1 {
    return n
  }
  return fib(n - 1) + fib(n - 2)
}

result = fib(10)
print(result)  # Output: 55
```

Run it:

```powershell
./target/release/killer fib.killer
```

## Step 4: Run Killer Tests

```powershell
# From the killer directory (SOURCE/src/v2-rust/killer)
cargo test --release

# Expected: 17 tests passing (recursion, shadowing, scoping verified)
```

## Step 5: Using KPM (Package Manager)

### Build KPM CLI

```powershell
cd tools/kpm
cargo build --release
# Binary: target/release/kpm.exe (Windows) or target/release/kpm (Unix)
```

### Install a Local Package

Create a package manifest `my-package/manifest.json`:

```json
{
  "name": "killer-stdlib",
  "version": "1.0.0",
  "files": ["lib.killer", "utils.killer"]
}
```

And sample files:

```powershell
# Add your .killer files to the directory
echo "def helper() { print(\"helper\") }" > lib.killer
```

Then install:

```powershell
./target/release/kpm install my-package/manifest.json --dest ./kpm_packages
```

### Publish a Package

```powershell
./target/release/kpm publish my-package/manifest.json .
# Output: Published killer-stdlib 1.0.0 -> ./killer-stdlib-1.0.0.tar.gz (sha256: abc123...)
```

### Resolve Package Versions

```powershell
./target/release/kpm resolve killer-stdlib 1.0.0
# Output: resolved: killer-stdlib@1.0.0
```

## Step 6: Advanced Topics

### Trit Logic

Killer supports balanced ternary (trit) types for quantum-inspired computing:

```killer
x = T(+1)   # Positive trit
y = T(-1)   # Negative trit
z = T(0)    # Zero trit
result = x && y  # Trit AND
print(result)
```

### Signals (Confidence Values)

```killer
sig = Signal(+1, 0.95, "confident")
print(sig.value)       # +1
print(sig.confidence)  # 0.95
print(sig.reason)      # "confident"
```

### Fuzzy Logic

```killer
a = 0.7
b = 0.3
fuzzy_and = min(a, b)  # 0.3
fuzzy_or = max(a, b)   # 0.7
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| "killer" command not found | Add `target/release` to PATH or use `./target/release/killer` |
| Cargo fails to build | Update Rust: `rustup update` |
| Tests fail locally | Run via GitHub Actions: `gh workflow run kpm-ci.yml` |
| KPM install fails | Check manifest.json is valid JSON and files exist |

## Next Steps

- **Read the docs**: [Language reference](../docs/language-reference.md)
- **Explore examples**: [examples/](../examples/)
- **Contribute**: See [CONTRIBUTING.md](../CONTRIBUTING.md)

Happy coding! 🚀
