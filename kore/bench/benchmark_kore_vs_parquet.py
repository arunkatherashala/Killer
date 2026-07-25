#!/usr/bin/env python3
"""
KORE v2 vs Parquet+Snappy — Head-to-Head Benchmark
====================================================
Generates a ~1GB CSV, then compresses with both formats.
Measures: file size, compression ratio, write speed, read speed.
"""

import os, sys, time, random, string, csv, struct, hashlib

# ─── Step 0: Check pyarrow ───────────────────────────────────────────────────
try:
    import pyarrow as pa
    import pyarrow.parquet as pq
    import pyarrow.csv as pacsv
    print(f"[OK] pyarrow {pa.__version__}")
except ImportError:
    print("[FAIL] pyarrow not installed. Run: pip install pyarrow")
    sys.exit(1)

BENCH_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "bench_data")
os.makedirs(BENCH_DIR, exist_ok=True)

CSV_PATH     = os.path.join(BENCH_DIR, "benchmark_1gb.csv")
PARQUET_PATH = os.path.join(BENCH_DIR, "benchmark_snappy.parquet")
PARQUET_ZSTD = os.path.join(BENCH_DIR, "benchmark_zstd.parquet")
PARQUET_GZIP = os.path.join(BENCH_DIR, "benchmark_gzip.parquet")
PARQUET_NONE   = os.path.join(BENCH_DIR, "benchmark_none.parquet")
PARQUET_BROTLI = os.path.join(BENCH_DIR, "benchmark_brotli.parquet")
KORE_PATH      = os.path.join(BENCH_DIR, "benchmark.kore")

# ─── Step 1: Generate ~1GB CSV ───────────────────────────────────────────────
# Schema (realistic enterprise data — 15 columns):
#   id (sequential int), timestamp (datetime), customer_id (int, 50K unique),
#   product_code (str, 500 unique), category (str, 20 unique),
#   region (str, 8 unique), quantity (int, 1-1000),
#   unit_price (float, 2 decimals), discount (float, 0-50%),
#   total (float), is_returned (bool), payment_method (str, 5 unique),
#   shipping_status (str, 4 unique), notes (str, variable length),
#   score (float, 0-100)

TARGET_SIZE_MB = 1024  # ~1GB
ROWS_ESTIMATE  = 10_000_000  # ~10M rows ≈ 1GB with this schema

CATEGORIES     = ["Electronics", "Clothing", "Food", "Books", "Home", "Sports",
                  "Toys", "Automotive", "Health", "Beauty", "Garden", "Office",
                  "Music", "Movies", "Games", "Software", "Hardware", "Furniture",
                  "Jewelry", "Pet"]
REGIONS        = ["US-East", "US-West", "EU-North", "EU-South", "APAC", "LATAM", "MEA", "Canada"]
PAYMENTS       = ["Credit", "Debit", "Cash", "Wire", "Crypto"]
SHIPPING       = ["Delivered", "InTransit", "Pending", "Returned"]
PRODUCTS       = [f"PRD-{i:04d}" for i in range(500)]
NOTES_POOL     = [
    "Rush delivery requested", "Gift wrap", "Fragile item", "",
    "Customer VIP", "Bulk order", "Seasonal promotion", "Return within 30 days",
    "International shipping", "Insurance added", "Express shipping",
    "Standard delivery", "Back-ordered", "Pre-order", "Clearance item", "",
    "", "", "", ""  # lots of empties
]

