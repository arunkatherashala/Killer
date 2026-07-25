Killer: KPM (Package Manager) & Phase F (GPU/SIMD) Roadmap

This document explains the initial scaffold created and next steps.

Created:
- `tools/kpm/` - minimal Rust CLI stub for package manager (kpm).
- `src/gpu/mod.rs` - GPU/SIMD integration stub with CPU fallback helpers.

Next steps (recommended order):
1. Implement `kpm` core features:
   - Registry design (central vs distributed), auth, package signing, and `quality` metadata verification.
   - Install/uninstall, dependency resolution, and sandboxed package execution.
   - Create `kpm` unit tests and integrate into CI.

2. Phase F GPU work:
   - Define `TritTensor` representation and memory layout.
   - Implement CPU SIMD intrinsics fallback (Rust `packed_simd` or `std::simd`).
   - Add GPU backends (CUDA via `rust-cuda` or OpenCL via `ocl`), with feature flags.
   - Integrate with VM as new opcodes (e.g., `GPUTritMatMul`).

3. Developer Experience:
   - Add LSP integration and debugging features.
   - Document KPM usage and Phase F design RFC.

How to run kpm stub locally:

```bash
cd tools/kpm
cargo run -- install example-package
```

