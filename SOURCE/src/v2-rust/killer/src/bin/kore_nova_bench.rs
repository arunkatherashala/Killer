// kore_nova_bench.rs — Benchmark: KORE v2 then Nova compress on top
// Usage: kore_nova_bench <csv_path> <kore_path>
// Measures: CSV→KORE, KORE→KORE+Nova(.nvz), and decompression round-trip

use std::time::Instant;
use killer_native::nova::{nova_compress, nova_decompress};
use killer_native::value::Value;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: kore_nova_bench <csv_path> <kore_path>");
        std::process::exit(1);
    }
    let csv_path  = &args[1];
    let kore_path = &args[2];
    let novz_path = format!("{}.nvz", kore_path);
    let round_path = format!("{}.round", kore_path);

    let csv_size = std::fs::metadata(csv_path).expect("Cannot stat CSV").len();
    println!("KORE + Nova Benchmark");
    println!("=====================");
    println!("CSV:  {} ({:.1} MB)", csv_path, csv_size as f64 / (1024.0*1024.0));

    // Step 1: CSV → KORE v2
    println!("\n--- Step 1: CSV -> KORE v2 ---");
    let t0 = Instant::now();
    match killer_native::kore_v2::csv_to_kore(csv_path, kore_path) {
        Ok(msg) => {
            let elapsed = t0.elapsed();
            let kore_size = std::fs::metadata(kore_path).map(|m| m.len()).unwrap_or(0);
            let ratio = kore_size as f64 / csv_size as f64 * 100.0;
            println!("  {}", msg);
            println!("  Time:  {:.2}s", elapsed.as_secs_f64());
            println!("  Size:  {:.1} MB ({:.1}% of CSV)", kore_size as f64 / (1024.0*1024.0), ratio);

            // Step 2: KORE → KORE+Nova
            println!("\n--- Step 2: KORE -> KORE+Nova (.nvz) ---");
            let t1 = Instant::now();
            match nova_compress(&[
                Value::Str(kore_path.to_string()),
                Value::Str(novz_path.clone()),
            ]) {
                Ok(_) => {
                    let elapsed2 = t1.elapsed();
                    let novz_size = std::fs::metadata(&novz_path).map(|m| m.len()).unwrap_or(0);
                    let ratio_csv = novz_size as f64 / csv_size as f64 * 100.0;
                    let ratio_kore = novz_size as f64 / kore_size as f64 * 100.0;
                    let savings = (1.0 - novz_size as f64 / kore_size as f64) * 100.0;
                    println!("  Time:   {:.2}s", elapsed2.as_secs_f64());
                    println!("  Size:   {:.1} MB ({:.1}% of CSV, {:.1}% of KORE)",
                             novz_size as f64 / (1024.0*1024.0), ratio_csv, ratio_kore);
                    println!("  Nova saved: {:.1}% on top of KORE", savings);

                    // Step 3: Decompress round-trip
                    println!("\n--- Step 3: Decompress KORE+Nova -> KORE ---");
                    let t2 = Instant::now();
                    match nova_decompress(&[
                        Value::Str(novz_path.clone()),
                        Value::Str(round_path.clone()),
                    ]) {
                        Ok(_) => {
                            let elapsed3 = t2.elapsed();
                            println!("  Decompress: {:.2}s", elapsed3.as_secs_f64());

                            // Verify round-trip
                            let orig = std::fs::read(kore_path).unwrap();
                            let round = std::fs::read(&round_path).unwrap();
                            if orig == round {
                                println!("  Round-trip: VERIFIED (byte-identical)");
                            } else {
                                println!("  Round-trip: FAILED (data mismatch!)");
                            }

                            // Read the decompressed KORE
                            println!("\n--- Step 4: Read decompressed KORE ---");
                            let t3 = Instant::now();
                            let reader = killer_native::kore_v2::KoreReader::open(&round_path)
                                .expect("Cannot open round-trip KORE");
                            let cols = reader.read_all_columns();
                            let read_elapsed = t3.elapsed();
                            let decoded_rows = if cols.is_empty() { 0 } else { cols[0].len() };
                            println!("  Read:  {:.2}s  ({} rows × {} cols)",
                                     read_elapsed.as_secs_f64(), decoded_rows, cols.len());
                        }
                        Err(e) => println!("  [ERROR] Decompress: {:?}", e),
                    }

                    // Summary
                    let total_write = t0.elapsed();
                    println!("\n=== SUMMARY ===");
                    println!("CSV:           {:.1} MB", csv_size as f64 / (1024.0*1024.0));
                    println!("KORE:          {:.1} MB ({:.1}%)", kore_size as f64 / (1024.0*1024.0),
                             kore_size as f64 / csv_size as f64 * 100.0);
                    println!("KORE+Nova:     {:.1} MB ({:.1}%)", novz_size as f64 / (1024.0*1024.0),
                             novz_size as f64 / csv_size as f64 * 100.0);
                    println!("Total write:   {:.2}s (CSV→KORE→Nova)", total_write.as_secs_f64());

                    // Cleanup round-trip file
                    let _ = std::fs::remove_file(&round_path);
                }
                Err(e) => println!("  [ERROR] Nova compress: {:?}", e),
            }
        }
        Err(e) => println!("  [ERROR] csv_to_kore: {}", e),
    }
}
