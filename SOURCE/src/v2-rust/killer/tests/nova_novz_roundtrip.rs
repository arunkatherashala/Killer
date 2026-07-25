//! NOVZ (`nova_compress` / `nova_decompress`) byte-for-byte round-trip on a temp file.

use killer_native::nova::{nova_compress, nova_decompress};
use killer_native::value::Value;
use std::path::PathBuf;

#[test]
fn novz_compress_decompress_recover_exact_bytes() {
    let dir = std::env::temp_dir();
    let stem = format!("killer_novz_test_{}", std::process::id());
    let src: PathBuf = dir.join(format!("{stem}.txt"));
    let mid: PathBuf = dir.join(format!("{stem}.nvz"));
    let dst: PathBuf = dir.join(format!("{stem}.out.txt"));

    let payload = b"The NOVZ pipeline should recover these bytes exactly.\nRepeated: abababab\n\x00\xFF\x7F";
    std::fs::write(&src, payload).expect("write src");

    nova_compress(&[
        Value::Str(src.to_string_lossy().into_owned()),
        Value::Str(mid.to_string_lossy().into_owned()),
    ])
    .expect("nova_compress");

    assert!(mid.exists(), ".nvz written");
    let compressed = std::fs::read(&mid).expect("read nvz");
    assert!(
        compressed.starts_with(b"NOVZ"),
        "expected NOVZ magic, got {:?}",
        &compressed[..compressed.len().min(8)]
    );

    nova_decompress(&[
        Value::Str(mid.to_string_lossy().into_owned()),
        Value::Str(dst.to_string_lossy().into_owned()),
    ])
    .expect("nova_decompress");

    let out = std::fs::read(&dst).expect("read out");
    assert_eq!(out, payload);

    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&mid);
    let _ = std::fs::remove_file(&dst);
}
