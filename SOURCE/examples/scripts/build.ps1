Write-Host "Building Killer Super Release..."
cargo clean
cargo build --release --bin killer_super
Write-Host "Release build complete."