def generate_csv(path, num_rows):
    """Generate a realistic CSV with mixed data types."""
    print(f"\n{'='*70}")
    print(f"  GENERATING {num_rows:,} row CSV (~1GB)")
    print(f"{'='*70}")

    if os.path.exists(path):
        size_mb = os.path.getsize(path) / (1024*1024)
        if size_mb > 500:
            print(f"  [SKIP] CSV already exists: {size_mb:.0f} MB")
            return
        else:
            print(f"  [REGEN] Existing CSV too small ({size_mb:.0f} MB), regenerating...")

    rng = random.Random(42)  # deterministic for reproducibility
    t0 = time.time()

    with open(path, 'w', newline='', buffering=1024*1024) as f:
        writer = csv.writer(f)
        # Header
        writer.writerow([
            "id", "timestamp", "customer_id", "product_code", "category",
            "region", "quantity", "unit_price", "discount", "total",
            "is_returned", "payment_method", "shipping_status", "notes", "score"
        ])
        # Data
        base_ts = 1577836800  # 2020-01-01 00:00:00 UTC
        for i in range(num_rows):
            cid = rng.randint(1, 50000)
            prod = rng.choice(PRODUCTS)
            cat = rng.choice(CATEGORIES)
            region = rng.choice(REGIONS)
            qty = rng.randint(1, 1000)
            price = round(rng.uniform(0.99, 999.99), 2)
            disc = round(rng.uniform(0, 0.50), 4)
            total = round(qty * price * (1 - disc), 2)
            ret = rng.random() < 0.05  # 5% return rate
            pay = rng.choice(PAYMENTS)
            ship = rng.choice(SHIPPING)
            note = rng.choice(NOTES_POOL)
            score = round(rng.uniform(0, 100), 2)
            ts = time.strftime("%Y-%m-%d %H:%M:%S", time.gmtime(base_ts + i * 3))

            writer.writerow([
                i + 1, ts, cid, prod, cat, region, qty,
                price, disc, total, int(ret), pay, ship, note, score
            ])
            if (i + 1) % 1_000_000 == 0:
                elapsed = time.time() - t0
                size_mb = os.path.getsize(path) / (1024*1024)
                print(f"  ... {i+1:>10,} rows  {size_mb:>7.0f} MB  ({elapsed:.1f}s)")

    elapsed = time.time() - t0
    size_mb = os.path.getsize(path) / (1024*1024)
    print(f"  DONE: {num_rows:,} rows, {size_mb:.1f} MB in {elapsed:.1f}s")
    print(f"  Rate: {num_rows/elapsed:,.0f} rows/sec")
    return size_mb


def benchmark_parquet(csv_path, compression, out_path):
    """Read CSV via pyarrow, write Parquet with given compression."""
    print(f"\n  --- Parquet + {compression.upper()} ---")

    # READ CSV
    t0 = time.time()
    table = pacsv.read_csv(csv_path)
    read_time = time.time() - t0
    print(f"  CSV read:  {read_time:.2f}s  ({table.num_rows:,} rows × {table.num_columns} cols)")

    # WRITE PARQUET
    t0 = time.time()
    pq.write_table(table, out_path, compression=compression)
    write_time = time.time() - t0
    size_bytes = os.path.getsize(out_path)
    size_mb = size_bytes / (1024*1024)
    csv_size = os.path.getsize(csv_path)
    ratio = size_bytes / csv_size * 100

    print(f"  Write:     {write_time:.2f}s")
    print(f"  Size:      {size_mb:.1f} MB  ({ratio:.1f}% of CSV)")
    print(f"  Speed:     {csv_size / (1024*1024) / write_time:.1f} MB/s")

    # READ PARQUET (measure decode speed)
    t0 = time.time()
    table2 = pq.read_table(out_path)
    decode_time = time.time() - t0
    print(f"  Read back: {decode_time:.2f}s  ({table2.num_rows:,} rows verified)")

    # COLUMN PRUNING (read single column)
    t0 = time.time()
    col = pq.read_table(out_path, columns=["total"])
    col_time = time.time() - t0
    print(f"  Col prune: {col_time:.3f}s  (read 'total' column only)")

    return {
        "format": f"Parquet+{compression}",
        "size_mb": size_mb,
        "ratio_pct": ratio,
        "write_sec": write_time,
        "read_sec": decode_time,
        "col_prune_sec": col_time,
        "size_bytes": size_bytes,
    }


