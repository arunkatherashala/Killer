#!/usr/bin/env python3
"""
KILLER V2 - COMPREHENSIVE PERFORMANCE TEST WITH FULL RECORD KEEPING
Records all performance data to CSV for historical tracking
Full-load testing (100K x 100 iterations, etc.)
"""

import subprocess
import time
import csv
import os
from datetime import datetime

# Configuration
TEST_OUTPUT_CSV = "performance_records_full_load.csv"
TEST_LOG_FILE = "performance_test_full_load.log"
KILLER_TIMEOUT = 300  # 5 minutes per test

def log_message(msg):
    """Log to both console and file"""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    log_entry = f"[{timestamp}] {msg}"
    print(log_entry)
    with open(TEST_LOG_FILE, "a") as f:
        f.write(log_entry + "\n")

def run_killer_test(test_code, test_name, timeout=KILLER_TIMEOUT):
    """Run a Killer test and capture timing"""
    log_message(f"Starting: {test_name}")
    
    try:
        # Save test code to temp file
        temp_file = f"temp_test_{int(time.time())}.killer"
        with open(temp_file, "w") as f:
            f.write(test_code)
        
        start_time = time.time()
        result = subprocess.run(
            [r"C:\Users\skathera\Killer\killer.bat", temp_file],
            capture_output=True,
            text=True,
            timeout=timeout
        )
        
        # Clean up temp file
        try:
            os.remove(temp_file)
        except:
            pass
        
        elapsed = time.time() - start_time
        
        if result.returncode == 0:
            log_message(f"  PASSED in {elapsed:.2f}s - {test_name}")
            return {
                "status": "PASSED",
                "elapsed_ms": int(elapsed * 1000),
                "output": result.stdout
            }
        else:
            log_message(f"  FAILED - {test_name}: {result.stderr}")
            return {
                "status": "FAILED",
                "elapsed_ms": int(elapsed * 1000),
                "error": result.stderr
            }
    except subprocess.TimeoutExpired:
        log_message(f"  TIMEOUT (>{timeout}s) - {test_name}")
        return {
            "status": "TIMEOUT",
            "elapsed_ms": timeout * 1000,
            "error": f"Test exceeded {timeout}s timeout"
        }
    except Exception as e:
        log_message(f"  ERROR - {test_name}: {str(e)}")
        return {
            "status": "ERROR",
            "elapsed_ms": 0,
            "error": str(e)
        }

# Test suites - FULL LOAD

test_round_1 = """
count = 0;
i = 0;
while (i < 1000000) {
    a = i * 2 + i / 2 - i % 7;
    b = a + 1;
    c = b * 3;
    d = c / 2;
    e = d - a;
    f = e + b;
    count = count + f;
    i = i + 1;
}
print("Round 1 Result: ", count);
"""

test_round_2 = """
result = 0;
i = 0;
while (i < 100000) {
    j = 0;
    while (j < 10) {
        result = result + i * j;
        j = j + 1;
    }
    i = i + 1;
}
print("Round 2 Result: ", result);
"""

test_round_3 = """
fn fib_mod(n, mod_val) {
    if (n == 0) { return 0; }
    if (n == 1) { return 1; }
    a = 1; b = 1; c = 1; d = 0;
    ma = 1; mb = 1; mc = 1; md = 0;
    exp = n;
    while (exp > 0) {
        if ((exp % 2) == 1) {
            na = (a * ma + b * mc) % mod_val;
            nb = (a * mb + b * md) % mod_val;
            nc = (c * ma + d * mc) % mod_val;
            nd = (c * mb + d * md) % mod_val;
            a = na; b = nb; c = nc; d = nd;
        }
        sa = (ma * ma + mb * mc) % mod_val;
        sb = (ma * mb + mb * md) % mod_val;
        sc = (mc * ma + md * mc) % mod_val;
        sd = (mc * mb + md * md) % mod_val;
        ma = sa; mb = sb; mc = sc; md = sd;
        exp = exp / 2;
    }
    return b;
}
count = 0;
i = 0;
while (i < 100) {
    result = fib_mod(10000000 + i, 1000000007);
    count = count + 1;
    i = i + 1;
}
print("Round 3 Result: ", count);
"""

