# Unsafe audit — killer-native

Crate policy: `#![deny(unsafe_code)]` in `lib.rs`, with **file-level** `#![allow(unsafe_code)]` only where required.

| Module | Why unsafe is used |
|--------|---------------------|
| `jit_x86.rs` | Executable pages (`VirtualAlloc` / `mmap`), patching machine code, calling JIT function pointers. |
| `vm.rs` | `unsafe impl Send/Sync` for VM; calling compiled JIT loop functions. |
| `value.rs` | `unsafe impl Send/Sync` for `Value` / `FutureHandle` (shared with async & pools). |
| `version.rs` | Low-level probe (platform-specific). |
| `memory_optimization.rs` | Global allocator hooks / raw allocation experiments. |
| `ffi.rs` / `ffi_dynamic.rs` | `libloading` and raw C interop. |

**Follow-ups:** narrow `Send`/`Sync` impls with proper `Arc`/channels where possible; document JIT W^X lifecycle in `jit_x86.rs` module docs; add `#[cfg(debug_assertions)]` assertions on JIT page permissions.
