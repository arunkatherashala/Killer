// Phase 4: LLVM Backend Integration Module
// Real LLVM compiler integration for 5-10x speedup

pub mod integration;
pub mod target_arch;

pub use integration::{LLVMBackend, LLVMBackendConfig, CompiledModule, LLVMBackendStats};
pub use target_arch::{TargetArchManager, TargetArch, TargetArchSupport};
