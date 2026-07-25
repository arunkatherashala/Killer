#!/usr/bin/env python3
import os
import subprocess
import sys

os.chdir(r'c:\Users\skathera\Downloads\killer\native\killer_vm')

# Recreate example files with proper encoding
files = {
    'examples/stdlib_len.killer': '''len_arr = len([1, 2, 3, 4, 5])
print(len_arr)

len_dict = len({"a": 1, "b": 2, "c": 3})
print(len_dict)

len_str = len("hello")
print(len_str)
''',
    'examples/stdlib_range.killer': '''r1 = range(5)
print(r1)

r2 = range(2, 8)
print(r2)

r3 = range(0, 10, 2)
print(r3)
''',
    'examples/stdlib_type_conv.killer': '''t1 = type(42)
print(t1)

t2 = type("hello")
print(t2)

t3 = type(true)
print(t3)

t4 = type([1, 2, 3])
print(t4)

t5 = type({"x": 1})
print(t5)

s1 = str(123)
print(s1)

s2 = str(3.14)
print(s2)

i1 = int("456")
print(i1)

i2 = int(3.99)
print(i2)
''',
    'examples/stdlib_dict_ops.killer': '''dict = {"name": "Alice", "age": "30", "city": "NYC"}

dict_keys = keys(dict)
print(dict_keys)

dict_values = values(dict)
print(dict_values)
''',
    'examples/stdlib_integration.killer': '''numbers = range(1, 6)
print(numbers)

dict = {"red": 255, "green": 128, "blue": 64}
print(len(dict))

dict_keys = keys(dict)
print(dict_keys)

arr = [1, "2", 3.5]
print(type(arr))

first = arr[0]
print(type(first))

str_v = str(42)
print(str_v)

int_v = int("99")
print(int_v)
''',
    'examples/test_implicit_return.killer': '''fn add(a, b) {
    a + b
}

fn multiply(x, y) {
    x * y
}

result = add(5, 3)
print(result)

product = multiply(2, 4)
print(product)
'''
}

print("=" * 60)
print("Creating example files with UTF-8 encoding...")
print("=" * 60)

for path, content in files.items():
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"✓ {path}")

print("\n" + "=" * 60)
print("Testing all examples...")
print("=" * 60)

test_files = [
    ('examples/stdlib_len.killer', ['5', '3', '5']),
    ('examples/stdlib_range.killer', ['[]', '[2, 3, 4, 5, 6, 7]', '[0, 2, 4, 6, 8]']),
    ('examples/stdlib_type_conv.killer', ['number', 'string', 'bool', 'array', 'dict', '123', '3.14', '456', '3']),
    ('examples/stdlib_dict_ops.killer', None),  # Just verify no errors
    ('examples/stdlib_integration.killer', None),  # Just verify no errors
    ('examples/test_implicit_return.killer', ['8', '8']),
]

for file_path, expected_outputs in test_files:
    print(f"\nTesting: {file_path}")
    try:
        result = subprocess.run(
            ['./target/release/killer-native.exe', '--killer', file_path],
            capture_output=True,
            text=True,
            timeout=5
        )
        
        output_lines = result.stdout.strip().split('\n') if result.stdout.strip() else []
        
        if result.returncode == 0:
            print(f"  ✓ Executed successfully")
            if expected_outputs:
                print(f"    Output: {output_lines}")
                matches = all(exp in output_lines for exp in expected_outputs)
                if matches:
                    print(f"  ✓ Output matches expectations")
                else:
                    print(f"  ⚠ Output mismatch. Expected: {expected_outputs}")
            else:
                print(f"    Output: {output_lines}")
        else:
            print(f"  ✗ Error: {result.stderr}")
    except Exception as e:
        print(f"  ✗ Exception: {e}")

print("\n" + "=" * 60)
print("Test suite complete!")
print("=" * 60)
