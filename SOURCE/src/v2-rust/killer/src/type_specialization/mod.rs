// Phase 3: Type Specialization Module
// Enables generation of specialized code paths for specific types
// Eliminates polymorphism overhead through type-driven code generation

pub mod type_inference;
pub mod code_generator;
pub mod dispatch_optimizer;

pub use type_inference::{TypeInferenceEngine, InferredType, TypeAssumption, TypeInferenceStats};
pub use code_generator::{
    TypeSpecializedCodeGenerator, SpecializedCodePath, CodeGenerationConfig, CodeGenerationStats,
};
pub use dispatch_optimizer::{
    DispatchOptimizer, DispatchStrategy, DispatchResult, DispatchStatistics,
};
