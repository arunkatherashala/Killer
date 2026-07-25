#!/usr/bin/env python3
"""
Phase 2 Direction 1: SAT Solver Experimental Framework

Tests how different SAT solvers perform on Pigeonhole formulas.
Measures: runtime, memory, decisions, backtracks, clauses learned, etc.

Solvers to integrate:
  1. MiniSat (baseline CDCL)
  2. CaDiCaL (modern CDCL)
  3. Kissat (2023 SAT race winner)
  4. Glucose (learning-focused)
  5. Custom DPLL (pure, no learning)

Framework tracks: Runtime scaling, solver comparison, efficiency metrics.
"""

import subprocess
import time
import json
import os
from pathlib import Path
from typing import Dict, List, Tuple, Optional
from dataclasses import dataclass, asdict
import statistics


@dataclass
class SolverResult:
    """Result from single solver run on formula"""
    formula_name: str
    solver_name: str
    run_number: int
    status: str  # "SATISFIABLE", "UNSATISFIABLE", "TIMEOUT", "ERROR"
    runtime_seconds: float
    peak_memory_mb: Optional[float] = None
    decisions: Optional[int] = None
    backtracks: Optional[int] = None
    clauses_learned: Optional[int] = None
    search_tree_size: Optional[int] = None
    output_log: str = ""
    error_log: str = ""


