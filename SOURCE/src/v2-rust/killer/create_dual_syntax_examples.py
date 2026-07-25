#!/usr/bin/env python3
"""
KILLER: THE FIRST LANGUAGE THAT SUPPORTS BOTH SYNTAXES
Supporting both Python-style indentation and Go-style braces
"""

examples = {
    "simple_add.killer": """# Python-style (indentation-based)
add(a, b)
    a + b

# Java-style (brace-based)
multiply(x, y) {
    x * y
}

print(add(3, 5))
print(multiply(4, 7))
""",

    "loops_both.killer": """# Brace-style loop
for (i in range(5)) {
    print(i)
}

# Indentation-style loop
for (item in [10, 20, 30])
    print(item)
""",

    "conditions_mixed.killer": """# Indentation-style if
if len([1, 2, 3]) > 2
    print("Array has items")

# Brace-style if
if true {
    print("Both styles work!")
}
""",

    "complex_mixed.killer": """# Mix styles freely - whatever feels right!
fn process(data)
    total = 0
    if len(data) > 0 {
        for item in data
            total = total + item
    }
    total

result = process([5, 10, 15])
print("Sum: ")
print(result)
""",

    "arrow_functions.killer": """# Arrow syntax (super clean)
add(a, b) => a + b
square(x) => x * x

print(add(3, 7))
print(square(5))
""",

    "real_world_example.killer": """# Real-world: data processing mix

fn fetch_data()
    {"name": "Alice", "score": 95, "tag": "top"}

fn process_scores(records)
    scores = []
    for record in records {
        if record["score"] > 80
            scores.push(record)
    }
    scores

fn format_result(data) => "Processed: " + str(len(data))

# Main logic
data = fetch_data()
processed = process_scores([data])
print(format_result(processed))
"""
}

import os
os.chdir(r'c:\Users\skathera\Downloads\killer\native\killer_vm')

print("=" * 70)
print("KILLER: Unique Language - Supports BOTH Syntaxes")
print("=" * 70)

for filename, content in examples.items():
    path = f'examples/{filename}'
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"✓ Created {filename}")

print("\n" + "=" * 70)
print("UNIQUE VALUE PROPOSITION:")
print("=" * 70)
print("""
1. PYTHON STYLE - Indentation-based (clean, minimal)
   if x > 5
       print x

2. GO/JAVA STYLE - Brace-based (explicit, familiar)
   if x > 5 {
       print x
   }

3. ARROW SYNTAX - Ultra-clean one-liners
   add(a, b) => a + b

4. MIX & MATCH - Use what feels right for each piece!

This makes Killer the EASIEST to learn AND most flexible language.

Perfect for:
- Teaching beginners (use Python-style)
- Enterprise teams (use brace-style)
- Quick scripts (use arrow syntax)
- Real projects (mix all three!)
""")

print("=" * 70)
print("Example files created in examples/ directory")
print("= " * 70)
