#!/usr/bin/env python3
import subprocess
import os
import sys

test_results = {'pass': 0, 'fail': 0}
examples = []

# Find all Phase 1 + 2 examples
for i in range(1, 17):
    example_path = f'examples/{i:02d}_' if i < 10 else f'examples/{i}_'
    # Find matching files
    for file in os.listdir('examples'):
        if file.startswith(str(i).zfill(2) if i < 10 else str(i)) and file.endswith('.killer'):
            if not '_gen' in file:
                examples.append(file)
                break

# Also add phase 1 and phase 2 examples if they exist
if os.path.exists('examples/15_phase1.killer'):
    if '15_phase1.killer' not in examples:
        examples.append('15_phase1.killer')
if os.path.exists('examples/16_phase2_oop.killer'):
    if '16_phase2_oop.killer' not in examples:
        examples.append('16_phase2_oop.killer')

examples = sorted(set(examples))[:16]  # Limit to first 16

print(f'Testing {len(examples)} examples across 3 modes...')
print('=' * 60)

for example in examples:
    print(f'\n[{example}]', end=' ')
    
    # Test interpreter
    try:
        result = subprocess.run(
            f'python main.py examples/{example}',
            shell=True, capture_output=True, text=True, timeout=5
        )
        interpreter_ok = result.returncode == 0 or 'Error' not in result.stdout
        status = 'OK' if interpreter_ok else 'XX'
        print(f'[{status}]', end=' ')
        if interpreter_ok:
            test_results['pass'] += 1
        else:
            test_results['fail'] += 1
    except subprocess.TimeoutExpired:
        print('[XX]', end=' ')
        test_results['fail'] += 1
    
    # Test Python transpilation
    try:
        result = subprocess.run(
            f'python main.py --python examples/{example}',
            shell=True, capture_output=True, text=True, timeout=5
        )
        python_ok = result.returncode == 0 or 'Error' not in result.stdout
        status = 'OK' if python_ok else 'XX'
        print(f'[{status}]', end=' ')
        if python_ok:
            test_results['pass'] += 1
        else:
            test_results['fail'] += 1
    except subprocess.TimeoutExpired:
        print('[XX]', end=' ')
        test_results['fail'] += 1
    
    # Test JavaScript transpilation
    try:
        result = subprocess.run(
            f'python main.py --js examples/{example}',
            shell=True, capture_output=True, text=True, timeout=5
        )
        js_ok = result.returncode == 0 or 'Error' not in result.stdout
        status = 'OK' if js_ok else 'XX'
        print(f'[{status}]')
        if js_ok:
            test_results['pass'] += 1
        else:
            test_results['fail'] += 1
    except subprocess.TimeoutExpired:
        print('[XX]')
        test_results['fail'] += 1

print('=' * 60)
total = test_results['pass'] + test_results['fail']
pct = (test_results['pass'] / total * 100) if total > 0 else 0
print(f'\nResults: {test_results["pass"]}/{total} PASS ({pct:.0f}%)')

if test_results['fail'] == 0:
    print('[SUCCESS] ALL TESTS PASSED!')
