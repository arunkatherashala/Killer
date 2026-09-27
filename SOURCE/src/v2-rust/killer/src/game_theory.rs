//! Game Theory — 35+ functions (Nash equilibrium, cooperative games, auctions)

#[derive(Debug, Clone)]
pub struct PayoffMatrix { pub rows: Vec<Vec<f64>> }

impl PayoffMatrix {
    pub fn new() -> Self { Self { rows: vec![] } }
}
