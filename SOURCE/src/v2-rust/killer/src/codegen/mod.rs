// killer_rcore/src/codegen/mod.rs
// Code generation module for JIT compilation
// Week 2 implementation

pub mod rust_codegen;
pub mod type_inference;
pub mod expr_converter;

pub use rust_codegen::{RustCodegen, GeneratedCode};
pub use type_inference::{TypeInference, InferredTypes};
pub use expr_converter::ExprConverter;