def main():
    print("""
╔══════════════════════════════════════════════════════════════════════╗
║         KORE v2 + Nova  vs  Parquet + Snappy/Zstd/Gzip            ║
║              HEAD-TO-HEAD BENCHMARK  (~1 GB CSV)                   ║
╚══════════════════════════════════════════════════════════════════════╝
""")

    # ── Generate CSV ──────────────────────────────────────────────────────
    generate_csv(CSV_PATH, ROWS_ESTIMATE)
    csv_size_bytes = os.path.getsize(CSV_PATH)
    csv_size_mb = csv_size_bytes / (1024*1024)
    print(f"\n  CSV file: {csv_size_mb:.1f} MB  ({CSV_PATH})")

    results = []

    # ── Parquet benchmarks ────────────────────────────────────────────────
    print(f"\n{'='*70}")
    print(f"  PARQUET BENCHMARKS")
    print(f"{'='*70}")

    for codec, path in [
        ("snappy", PARQUET_PATH),
        ("zstd",   PARQUET_ZSTD),
        ("gzip",   PARQUET_GZIP),
        ("brotli", PARQUET_BROTLI),
        ("none",   PARQUET_NONE),
    ]:
        try:
            r = benchmark_parquet(CSV_PATH, codec, path)
            results.append(r)
        except Exception as e:
            print(f"  [ERROR] Parquet+{codec}: {e}")

    # ── KORE v2 benchmark (via Rust binary) ───────────────────────────────
    print(f"\n{'='*70}")
    print(f"  KORE v2 + NOVA BENCHMARK")
    print(f"{'='*70}")

    kore_bin = os.path.join(
        r"C:\Users\skathera\Downloads\killer_M29\killer\SOURCE\src\v2-rust\killer",
        "target", "release", "kore_bench.exe"
    )
    kore_bin_debug = kore_bin.replace("release", "debug")

    # Try release first, then debug
    bin_path = kore_bin if os.path.exists(kore_bin) else kore_bin_debug

    if os.path.exists(bin_path):
        import subprocess
        print(f"\n  --- KORE v2 + Nova (Rust native) ---")
        t0 = time.time()
        result = subprocess.run(
            [bin_path, CSV_PATH, KORE_PATH],
            capture_output=True, text=True, timeout=600
        )
        total_time = time.time() - t0
        print(result.stdout)
        if result.stderr:
            print(f"  STDERR: {result.stderr[:500]}")

        if os.path.exists(KORE_PATH):
            kore_size = os.path.getsize(KORE_PATH)
            kore_mb = kore_size / (1024*1024)
            kore_ratio = kore_size / csv_size_bytes * 100
            results.append({
                "format": "KORE v2 + Nova",
                "size_mb": kore_mb,
                "ratio_pct": kore_ratio,
                "write_sec": total_time,
                "read_sec": 0,  # will be filled by Rust binary output
                "col_prune_sec": 0,
                "size_bytes": kore_size,
            })
    else:
        print(f"  [SKIP] KORE binary not found at {bin_path}")
        print(f"  Build it with: cargo build --release --bin kore_bench")
        print(f"  (We'll still show Parquet results)")

    # ── RESULTS TABLE ─────────────────────────────────────────────────────
    print(f"\n{'='*70}")
    print(f"  FINAL RESULTS — {csv_size_mb:.0f} MB CSV ({ROWS_ESTIMATE:,} rows × 15 cols)")
    print(f"{'='*70}")
    print(f"  {'Format':<22} {'Size MB':>10} {'Ratio':>8} {'Write(s)':>10} {'Read(s)':>10} {'ColPrune':>10}")
    print(f"  {'-'*22} {'-'*10} {'-'*8} {'-'*10} {'-'*10} {'-'*10}")

    # Add raw CSV as baseline
    print(f"  {'CSV (raw)':<22} {csv_size_mb:>10.1f} {'100.0%':>8} {'-':>10} {'-':>10} {'-':>10}")

    for r in sorted(results, key=lambda x: x["size_mb"]):
        print(f"  {r['format']:<22} {r['size_mb']:>10.1f} {r['ratio_pct']:>7.1f}% {r['write_sec']:>10.2f} {r['read_sec']:>10.2f} {r['col_prune_sec']:>10.3f}")

    if len(results) >= 2:
        best = min(results, key=lambda x: x["size_mb"])
        worst = max(results, key=lambda x: x["size_mb"])
        print(f"\n  🏆 WINNER (size):  {best['format']}  ({best['size_mb']:.1f} MB)")
        print(f"  📊 Savings vs worst: {(1 - best['size_bytes']/worst['size_bytes'])*100:.1f}%")

        fastest = min(results, key=lambda x: x["write_sec"])
        print(f"  ⚡ FASTEST write:  {fastest['format']}  ({fastest['write_sec']:.2f}s)")

    print(f"\n{'='*70}")
    print(f"  Benchmark complete. Files in: {BENCH_DIR}")
    print(f"{'='*70}\n")


if __name__ == "__main__":
    main()
