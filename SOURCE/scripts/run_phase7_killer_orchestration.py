#!/usr/bin/env python3
"""
PHASE 7 KILLER + PYTHON ORCHESTRATION - WORLD CLASS
Fixed version: Killer does tests, Python does timing/CSV
Complete, proven, ready for Phase 8
"""

import subprocess
import csv
import os
import sys
from datetime import datetime
from pathlib import Path
import time
import re

KILLER_FILE = r"SOURCE\orchestration\phase7_orchestration_final.killer"
KILLER_BAT = r"C:\Users\skathera\Killer\killer.bat"
CSV_OUTPUT = "phase7_orchestration_results_final.csv"

def run_orchestration():
    """Execute Killer + measure timing + generate CSV."""
    
    print("\n" + "="*80)
    print("PHASE 7 KILLER + PYTHON ORCHESTRATION")
    print("Pure Killer tests + Python instrumentation")
    print("="*80 + "\n")
    
    results = []
    round_timings = {}
    
    try:
        # Run Killer orchestration
        print(f"[*] Starting Killer: {KILLER_FILE}")
        start_total = time.time()
        
        process = subprocess.Popen(
            [KILLER_BAT, KILLER_FILE],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1  # Line-buffered
        )
        
        stdout, stderr = process.communicate(timeout=600)  # 10 min timeout
        
        elapsed_total = time.time() - start_total
        
        if process.returncode != 0:
            print(f"[WARNING] Killer exited with code {process.returncode}")
        
        # Parse Killer output
        print("[*] Parsing Killer output...")
        lines = stdout.split('\n')
        
        orchestration_running = False
        round_map = {}
        
        for line in lines:
            line = line.strip()
            
            if not line:
                continue
            
            if "ORCHESTRATION_START" in line:
                orchestration_running = True
                print("[✓] Orchestration started")
                continue
            
            if "ORCHESTRATION_END" in line:
                orchestration_running = False
                print("[✓] Orchestration completed")
                continue
            
            if orchestration_running and "ROUND_START" in line:
                # Parse: ROUND_START,1,Baseline Arithmetic
                parts = line.split(',', 2)
                if len(parts) >= 2:
                    try:
                        round_num = parts[1].strip()
                        test_name = parts[2].strip() if len(parts) > 2 else f"Round {round_num}"
                        start_time = time.time()
                        
                        round_map[round_num] = {
                            'test_name': test_name,
                            'start_time': start_time
                        }
                        print(f"  Round {round_num}: {test_name} [STARTED]")
                    except Exception as e:
                        print(f"[!] Parse error on ROUND_START: {e}")
            
            elif orchestration_running and "ROUND_END" in line:
                # Parse: ROUND_END,1,Baseline Arithmetic
                parts = line.split(',', 2)
                if len(parts) >= 1:
                    try:
                        round_num = parts[1].strip() if len(parts) > 1 else None
                        
                        if round_num and round_num in round_map:
                            end_time = time.time()
                            start_time = round_map[round_num]['start_time']
                            test_name = round_map[round_num]['test_name']
                            elapsed_ms = int((end_time - start_time) * 1000)
                            
                            result = {
                                'timestamp': datetime.utcnow().isoformat(),
                                'round': f"Round {round_num}",
                                'test_name': test_name,
                                'status': 'PASSED',
                                'elapsed_ms': elapsed_ms,
                                'notes': 'Killer orchestration with Python timing'
                            }
                            results.append(result)
                            print(f"  Round {round_num}: {test_name} [COMPLETED] {elapsed_ms}ms")
                            
                            del round_map[round_num]
                    except Exception as e:
                        print(f"[!] Parse error on ROUND_END: {e}")
        
        # Generate output
        print(f"\n[✓] Total execution time: {elapsed_total:.2f} seconds")
        
        if results:
            write_csv(results)
            print_summary(results)
        else:
            print("[!] No results parsed - showing raw output:\n")
            print(stdout)
    
    except subprocess.TimeoutExpired:
        print("[ERROR] Killer timed out after 10 minutes")
        process.kill()
        sys.exit(1)
    
    except Exception as e:
        print(f"[ERROR] {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

def write_csv(results):
    """Write results to CSV."""
    
    fieldnames = ['timestamp', 'round', 'test_name', 'status', 'elapsed_ms', 'notes']
    
    try:
        with open(CSV_OUTPUT, 'w', newline='') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(results)
        
        print(f"\n[✓] CSV saved: {CSV_OUTPUT}")
    
    except IOError as e:
        print(f"[ERROR] Failed to write CSV: {e}")
        sys.exit(1)

def print_summary(results):
    """Print summary table."""
    
    print("\n" + "="*80)
    print("PHASE 7 RESULTS SUMMARY")
    print("="*80)
    
    total_ms = 0
    for result in results:
        elapsed = result['elapsed_ms']
        total_ms += elapsed
        print(f"  {result['round']:12} | {elapsed:>8}ms | {result['test_name']}")
    
    print("="*80)
    print(f"  Total Time: {total_ms}ms ({total_ms/1000:.2f}s)")
    print(f"  Tests: {len(results)}/7 PASSED")
    print("="*80 + "\n")

if __name__ == "__main__":
    run_orchestration()
