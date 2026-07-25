#!/usr/bin/env python3
import subprocess
import os
import sys

os.chdir(r'c:\Users\skathera\Downloads\killer\native\killer_vm')

print("=" * 80)
print("KILLER DUAL-SYNTAX BUILD & TEST")
print("=" * 80)

# Step 1: Build
print("\n[1/3] Building Killer with dual-syntax support...")
result = subprocess.run(
    [r'C:\Users\skathera\.cargo\bin\cargo.exe', 'build', '--release'],
    capture_output=True,
    text=True,
    timeout=300
)

if result.returncode != 0:
    print("❌ Build FAILED")
    print("STDERR:", result.stderr[-500:])
    sys.exit(1)

print("✅ Build successful!")

# Step 2: Create examples
print("\n[2/3] Creating dual-syntax examples...")

examples = {
    'brace_style.killer': '''len_arr = len([1, 2, 3, 4, 5])
print(len_arr)

len_dict = len({"a": 1, "b": 2, "c": 3})
print(len_dict)

len_str = len("hello")
print(len_str)
''',
    
    'arrow_functions.killer': '''add(a, b) => a + b
multiply(x, y) => x * y

print(add(3, 5))
print(multiply(4, 7))
''',

    'mixed_style.killer': '''fn process(arr) {
    total = 0
    for (item in arr) {
        total = total + item
    }
    total
}

nums = [2, 4, 6, 8]
result = process(nums)
print(result)
'''
}

for filename, content in examples.items():
    path = f'examples/{filename}'
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"  ✓ {filename}")

# Step 3: Test
print("\n[3/3] Testing examples...")

test_files = [
    ('examples/brace_style.killer', ['5', '3', '5']),
    ('examples/arrow_functions.killer', ['8', '28']),
    ('examples/mixed_style.killer', ['20']),
]

all_passed = True
for test_file, expected_outputs in test_files:
    print(f"\n  Testing: {test_file}")
    try:
        result = subprocess.run(
            ['./target/release/killer-native.exe', '--killer', test_file],
            capture_output=True,
            text=True,
            timeout=5,
            cwd=r'c:\Users\skathera\Downloads\killer\native\killer_vm'
        )
        
        if result.returncode == 0:
            output_lines = [line.strip() for line in result.stdout.strip().split('\n') if line.strip()]
            print(f"    Output: {output_lines}")
            
            # Check expected outputs
            matches = all(any(exp in line for line in output_lines) for exp in expected_outputs)
            if matches:
                print(f"    ✅ PASS")
            else:
                print(f"    ⚠️  Output mismatch. Expected: {expected_outputs}")
                all_passed = False
        else:
            print(f"    ❌ FAIL - Runtime error")
            print(f"    Error: {result.stderr[:200]}")
            all_passed = False
    except Exception as e:
        print(f"    ❌ FAIL - {e}")
        all_passed = False

# Summary
print("\n" + "=" * 80)
if all_passed:
    print("✅ ALL TESTS PASSED - Dual-syntax works!")
else:
    print("⚠️ Some tests had issues - check output above")
print("=" * 80)
