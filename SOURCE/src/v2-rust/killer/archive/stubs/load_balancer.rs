//! Load Balancer — Distribute requests across backends

pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastConnections,
    Random,
}

pub struct LoadBalancer {
    pub strategy: LoadBalancingStrategy,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalancingStrategy) -> Self { Self { strategy } }
}
