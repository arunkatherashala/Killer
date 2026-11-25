//! Mutation fuzzing of binary/text decoders: no panic, no hang, no huge allocation.
use killer_native::kore_v2::{csv_to_kore, DeleteBitmap, KoreReader};
use std::sync::mpsc;
use std::time::Duration;

/// Small deterministic PRNG (xorshift64*), so failures are reproducible.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n.max(1) as u64)) as usize
    }
}

/// Run `f(input)` on a thread; fail on panic or when it exceeds the watchdog.
fn guarded<F>(label: &str, input: Vec<u8>, f: F)
where
    F: FnOnce(Vec<u8>) + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let h = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            f(input);
            let _ = tx.send(());
        })
        .unwrap();
    match rx.recv_timeout(Duration::from_secs(20)) {
        Ok(()) => {
            h.join().unwrap();
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            let e = h.join().err();
            panic!("{label}: decoder panicked: {:?}", e.and_then(|e| e.downcast_ref::<String>().cloned()));
        }
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("{label}: decoder hung (>20s)"),
    }
}

fn sample_kore() -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("killer_fuzz_kore_{}_{}", std::process::id(), { static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0); N.fetch_add(1, std::sync::atomic::Ordering::SeqCst) }));
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("s.csv");
    let kore = dir.join("s.kore");
    let mut text = String::from("id,name,score,city,flag\n");
    for i in 0..400 {
        text.push_str(&format!(
            "{},user{},{}.{},{},{}\n",
            i,
            i % 37,
            i * 3 % 101,
            i % 10,
            ["paris", "rome", "oslo", "lima"][i % 4],
            i % 2 == 0
        ));
    }
    std::fs::write(&csv, text).unwrap();
    csv_to_kore(csv.to_str().unwrap(), kore.to_str().unwrap()).expect("csv_to_kore");
    let b = std::fs::read(&kore).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    b
}

fn exercise_kore(data: Vec<u8>) {
    if let Ok(r) = KoreReader::from_bytes(data) {
        let _ = r.read_all();
        let _ = r.info();
        let names: Vec<String> = r.columns.iter().map(|c| c.name.clone()).collect();
        for n in &names {
            let _ = r.read_column(n);
            let _ = r.column_stats(n);
        }
    }
}

#[test]
fn kore_header_claims_are_validated_before_allocating() {
    let good = sample_kore();
    // sanity: the unmodified file decodes
    assert!(KoreReader::from_bytes(good.clone()).is_ok());
    // header: ncols u16 @6, nrows u64 @8, nchunks u32 @16, chunk_size u32 @20
    for (off, len) in [(8usize, 8usize), (16, 4), (20, 4), (6, 2)] {
        let mut b = good.clone();
        for i in 0..len {
            b[off + i] = 0xFF;
        }
        guarded(&format!("header@{off}"), b, exercise_kore);
    }
    // footer trailer: comp_len u32 + offset u64 in the last 12 bytes
    let mut b = good.clone();
    let n = b.len();
    for i in 0..12 {
        b[n - 12 + i] = 0xFF;
    }
    guarded("trailer", b, exercise_kore);
}

