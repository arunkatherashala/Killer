#!/usr/bin/env python3
"""
Pigeonhole Formula Generator for Phase 2 Direction 1 Experiments

Generates unsatisfiable CNF formulas encoding the pigeonhole principle:
"n+1 pigeons cannot fit into n holes (one per hole)"

This creates formulas empirically verified to be hard for SAT solvers,
consistent with Haken's 2^Ω(n) lower bound on resolution proofs.
"""

import os
from pathlib import Path
from typing import List, Tuple
import json


def generate_pigeonhole_cnf(n: int) -> Tuple[str, dict]:
    """
    Generate Pigeonhole formula PHP_n in DIMACS CNF format.
    
    Args:
        n: Number of holes (n+1 pigeons)
    
    Returns:
        Tuple of (DIMACS_string, metadata_dict)
    
    Formula Structure:
      Variables: x_{i,j} for i in [1, n+1], j in [1, n]
                 x_{i,j} = true means "pigeon i is in hole j"
      
      Clauses:
      1. Covering: Each pigeon in at least one hole
         For each i in [1, n+1]: (x_{i,1} OR x_{i,2} OR ... OR x_{i,n})
         Total: n+1 clauses
      
      2. Uniqueness: Each hole has at most one pigeon
         For each j in [1, n] and i1 < i2 in [1, n+1]:
           (NOT x_{i1,j} OR NOT x_{i2,j})
         Total: n * C(n+1, 2) = n * (n+1)*n/2 clauses
    
    Total clauses: (n+1) + n*(n+1)*n/2 ≈ n³/2 + O(n²)
    Total variables: (n+1) * n ≈ n² + n
    
    Unsatisfiable: YES (pigeonhole principle)
    Haken lower bound: Any resolution proof requires ≥ 2^(c*n) clauses
    """
    
    num_pigeons = n + 1
    num_holes = n
    num_vars = num_pigeons * num_holes
    
    clauses = []
    
    # 1. Covering clauses: each pigeon in at least one hole
    for pigeon in range(1, num_pigeons + 1):
        clause = []
        for hole in range(1, num_holes + 1):
            var_num = (pigeon - 1) * num_holes + hole
            clause.append(var_num)
        clauses.append(clause)
    
    # 2. Uniqueness clauses: each hole has at most one pigeon
    for hole in range(1, num_holes + 1):
        for pigeon1 in range(1, num_pigeons + 1):
            for pigeon2 in range(pigeon1 + 1, num_pigeons + 1):
                var1 = (pigeon1 - 1) * num_holes + hole
                var2 = (pigeon2 - 1) * num_holes + hole
                # Clause: (NOT var1 OR NOT var2)
                clauses.append([-var1, -var2])
    
    # Build DIMACS format
    num_clauses = len(clauses)
    dimacs_lines = [
        f"c Pigeonhole formula PHP_{n}",
        f"c {num_pigeons} pigeons, {num_holes} holes",
        f"c Unsatisfiable: YES (pigeonhole principle)",
        f"c Variables: {num_vars}",
        f"c Clauses: {num_clauses}",
        f"p cnf {num_vars} {num_clauses}"
    ]
    
    for clause in clauses:
        dimacs_lines.append(" ".join(map(str, clause)) + " 0")
    
    dimacs_str = "\n".join(dimacs_lines) + "\n"
    
    metadata = {
        "formula_type": "pigeonhole",
        "n": n,
        "num_pigeons": num_pigeons,
        "num_holes": num_holes,
        "num_variables": num_vars,
        "num_clauses": num_clauses,
        "satisfiable": False,
        "haken_lower_bound": f"2^Ω({n})",
        "covering_clauses": n + 1,
        "uniqueness_clauses": num_clauses - (n + 1)
    }
    
    return dimacs_str, metadata


def generate_pigeonhole_suite(n_values: List[int], output_dir: str = ".") -> dict:
    """
    Generate a suite of Pigeonhole formulas.
    
    Args:
        n_values: List of n values (number of holes)
        output_dir: Output directory for CNF files
    
    Returns:
        Dictionary mapping n -> (filepath, metadata)
    """
    
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)
    
    results = {}
    metadata_list = []
    
    for n in sorted(n_values):
        print(f"Generating PHP_{n}...", end=" ")
        
        dimacs_str, metadata = generate_pigeonhole_cnf(n)
        
        # Save CNF file
        cnf_file = output_path / f"php_{n}.cnf"
        with open(cnf_file, 'w') as f:
            f.write(dimacs_str)
        
        results[n] = {
            "filepath": str(cnf_file),
            "metadata": metadata
        }
        
        metadata_list.append(metadata)
        print(f"OK → {cnf_file.name}")
    
    # Save metadata summary
    metadata_file = output_path / "pigeonhole_suite_metadata.json"
    with open(metadata_file, 'w') as f:
        json.dump(metadata_list, f, indent=2)
    
    print(f"\n✓ Generated {len(n_values)} Pigeonhole formulas")
    print(f"  Metadata saved: {metadata_file}")
    
    return results


def generate_for_experiments(output_dir: str = "EXPERIMENTS/DIRECTION_1_DATA"):
    """
    Generate complete Pigeonhole formula suite for Phase 2 Direction 1 experiments.
    
    Generates PHP_n for n = 5 to 40 (standard experimental range)
    """
    
    n_values = [5, 10, 15, 20, 25, 30, 35, 40]
    
    print("=== Phase 2 Direction 1: Pigeonhole Formula Generation ===\n")
    print(f"Generating Pigeonhole formulas for n = {n_values}")
    print(f"Output directory: {output_dir}\n")
    
    results = generate_pigeonhole_suite(n_values, output_dir)
    
    # Print summary
    print("\n=== Generated Formulas Summary ===\n")
    for n in sorted(results.keys()):
        meta = results[n]["metadata"]
        print(f"PHP_{n}:")
        print(f"  File: {results[n]['filepath']}")
        print(f"  Variables: {meta['num_variables']}")
        print(f"  Clauses: {meta['num_clauses']}")
        print(f"  Lower bound: {meta['haken_lower_bound']}")
    
    print(f"\n✓ All formulas generated successfully!")
    print(f"Ready for solver experiments (Phase 2 Direction 1)\n")
    
    return results


if __name__ == "__main__":
    # Generate formula suite for experiments
    results = generate_for_experiments()
    
    # Print quick reference
    print("\n=== Quick Reference ===")
    print("To test with a solver, use the generated .cnf files:")
    print("  Example: solver EXPERIMENTS/DIRECTION_1_DATA/php_20.cnf")
    print("\nTo run experiments, see: PHASE_2_DIRECTION_1_SOLVER_FRAMEWORK.py")
