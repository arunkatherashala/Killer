// killer_rcore/src/optimizer/mod.rs
// Module exports for optimizer

pub mod loop_detector;
pub mod loop_analysis;

pub use loop_detector::{LoopDetector, LoopProfile, ExitCondition, Bound};
