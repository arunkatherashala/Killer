// JIT compilation module
// Week 3: Runtime compilation and caching infrastructure

pub mod cache;
pub mod signature;
pub mod compiler;
pub mod loader;

// Re-export main types
pub use cache::JITCache;
pub use signature::LoopSignature;
pub use compiler::{RustCompiler, CompileResult};
pub use loader::{JITLoader, LoadError, JITLoopFn, JITLoopWithParamFn};
