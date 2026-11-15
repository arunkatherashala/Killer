//! Raft Consensus — Distributed consensus algorithm

#[derive(Debug, Clone)]
pub enum RaftState { Follower, Candidate, Leader }

pub struct RaftNode {
    pub state: RaftState,
    pub term: u64,
}

impl RaftNode {
    pub fn new() -> Self { Self { state: RaftState::Follower, term: 0 } }
}
