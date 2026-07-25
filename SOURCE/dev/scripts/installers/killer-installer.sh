#!/bin/bash

# =============================================================================
# Killer Programming Language - Mac/Linux Installer
# Version: 2.0
# =============================================================================

set -e  # Exit on any error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
INSTALL_DIR="${HOME}/.local/bin/killer"
SYSTEM_BIN="/usr/local/bin/killer"
LOGO="
╔═══════════════════════════════════════════════════════════════════════════╗
║                    KILLER PROGRAMMING LANGUAGE                            ║
║                      Mac/Linux Installer v2.0                            ║
╚═══════════════════════════════════════════════════════════════════════════╝
"

# Functions
print_header() {
    echo -e "${BLUE}${LOGO}${NC}"
}

print_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_info() {
    echo -e "${BLUE}[i]${NC} $1"
}

# Start installation
clear
print_header

# Check for Python
echo "Checking Python installation..."
if ! command -v python3 &> /dev/null; then
    print_error "Python 3.6+ is required but not installed!"
    echo "Install with:"
    echo "  macOS: brew install python3"
    echo "  Ubuntu/Debian: sudo apt-get install python3"
    echo "  Fedora: sudo dnf install python3"
    exit 1
fi

PYTHON_VERSION=$(python3 --version 2>&1)
print_success "$PYTHON_VERSION detected"
echo ""

# Detect OS
echo "Detecting operating system..."
OS_TYPE=$(uname -s)
case "${OS_TYPE}" in
    Linux*)     OS="Linux"; ARCH=$(uname -m) ;;
    Darwin*)    OS="Mac"; ARCH="$(uname -m)" ;;
    *)          OS="UNKNOWN" ;;
esac

print_success "Detected: $OS ($ARCH)"
echo ""

# Check if already installed
if [ -d "$INSTALL_DIR" ]; then
    print_warning "Killer is already installed at $INSTALL_DIR"
    read -p "Do you want to reinstall? (y/n): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Installation cancelled."
        exit 0
    fi
    echo "Removing existing installation..."
    rm -rf "$INSTALL_DIR"
    print_success "Previous installation removed"
    echo ""
fi

# Create installation directory
echo "Creating installation directory..."
mkdir -p "$INSTALL_DIR"
print_success "Installation directory created: $INSTALL_DIR"
echo ""

# Check if source files exist
if [ ! -d "src" ]; then
    print_error "Source files not found! Run installer from killer root directory."
    exit 1
fi

# Copy source files
echo "Copying source files..."
cp -r src "$INSTALL_DIR/"
cp main.py "$INSTALL_DIR/"
cp -r examples "$INSTALL_DIR/" 2>/dev/null || true
[ -f "DOCUMENTATION.md" ] && cp "DOCUMENTATION.md" "$INSTALL_DIR/" || true
[ -f "README.md" ] && cp "README.md" "$INSTALL_DIR/" || true

print_success "Source files copied"
echo ""

# Create launcher script
echo "Creating command-line launcher..."
cat > "$INSTALL_DIR/killer-launcher.sh" << 'EOF'
#!/bin/bash
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
cd "$SCRIPT_DIR"
exec python3 main.py "$@"
EOF

chmod +x "$INSTALL_DIR/killer-launcher.sh"
print_success "Launcher created"
echo ""

# Try to create system-wide symlink
echo "Setting up system-wide access..."
if [ -w "/usr/local/bin" ]; then
    ln -sf "$INSTALL_DIR/killer-launcher.sh" "$SYSTEM_BIN"
    print_success "System-wide launcher created at $SYSTEM_BIN"
    KILLER_CMD="killer"
else
    print_warning "Cannot write to /usr/local/bin (requires sudo)"
    print_info "You can still run: $INSTALL_DIR/killer-launcher.sh"
    KILLER_CMD="$INSTALL_DIR/killer-launcher.sh"
    
    # Try to add ~/.local/bin to PATH if exists
    if [ -d "$HOME/.local/bin" ]; then
        echo ""
        echo "Adding ~/.local/bin to PATH..."
        cat > "$HOME/.local/bin/killer" << EOF
#!/bin/bash
$INSTALL_DIR/killer-launcher.sh "\$@"
EOF
        chmod +x "$HOME/.local/bin/killer"
        KILLER_CMD="killer"
    fi
fi
echo ""

# Verify installation
echo "Verifying installation..."
if "$KILLER_CMD" --version > /dev/null 2>&1 || \
   "$INSTALL_DIR/killer-launcher.sh" --version > /dev/null 2>&1; then
    print_success "Installation verified successfully!"
else
    print_warning "Installation completed but verification failed"
    print_info "Try running: $INSTALL_DIR/killer-launcher.sh --version"
fi
echo ""

# Create uninstaller
echo "Creating uninstaller..."
cat > "$INSTALL_DIR/uninstall.sh" << EOF
#!/bin/bash
echo "Removing Killer installation..."
rm -f "$SYSTEM_BIN" 2>/dev/null || true
rm -f "$HOME/.local/bin/killer" 2>/dev/null || true
rm -rf "$INSTALL_DIR"
echo "Killer has been uninstalled."
EOF

chmod +x "$INSTALL_DIR/uninstall.sh"
print_success "Uninstaller created"
echo ""

# Installation complete
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║                  INSTALLATION COMPLETE                                    ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "Installation Details:"
echo "  Location: $INSTALL_DIR"
echo "  Launcher: $KILLER_CMD"
echo "  Examples: $INSTALL_DIR/examples"
echo "  Docs: $INSTALL_DIR/DOCUMENTATION.md"
echo ""
echo "Quick Start:"
echo "  1. Open a new terminal"
echo "  2. Create a file: echo 'print(\"Hello, Killer!\");' > hello.killer"
echo "  3. Run: $KILLER_CMD hello.killer"
echo ""
echo "Documentation:"
echo "  https://localhost:8888/docs.html"
echo "  Local: $INSTALL_DIR/DOCUMENTATION.md"
echo ""
echo "To uninstall:"
echo "  bash $INSTALL_DIR/uninstall.sh"
echo ""
echo "Happy coding! 🚀"
echo ""
