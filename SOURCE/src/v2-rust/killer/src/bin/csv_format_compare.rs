//! ~10 MiB CSV benchmark — explicit pipelines:
//! - **CSV → NOVZ** (compress raw `.csv`), **CSV → KORE** (`nova_write`), **CSV → KPAR** (`nova_write` then `nova_to_parquet`; KPAR is Nova’s layout, not Apache Parquet).
//! - **KORE → NOVZ** (NOVZ wrap of the `.kore` blob — “novz kore”).
//! - **NOVD/NOVT** trit demo + **NOVD→NOVZ** / **NOVT→NOVZ**.
//!
//! Run from crate root:
//!   cargo run --release --bin csv_format_compare
//!
//! Note: `nova_to_parquet` here is **KORE-Parquet / KPAR** (see `nova.rs`), not Apache Parquet.

use std::io::Write;
use std::path::Path;

use killer_native::nova::{nova_compress, nova_decompress, nova_to_parquet, nova_write};
use killer_native::nova_trit_codec::{pack_trits_novd, pack_trits_novt};
use killer_native::value::Value;

const TARGET_CSV_BYTES: usize = 10 * 1024 * 1024;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = std::env::temp_dir();
    let stem = format!("killer_csv_cmp_{}", std::process::id());
    let csv_path = tmp.join(format!("{stem}.csv"));
    let kore_path = tmp.join(format!("{stem}.kore"));
    let kpar_path = tmp.join(format!("{stem}.kpar"));
    let novz_path = tmp.join(format!("{stem}.novz"));
    let round_csv_path = tmp.join(format!("{stem}.round.csv"));
    let kore_novz_path = tmp.join(format!("{stem}.kore.nvz"));
    let round_kore_path = tmp.join(format!("{stem}.kore.round"));
    let novd_raw_path = tmp.join(format!("{stem}.novd"));
    let novd_novz_path = tmp.join(format!("{stem}.novd.nvz"));
    let novd_round_path = tmp.join(format!("{stem}.novd.round"));
    let novt_raw_path = tmp.join(format!("{stem}.novt"));
    let novt_novz_path = tmp.join(format!("{stem}.novt.nvz"));
    let novt_round_path = tmp.join(format!("{stem}.novt.round"));

    eprintln!("Building ~{} MiB CSV at {} …", TARGET_CSV_BYTES / (1024 * 1024), csv_path.display());

    let csv_size = write_big_csv(&csv_path)? as u64;

    nova_write(&[
        Value::Str(csv_path.to_string_lossy().into_owned()),
        Value::Str(kore_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_write: {e}"))?;

    nova_to_parquet(&[
        Value::Str(kore_path.to_string_lossy().into_owned()),
        Value::Str(kpar_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_to_parquet: {e}"))?;

    nova_compress(&[
        Value::Str(csv_path.to_string_lossy().into_owned()),
        Value::Str(novz_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_compress: {e}"))?;

    // Sanity: CSV → NOVZ round-trip
    nova_decompress(&[
        Value::Str(novz_path.to_string_lossy().into_owned()),
        Value::Str(round_csv_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_decompress CSV: {e}"))?;
    let round_bytes = std::fs::read(&round_csv_path)?;
    let orig_bytes = std::fs::read(&csv_path)?;
    assert_eq!(round_bytes, orig_bytes, "NOVZ round-trip must match");

    let kore_size = std::fs::metadata(&kore_path)?.len();
    let kpar_size = std::fs::metadata(&kpar_path)?.len();
    let novz_csv_size = std::fs::metadata(&novz_path)?.len();

    let kore_bytes = std::fs::read(&kore_path)?;
    nova_compress(&[
        Value::Str(kore_path.to_string_lossy().into_owned()),
        Value::Str(kore_novz_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_compress KORE: {e}"))?;
    let kore_novz_size = std::fs::metadata(&kore_novz_path)?.len();
    nova_decompress(&[
        Value::Str(kore_novz_path.to_string_lossy().into_owned()),
        Value::Str(round_kore_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_decompress KORE: {e}"))?;
    assert_eq!(
        std::fs::read(&round_kore_path)?,
        kore_bytes,
        "KORE → NOVZ round-trip must match NOVA/KORE bytes"
    );

    // NOVT: each raw byte → trit via (b % 3) mapped to {-1,0,1} — density demo only
    let trits: Vec<i8> = orig_bytes
        .iter()
        .map(|&b| match b % 3 {
            0 => -1i8,
            1 => 0,
            _ => 1,
        })
        .collect();
    let novt_blob = pack_trits_novt(&trits)?;
    let novt_size = novt_blob.len() as u64;
    let novd_blob = pack_trits_novd(&trits)?;
    let novd_size = novd_blob.len() as u64;
    let naive_trits_as_i8 = trits.len() as u64;

    // Stack: trit pack → NOVZ on the packed bytes (good when blob has patterns; may expand if already random-like).
    std::fs::write(&novd_raw_path, &novd_blob)?;
    nova_compress(&[
        Value::Str(novd_raw_path.to_string_lossy().into_owned()),
        Value::Str(novd_novz_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_compress NOVD: {e}"))?;
    let novd_novz_size = std::fs::metadata(&novd_novz_path)?.len();
    nova_decompress(&[
        Value::Str(novd_novz_path.to_string_lossy().into_owned()),
        Value::Str(novd_round_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_decompress NOVD: {e}"))?;
    assert_eq!(
        std::fs::read(&novd_round_path)?,
        novd_blob,
        "NOVD → NOVZ round-trip must preserve packed bytes"
    );

    std::fs::write(&novt_raw_path, &novt_blob)?;
    nova_compress(&[
        Value::Str(novt_raw_path.to_string_lossy().into_owned()),
        Value::Str(novt_novz_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_compress NOVT: {e}"))?;
    let novt_novz_size = std::fs::metadata(&novt_novz_path)?.len();
    nova_decompress(&[
        Value::Str(novt_novz_path.to_string_lossy().into_owned()),
        Value::Str(novt_round_path.to_string_lossy().into_owned()),
    ])
    .map_err(|e| format!("nova_decompress NOVT: {e}"))?;
    assert_eq!(
        std::fs::read(&novt_round_path)?,
        novt_blob,
        "NOVT → NOVZ round-trip must preserve packed bytes"
    );

    const TW: usize = 88;
    println!("~10 MiB CSV — output size by pipeline (same input file)");
    println!("{}", "=".repeat(TW));
    println!(
        "{:<38} {:>12} {:>18} {:>10}",
        "Pipeline → artifact",
        "Size",
        "Bytes",
        "% of CSV"
    );
    println!("{}", "-".repeat(TW));
    print_row("Raw CSV", csv_size, csv_size);
    print_row("CSV → NOVZ (compress .csv)", novz_csv_size, csv_size);
    print_row("CSV → KORE / NOVA (nova_write)", kore_size, csv_size);
    print_row("CSV → KPAR (KORE → nova_to_parquet)", kpar_size, csv_size);
    print_row("KORE → NOVZ (compress .kore)", kore_novz_size, csv_size);
    print_row("NOVT blob (2 bit/trit, byte→trit demo)", novt_size, csv_size);
    print_row("NOVD blob (~log₂(3) bit/trit dense)", novd_size, csv_size);
    print_row("NOVD blob → NOVZ", novd_novz_size, csv_size);
    print_row("NOVT blob → NOVZ", novt_novz_size, csv_size);
    println!("{}", "-".repeat(TW));
    println!(
        "NOVT vs naive i8 trit buffer: {} bytes packed vs {} bytes if stored as i8 each (×{:.2} shrink)",
        novt_size,
        naive_trits_as_i8,
        naive_trits_as_i8 as f64 / novt_size as f64
    );
    println!(
        "NOVD vs NOVT on same trits: ×{:.2} smaller file (header+dense base-3 payload)",
        novt_size as f64 / novd_size as f64
    );
    println!();
    println!("Note: KPAR is Nova’s KORE-Parquet layout (see nova.rs), not Apache Parquet.");
    println!("KORE → NOVZ is a second NOVZ wrapper on the columnar file after CSV → KORE.");
    println!("NOVT/NOVD rows: packing demo from CSV bytes → synthetic trits, not semantic column trits.");
    println!("NOVD/NOVT → NOVZ: same synthetic trits; compares stacked byte compression on packed blobs.");

    let _ = std::fs::remove_file(&csv_path);
    let _ = std::fs::remove_file(&kore_path);
    let _ = std::fs::remove_file(&kpar_path);
    let _ = std::fs::remove_file(&novz_path);
    let _ = std::fs::remove_file(&round_csv_path);
    let _ = std::fs::remove_file(&kore_novz_path);
    let _ = std::fs::remove_file(&round_kore_path);
    let _ = std::fs::remove_file(&novd_raw_path);
    let _ = std::fs::remove_file(&novd_novz_path);
    let _ = std::fs::remove_file(&novd_round_path);
    let _ = std::fs::remove_file(&novt_raw_path);
    let _ = std::fs::remove_file(&novt_novz_path);
    let _ = std::fs::remove_file(&novt_round_path);

    Ok(())
}

fn write_big_csv(path: &Path) -> std::io::Result<usize> {
    let mut f = std::fs::File::create(path)?;
    let header = "id,region,value,note\n";
    f.write_all(header.as_bytes())?;
    // Vary `id` per row so columnar Nova is not pathological (all-identical rows → tiny file).
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

fn print_row(label: &str, size: u64, base: u64) {
    let pct = if base > 0 {
        (size as f64 / base as f64) * 100.0
    } else {
        0.0
    };
    println!(
        "{label:<38} {:>12} {:>18} {pct:>9.2}%",
        human_size_ib(size),
        bytes_with_commas(size),
    );
}

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
