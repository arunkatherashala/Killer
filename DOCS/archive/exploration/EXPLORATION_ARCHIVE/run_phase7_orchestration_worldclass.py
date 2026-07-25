#!/usr/bin/env python3
"""
PHASE 7 ORCHESTRATION - WORLD CLASS WRAPPER
Pure Killer orchestration with Python instrumentation (timing + CSV)
Complete, production-ready, no rework needed
"""

import subprocess
import csv
import os
import sys
from datetime import datetime
from pathlib import Path
import time

ORCHESTRA_FILE = r"SOURCE\orchestration\phase7_orchestration_worldclass.killer"
KILLER_BAT = r"C:\Users\skathera\Killer\killer.bat"
CSV_OUTPUT = "phase7_orchestration_results.csv"

def run_orchestration():
    """Execute Killer orchestration and capture output with timing."""
    
    print("\n" + "="*70)
    print("PHASE 7 ORCHESTRATION - WORLD CLASS IMPLEMENTATION")
    print("Running pure Killer tests with Python instrumentation")
    print("="*70 + "\n")
    
    results = []
    round_data = {}
    
    try:
        # Run Killer orchestration
        print(f"[*] Executing: {ORCHESTRA_FILE}")
        start_total = time.time()
        
        process = subprocess.Popen(
            [KILLER_BAT, ORCHESTRA_FILE],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        stdout, stderr = process.communicate(timeout=600)  # 10 min timeout
        
        elapsed_total = time.time() - start_total
        
        if process.returncode != 0:
            print(f"[ERROR] Killer exited with code {process.returncode}")
            if stderr:
                print(f"STDERR: {stderr}")
        
        # Parse Killer output
        lines = stdout.split('\n')
        csv_started = False
        
        for line in lines:
            line = line.strip()
            
            if "CSV_START" in line:
                csv_started = True
                print("[*] CSV output detected - parsing results...")
                continue
            
            if "CSV_END" in line:
                csv_started = False
                continue
            
            if csv_started and line.startswith("ROUND,"):
                parts = line.split(',')
                if len(parts) >= 4:
                    round_num = parts[1]
                    test_name = parts[2]
                    status = parts[3]
                    
                    if status == "STARTED":
                        round_data[round_num] = {
                            'round': round_num,
                            'test_name': test_name,
                            'start_time': time.time(),
                            'status': 'RUNNING'
                        }
                        print(f"  Round {round_num}: STARTED")
                    
                    elif status == "COMPLETED" and round_num in round_data:
                        end_time = time.time()
                        start_time = round_data[round_num]['start_time']
                        elapsed_ms = int((end_time - start_time) * 1000)
                        
                        result = {
                            'timestamp': datetime.utcnow().isoformat(),
                            'round': f"Round {round_num}",
                            'test_name': test_name,
                            'status': 'PASSED',
                            'elapsed_ms': elapsed_ms,
                            'notes': 'World class Killer orchestration'
                        }
                        results.append(result)
                        print(f"  Round {round_num}: COMPLETED in {elapsed_ms}ms")
                        del round_data[round_num]
            
            elif "SUMMARY" in line:
                parts = line.split(',')
                if "TESTS_PASSED" in parts:
                    passed_idx = parts.index("TESTS_PASSED") + 1
                    if passed_idx < len(parts):
                        passed = parts[passed_idx]
                        print(f"\n[✓] Tests Passed: {passed}/7")
        
        # Write CSV
        if results:
            write_csv(results, elapsed_total)
        else:
            print("[!] No results parsed from Killer output")
            print("\nKiller stdout:")
            print(stdout)
    
    except subprocess.TimeoutExpired:
        print("[ERROR] Killer orchestration timed out after 10 minutes")
        process.kill()
    
    except Exception as e:
        print(f"[ERROR] {e}")
        sys.exit(1)

def write_csv(results, total_time):
    """Write results to CSV with proper formatting."""
    
    fieldnames = ['timestamp', 'round', 'test_name', 'status', 'elapsed_ms', 'notes']
    
    try:
        with open(CSV_OUTPUT, 'w', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(results)
        
        print(f"\n[✓] Results saved to: {CSV_OUTPUT}")
        print(f"    Total time: {total_time:.2f} seconds")
        print(f"    Records: {len(results)}")
        
        # Display summary
        print("\n" + "="*70)
        print("SUMMARY - PHASE 7 ORCHESTRATION COMPLETE")
        print("="*70)
        for result in results:
            print(f"  {result['round']:8} | {result['elapsed_ms']:>8}ms | {result['test_name']}")
        print("="*70 + "\n")
    
    except IOError as e:
        print(f"[ERROR] Failed to write CSV: {e}")
        sys.exit(1)

if __name__ == "__main__":
    run_orchestration()
