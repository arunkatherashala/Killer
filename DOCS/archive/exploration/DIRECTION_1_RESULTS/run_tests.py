#!/usr/bin/env python3
"""
Stream A: Empirical Validation of Pigeonhole Formula Hardness
Tests: PHP_n formulas for n = 5, 10, 15, 20, 25, 30
Expected: Exponential scaling in SAT solver runtime
"""

import os
import sys
import time
import csv
from datetime import datetime
from pathlib import Path

try:
    from pysat.solvers import Glucose3
except ImportError:
    print("ERROR: python-sat not installed. Run: pip install python-sat")
    sys.exit(1)


def parse_cnf(filename):
    """Parse DIMACS CNF format and return (num_vars, num_clauses, clauses)"""
    clauses = []
    num_vars = 0
    num_clauses = 0
    
    with open(filename, 'r') as f:
        for line in f:
            line = line.strip()
            
            # Skip comments
            if line.startswith('c'):
                continue
            
            # Parse header
            if line.startswith('p cnf'):
                parts = line.split()
                num_vars = int(parts[2])
                num_clauses = int(parts[3])
                continue
            
            # Parse clause
            if line and not line.startswith('c') and not line.startswith('p'):
                clause = [int(x) for x in line.split() if x != '0']
                if clause:
                    clauses.append(clause)
    
    return num_vars, num_clauses, clauses


def test_formula(formula_file, timeout_sec=300):
    """
    Test a single formula file.
    Returns: (satisfiable, runtime_sec, status)
    """
    try:
        num_vars, expected_clauses, clauses = parse_cnf(formula_file)
        
        # Create solver
        solver = Glucose3()
        
        # Add clauses
        for clause in clauses:
            solver.add_clause(clause)
        
        # Time the solve
        start = time.time()
        is_sat = solver.solve()
        elapsed = time.time() - start
        
        solver.delete()
        
        return is_sat, elapsed, f"Solved ({elapsed:.3f}s)"
    
    except Exception as e:
        return None, None, f"Error: {str(e)}"


def main():
    base_dir = Path(__file__).parent
    
    # Find all formulas
    formulas = sorted([
        f for f in base_dir.glob("php_*_example.cnf")
    ])
    
    if not formulas:
        print("ERROR: No php_*_example.cnf files found")
        sys.exit(1)
    
    print(f"\n{'='*70}")
    print(f"STREAM A: SAT EMPIRICAL VALIDATION")
    print(f"{'='*70}")
    print(f"Test Date: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"Solver: Glucose3 (glucose-like SAT solver)")
    print(f"Found {len(formulas)} test formulas\n")
    
    # Open CSV for logging
    csv_file = base_dir / "DIRECTION_1_RESULTS.csv"
    results = []
    
    for formula_path in formulas:
        formula_name = formula_path.name
        
        # Extract n from php_N_example.cnf
        n_str = formula_name.split('_')[1]
        n = int(n_str)
        
        # Parse formula metadata
        num_vars, num_clauses, clauses = parse_cnf(str(formula_path))
        file_size_kb = formula_path.stat().st_size / 1024
        file_lines = len(open(formula_path).readlines())
        
        print(f"[{formula_name}]")
        print(f"  n={n}, pigeons={n+1}, holes={n}")
        print(f"  Variables: {num_vars}, Clauses: {num_clauses}")
        print(f"  File size: {file_size_kb:.2f} KB, Lines: {file_lines}")
        
        # Run test
        print(f"  Testing... ", end='', flush=True)
        is_sat, runtime, status = test_formula(str(formula_path))
        
        print(status)
        print(f"  Result: {'SATISFIABLE' if is_sat else 'UNSATISFIABLE'}")
        print(f"  Runtime: {runtime:.4f}s\n" if runtime else "")
        
        # Log result
        results.append({
            'n': n,
            'pigeons': n + 1,
            'holes': n,
            'variables': num_vars,
            'clauses': num_clauses,
            'file_size_kb': f"{file_size_kb:.2f}",
            'lines': file_lines,
            'satisfiable': 'NO' if is_sat == False else ('YES' if is_sat else 'ERROR'),
            'runtime_sec': f"{runtime:.4f}" if runtime else "TIMEOUT",
            'date_created': datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        })
    
    # Write results to CSV
    if results:
        fieldnames = ['n', 'pigeons', 'holes', 'variables', 'clauses', 
                      'file_size_kb', 'lines', 'satisfiable', 'runtime_sec', 'date_created']
        
        with open(csv_file, 'w', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(results)
        
        print(f"{'='*70}")
        print(f"Results saved to: {csv_file}")
        print(f"{'='*70}\n")
        
        # Display summary
        print("SUMMARY - Exponential Hardness Analysis:")
        print("-" * 50)
        for result in results:
            print(f"n={result['n']:2d}: {result['clauses']:6s} clauses, {result['runtime_sec']:>10s}s")
        
        print("\nEXPECTED: Runtime scales exponentially with n")
        print("Proof model: 2^Ω(n) resolution complexity\n")


if __name__ == "__main__":
    main()
