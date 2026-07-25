# Killer Programming Language - Installation Guide

**Version:** 2.0  
**Status:** Production Ready

## Quick Start

### Windows
```bash
killer-standalone-installer.bat
```

### macOS / Linux
```bash
chmod +x killer-standalone-installer.sh
./killer-standalone-installer.sh
```

Developer/source installer (optional):

```bash
# Windows
killer-installer.bat

# macOS/Linux
chmod +x killer-installer.sh
./killer-installer.sh
```

Developer environment automation:

```bash
# Windows (venv default)
scripts\installers\setup-dev-env.bat

# Windows (conda)
scripts\installers\setup-dev-env.bat --conda

# macOS/Linux (venv default)
chmod +x scripts/installers/setup-dev-env.sh
./scripts/installers/setup-dev-env.sh

# macOS/Linux (conda)
./scripts/installers/setup-dev-env.sh --conda
```

---

## System Requirements

- **Python 3.6+** (required - will be checked by installer)
- **Windows:** Windows 7+
- **macOS:** macOS 10.9+
- **Linux:** Ubuntu 16.04+, Fedora 28+, Debian 9+

## Installation Steps

### Windows

1. **Download the installer**
   - Get `killer-standalone-installer.bat` from the Killer repository

2. **Run the installer**
   - Double-click `killer-standalone-installer.bat`
   - Or run from Command Prompt: `killer-standalone-installer.bat`

3. **Follow the prompts**
   - If Killer is already installed, choose **Upgrade** to update in place
   - Create installation directory: `C:\Program Files\Killer`
   - Add Killer globally to your user PATH (safe update, no duplicate entries)
   - Create documentation link on Desktop

4. **Verify installation**
   - Open a NEW Command Prompt
   - Run: `killer --version`
   - Should display version information

> If you run from the project folder without installing, PowerShell may require `./killer` or `.\killer`.
> After installer setup, `killer` works globally in new terminals.

5. **Create your first program**
   ```batch
   echo print("Hello, Killer!"); > hello.killer
   killer hello.killer
   ```

### macOS / Linux

1. **Download the installer**
   ```bash
   git clone <killer-repo>
   cd killer
   ```

2. **Make the installer executable**
   ```bash
   chmod +x killer-installer.sh
   ```

3. **Run the installer**
   ```bash
   ./killer-installer.sh
   ```

4. **Choose installation path**
   - **System-wide** (if you have sudo privileges)
     - Installed to: `/usr/local/bin/killer`
     - Command: `killer script.killer`
   
   - **User-only** (if sudo not available)
     - Installed to: `~/.local/bin/killer`
     - Command: `~/.local/bin/killer script.killer`

5. **Verify installation**
   ```bash
   killer --version
   ```

6. **Create your first program**
   ```bash
   echo 'print("Hello, Killer!");' > hello.killer
   killer hello.killer
   ```

---

## What Gets Installed

### Directory Structure (Windows)
```
C:\Program Files\Killer\
├── main.py                 # Main interpreter
├── killer.bat              # Launcher script
├── src/                    # Source code
│   ├── lexer.py
│   ├── parser.py
│   ├── interpreter.py
│   └── transpilers/
├── examples/               # 16 example programs
├── DOCUMENTATION.md        # Full reference
├── README.md              # Quick reference
└── killer-uninstall.bat   # Uninstaller
```

### Directory Structure (macOS/Linux)
```
~/.local/bin/killer/ (or /usr/local/bin/killer)
├── main.py                 # Main interpreter
├── killer-launcher.sh      # Launcher script
├── src/                    # Source code
├── examples/               # 16 example programs
├── DOCUMENTATION.md        # Full reference
├── README.md              # Quick reference
└── uninstall.sh           # Uninstaller
```

---

## Usage

### Basic Commands

Run a Killer file:
```bash
killer script.killer
```

Transpile to Python:
```bash
killer --python script.killer > script.py
python3 script.py
```

