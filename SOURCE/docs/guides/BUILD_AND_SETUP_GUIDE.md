# Killer Language - Setup & Build Guide

## 🚀 System Setup

### Step 1: Install Rust (Required)

**Windows (Using winget):**
```powershell
winget install Rustlang.Rustup
```

**Windows (Manual):**
- Visit [rustup.rs](https://rustup.rs/)
- Download and run rustup-init.exe
- Follow installation prompts

**macOS/Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Verify Installation:**
```bash
rustc --version     # Should show rustc 1.70+
cargo --version     # Should show cargo 1.70+
```

### Step 2: Verify Prerequisites

```powershell
# Windows - Check all dependencies
pwsh src/v2-rust/killer_vm/scripts/check-prereqs.ps1

# Should show:
# [OK] cargo found
# [OK] rustup found
# [OK] x64 dbghelp.lib found
```

### Step 3: Install Python 3.8+ (Optional, for tools)

**Windows:**
```powershell
winget install Python.Python.3.11
```

**Verify:**
```bash
python --version  # Should show 3.8+
```

---

## 🔨 Building Killer

### Full Release Build

```bash
cd Killer/src/v2-rust/killer_vm

# Build optimized release binary
cargo build --release

# Binary location:
# target/release/killer_vm.exe  (Windows)
# target/release/killer_vm      (macOS/Linux)
```

**Build time:** ~2-5 minutes on first build, ~30 seconds incremental

### Dev/Debug Build (Faster)

```bash
# Use for development (slower execution, faster builds)
cargo build

# Binary location:
# target/debug/killer_vm.exe
```

### Clean Build

```bash
# Remove all build artifacts
cargo clean

# Then build fresh
cargo build --release
```

---

## ✅ Testing

### Run Test Suite

```bash
cd src/v2-rust/killer_vm

# Run all tests
cargo test

# Run tests with output
cargo test -- --show-output

# Run specific test
cargo test test_name
```

### Test a Killer Program

```bash
# Create test file
echo 'print(5 + 3)' > test.killer

# Run it
./target/release/killer_vm test.killer  # Should output: 8

# Verify with formatter
./killer_fmt.py test.killer
```

### Run Example Suite

```bash
# Test comprehensive examples
./target/release/killer_vm examples/killer_showcase_examples.killer

# Expected output: Lists and demonstrates all features
```

---

## 📝 Using the Auto-Formatter

### Python Script (killer_fmt.py)

```bash
cd src/v2-rust/killer_vm

# Format a single file in place
python killer_fmt.py mycode.killer

# Check formatting without changes
python killer_fmt.py mycode.killer --check

# Print formatted output to stdout
python killer_fmt.py mycode.killer

# Format multiple files
python killer_fmt.py file1.killer file2.killer file3.killer
```

### Create Command-Line Shortcut

**Windows (PowerShell):**
```powershell
# Create alias
$profile_dir = Split-Path $PROFILE
$profile_content = @"
function killer {
    param([Parameter(ValueFromRemainingArguments)]$args)
    & "C:\path\to\Killer\src\v2-rust\killer_vm\target\release\killer_vm.exe" @args
}

function killer_fmt {
    param([Parameter(ValueFromRemainingArguments)]$args)
    python "C:\path\to\Killer\src\v2-rust\killer_vm\killer_fmt.py" @args
}
"@

Add-Content $PROFILE $profile_content -Encoding UTF8
```

**macOS/Linux (Bash):**
```bash
# Add to ~/.bashrc or ~/.zshrc
alias killer='/path/to/Killer/src/v2-rust/killer_vm/target/release/killer_vm'
alias killer_fmt='python /path/to/Killer/src/v2-rust/killer_vm/killer_fmt.py'
```

---

## 🔍 Troubleshooting

### Issue: "cargo: command not found"

**Solution:** Restart your terminal or add Rust to PATH
```powershell
$env:Path += ';' + "$env:USERPROFILE\.cargo\bin"
```

### Issue: "error: linking with cc failed"

**Solution:** Missing Windows SDK. Install Visual Studio Build Tools:
```powershell
winget install Microsoft.VisualStudio.BuildTools
```

Select: "Desktop development with C++" + "Windows 10 SDK"

### Issue: "UTF-8 BOM error"

**Solution:** Already fixed! Lexer strips BOM automatically. Make sure you have latest code.

### Issue: Build takes forever

**Solution:** Use release build with incremental compilation:
```bash
cargo build --release  # First time: slow
cargo build --release  # Second time: fast (incremental)
```

### Issue: Test file won't run

**Diagnose:**
```bash
# Check if file is accessible
ls -la myfile.killer

# Check file encoding
file myfile.killer  # Should show "UTF-8 Unicode"

# Try simple test
echo 'print("hello")' > simple.killer
./target/release/killer_vm simple.killer
```

---

## 📦 Project Structure

```
Killer/
├── src/v2-rust/
│   └── killer_vm/                 # Rust VM implementation
│       ├── src/
│       │   ├── main.rs            # Entry point
│       │   ├── lib.rs             # Library exports
│       │   ├── lexer.rs           # Tokenizer (dual-syntax)
│       │   ├── parser.rs          # Parser (flexible blocks)
│       │   ├── ast.rs             # AST definitions
│       │   ├── compiler.rs        # Bytecode compiler
│       │   ├── bytecode.rs        # Bytecode instructions
│       │   ├── vm.rs              # Virtual machine (25+ stdlib functions)
│       │   ├── value.rs           # Value types
│       │   └── error.rs           # Error handling
│       ├── target/
│       │   ├── debug/             # Dev build output
│       │   ├── release/           # Release build output
│       │   │   └── killer_vm.exe
│       │   └── ...
│       ├── Cargo.toml             # Rust dependencies
│       ├── Cargo.lock             # Lock file
│       ├── killer_fmt.py          # Auto-formatter tool
│       ├── tests/                 # Test files
│       ├── examples/              # Example code
│       └── scripts/
│           └── check-prereqs.ps1
│
├── examples/                      # Public examples
│   └── killer_showcase_examples.killer
│
├── docs/
├── QUICK_START_GUIDE.md
├── DUAL_SYNTAX_ARCHITECTURE.md
├── PHASE_2_1_SUMMARY.md
├── MARKETING_LAUNCH_STRATEGY.md
├── README_LAUNCH.md
└── README.md
```

---

## 🚀 Deployment

### Creating Release Bundle

**Windows:**
```batch
REM Create distribution
mkdir killer-release
copy src\v2-rust\killer_vm\target\release\killer_vm.exe killer-release\
copy src\v2-rust\killer_vm\killer_fmt.py killer-release\
copy QUICK_START_GUIDE.md killer-release\
copy examples\*.killer killer-release\examples\

REM Zip it
tar -czf killer-release.zip killer-release\
```

**macOS/Linux:**
```bash
# Create distribution
mkdir killer-release
cp src/v2-rust/killer_vm/target/release/killer_vm killer-release/
cp src/v2-rust/killer_vm/killer_fmt.py killer-release/
cp QUICK_START_GUIDE.md killer-release/
cp examples/*.killer killer-release/examples/

# Tar it
tar -czf killer-release.tar.gz killer-release/
```

### GitHub Release

1. Tag the release:
   ```bash
   git tag -a v0.2.5 -m "Phase 2.5: Extended stdlib, dual-syntax, auto-formatter"
   git push origin v0.2.5
   ```

2. Create GitHub Release with:
   - Binary (killer-release.zip)
   - Quick Start Guide
   - Change log

---

## 🧪 CI/CD Pipeline (Recommended)

Create `.github/workflows/build.yml`:

```yaml
name: Build and Test

on: [push, pull_request]

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Build
        run: |
          cd v2-rust/killer_vm
          cargo build --release
      
      - name: Test
        run: |
          cd v2-rust/killer_vm
          cargo test
      
      - name: Format Check
        run: |
          cd v2-rust/killer_vm
          cargo fmt -- --check
      
      - name: Clippy
        run: |
          cd v2-rust/killer_vm
          cargo clippy -- -D warnings
```

---

## 📊 Build Statistics

| Operation | Time | Size |
|-----------|------|------|
| Clean build (release) | 2-5 min | 8-12 MB |
| Incremental build | 10-30 sec | Same |
| Debug build | 30-60 sec | 20-40 MB |
| Test suite | 30-60 sec | - |
| Binary (stripped) | - | 3-5 MB |

---

## ✨ Development Tips

### Faster Iteration

```bash
# While developing:
cargo build     # Fast debug build
cargo test --lib  # Test library only
cargo clippy    # Check for warnings

# Before commit/release:
cargo build --release
cargo test
cargo fmt
cargo clippy
```

### Code Quality

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy

# Run all checks
cargo test && cargo clippy && cargo fmt --check
```

### Debugging

```bash
# Build with debugging info
RUST_BACKTRACE=1 cargo build

# Run with backtrace
RUST_BACKTRACE=1 ./target/debug/killer_vm test.killer

# More verbose
RUST_BACKTRACE=full ./target/debug/killer_vm test.killer
```

---

## 📚 Next Steps

1. ✅ **Build:** `cargo build --release`
2. ✅ **Test:** `cargo test`
3. ✅ **Format:** Use `killer_fmt.py` on your code
4. ✅ **Examples:** Run `examples/killer_showcase_examples.killer`
5. 🚀 **Launch:** Share with community!

---

## 🆘 Need Help?

- **Build issues:** Check trouble shooting above
- **Language questions:** See QUICK_START_GUIDE.md
- **Feature requests:** GitHub Issues
- **Community:** Join Discord

---

**Happy hacking! 🚀**

*Last Updated: March 2026*
