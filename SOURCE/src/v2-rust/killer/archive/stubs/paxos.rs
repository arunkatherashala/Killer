//! Paxos Consensus — Distributed consensus algorithm (alternative to Raft)

pub enum PaxosRole { Proposer, Acceptor, Learner }

pub struct PaxosNode { pub role: PaxosRole }

impl PaxosNode {
    pub fn new(role: PaxosRole) -> Self { Self { role } }
}
