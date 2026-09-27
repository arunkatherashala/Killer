//! Millennium Prize Problems — 20+ functions (P vs NP, Riemann, Navier-Stokes, Yang-Mills)

pub enum MillenniumProblem {
    PVsNP,
    RiemannHypothesis,
    NavierStokes,
    YangMills,
}

impl MillenniumProblem {
    pub fn name(&self) -> &str {
        match self {
            Self::PVsNP => "P vs NP",
            Self::RiemannHypothesis => "Riemann Hypothesis",
            Self::NavierStokes => "Navier-Stokes Existence",
            Self::YangMills => "Yang-Mills Existence",
        }
    }
}
