//! Size + wall-clock encode/decode benchmark: **KORE**, **KPAR**, and **NOVZ** on the same ~10 MiB CSV.
//!
//! Compares:
//! - **CSV → NOVZ** — generic compressed *bytes* (same class of tool as external gzip/zstd on the file).
//! - **CSV → KORE** — typed columnar Nova format (your “latest” pipeline).
//! - **CSV → KPAR** — KORE + `nova_to_parquet` layout (not Apache Parquet).
//! - **KORE → NOVZ** — optional second wrapper on the `.kore` file.
//!
//! Run (release recommended):
//!   cargo run --release --bin kore_nova_format_bench
//!
//! For **gzip/zstd** baselines, compress the same CSV with your toolchain and compare **size + time**
//! to the printed **CSV → NOVZ** row (same role: “shrink the CSV blob”).
//!
//! `nova_compress` / `nova_decompress` print progress lines to stderr; the table is stdout.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use killer_native::nova::{
    nova_compress, nova_decompress, nova_from_parquet, nova_read_all, nova_to_parquet, nova_write,
};
use killer_native::value::Value;

const TARGET_CSV_BYTES: usize = 10 * 1024 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = std::env::temp_dir();
    let stem = format!("killer_kore_bench_{}", std::process::id());
    let csv_path = tmp.join(format!("{stem}.csv"));
    let kore_path = tmp.join(format!("{stem}.kore"));
    let kpar_path = tmp.join(format!("{stem}.kpar"));
    let novz_csv_path = tmp.join(format!("{stem}.csv.nvz"));
    let round_csv_path = tmp.join(format!("{stem}.csv.round"));
    let kore_novz_path = tmp.join(format!("{stem}.kore.nvz"));
    let round_kore_path = tmp.join(format!("{stem}.kore.round"));
    let kpar_round_kore = tmp.join(format!("{stem}.from_kpar.kore"));

    eprintln!("Building ~{} MiB CSV → {}", TARGET_CSV_BYTES / (1024 * 1024), csv_path.display());

    let t0 = Instant::now();
    let written = write_big_csv(&csv_path)? as u64;
    let gen_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let csv_bytes = std::fs::read(&csv_path)?;
    assert_eq!(csv_bytes.len() as u64, written);

    // —— CSV → KORE ——
    let t0 = Instant::now();
    nova_write(&[
        Value::Str(csv_path.to_string_lossy().into_owned()),
        Value::Str(kore_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_write: {e}"))?;
    let kore_write_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let kore_size = std::fs::metadata(&kore_path)?.len();

    let t0 = Instant::now();
    let _table = nova_read_all(&[Value::Str(kore_path.to_string_lossy().into_owned())])
        .map_err(|e| format!("nova_read_all: {e}"))?;
    let kore_read_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // —— CSV → KPAR ——
    let t0 = Instant::now();
    nova_to_parquet(&[
        Value::Str(kore_path.to_string_lossy().into_owned()),
        Value::Str(kpar_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_to_parquet: {e}"))?;
    let kpar_write_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let kpar_size = std::fs::metadata(&kpar_path)?.len();

    let t0 = Instant::now();
    nova_from_parquet(&[
        Value::Str(kpar_path.to_string_lossy().into_owned()),
        Value::Str(kpar_round_kore.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_from_parquet: {e}"))?;
    let kpar_to_kore_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let kore_from_kpar = std::fs::read(&kpar_round_kore)?;
    let kore_disk = std::fs::read(&kore_path)?;
    assert_eq!(
        kore_from_kpar, kore_disk,
        "KPAR round-trip must reconstruct KORE bytes"
    );

    let t0 = Instant::now();
    let _ = nova_read_all(&[Value::Str(kpar_round_kore.to_string_lossy().into_owned())])
        .map_err(|e| format!("nova_read_all after KPAR: {e}"))?;
    let kpar_then_read_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // —— CSV → NOVZ ——
    let t0 = Instant::now();
    nova_compress(&[
        Value::Str(csv_path.to_string_lossy().into_owned()),
        Value::Str(novz_csv_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_compress csv: {e}"))?;
    let novz_enc_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let novz_csv_size = std::fs::metadata(&novz_csv_path)?.len();

    let t0 = Instant::now();
    nova_decompress(&[
        Value::Str(novz_csv_path.to_string_lossy().into_owned()),
        Value::Str(round_csv_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_decompress csv: {e}"))?;
    let novz_dec_ms = t0.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(std::fs::read(&round_csv_path)?, csv_bytes);

    // —— KORE → NOVZ ——
    let t0 = Instant::now();
    nova_compress(&[
        Value::Str(kore_path.to_string_lossy().into_owned()),
        Value::Str(kore_novz_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_compress kore: {e}"))?;
    let kore_novz_enc_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let kore_novz_size = std::fs::metadata(&kore_novz_path)?.len();

    let t0 = Instant::now();
    nova_decompress(&[
        Value::Str(kore_novz_path.to_string_lossy().into_owned()),
        Value::Str(round_kore_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_decompress kore: {e}"))?;
    let kore_novz_dec_ms = t0.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(std::fs::read(&round_kore_path)?, kore_disk);

    let csv_sz = csv_bytes.len() as u64;

    // Efficiency summary vs generic byte compression (NOVZ)
    let kore_vs_novz_pct = (kore_size as f64 / novz_csv_size as f64) * 100.0;
    let kpar_vs_novz_pct = (kpar_size as f64 / novz_csv_size as f64) * 100.0;

    println!();
    println!(
        "KORE / Nova benchmark — synthetic ~10 MiB CSV (same generator as `csv_format_compare`)"
    );
    const W: usize = 116;
    println!("{}", "=".repeat(W));
    println!(
        "{:<40} {:>12} {:>18} {:>8} {:>11} {:>11}  {}",
        "Conversion → output",
        "Size",
        "Bytes",
        "% CSV",
        "encode ms",
        "decode ms",
        "notes"
    );
    println!("{}", "-".repeat(W));

    print_bench_row(
        "CSV (raw baseline)",
        csv_sz,
        csv_sz,
        gen_ms,
        read_file_ms(&csv_path)?,
        "write + read",
    );
    print_bench_row(
        "CSV → NOVZ",
        novz_csv_size,
        csv_sz,
        novz_enc_ms,
        novz_dec_ms,
        "generic compressed bytes",
    );
    print_bench_row(
        "CSV → KORE (nova_write)",
        kore_size,
        csv_sz,
        kore_write_ms,
        kore_read_ms,
        "columnar NOVA file",
    );
    print_bench_row(
        "CSV → KPAR (KORE + nova_to_parquet)",
        kpar_size,
        csv_sz,
        kore_write_ms + kpar_write_ms,
        kpar_to_kore_ms + kpar_then_read_ms,
        "KPAR layout (not Apache)",
    );
    print_bench_row(
        "KORE → NOVZ",
        kore_novz_size,
        csv_sz,
        kore_novz_enc_ms,
        kore_novz_dec_ms,
        "NOVZ on .kore",
    );

    println!("{}", "=".repeat(W));
    println!(
        "Efficiency vs generic byte compression (this run): KORE size {:.1}% of NOVZ size; KPAR {:.1}% of NOVZ size. (<100%% = smaller than compressing raw CSV with NOVZ.)",
        kore_vs_novz_pct, kpar_vs_novz_pct
    );
    println!(
        "Speed: lower encode/decode ms is better. KORE encode cost grows with CSV parse + column encoding — profile on your hardware and datasets."
    );
    println!(
        "Sharing results: quote **CSV**, **KORE**, **KPAR**, **NOVZ** sizes + ms from this table; add **gzip -9** (or zstd) size/time on the same file for a familiar industry baseline."
    );

    cleanup_files(vec![
        csv_path,
        kore_path,
        kpar_path,
        novz_csv_path,
        round_csv_path,
        kore_novz_path,
        round_kore_path,
        kpar_round_kore,
    ]);

    Ok(())
}

fn print_bench_row(name: &str, size: u64, csv: u64, enc_ms: f64, dec_ms: f64, notes: &str) {
    let pct = if csv > 0 {
        (size as f64 / csv as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "{name:<40} {:>12} {:>18} {pct:>7.2}% {enc_ms:>11.2} {dec_ms:>11.2}  {notes}",
        human_size_ib(size),
        bytes_with_commas(size),
    );
}

/// Binary KiB / MiB / GiB for “how big on disk”.
fn human_size_ib(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.2} GiB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.2} MiB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.2} KiB", n as f64 / KB as f64)
    } else {
        format!("{} B", n)
    }
}

fn bytes_with_commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

fn read_file_ms(path: &Path) -> Result<f64, std::io::Error> {
    let t0 = Instant::now();
    let _ = std::fs::read(path)?;
    Ok(t0.elapsed().as_secs_f64() * 1000.0)
}

fn cleanup_files(paths: Vec<PathBuf>) {
    for p in paths {
        let _ = std::fs::remove_file(p);
    }
}

fn write_big_csv(path: &Path) -> std::io::Result<usize> {
    let mut f = std::fs::File::create(path)?;
    let header = "id,region,value,note\n";
    f.write_all(header.as_bytes())?;
    const TAIL: &[u8] = b",north-east,42.5,repeated_tail_for_lz\n";
    let mut written = header.len();
    let mut id: u64 = 0;
    while written < TARGET_CSV_BYTES {
        write!(f, "{:08}", id % 100_000_000)?;
        f.write_all(TAIL)?;
        written += 8 + TAIL.len();
        id = id.wrapping_add(1);
    }
    Ok(written)
}