Transpile to JavaScript:
```bash
killer --js script.killer > script.js
node script.js
```

Check version:
```bash
killer --version
```

### Examples

```bash
# Run example
killer examples/01_hello.killer

# Create and run program
echo 'print("Hello, World!");' > hello.killer
killer hello.killer

# Transpile to Python
killer --python hello.killer > hello.py
python3 hello.py

# Transpile to JavaScript
killer --js hello.killer > hello.js
node hello.js
```

---

## Troubleshooting

### "killer command not found"

**Windows:**
- Ensure you ran the installer as Administrator
- Open a NEW Command Prompt after installation
- Check PATH: `echo %PATH%` should include `C:\Program Files\Killer`

**macOS/Linux:**
- If installed to `~/.local/bin`, add to PATH:
  ```bash
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
  source ~/.bashrc
  ```
- Or use full path: `~/.local/bin/killer script.killer`

### "Python not found"

Install Python:
- **Windows:** https://www.python.org/downloads/
- **macOS:** `brew install python3`
- **Ubuntu:** `sudo apt-get install python3`
- **Fedora:** `sudo dnf install python3`

Make sure to add Python to PATH during installation.

### Installer says "requires admin"

**Windows:**
- Right-click `killer-installer.bat` → "Run as administrator"

**macOS/Linux:**
- Run with sudo: `sudo ./killer-installer.sh`
- Or install to user directory (default)

### Script won't run after installation

1. Make sure it's a `.killer` file
2. Check syntax: Look at `examples/` for correct syntax
3. Try a simple example first:
   ```bash
   killer examples/01_hello.killer
   ```

---

## Uninstallation

### Windows
```batch
killer-uninstall.bat
```

Or manually:
```batch
rmdir /s "C:\Program Files\Killer"
```

### macOS/Linux
```bash
bash ~/.local/bin/killer/uninstall.sh
```

Or manually:
```bash
rm -rf ~/.local/bin/killer
rm /usr/local/bin/killer 2>/dev/null
```

---

## Environment Setup

### Windows

To add Killer to system PATH manually:
1. Press `Win + X` → "System"
2. Click "Advanced system settings"
3. Click "Environment Variables"
4. Under "System variables", select "Path" → Edit
5. Add: `C:\Program Files\Killer`
6. Click OK and restart Command Prompt

### macOS / Linux

Add to `~/.bashrc` or `~/.zshrc`:
```bash
export PATH="${HOME}/.local/bin:$PATH"
```

Then reload:
```bash
source ~/.bashrc
```

---

## Advanced Installation

### Custom Install Location (Windows)

Edit `killer-installer.bat` line 40:
```batch
set INSTALL_DIR=C:\Custom\Path\To\Killer
```

Then run the installer.

### Custom Install Location (macOS/Linux)

Edit `killer-installer.sh` line 20:
```bash
INSTALL_DIR="${HOME}/custom/path/killer"
```

Then run the installer.

---

## Getting Started

After installation:

1. **Read the docs**
   ```bash
   killer DOCUMENTATION.md    # View in terminal
   ```
   Or open: `http://localhost:8888/docs.html`

2. **Run examples**
   ```bash
   killer examples/01_hello.killer
   killer examples/02_variables.killer
   # ... and more
   ```

3. **Create your first program**
   ```bash
   echo 'print("Learn Killer");' > learn.killer
   killer learn.killer
   ```

4. **Explore transpilation**
   ```bash
   killer --python examples/01_hello.killer
   killer --js examples/01_hello.killer
   ```

---

## Support

- **Documentation:** `DOCUMENTATION.md` or `http://localhost:8888/docs.html`
- **Examples:** `examples/` folder
- **Testing:** `python tests/python/test_all_phases.py`

---

## Version History

- **v2.0** - Production release, professional installers, 48/48 tests passing
- **v1.0** - Initial release

---

## License

Killer Programming Language © 2024-2026

Happy coding! 🚀
