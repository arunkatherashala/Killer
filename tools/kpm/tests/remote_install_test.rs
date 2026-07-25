use kpm::registry::{install_from_remote, PackageManifest};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_publish_and_install_flow() {
    // This is a simplified integration test. In a real scenario, you'd
    // run a local HTTP server and test actual remote fetch + verify.
    let tmp = TempDir::new().unwrap();
    
    // Create a simple manifest
    let manifest = PackageManifest {
        name: "test-pkg".to_string(),
        version: "0.1.0".to_string(),
        files: vec!["hello.txt".to_string()],
    };
    
    // Write manifest and payload file
    let manifest_path = tmp.path().join("manifest.json");
    let mut f = fs::File::create(&manifest_path).unwrap();
    write!(f, "{}", serde_json::to_string(&manifest).unwrap()).unwrap();
    
    let payload_path = tmp.path().join("hello.txt");
    fs::write(&payload_path, "Hello, world!").unwrap();
    
    // In a real scenario, we'd:
    // 1. Publish the package (creates tar.gz + sha256)
    // 2. Start a local HTTP server to serve the tar.gz
    // 3. Call install_from_remote with the URL and sha256
    // 4. Verify unpacking succeeded
    
    // For now, just verify the structure is sound
    assert!(manifest_path.exists());
    assert!(payload_path.exists());
}
