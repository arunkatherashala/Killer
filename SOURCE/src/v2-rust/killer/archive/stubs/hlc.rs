//! Hybrid Logical Clock — Vector clock for causal ordering

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HLC { pub logical: u64, pub counter: u64 }

impl HLC {
    pub fn new() -> Self { Self { logical: 0, counter: 0 } }
    pub fn tick(&mut self) { self.counter += 1; }
}
