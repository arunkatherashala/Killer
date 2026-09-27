//! State Machines — Finite state machine framework

pub trait State: Send + Sync {
    fn enter(&self);
    fn exit(&self);
}

pub struct StateMachine {
    _current: String,
}

impl StateMachine {
    pub fn new() -> Self { Self { _current: "initial".to_string() } }
}