class SolverExperiment:
    """Manages Phase 2 Direction 1 SAT solver experiments"""
    
    def __init__(self, formulas_dir: str, output_dir: str = "EXPERIMENTS/DIRECTION_1_RESULTS"):
        self.formulas_dir = Path(formulas_dir)
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.results: List[SolverResult] = []
        
        # Available solvers (with installation paths/commands)
        self.solvers = {
            "minisat": {
                "installed": False,
                "command": "minisat",
                "parser": self._parse_minisat
            },
            "cadical": {
                "installed": False,
                "command": "cadical",
                "parser": self._parse_cadical
            },
            "kissat": {
                "installed": False,
                "command": "kissat",
                "parser": self._parse_kissat
            },
            "glucose": {
                "installed": False,
                "command": "glucose",
                "parser": self._parse_glucose
            },
            "custom_dpll": {
                "installed": False,
                "command": "python custom_dpll_solver.py",
                "parser": self._parse_custom_dpll
            }
        }
        
        # Detect installed solvers
        self._detect_installed_solvers()
    
    def _detect_installed_solvers(self):
        """Check which solvers are available"""
        print("Detecting installed SAT solvers...\n")
        
        for solver_name in self.solvers:
            cmd = self.solvers[solver_name]["command"].split()[0]
            try:
                subprocess.run([cmd, "--version"], capture_output=True, timeout=5)
                self.solvers[solver_name]["installed"] = True
                print(f"  ✓ {solver_name.upper()}: Found")
            except (FileNotFoundError, subprocess.TimeoutExpired):
                print(f"  ✗ {solver_name.upper()}: Not installed")
        
        print()
    
    def get_available_solvers(self) -> List[str]:
        """Get list of available solvers"""
        return [s for s in self.solvers if self.solvers[s]["installed"]]
    
    def run_experiment(self, solver: str, formula_file: str, run_num: int = 1, 
                      timeout: int = 3600) -> SolverResult:
        """
        Run a single solver on a formula
        
        Args:
            solver: Solver name
            formula_file: Path to CNF file
            run_num: Run number (for repeated experiments)
            timeout: Timeout in seconds
        
        Returns:
            SolverResult with runtime and metrics
        """
        
        formula_name = Path(formula_file).stem
        start_time = time.time()
        
        try:
            # Run solver
            cmd = f"{self.solvers[solver]['command']} {formula_file}"
            result = subprocess.run(
                cmd,
                shell=True,
                capture_output=True,
                timeout=timeout,
                text=True
            )
            
            runtime = time.time() - start_time
            status = "UNKNOWN"
            
            # Parse output
            output_log = result.stdout
            error_log = result.stderr
            
            # Determine status
            if "SATISFIABLE" in output_log or result.returncode == 10:
                status = "SATISFIABLE"
            elif "UNSATISFIABLE" in output_log or result.returncode == 20:
                status = "UNSATISFIABLE"
            else:
                status = "UNKNOWN"
            
            # Parse metrics using solver-specific parser
            metrics = self.solvers[solver]["parser"](output_log)
            
            result_obj = SolverResult(
                formula_name=formula_name,
                solver_name=solver,
                run_number=run_num,
                status=status,
                runtime_seconds=runtime,
                output_log=output_log[:500],  # First 500 chars
                error_log=error_log[:500],
                **metrics
            )
            
            self.results.append(result_obj)
            return result_obj
            
        except subprocess.TimeoutExpired:
            runtime = time.time() - start_time
            result_obj = SolverResult(
                formula_name=formula_name,
                solver_name=solver,
                run_number=run_num,
                status="TIMEOUT",
                runtime_seconds=runtime
            )
            self.results.append(result_obj)
            return result_obj
        
        except Exception as e:
            runtime = time.time() - start_time
            result_obj = SolverResult(
                formula_name=formula_name,
                solver_name=solver,
                run_number=run_num,
                status="ERROR",
                runtime_seconds=runtime,
                error_log=str(e)
            )
            self.results.append(result_obj)
            return result_obj
    
    # Parser methods for each solver
    def _parse_minisat(self, output: str) -> Dict:
        """Parse MiniSat output"""
        metrics = {}
        # Look for typical MiniSat output patterns
        for line in output.split('\n'):
            if "CPU time" in line and "s" in line:
                pass  # Time already captured separately
        return metrics
    
    def _parse_cadical(self, output: str) -> Dict:
        """Parse CaDiCaL output"""
        metrics = {}
        for line in output.split('\n'):
            if "decisions:" in line.lower():
                try:
                    metrics["decisions"] = int(line.split()[-1])
                except:
                    pass
        return metrics
    
    def _parse_kissat(self, output: str) -> Dict:
        """Parse Kissat output"""
        return {}
    
    def _parse_glucose(self, output: str) -> Dict:
        """Parse Glucose output"""
        return {}
    
    def _parse_custom_dpll(self, output: str) -> Dict:
        """Parse custom DPLL solver output (JSON format)"""
        try:
            import json
            data = json.loads(output)
            return {
                "decisions": data.get("decisions"),
                "backtracks": data.get("backtracks"),
                "search_tree_size": data.get("tree_size")
            }
        except:
            return {}
    
    def run_solver_comparison(self, formula_file: str, solvers: Optional[List[str]] = None,
                             num_runs: int = 3, timeout: int = 3600):
        """
        Compare multiple solvers on same formula
        
        Args:
            formula_file: Path to CNF file
            solvers: List of solvers (default: all available)
            num_runs: Number of repeated runs per solver
            timeout: Timeout per run
        """
        
        if solvers is None:
            solvers = self.get_available_solvers()
        
        formula_name = Path(formula_file).stem
        print(f"\n=== Testing {formula_name} ===")
        print(f"Solvers: {', '.join(solvers)}")
        print(f"Runs per solver: {num_runs}\n")
        
        for solver in solvers:
            print(f"{solver.upper()}:")
            for run in range(1, num_runs + 1):
                result = self.run_experiment(solver, formula_file, run, timeout)
                print(f"  Run {run}: {result.status} ({result.runtime_seconds:.2f}s)")
    
    def summarize_results(self):
        """Summarize and analysis experimental results"""
        
        if not self.results:
            print("No results to summarize")
            return
        
        # Group results by formula and solver
        by_formula = {}
        by_solver = {}
        
        for result in self.results:
            if result.formula_name not in by_formula:
                by_formula[result.formula_name] = []
            by_formula[result.formula_name].append(result)
            
            if result.solver_name not in by_solver:
                by_solver[result.solver_name] = []
            by_solver[result.solver_name].append(result)
        
        print("\n=== EXPERIMENTAL RESULTS SUMMARY ===\n")
        
        # Per-formula summary
        print("By Formula:")
        for formula in sorted(by_formula.keys()):
            results = by_formula[formula]
            times = [r.runtime_seconds for r in results if r.status != "ERROR"]
            if times:
                print(f"  {formula}:")
                print(f"    Mean time: {statistics.mean(times):.2f}s")
                print(f"    Median time: {statistics.median(times):.2f}s")
                print(f"    Min: {min(times):.2f}s, Max: {max(times):.2f}s")
        
        # Per-solver summary
        print("\nBy Solver:")
        for solver in sorted(by_solver.keys()):
            results = by_solver[solver]
            successful = [r for r in results if r.status == "UNSATISFIABLE"]
            timeout_count = sum(1 for r in results if r.status == "TIMEOUT")
            print(f"  {solver}:")
            print(f"    Successful: {len(successful)}/{len(results)}")
            print(f"    Timeouts: {timeout_count}")
            if successful:
                times = [r.runtime_seconds for r in successful]
                print(f"    Avg time: {statistics.mean(times):.2f}s")
        
        # Save summarized results to JSON
        results_file = self.output_dir / "experiment_results.json"
        with open(results_file, 'w') as f:
            json.dump([asdict(r) for r in self.results], f, indent=2)
        
        print(f"\n✓ Results saved: {results_file}")


def main():
    """Main: Run Direction 1 experiments"""
    
    print("=== Phase 2 Direction 1: SAT Solver Experiments ===\n")
    
    # Initialize experiment
    exp = SolverExperiment("EXPERIMENTS/DIRECTION_1_DATA")
    
    available = exp.get_available_solvers()
    if not available:
        print("WARNING: No SAT solvers detected. To run experiments, install:")
        print("  - MiniSat: apt-get install minisat")
        print("  - CaDiCaL: https://github.com/arminbiere/cadical")
        print("  - Kissat: https://github.com/arminbiere/kissat")
        print("  - Glucose: https://www.labri.fr/perso/lsimon/research/glucose/")
        return
    
    # Find first Pigeonhole formula
    formulas = list(Path("EXPERIMENTS/DIRECTION_1_DATA").glob("php_*.cnf"))
    
    if formulas:
        # Run comparison on smallest formula (quickest)
        test_formula = min(formulas, key=lambda f: f.stat().st_size)
        exp.run_solver_comparison(str(test_formula), num_runs=1, timeout=60)
        
        # Summarize
        exp.summarize_results()
    else:
        print("No Pigeonhole formulas found. Run generator first:")
        print("  python phase2_direction1_pigeonhole_generator.py")


if __name__ == "__main__":
    main()
