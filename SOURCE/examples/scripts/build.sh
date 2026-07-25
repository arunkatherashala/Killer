#!/bin/bash
# Killer Super Release Build Script

echo "Building Killer Super v4.0.0..."
echo ""

cd killer_rcore
cargo build --release --bin killer_super

if [ $? -eq 0 ]; then
    echo ""
    echo "Build successful!"
    mkdir -p ../release_builds
    cp target/release/killer_super ../release_builds/
    echo "Binary: ../release_builds/killer_super"
    
    # Run tests
    echo ""
    echo "Running tests..."
    cargo test --lib 2>&1 | grep "test result:"
else
    echo "Build failed!"
    exit 1
fi
