//! Runs every example program and compares its output with the checked-in `.expected` file, so
//! the examples (and the docs that point at them) cannot silently stop working.

use std::path::PathBuf;
use std::process::Command;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn run_example(path: &PathBuf) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_killer_super"))
        .arg(path)
        .arg("--run")
        .output()
        .unwrap();
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
    .replace("\r\n", "\n")
}

fn check_dir(sub: &str) -> usize {
    let dir = examples_dir().join(sub);
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("killer") {
            continue;
        }
        let expected_path = path.with_extension("expected");
        let expected = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|_| panic!("missing {}", expected_path.display()))
            .replace("\r\n", "\n");
        let got = run_example(&path);
        assert_eq!(
            got.trim_end(),
            expected.trim_end(),
            "output of {} changed",
            path.display()
        );
        checked += 1;
    }
    checked
}

#[test]
fn uncertainty_examples_produce_their_expected_output() {
    assert!(check_dir("uncertainty") >= 4);
}
