use kpm::registry::install_from_local;
use std::fs::{self, File};
use std::io::Write;

#[test]
fn test_local_install_creates_dest() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let manifest_path = tmp_dir.path().join("pkg.json");
    let data_dir = tmp_dir.path().join("data");
    fs::create_dir_all(&data_dir).unwrap();
    let file_path = data_dir.join("hello.txt");
    let mut f = File::create(&file_path).unwrap();
    writeln!(f, "hello").unwrap();
    let manifest = serde_json::json!({
        "name": "hello",
        "version": "0.1.0",
        "files": ["data/hello.txt"]
    });
    fs::write(&manifest_path, serde_json::to_string(&manifest).unwrap()).unwrap();

    install_from_local(manifest_path.to_str().unwrap(), tmp_dir.path().to_str().unwrap()).unwrap();
    let installed = tmp_dir.path().join("hello-0.1.0/data/hello.txt");
    assert!(installed.exists());
}
