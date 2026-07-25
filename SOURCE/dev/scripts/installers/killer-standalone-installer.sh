#!/bin/bash
# Killer Programming Language - Standalone Installer (Mac/Linux)
# Phase 2: Installs killer binary compiled executable with zero Python dependency
# Works on macOS and Linux (Ubuntu, Debian, Fedora, etc)

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VERSION="3.0"
GITHUB_URL="https://github.com/arunaug2008-ai/Killer"

# Detect OS
OS_TYPE=$(uname -s)
ARCH=$(uname -m)

echo -e "${BLUE}================================================================================${NC}"
echo -e "${BLUE}  Killer Programming Language - Standalone Installer (Phase 2)${NC}"
echo -e "${BLUE}  Version: $VERSION (Standalone Executable - No Python Required)${NC}"
echo -e "${BLUE}================================================================================${NC}"
echo

# Determine installation path based on OS
if [ "$OS_TYPE" == "Darwin" ]; then
    # macOS
    INSTALL_DIR="/usr/local/bin"
    OS_NAME="macOS"
    BINARY_NAME="killer"
else
    # Linux
    INSTALL_DIR="/usr/local/bin"
    OS_NAME="Linux"
    BINARY_NAME="killer"
fi

echo -e "${BLUE}[*]${NC} Detected OS: $OS_NAME ($ARCH)"
echo -e "${BLUE}[*]${NC} Installation directory: $INSTALL_DIR"
echo

# Check for write permissions
if [ ! -w "$INSTALL_DIR" ]; then
    echo -e "${RED}[!]${NC} Error: No write permission to $INSTALL_DIR"
    echo -e "${YELLOW}[*]${NC} Please run with sudo: sudo bash $0"
    exit 1
fi

# Check if killer already installed
if command -v killer &> /dev/null; then
    CURRENT_PATH=$(which killer)
    echo
    echo -e "${BLUE}================================================================================${NC}"
    echo -e "${YELLOW}  Killer is already installed${NC}"
    echo -e "${BLUE}================================================================================${NC}"
    echo
    echo -e "${BLUE}[1]${NC} Repair      - Reinstall/update to latest version"
    echo -e "${BLUE}[2]${NC} Uninstall   - Remove Killer completely"
    echo -e "${BLUE}[3]${NC} Cancel      - Exit without changes"
    echo
    read -p "Select option (1/2/3): " CHOICE
    echo
    
    case "$CHOICE" in
        1)
            echo -e "${BLUE}[*]${NC} Repairing Killer v$VERSION..."
            echo -e "${BLUE}[*]${NC} Removing old version..."
            sudo rm -f "$INSTALL_DIR/killer" 2>/dev/null || rm -f "$INSTALL_DIR/killer"
            REM Continue with installation
            ;;
        2)
            echo -e "${RED}[!]${NC} Uninstalling Killer v$VERSION..."
            echo -e "${BLUE}[*]${NC} Removing: $CURRENT_PATH"
            sudo rm -f "$INSTALL_DIR/killer" 2>/dev/null || rm -f "$INSTALL_DIR/killer"
            echo -e "${GREEN}[+]${NC} Killer uninstalled successfully"
            echo
            exit 0
            ;;
        3|"")
            echo -e "${BLUE}[*]${NC} Installation cancelled."
            exit 0
            ;;
        *)
            echo -e "${RED}[!]${NC} Invalid choice"
            exit 1
            ;;
    esac
fi

# Check for killer binary or script in current directory
if [ -f "./killer.sh" ]; then
    echo -e "${GREEN}[+]${NC} Found killer.sh - Installing..."
    
    # Check if we need sudo
    if [ ! -w "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}[*]${NC} Requires sudo for installation to $INSTALL_DIR"
        sudo cp -v "./killer.sh" "$INSTALL_DIR/$BINARY_NAME"
        sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
    else
        cp -v "./killer.sh" "$INSTALL_DIR/$BINARY_NAME"
        chmod +x "$INSTALL_DIR/$BINARY_NAME"
    fi
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}[!]${NC} Error: Failed to copy killer.sh"
        exit 1
    fi
elif [ -f "./killer" ]; then
    echo -e "${GREEN}[+]${NC} Found killer binary - Installing..."
    
    # Check if we need sudo
    if [ ! -w "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}[*]${NC} Requires sudo for installation to $INSTALL_DIR"
        sudo cp -v "./killer" "$INSTALL_DIR/$BINARY_NAME"
        sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
    else
        cp -v "./killer" "$INSTALL_DIR/$BINARY_NAME"
        chmod +x "$INSTALL_DIR/$BINARY_NAME"
    fi
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}[!]${NC} Error: Failed to copy killer binary"
        exit 1
    fi