test_round_4 = """
result = 0;
i = 0;
while (i < 100000) {
    j = 0;
    while (j < 100) {
        val = (i * j) % 1000;
        result = result + val;
        j = j + 1;
    }
    i = i + 1;
}
print("Round 4 Result: ", result);
"""

test_round_5 = """
result = 0;
i = 1;
while (i < 100000) {
    j = 1;
    while (j < 100) {
        val = i / j;
        result = result + val;
        j = j + 1;
    }
    i = i + 1;
}
print("Round 5 Result: ", result);
"""

test_round_6 = """
count = 0;
i = 0;
while (i < 100000) {
    if (i % 2 == 0) { count = count + 1; }
    if (i % 3 == 0) { count = count + 2; }
    if (i % 5 == 0) { count = count + 3; }
    if (i % 7 == 0) { count = count + 4; }
    if (i % 11 == 0) { count = count + 5; }
    i = i + 1;
}
print("Round 6 Result: ", count);
"""

test_round_7 = """
result = 0;
i = 1;
while (i < 10000) {
    a = i;
    b = a * a;
    c = b * b;
    d = c * c;
    result = result + d;
    i = i + 1;
}
print("Round 7 Result: ", result);
"""

def main():
    """Run all 7 tests and record results"""
    
    log_message("=" * 80)
    log_message("KILLER V2 - COMPREHENSIVE PERFORMANCE TEST (FULL LOAD)")
    log_message("=" * 80)
    
    # Clear/initialize log
    open(TEST_LOG_FILE, "w").close()
    
    # Initialize CSV with headers
    csv_headers = ["timestamp", "round", "test_name", "status", "elapsed_ms", "notes"]
    if not os.path.exists(TEST_OUTPUT_CSV):
        with open(TEST_OUTPUT_CSV, "w", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=csv_headers)
            writer.writeheader()
    
    tests = [
        ("Round 1", "Baseline Arithmetic (1M iterations)", test_round_1),
        ("Round 2", "Nested Loops (100K x 10)", test_round_2),
        ("Round 3", "Fibonacci O(log n) (100 computations)", test_round_3),
        ("Round 4", "Modulo Operations (100K x 100 - FULL LOAD)", test_round_4),
        ("Round 5", "Division Operations (100K x 100 - FULL LOAD)", test_round_5),
        ("Round 6", "Conditional Branching (100K)", test_round_6),
        ("Round 7", "Power Operations (10K)", test_round_7),
    ]
    
    results = []
    total_start = time.time()
    
    for round_num, test_name, test_code in tests:
        result = run_killer_test(test_code, test_name)
        results.append((round_num, test_name, result))
        
        # Record to CSV
        with open(TEST_OUTPUT_CSV, "a", newline="") as f:
            writer = csv.DictWriter(f, fieldnames=csv_headers)
            writer.writerow({
                "timestamp": datetime.now().isoformat(),
                "round": round_num,
                "test_name": test_name,
                "status": result["status"],
                "elapsed_ms": result["elapsed_ms"],
                "notes": result.get("error", "")
            })
    
    total_time = time.time() - total_start
    
    # Summary
    log_message("\n" + "=" * 80)
    log_message("SUMMARY")
    log_message("=" * 80)
    
    passed = sum(1 for _, _, r in results if r["status"] == "PASSED")
    total = len(results)
    
    for round_num, test_name, result in results:
        log_message(f"  {round_num}: {test_name}")
        log_message(f"    Status: {result['status']} | Time: {result['elapsed_ms']}ms")
    
    log_message(f"\nTotal Results: {passed}/{total} PASSED")
    log_message(f"Total Time: {total_time:.2f}s")
    log_message(f"Records saved to: {TEST_OUTPUT_CSV}")
    log_message("=" * 80)
    
    print(f"\n✅ Performance records saved to: {TEST_OUTPUT_CSV}")
    print(f"✅ Test log saved to: {TEST_LOG_FILE}")

if __name__ == "__main__":
    main()
