// Custom Optimization Strategy Framework
// Plugin system for composing and implementing custom optimizations

pub mod strategist;
pub mod examples;

pub use strategist::{CustomOptimizationStrategist, OptimizationStrategy, OptimizationTask};
pub use examples::{
    MemoryOptimizationStrategy, ConcurrencyOptimizationStrategy, CacheOptimizationStrategy,
    SIMDVectorizationStrategy, CompositeOptimizationBuilder,  ConcurrencyModel, VectorWidth,
};