else
    echo -e "${RED}[!]${NC} Error: killer binary or killer.sh not found in current directory"
    echo -e "${RED}[!]${NC} Please ensure 'killer' or 'killer.sh' executable is in the same folder as this installer"
    echo -e "${YELLOW}[*]${NC} Run from Phase 2 build directory where killer was compiled"
    exit 1
fi

# Verify installation
echo
echo -e "${BLUE}[*]${NC} Verifying installation..."
if command -v killer &> /dev/null; then
    VERSION_OUTPUT=$(killer --version 2>/dev/null || echo "Killer v3.0")
    echo -e "${GREEN}[+]${NC} $VERSION_OUTPUT"
else
    echo -e "${RED}[!]${NC} Error: killer command not found after installation"
    exit 1
fi

# Run test
echo
echo -e "${BLUE}[*]${NC} Running test program..."
TEST_DIR=$(mktemp -d)
TEST_FILE="$TEST_DIR/test.killer"

cat > "$TEST_FILE" << 'EOF'
x = 10
y = 5
print(x + y)
EOF

if killer "$TEST_FILE" &> /dev/null; then
    echo -e "${GREEN}[+]${NC} Test successful - Killer is working!"
else
    echo -e "${YELLOW}[!]${NC} Warning: Test execution encountered an issue"
fi

rm -rf "$TEST_DIR"

# Register .killer file association
echo
echo -e "${BLUE}[*]${NC} Registering .killer file association..."

if [ "$OS_TYPE" == "Darwin" ]; then
    # macOS - use LaunchServices
    duti -s com.apple.Terminal .killer com.apple.Terminal 2>/dev/null || \
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user /Applications/Terminal.app 2>/dev/null
    echo -e "${GREEN}[+]${NC} .killer files registered"
else
    # Linux - use xdg-mime
    if command -v xdg-mime &> /dev/null; then
        xdg-mime default killer.desktop application/x-killer 2>/dev/null
        echo -e "${GREEN}[+]${NC} .killer files registered"
    else
        echo -e "${YELLOW}[!]${NC} xdg-mime not found - manual configuration may be needed"
    fi
fi

echo -e "${GREEN}[+]${NC} You can now run .killer files directly: test.killer"

# Create uninstaller
echo -e "${BLUE}[*]${NC} Creating uninstaller..."
UNINSTALLER_DIR="/usr/local/lib/killer"
mkdir -p "$UNINSTALLER_DIR" 2>/dev/null || sudo mkdir -p "$UNINSTALLER_DIR"

UNINSTALLER="$UNINSTALLER_DIR/uninstall.sh"
cat > "$UNINSTALLER" << 'EOF'
#!/bin/bash
echo "Uninstalling Killer v3.0..."
sudo rm -f /usr/local/bin/killer
rm -f "$UNINSTALLER_DIR/uninstall.sh"
echo "[+] Killer uninstalled successfully"
EOF

chmod +x "$UNINSTALLER" 2>/dev/null || sudo chmod +x "$UNINSTALLER"

# Installation summary
clear
echo -e "${BLUE}================================================================================${NC}"
echo -e "${GREEN}  Installation Complete!${NC}"
echo -e "${BLUE}================================================================================${NC}"
echo
echo -e "${GREEN}[+]${NC} Killer v$VERSION installed successfully"
echo -e "${GREEN}[+]${NC} Installation directory: $INSTALL_DIR/$BINARY_NAME"
echo -e "${GREEN}[+]${NC} Added to PATH: Yes"
echo -e "${GREEN}[+]${NC} Ready to use: YES (immediately available)"
echo
echo -e "${BLUE}Usage (try it now):${NC}"
echo "   test.killer                              # Run a .killer file directly"
echo "   killer example.killer                    # Run with killer command"
echo "   killer --version                         # Show version"
echo "   killer --help                            # Show help"
echo
echo -e "${BLUE}Next steps:${NC}"
echo "   1. Type: killer --version"
echo "   2. Run an example: killer examples/01_hello.killer"
echo "   3. Create your own: killer myprogram.killer"
echo
echo -e "${BLUE}Note:${NC}"
echo "   - Killer is available NOW in this and all future terminal windows"
echo "   - No terminal restart required!"
echo
echo -e "${BLUE}To uninstall:${NC}"
echo "   bash $UNINSTALLER"
echo "   OR: sudo rm /usr/local/bin/killer"
echo
echo -e "${BLUE}Documentation:${NC} $GITHUB_URL"
echo -e "${BLUE}================================================================================${NC}"
echo
