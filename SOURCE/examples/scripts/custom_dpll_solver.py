#!/usr/bin/env python3
"""
Custom Pure DPLL SAT Solver - For Phase 2 Direction 1 Baseline

Simple DPLL implementation without clause learning (unlike CDCL).
Used for comparison: pure DPLL vs modern CDCL solvers on hard instances.

Output format: JSON with metrics (decisions, backtracks, runtime, etc.)
"""

import json
import sys
import time
from typing import List, Dict, Set, Tuple, Optional
from pathlib import Path


class CNFFormula:
    """Represents a CNF formula"""
    
    def __init__(self, variables: int, clauses: List[List[int]]):
        self.num_variables = variables
        self.clauses = [set(c) for c in clauses]  # Convert to sets for fast lookup
    
    def is_satisfied(self, assignment: Dict[int, bool]) -> bool:
        """Check if all clauses satisfied under assignment"""
        for clause in self.clauses:
            clause_result = False
            for lit in clause:
                var = abs(lit)
                if var not in assignment:
                    # Unassigned literal - clause not determined
                    clause_result = None
                    break
                if (lit > 0 and assignment[var]) or (lit < 0 and not assignment[var]):
                    clause_result = True
                    break
            
            if clause_result is False:
                return False  # Conflict: clause unsatisfiable
        
        return True
    
    def check_conflict(self, assignment: Dict[int, bool]) -> Optional[int]:
        """Check for conflict and return conflicting var if any"""
        for clause in self.clauses:
            lits_unknown = []
            clause_sat = False
            
            for lit in clause:
                var = abs(lit)
                if var not in assignment:
                    lits_unknown.append(var)
                elif (lit > 0 and assignment[var]) or (lit < 0 and not assignment[var]):
                    clause_sat = True
                    break
            
            if not clause_sat and not lits_unknown:
                # Unit clause violated
                return list(clause)[0]  # Return any literal from conflicting clause
        
        return None
    
    def unit_propagate(self, assignment: Dict[int, bool]) -> List[int]:
        """Unit propagation: assign variables forced by unit clauses"""
        assignments = []
        changed = True
        
        while changed:
            changed = False
            for clause in self.clauses:
                lits_unknown = []
                lit_true = False
                
                for lit in clause:
                    var = abs(lit)
                    if var in assignment:
                        if (lit > 0 and assignment[var]) or (lit < 0 and not assignment[var]):
                            lit_true = True
                            break
                    else:
                        lits_unknown.append(lit)
                
                if not lit_true and len(lits_unknown) == 1:
                    # Unit clause: must set this variable
                    lit = lits_unknown[0]
                    var = abs(lit)
                    assignment[var] = (lit > 0)
                    assignments.append(var)
                    changed = True
        
        return assignments


class DPLLSolver:
    """Pure DPLL SAT Solver without clause learning"""
    
    def __init__(self, formula: CNFFormula):
        self.formula = formula
        self.assignment = {}
        self.decisions = 0
        self.backtracks = 0
        self.tree_size = 0
        self.start_time = None
    
    def solve(self) -> Tuple[bool, Dict[int, bool]]:
        """
        Solve using DPLL algorithm
        
        Returns:
            (satisfiable, assignment)
        """
        self.start_time = time.time()
        self.assignment = {}
        self.decisions = 0
        self.backtracks = 0
        self.tree_size = 0
        
        result = self._dpll(self.assignment)
        
        return result, self.assignment
    
    def _dpll(self, assignment: Dict[int, bool]) -> bool:
        """DPLL recursion"""
        self.tree_size += 1
        
        # Unit propagation
        assigned = self.formula.unit_propagate(assignment)
        
        # Check for conflict
        if self.formula.check_conflict(assignment):
            # Backtrack
            if assigned:
                for var in assigned:
                    del assignment[var]
            self.backtracks += 1
            return False
        
        # Check if all variables assigned
        if len(assignment) == self.formula.num_variables:
            # All variables assigned and no conflict -> SAT
            return True
        
        # Choose unassigned variable (first unassigned)
        unassigned_var = None
        for var in range(1, self.formula.num_variables + 1):
            if var not in assignment:
                unassigned_var = var
                break
        
        if unassigned_var is None:
            return True
        
        # Try assignment
        self.decisions += 1
        
        # Try True first
        assignment[unassigned_var] = True
        if self._dpll(assignment):
            return True
        
        # Backtrack and try False
        self.backtracks += 1
        if unassigned_var in assignment:
            del assignment[unassigned_var]
        
        for var in assigned:
            if var in assignment:
                del assignment[var]
        
        assignment[unassigned_var] = False
        if self._dpll(assignment):
            return True
        
        # Backtrack
        self.backtracks += 1
        if unassigned_var in assignment:
            del assignment[unassigned_var]
        
        for var in assigned:
            if var in assignment:
                del assignment[var]
        
        return False


def parse_dimacs(filename: str) -> CNFFormula:
    """Parse DIMACS CNF format"""
    clauses = []
    num_vars = 0
    num_clauses_expected = 0
    
    with open(filename, 'r') as f:
        for line in f:
            line = line.strip()
            if line.startswith('c'):
                continue  # Comment
            if line.startswith('p'):
                parts = line.split()
                num_vars = int(parts[2])
                num_clauses_expected = int(parts[3])
                continue
            
            if line:
                clause = list(map(int, line.split()))
                clause = clause[:-1]  # Remove trailing 0
                if clause:
                    clauses.append(clause)
    
    return CNFFormula(num_vars, clauses)


def main():
    """Main: solve CNF file using pure DPLL"""
    
    if len(sys.argv) != 2:
        print("Usage: python custom_dpll_solver.py <cnf_file>")
        sys.exit(1)
    
    cnf_file = sys.argv[1]
    
    # Parse formula
    formula = parse_dimacs(cnf_file)
    
    # Solve
    solver = DPLLSolver(formula)
    satisfiable, assignment = solver.solve()
    runtime = time.time() - solver.start_time
    
    # Output result
    if satisfiable:
        print("SATISFIABLE")
        print("v " + " ".join(f"{i if assignment.get(i, False) else -i}" 
                             for i in range(1, formula.num_variables + 1)) + " 0")
    else:
        print("UNSATISFIABLE")
    
    # Output metrics in JSON (for framework parsing)
    metrics = {
        "result": "SATISFIABLE" if satisfiable else "UNSATISFIABLE",
        "runtime_seconds": runtime,
        "decisions": solver.decisions,
        "backtracks": solver.backtracks,
        "tree_size": solver.tree_size,
        "formula_vars": formula.num_variables,
        "formula_clauses": len(formula.clauses)
    }
    
    print("c JSON_METRICS: " + json.dumps(metrics))


if __name__ == "__main__":
    main()
