#!/bin/bash

# Killer Programming Language - Installation Verification Script
# Tests if Killer is properly installed and working

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PASS=0
FAIL=0

echo -e "${BLUE}"
echo "╔═══════════════════════════════════════════════════════════════════════════╗"
echo "║          KILLER PROGRAMMING LANGUAGE - INSTALLATION VERIFICATION          ║"
echo "╚═══════════════════════════════════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""

# Test 1: Check Python
echo -n "Test 1: Python installation... "
if command -v python3 &> /dev/null; then
    PYTHON_VERSION=$(python3 --version 2>&1)
    echo -e "${GREEN}✓${NC} ($PYTHON_VERSION)"
    ((PASS++))
else
    echo -e "${RED}✗ Python 3 not found${NC}"
    ((FAIL++))
fi

# Test 2: Check Killer command
echo -n "Test 2: Killer command... "
if command -v killer &> /dev/null; then
    echo -e "${GREEN}✓${NC} (found in PATH)"
    KILLER_CMD="killer"
    ((PASS++))
elif [ -f "$HOME/.local/bin/killer-launcher.sh" ]; then
    echo -e "${YELLOW}⚠${NC} (found but not in PATH, using full path)"
    KILLER_CMD="$HOME/.local/bin/killer-launcher.sh"
    ((PASS++))
elif [ -f "/usr/local/bin/killer" ]; then
    echo -e "${GREEN}✓${NC} (system-wide installation)"
    KILLER_CMD="/usr/local/bin/killer"
    ((PASS++))
else
    echo -e "${RED}✗ Killer command not found${NC}"
    echo "   Try: source ~/.bashrc"
    ((FAIL++))
    KILLER_CMD="killer"  # Default for later tests
fi

# Test 3: Check version
echo -n "Test 3: Version check... "
if $KILLER_CMD --version > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC}"
    ((PASS++))
else
    echo -e "${RED}✗${NC}"
    ((FAIL++))
fi

# Test 4: Check installation directory
echo -n "Test 4: Installation directory... "
if [ -d "$HOME/.local/bin/killer" ]; then
    echo -e "${GREEN}✓${NC} ($HOME/.local/bin/killer)"
    ((PASS++))
elif [ -d "/usr/local/bin/killer" ]; then
    echo -e "${GREEN}✓${NC} (/usr/local/bin/killer)"
    ((PASS++))
else
    echo -e "${RED}✗ Installation directory not found${NC}"
    ((FAIL++))
fi

# Test 5: Check source files
echo -n "Test 5: Source files... "
if [ -d "$HOME/.local/bin/killer/src" ] || [ -d "/usr/local/bin/killer/src" ]; then
    echo -e "${GREEN}✓${NC}"
    ((PASS++))
else
    echo -e "${RED}✗ Source directory not found${NC}"
    ((FAIL++))
fi

# Test 6: Check examples
echo -n "Test 6: Example files... "
if [ -d "$HOME/.local/bin/killer/examples" ] || [ -d "/usr/local/bin/killer/examples" ]; then
    echo -e "${GREEN}✓${NC}"
    ((PASS++))
else
    echo -e "${YELLOW}⚠${NC} (not critical)"
    ((PASS++))
fi

# Test 7: Run simple program
echo -n "Test 7: Simple execution... "
TEMP_FILE=$(mktemp).killer
echo 'print("Installation verified!");' > "$TEMP_FILE"
if $KILLER_CMD "$TEMP_FILE" &> /dev/null; then
    echo -e "${GREEN}✓${NC}"
    ((PASS++))
else
    echo -e "${RED}✗${NC}"
    ((FAIL++))
fi
rm -f "$TEMP_FILE"

# Test 8: Transpilation
echo -n "Test 8: Transpilation to Python... "
TEMP_FILE=$(mktemp).killer
echo 'x = 10; print(x);' > "$TEMP_FILE"
if $KILLER_CMD --python "$TEMP_FILE" &> /dev/null; then
    echo -e "${GREEN}✓${NC}"
    ((PASS++))
else
    echo -e "${RED}✗${NC}"
    ((FAIL++))
fi
rm -f "$TEMP_FILE"

# Test 9: Transpilation to JS
echo -n "Test 9: Transpilation to JavaScript... "
TEMP_FILE=$(mktemp).killer
echo 'x = 10; print(x);' > "$TEMP_FILE"
if $KILLER_CMD --js "$TEMP_FILE" &> /dev/null; then
    echo -e "${GREEN}✓${NC}"
    ((PASS++))
else
    echo -e "${RED}✗${NC}"
    ((FAIL++))
fi
rm -f "$TEMP_FILE"

# Summary
echo ""
echo "╔═══════════════════════════════════════════════════════════════════════════╗"
echo "║                           TEST RESULTS                                    ║"
echo "╚═══════════════════════════════════════════════════════════════════════════╝"
echo ""
echo -e "Passed: ${GREEN}$PASS${NC}/9"
echo -e "Failed: ${RED}$FAIL${NC}/9"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed! Installation is working correctly.${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Create a file: echo 'print(\"Hello, Killer!\");' > hello.killer"
    echo "  2. Run it: $KILLER_CMD hello.killer"
    echo "  3. Read docs: $KILLER_CMD DOCUMENTATION.md"
    echo ""
    exit 0
else
    echo -e "${RED}✗ Some tests failed. Check the output above for details.${NC}"
    echo ""
    echo "Troubleshooting:"
    echo "  • Make sure Python 3 is installed and in PATH"
    echo "  • Installer may need to be run as administrator"
    echo "  • Try opening a NEW terminal after installation"
    echo "  • Run installation again: ./killer-installer.sh"
    echo ""
    exit 1
fi
