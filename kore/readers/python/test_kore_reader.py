import sys
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kore_reader import KoreReader

test_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'test')
r = KoreReader(os.path.join(test_dir, 'test_v2.kore'))
print(r.info())

# Test read single column
ids = r.read_column("id")
print(f"id column: {ids}")

names = r.read_column("name")
print(f"name column: {names}")

scores = r.read_column("score")
print(f"score column: {scores}")

active = r.read_column("active")
print(f"active column: {active}")

# Test read_all_columns
cols = r.read_all_columns()
print(f"\nAll columns: {len(cols)} columns")
for name, vals in cols.items():
    print(f"  {name}: {vals}")

# Test to_dict
d = r.to_dict()
print(f"\nto_dict keys: {list(d.keys())}")

print("\n=== ALL PYTHON READER TESTS PASSED ===")
