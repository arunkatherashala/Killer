#!/usr/bin/env python3
import os

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
'''
}

for path, content in files.items():
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"Created {path}")