#[test]
fn kore_mutation_fuzz() {
    let good = sample_kore();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let iters: usize = std::env::var("KILLER_FUZZ_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
    for it in 0..iters {
        let mut b = good.clone();
        match rng.below(4) {
            0 => {
                for _ in 0..1 + rng.below(8) {
                    let i = rng.below(b.len());
                    b[i] = rng.next() as u8;
                }
            }
            1 => {
                let keep = rng.below(b.len());
                b.truncate(keep);
            }
            2 => {
                // overwrite an aligned-ish window with 0xFF / 0x00
                let i = rng.below(b.len());
                let w = 1 + rng.below(8);
                let v = if rng.below(2) == 0 { 0xFF } else { 0x00 };
                for j in i..(i + w).min(b.len()) {
                    b[j] = v;
                }
            }
            _ => {
                // bit flips in the first 64 bytes / last 64 bytes
                let i = if rng.below(2) == 0 { rng.below(64.min(b.len())) } else { b.len() - 1 - rng.below(64.min(b.len())) };
                b[i] ^= 1 << rng.below(8);
            }
        }
        guarded(&format!("kore iter {it}"), b, exercise_kore);
    }
}

#[test]
fn kore_delete_bitmap_hostile_header() {
    let dir = std::env::temp_dir().join(format!("killer_fuzz_del_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let kore = dir.join("x.kore");
    let del = dir.join("x.kore.del");
    for (rows, deleted) in [(1u64 << 63, 0u64), (u64::MAX, u64::MAX), (1 << 40, 5), (10, 99)] {
        let mut b = Vec::new();
        b.extend_from_slice(&rows.to_le_bytes());
        b.extend_from_slice(&deleted.to_le_bytes());
        b.extend_from_slice(&[0xFF; 16]);
        std::fs::write(&del, &b).unwrap();
        let p = kore.to_str().unwrap().to_string();
        guarded("del", vec![], move |_| {
            if let Ok(bm) = DeleteBitmap::load(&p) {
                let _ = bm.active_count();
                let _ = bm.is_deleted(5);
            }
        });
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn msgpack_and_toml_mutation_fuzz() {
    use killer_native::killer_serialization::{msgpack_decode, msgpack_encode, toml_parse, SerValue};
    let sample = SerValue::Map(vec![
        ("name".into(), SerValue::Str("killer".into())),
        ("n".into(), SerValue::Int(-5)),
        ("f".into(), SerValue::Float(1.5)),
        ("list".into(), SerValue::Array(vec![SerValue::Int(1), SerValue::Null, SerValue::Bool(true), SerValue::Str("x".into())])),
    ]);
    let mp = msgpack_encode(&sample);
    let toml = b"title = \"x\"\nn = 3\narr = [1, [2, 3], \"a\"]\n[server]\nhost = 'h'\nport = 80\n".to_vec();
    let mut rng = Rng(0xDEADBEEFCAFEF00D);
    for it in 0..4000 {
        for (label, base) in [("msgpack", &mp), ("toml", &toml)] {
            let mut b = base.clone();
            match rng.below(3) {
                0 => {
                    for _ in 0..1 + rng.below(4) {
                        let i = rng.below(b.len());
                        b[i] = rng.next() as u8;
                    }
                }
                1 => b.truncate(rng.below(b.len())),
                _ => {
                    let i = rng.below(b.len());
                    b.insert(i, rng.next() as u8);
                }
            }
            let is_mp = label == "msgpack";
            guarded(&format!("{label} iter {it}"), b, move |d| {
                if is_mp {
                    let _ = msgpack_decode(&d);
                } else {
                    let _ = toml_parse(&String::from_utf8_lossy(&d));
                }
            });
        }
    }
}

fn sample_pack() -> Vec<u8> {
    killer_native::pkg_files::pack(&[
        ("killer.toml".to_string(), b"[package]\nname = \"fz\"\nversion = \"1.0.0\"\n\n[dependencies]\nother = \"^1.2\"\n".to_vec()),
        ("src/lib.killer".to_string(), b"fn f() {\n  return 1\n}\n".to_vec()),
        ("README.md".to_string(), b"# fz\n\nline\n".to_vec()),
        ("data/a/b.txt".to_string(), Vec::new()),
    ])
}

/// Everything a registry or `killer_pkg install` does with an archive it was handed: unpack, check the manifest
/// against it, hash it, store it as a release and write the files out (to a fresh scratch directory).
fn exercise_pack(data: Vec<u8>, scratch: std::path::PathBuf) {
    use killer_native::package_manager::Version;
    use killer_native::{pkg_files, pkg_registry};
    if let Ok(files) = pkg_files::unpack(&data) {
        let _ = pkg_files::content_hash(&files);
        if let Some((_, m)) = files.iter().find(|(p, _)| p == "killer.toml") {
            let _ = pkg_registry::verify_manifest_matches_archive(&String::from_utf8_lossy(m), &files);
        }
        let _ = std::fs::remove_dir_all(&scratch);
        let _ = pkg_registry::store_release(&scratch.join("reg"), "fz", &Version::parse("1.0.0").unwrap(), &data);
        let _ = pkg_files::write_files(&scratch.join("out"), &files);
        let _ = std::fs::remove_dir_all(&scratch);
    }
}

#[test]
fn package_archive_mutation_fuzz() {
    let good = sample_pack();
    assert_eq!(killer_native::pkg_files::unpack(&good).map(|f| f.len()), Ok(4));
    let base = std::env::temp_dir().join(format!("killer_fuzz_pkg_{}", std::process::id()));
    let mut rng = Rng(0x51ED_2701_C0FF_EE11);
    let iters: usize = std::env::var("KILLER_FUZZ_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(1500);
    // hand-picked hostile headers first, then random mutations
    let mut inputs: Vec<Vec<u8>> = [
        &b"KPK1\n"[..],
        b"KPK1\n0 a\n\n0 a\n\n",
        b"KPK1\n1 a\nx\n0 a/b\n\n",
        b"KPK1\n0 a/b\n\n1 a\nx\n",
        b"KPK1\n99999999999999999999 a\nx\n",
        b"KPK1\n-1 a\nx\n",
        b"KPK1\n+1 a\nx\n",
        b"KPK1\n1 \nx\n",
        b"KPK1\n1 a\x00b\nx\n",
        b"KPK1\n1 CON\nx\n",
        b"KPK1\n1 a/NUL.txt\nx\n",
        b"KPK1\n1 a.\nx\n",
        b"KPK1\n1 a \nx\n",
        b"KPK1\n1 a*?\nx\n",
        b"KPK1\n1 killer.toml\n[\n",
        b"KPK1\n1",
        b"KPK1\n\xff\xfe a\nx\n",
    ]
    .iter()
    .map(|b| b.to_vec())
    .collect();
    for _ in 0..iters {
        let mut b = good.clone();
        match rng.below(5) {
            0 => {
                for _ in 0..1 + rng.below(4) {
                    let i = rng.below(b.len());
                    b[i] = rng.next() as u8;
                }
            }
            1 => b.truncate(rng.below(b.len())),
            2 => {
                let i = rng.below(b.len());
                b.insert(i, [b'\n', b' ', b'/', b'.', b'0', b'9', 0][rng.below(7)]);
            }
            3 => {
                // rewrite one length field with a hostile number
                let heads: Vec<usize> = (5..b.len()).filter(|&i| b[i - 1] == b'\n' && b[i].is_ascii_digit()).collect();
                if let Some(&h) = heads.get(rng.below(heads.len())) {
                    let end = h + b[h..].iter().position(|&c| c == b' ').unwrap_or(0);
                    let n = ["0", "1", "18446744073709551615", "18446744073709551616", "4294967296", "999999"][rng.below(6)];
                    b.splice(h..end, n.bytes());
                }
            }
            _ => {
                // duplicate a slice (repeated entries, paths that are both a file and a directory)
                let i = rng.below(b.len());
                let j = (i + rng.below(64)).min(b.len());
                let piece = b[i..j].to_vec();
                let at = rng.below(b.len());
                b.splice(at..at, piece);
            }
        }
        inputs.push(b);
    }
    for (it, b) in inputs.into_iter().enumerate() {
        let scratch = base.join(format!("i{it}"));
        guarded(&format!("pkg iter {it}"), b, move |d| exercise_pack(d, scratch));
    }
    let _ = std::fs::remove_dir_all(&base);
}
