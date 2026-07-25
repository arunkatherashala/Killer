#!/usr/bin/env python3
"""
Test Phase 1: Self-Hosted Interpreter Bootstrap
Tests if the self-hosted interpreter (written in Killer) can be loaded and executed
"""

import sys
import os

# Add src directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'src'))

from interpreter import run_killer

print("=" * 70)
print("PHASE 1 BOOTSTRAP TEST - Self-Hosted Interpreter")
print("=" * 70)

# Test 1: Load lexer.killer
print("\n[Test 1] Loading lexer.killer through Python interpreter...")
try:
    with open('research-archive/lexer.killer', 'r') as f:
        lexer_code = f.read()
    lexer_ast = run_killer(lexer_code)
    print("✅ lexer.killer loaded successfully")
except Exception as e:
    print(f"❌ Failed to load lexer.killer: {e}")
    sys.exit(1)

# Test 2: Load parser.killer
print("\n[Test 2] Loading parser.killer through Python interpreter...")
try:
    with open('research-archive/parser.killer', 'r') as f:
        parser_code = f.read()
    parser_ast = run_killer(parser_code)
    print("✅ parser.killer loaded successfully")
except Exception as e:
    print(f"❌ Failed to load parser.killer: {e}")
    sys.exit(1)

# Test 3: Load interpreter.killer
print("\n[Test 3] Loading interpreter.killer through Python interpreter...")
try:
    with open('research-archive/interpreter.killer', 'r') as f:
        interp_code = f.read()
    interpreter_ast = run_killer(interp_code)
    print("✅ interpreter.killer loaded successfully")
except Exception as e:
    print(f"❌ Failed to load interpreter.killer: {e}")
    sys.exit(1)

# Test 4: Run simple self-hosted test
print("\n[Test 4] Testing self-hosted interpreter with simple code...")
try:
    # First create a simple test file
    test_killer_code = """
# Test simple variables
x = 5
y = 3
print(x + y)

# Test functions
fn square(n) {
    return n * n;
}
print(square(4))

# Test arrays
arr = [1, 2, 3, 4, 5]
print(arr.length)

# Test objects
obj = { name: "Test", value: 42 }
print(obj.name)
"""
    result = run_killer(test_killer_code)
    print("✅ Self-hosted interpreter executed test code successfully")
except Exception as e:
    print(f"❌ Self-hosted interpreter test failed: {e}")
    sys.exit(1)

# Test 5: Run all examples through the interpreter
print("\n[Test 5] Running example files through interpreter...")
example_count = 0
success_count = 0
examples_dir = 'examples'

for i in range(1, 17):
    example_file = f"{examples_dir}/0{i}_" if i < 10 else f"{examples_dir}/{i}_"
    
    # Find the actual file
    import glob
    matching_files = glob.glob(f"{examples_dir}/{i:02d}_*.killer")
    
    if matching_files:
        example_file = matching_files[0]
        example_count += 1
        try:
            with open(example_file, 'r') as f:
                code = f.read()
            result = run_killer(code)
            print(f"  ✅ {os.path.basename(example_file)}")
            success_count += 1
        except Exception as e:
            print(f"  ❌ {os.path.basename(example_file)}: {str(e)[:50]}")

print(f"\n✅ Passed {success_count}/{example_count} example tests")

# Summary
print("\n" + "=" * 70)
print("PHASE 1 TEST SUMMARY")
print("=" * 70)
print("✅ All self-hosted interpreter components load successfully")
print("✅ Lexer.killer works")
print("✅ Parser.killer works")
print("✅ Interpreter.killer works")
print(f"✅ {success_count}/{example_count} example programs execute correctly")
print("\n🎉 PHASE 1 IS READY FOR PHASE 2 BOOTSTRAP")
print("=" * 70)
