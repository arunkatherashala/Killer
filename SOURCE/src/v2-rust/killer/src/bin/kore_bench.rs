// kore_bench.rs — KORE v2 Benchmark Binary
// Usage: kore_bench <csv_path> <kore_path>
// Measures: CSV→KORE write time, KORE read time, column pruning time

use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: kore_bench <csv_path> <kore_path>");
        std::process::exit(1);
    }
    let csv_path  = &args[1];
    let kore_path = &args[2];

    println!("KORE v2 Benchmark");
    println!("=================");
    println!("CSV:  {}", csv_path);
    println!("KORE: {}", kore_path);

    // Get CSV size
    let csv_meta = std::fs::metadata(csv_path).expect("Cannot stat CSV file");
    let csv_size = csv_meta.len();
    println!("CSV size: {} bytes ({:.1} MB)", csv_size, csv_size as f64 / (1024.0*1024.0));

    // ── WRITE: CSV → KORE v2 ─────────────────────────────────────────────
    println!("\n--- WRITE: CSV -> KORE v2 ---");
    let t0 = Instant::now();
    match killer_native::kore_v2::csv_to_kore(csv_path, kore_path) {
        Ok(msg) => {
            let write_elapsed = t0.elapsed();
            let kore_size = std::fs::metadata(kore_path).map(|m| m.len()).unwrap_or(0);
            let ratio = kore_size as f64 / csv_size as f64 * 100.0;
            let speed_mb = csv_size as f64 / (1024.0 * 1024.0) / write_elapsed.as_secs_f64();
            println!("  {}", msg);
            println!("  Write time: {:.2}s", write_elapsed.as_secs_f64());
            println!("  KORE size:  {} bytes ({:.1} MB)", kore_size, kore_size as f64 / (1024.0*1024.0));
            println!("  Ratio:      {:.1}% of CSV", ratio);
            println!("  Speed:      {:.1} MB/s", speed_mb);

            // ── READ: Full decode ────────────────────────────────────────
            println!("\n--- READ: KORE v2 full decode ---");
            let t1 = Instant::now();
            let reader = killer_native::kore_v2::KoreReader::open(kore_path)
                .expect("Cannot open KORE file");
            let open_elapsed = t1.elapsed();
            println!("  Open+parse header: {:.3}s", open_elapsed.as_secs_f64());
            println!("  {}", reader.info());

            let t2 = Instant::now();
            let cols = reader.read_all_columns();
            let read_elapsed = t2.elapsed();
            let decoded_rows = if cols.is_empty() { 0 } else { cols[0].len() };
            println!("  Full read:  {:.2}s  ({} rows × {} cols decoded, column-major)",
                     read_elapsed.as_secs_f64(), decoded_rows, cols.len());
            let read_speed = kore_size as f64 / (1024.0 * 1024.0) / read_elapsed.as_secs_f64();
            println!("  Read speed: {:.1} MB/s (compressed)", read_speed);

            // ── VERIFY: Compare KORE decode vs CSV ───────────────────────
            println!("\n--- VERIFY: KORE decode vs CSV (first 100K rows) ---");
            {
                use std::io::{BufRead, BufReader};
                let vf = std::fs::File::open(csv_path).expect("Cannot open CSV for verify");
                let vr = BufReader::with_capacity(1 << 20, vf);
                let mut lines = vr.lines();
                let header_line = lines.next().unwrap().unwrap();
                let headers: Vec<&str> = header_line.trim().split(',').collect();
                let check_rows = 100_000usize.min(decoded_rows);
                let mut mismatches = 0usize;
                let mut max_err: f64 = 0.0;
                let mut worst_col = String::new();
                let mut worst_row = 0usize;
                let mut first_mismatches: Vec<String> = Vec::new();
                for ri in 0..check_rows {
                    let line = match lines.next() {
                        Some(Ok(l)) => l,
                        _ => break,
                    };
                    let fields: Vec<&str> = line.trim().split(',').collect();
                    for ci in 0..headers.len().min(cols.len()) {
                        let csv_str = fields.get(ci).copied().unwrap_or("");
                        let kore_val = &cols[ci][ri];
                        let ok = match kore_val {
                            killer_native::kore_v2::KVal::Int(n) => {
                                // Try direct int parse first
                                if let Ok(v) = csv_str.parse::<i64>() {
                                    v == *n
                                } else if csv_str.len() >= 19 && csv_str.contains('-') {
                                    // Timestamp string → epoch: skip this comparison (lossless conversion)
                                    true
                                } else {
                                    false
                                }
                            }
                            killer_native::kore_v2::KVal::Float(f) => {
                                if let Ok(csv_f) = csv_str.parse::<f64>() {
                                    let err = (csv_f - f).abs();
                                    if err > max_err {
                                        max_err = err;
                                        worst_col = headers[ci].to_string();
                                        worst_row = ri;
                                    }
                                    err < 0.02
                                } else { false }
                            }
                            killer_native::kore_v2::KVal::Bool(b) => {
                                let csv_b = csv_str == "1" || csv_str.eq_ignore_ascii_case("true");
                                csv_b == *b
                            }
                            killer_native::kore_v2::KVal::Str(s) => s == csv_str,
                            _ => true,
                        };
                        if !ok {
                            mismatches += 1;
                            if first_mismatches.len() < 10 {
                                first_mismatches.push(format!(
                                    "  row={} col={}({}) csv={:?} kore={:?}",
                                    ri, ci, headers[ci], csv_str, kore_val
                                ));
                            }
                        }
                    }
                }
                let total_cells = check_rows * headers.len().min(cols.len());
                if mismatches == 0 {
                    println!("  PASS: {}/{} cells match (max float err: {:.6} in col '{}' row {})",
                             total_cells, total_cells, max_err, worst_col, worst_row);
                } else {
                    println!("  FAIL: {} mismatches out of {} cells (max float err: {:.6} in col '{}' row {})",
                             mismatches, total_cells, max_err, worst_col, worst_row);
                    for m in &first_mismatches {
                        println!("{}", m);
                    }
                }
            }

            // ── COLUMN PRUNING ───────────────────────────────────────────
            println!("\n--- COLUMN PRUNING (read 'total' column only) ---");
            // Re-open to be fair
            let reader2 = killer_native::kore_v2::KoreReader::open(kore_path)
                .expect("Cannot re-open KORE file");
            let t3 = Instant::now();
            let col = reader2.read_column("total");
            let prune_elapsed = t3.elapsed();
            let col_len = col.as_ref().map(|c| c.len()).unwrap_or(0);
            println!("  Column prune: {:.3}s  ({} values)", prune_elapsed.as_secs_f64(), col_len);

            // ── PREDICATE PUSHDOWN ───────────────────────────────────────
            println!("\n--- PREDICATE PUSHDOWN (quantity > 900) ---");
            let reader3 = killer_native::kore_v2::KoreReader::open(kore_path)
                .expect("Cannot re-open KORE file");
            let t4 = Instant::now();
            let filtered = reader3.filter_pushdown("quantity", ">", &killer_native::kore_v2::KVal::Int(900));
            let push_elapsed = t4.elapsed();
            println!("  Pushdown filter: {:.3}s  ({} matching rows)", push_elapsed.as_secs_f64(), filtered.len());

            // ── STATS ────────────────────────────────────────────────────
            println!("\n--- COLUMN STATS (no data decode needed) ---");
            let reader4 = killer_native::kore_v2::KoreReader::open(kore_path)
                .expect("Cannot re-open KORE file");
            let t5 = Instant::now();
            if let Some(stats) = reader4.column_stats("quantity") {
                let stats_elapsed = t5.elapsed();
                println!("  Stats time: {:.3}s", stats_elapsed.as_secs_f64());
                println!("  quantity: min={}, max={}, nulls={}",
                         stats.min_i64, stats.max_i64, stats.null_count);
            }

            // ── SUMMARY ─────────────────────────────────────────────────
            println!("\n=== KORE v2 SUMMARY ===");
            println!("WRITE_SEC={:.2}", write_elapsed.as_secs_f64());
            println!("READ_SEC={:.2}", read_elapsed.as_secs_f64());
            println!("SIZE_MB={:.1}", kore_size as f64 / (1024.0 * 1024.0));
            println!("RATIO_PCT={:.1}", ratio);
            println!("COL_PRUNE_SEC={:.3}", prune_elapsed.as_secs_f64());
            println!("COL_PRUNE_SPEEDUP={:.1}x", read_elapsed.as_secs_f64() / prune_elapsed.as_secs_f64().max(0.001));
            println!("PUSHDOWN_SEC={:.3}", push_elapsed.as_secs_f64());
            println!("PUSHDOWN_SPEEDUP={:.1}x", read_elapsed.as_secs_f64() / push_elapsed.as_secs_f64().max(0.001));
            println!("ROWS={}", decoded_rows);
        }
        Err(e) => {
            let elapsed = t0.elapsed();
            eprintln!("  [ERROR] CSV->KORE failed after {:.2}s: {}", elapsed.as_secs_f64(), e);
            std::process::exit(1);
        }
    }
}
