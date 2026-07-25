// ================================================================
// RAFT CONSENSUS - Phase 28.1
// Distributed consensus algorithm for leader election and log replication
// ================================================================

use std::collections::HashMap;

/// Raft node state
#[derive(Clone, Debug, PartialEq)]
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

/// Raft server status
#[derive(Clone, Debug)]
pub struct RaftServer {
    pub id: String,
    pub state: RaftState,
    pub current_term: u64,
    pub voted_for: Option<String>,
    pub log_entries: Vec<(u64, String)>,
    pub commit_index: u64,
    pub last_applied: u64,
    pub next_index: HashMap<String, u64>,
    pub match_index: HashMap<String, u64>,
    pub election_timer: u64,
    pub last_heartbeat: u64,
}

/// Log entry
#[derive(Clone, Debug)]
pub struct LogEntry {
    pub term: u64,
    pub index: u64,
    pub command: String,
}

pub struct RaftSolver;

impl RaftSolver {
    // ================================================================
    // RAFT RULES (1-12)
    // ================================================================

    /// Problem 1: Create Raft state
    pub fn create_raft_state(server_id: &str) -> RaftServer {
        RaftServer {
            id: server_id.to_string(),
            state: RaftState::Follower,
            current_term: 0,
            voted_for: None,
            log_entries: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            next_index: HashMap::new(),
            match_index: HashMap::new(),
            election_timer: 150,
            last_heartbeat: 0,
        }
    }

    /// Problem 2: Reset election timer
    pub fn reset_election_timer(server: &mut RaftServer) {
        // Use time-based pseudo-randomness instead of rand crate
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        server.election_timer = 150 + (seed ^ (server.id.len() as u64)) % 150;
    }

    /// Problem 3: Increment term
    pub fn increment_term(server: &mut RaftServer) {
        server.current_term += 1;
        server.voted_for = None;
    }

    /// Problem 4: Request vote
    pub fn request_vote(
        server: &mut RaftServer,
        term: u64,
        candidate_id: &str,
        last_log_index: u64,
        last_log_term: u64,
    ) -> bool {
        if term < server.current_term {
            return false;
        }

        if term > server.current_term {
            server.current_term = term;
            server.voted_for = None;
        }

        let last_entry_term = if server.log_entries.is_empty() {
            0
        } else {
            server.log_entries[server.log_entries.len() - 1].0
        };
        let last_entry_index = server.log_entries.len() as u64;

        if server.voted_for.is_none()
            && last_log_term >= last_entry_term
            && last_log_index >= last_entry_index
        {
            server.voted_for = Some(candidate_id.to_string());
            true
        } else {
            false
        }
    }

    /// Problem 5: Append entries
    pub fn append_entries(
        server: &mut RaftServer,
        term: u64,
        leader_id: &str,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: &[(u64, String)],
        leader_commit: u64,
    ) -> bool {
        if term < server.current_term {
            return false;
        }

        if term > server.current_term {
            server.current_term = term;
            server.state = RaftState::Follower;
            server.voted_for = None;
        }

        // Log matching check
        if prev_log_index > 0 && prev_log_index as usize <= server.log_entries.len() {
            if server.log_entries[(prev_log_index - 1) as usize].0 != prev_log_term {
                return false;
            }
        }

        // Append entries not already in log
        for (i, entry) in entries.iter().enumerate() {
            let index = prev_log_index + 1 + i as u64;
            if index as usize > server.log_entries.len() {
                server.log_entries.push(entry.clone());
            }
        }

        // Advance commit index
        if leader_commit > server.commit_index {
            server.commit_index = std::cmp::min(leader_commit, server.log_entries.len() as u64);
        }

        true
    }

    /// Problem 6: Become leader
    pub fn become_leader(server: &mut RaftServer, peers: &[String]) {
        server.state = RaftState::Leader;
        for peer in peers {
            if peer != &server.id {
                server.next_index.insert(peer.clone(), server.log_entries.len() as u64 + 1);
                server.match_index.insert(peer.clone(), 0);
            }
        }
    }

    /// Problem 7: Become follower
    pub fn become_follower(server: &mut RaftServer, term: u64) {
        server.state = RaftState::Follower;
        server.current_term = term;
        server.voted_for = None;
    }

    /// Problem 8: Become candidate
    pub fn become_candidate(server: &mut RaftServer) {
        server.state = RaftState::Candidate;
        Self::increment_term(server);
        server.voted_for = Some(server.id.clone());
        Self::reset_election_timer(server);
    }

    /// Problem 9: Set commit index
    pub fn set_commit_index(server: &mut RaftServer, index: u64) {
        if index > server.commit_index && index <= server.log_entries.len() as u64 {
            server.commit_index = index;
        }
    }

    /// Problem 10: Apply log entry
    pub fn apply_log_entry(server: &mut RaftServer) -> Option<String> {
        if server.last_applied < server.commit_index
            && server.last_applied < server.log_entries.len() as u64
        {
            let entry = &server.log_entries[server.last_applied as usize];
            server.last_applied += 1;
            Some(entry.1.clone())
        } else {
            None
        }
    }

    /// Problem 11: Check safety invariants
    pub fn check_safety_invariants(server: &RaftServer) -> bool {
        server.commit_index <= server.log_entries.len() as u64
            && server.last_applied <= server.commit_index
    }

    /// Problem 12: Validate log term
    pub fn validate_log_term(server: &RaftServer, index: u64, term: u64) -> bool {
        if index == 0 {
            return term == 0;
        }
        if index as usize > server.log_entries.len() {
            return false;
        }
        server.log_entries[(index - 1) as usize].0 == term
    }

    // ================================================================
    // LOG MANAGEMENT (13-22)
    // ================================================================

    /// Problem 13: Append to log
    pub fn append_to_log(server: &mut RaftServer, command: &str) -> u64 {
        server.log_entries.push((server.current_term, command.to_string()));
        server.log_entries.len() as u64
    }

    /// Problem 14: Get log entry
    pub fn get_log_entry(server: &RaftServer, index: u64) -> Option<(u64, String)> {
        if index == 0 || index as usize > server.log_entries.len() {
            None
        } else {
            Some(server.log_entries[(index - 1) as usize].clone())
        }
    }

    /// Problem 15: Get last log index
    pub fn get_last_log_index(server: &RaftServer) -> u64 {
        server.log_entries.len() as u64
    }

    /// Problem 16: Get last log term
    pub fn get_last_log_term(server: &RaftServer) -> u64 {
        if server.log_entries.is_empty() {
            0
        } else {
            server.log_entries[server.log_entries.len() - 1].0
        }
    }

    /// Problem 17: Advance commit index
    pub fn advance_commit_index(
        server: &mut RaftServer,
        match_indices: &HashMap<String, u64>,
    ) {
        let mut indices: Vec<u64> = match_indices.values().copied().collect();
        indices.sort();
        let median = indices[indices.len() / 2];
        if median > server.commit_index {
            server.commit_index = median;
        }
    }

    /// Problem 18: Replay log from index
    pub fn replay_log_from_index(server: &RaftServer, start_index: u64) -> Vec<(u64, String)> {
        if start_index == 0 {
            server.log_entries.clone()
        } else {
            server
                .log_entries
                .iter()
                .skip((start_index - 1) as usize)
                .cloned()
                .collect()
        }
    }

    /// Problem 19: Compact log
    pub fn compact_log(server: &mut RaftServer, up_to_index: u64) {
        if up_to_index > 0 && up_to_index < server.log_entries.len() as u64 {
            server.log_entries = server
                .log_entries
                .iter()
                .skip(up_to_index as usize)
                .cloned()
                .collect();
        }
    }

    /// Problem 20: Truncate log
    pub fn truncate_log(server: &mut RaftServer, at_index: u64) {
        if at_index < server.log_entries.len() as u64 {
            server.log_entries.truncate(at_index as usize);
        }
    }

    /// Problem 21: Log matches term
    pub fn log_matches_term(server: &RaftServer, index: u64, term: u64) -> bool {
        Self::validate_log_term(server, index, term)
    }

    /// Problem 22: Get log consistency point
    pub fn get_log_consistency_point(server1: &RaftServer, server2: &RaftServer) -> u64 {
        let mut i = std::cmp::min(server1.log_entries.len(), server2.log_entries.len());
        while i > 0 {
            if server1.log_entries[i - 1].0 == server2.log_entries[i - 1].0 {
                break;
            }
            i -= 1;
        }
        i as u64
    }

    // ================================================================
    // ELECTION MANAGEMENT (23-34)
    // ================================================================

    /// Problem 23: Start election
    pub fn start_election(server: &mut RaftServer) {
        Self::become_candidate(server);
    }

    /// Problem 24: Handle election timeout
    pub fn handle_election_timeout(server: &mut RaftServer, now: u64) -> bool {
        if server.state == RaftState::Leader {
            false
        } else if now - server.last_heartbeat > server.election_timer {
            Self::start_election(server);
            true
        } else {
            false
        }
    }

    /// Problem 25: Request votes from peers
    pub fn request_votes_from_peers(server: &RaftServer, peers: &[String]) -> Vec<String> {
        peers
            .iter()
            .filter(|p| p != &&server.id)
            .cloned()
            .collect()
    }

    /// Problem 26: Count granted votes
    pub fn count_granted_votes(votes: &[bool]) -> usize {
        votes.iter().filter(|&v| *v).count()
    }

    /// Problem 27: Win election
    pub fn win_election(server: &mut RaftServer, peers: &[String]) -> bool {
        let majority = peers.len() / 2 + 1;
        if Self::count_granted_votes(&vec![true; majority]) >= majority {
            Self::become_leader(server, peers);
            true
        } else {
            false
        }
    }

    /// Problem 28: Reset peer indices
    pub fn reset_peer_indices(server: &mut RaftServer, peers: &[String]) {
        for peer in peers {
            if peer != &server.id {
                server.next_index.insert(peer.clone(), server.log_entries.len() as u64 + 1);
                server.match_index.insert(peer.clone(), 0);
            }
        }
    }

    /// Problem 29: Convert to follower
    pub fn convert_to_follower(server: &mut RaftServer, term: u64) {
        Self::become_follower(server, term);
    }

    /// Problem 30: Handle stale vote request
    pub fn handle_stale_vote_request(server: &RaftServer, term: u64) -> bool {
        term >= server.current_term
    }

    /// Problem 31: Handle stale append entries
    pub fn handle_stale_append_entries(server: &RaftServer, term: u64) -> bool {
        term >= server.current_term
    }

    /// Problem 32: Extend election timeout
    pub fn extend_election_timeout(server: &mut RaftServer) {
        server.last_heartbeat = 0;
    }

    /// Problem 33: Detect election deadlock
    pub fn detect_election_deadlock(votes_received: usize, total_peers: usize) -> bool {
        votes_received == total_peers / 2
    }

    /// Problem 34: Accelerate election recovery
    pub fn accelerate_election_recovery(server: &mut RaftServer) {
        server.election_timer = 150;
    }

    // ================================================================
    // REPLICATION (35-46)
    // ================================================================

    /// Problem 35: Replicate log entries
    pub fn replicate_log_entries(
        server: &mut RaftServer,
        peers: &[String],
    ) -> HashMap<String, Vec<(u64, String)>> {
        let mut replication = HashMap::new();
        for peer in peers {
            if peer != &server.id {
                let next_idx = server.next_index.get(peer).copied().unwrap_or(1);
                let entries = Self::replay_log_from_index(server, next_idx);
                replication.insert(peer.clone(), entries);
            }
        }
        replication
    }

    /// Problem 36: Send heartbeat
    pub fn send_heartbeat(server: &RaftServer, peers: &[String]) -> Vec<String> {
        peers
            .iter()
            .filter(|p| p != &&server.id)
            .cloned()
            .collect()
    }

    /// Problem 37: Handle replication failure
    pub fn handle_replication_failure(server: &mut RaftServer, peer: &str) {
        if let Some(next_idx) = server.next_index.get_mut(peer) {
            if *next_idx > 1 {
                *next_idx -= 1;
            }
        }
    }

    /// Problem 38: Update match index
    pub fn update_match_index(server: &mut RaftServer, peer: &str, index: u64) {
        server.match_index.insert(peer.to_string(), index);
    }

    /// Problem 39: Update next index
    pub fn update_next_index(server: &mut RaftServer, peer: &str, index: u64) {
        server.next_index.insert(peer.to_string(), index);
    }

    /// Problem 40: Advance replicas safely
    pub fn advance_replicas_safely(server: &mut RaftServer) {
        if server.state == RaftState::Leader {
            Self::advance_commit_index(server, &server.match_index.clone());
        }
    }

    /// Problem 41: Detect slow follower
    pub fn detect_slow_follower(
        server: &RaftServer,
        peer: &str,
        leader_log_len: u64,
    ) -> bool {
        let match_idx = server.match_index.get(peer).copied().unwrap_or(0);
        match_idx < leader_log_len - 10
    }

    /// Problem 42: Catch up slow follower
    pub fn catch_up_slow_follower(server: &mut RaftServer, peer: &str) {
        if let Some(next_idx) = server.next_index.get_mut(peer) {
            *next_idx = std::cmp::min(*next_idx + 1, server.log_entries.len() as u64 + 1);
        }
    }

    /// Problem 43: Validate replication quorum
    pub fn validate_replication_quorum(
        replicas: &HashMap<String, u64>,
        total_peers: usize,
    ) -> bool {
        replicas.len() >= total_peers / 2 + 1
    }

    /// Problem 44: Calculate median index
    pub fn calculate_median_index(indices: &HashMap<String, u64>) -> u64 {
        let mut vals: Vec<u64> = indices.values().copied().collect();
        vals.sort();
        if vals.is_empty() {
            0
        } else {
            vals[vals.len() / 2]
        }
    }

    /// Problem 45: Handle conflicting entries
    pub fn handle_conflicting_entries(
        server: &mut RaftServer,
        prev_log_index: u64,
        prev_log_term: u64,
    ) -> bool {
        if prev_log_index == 0 {
            return true;
        }
        if prev_log_index as usize > server.log_entries.len() {
            return false;
        }
        server.log_entries[(prev_log_index - 1) as usize].0 == prev_log_term
    }

    /// Problem 46: Apply committed entries
    pub fn apply_committed_entries(server: &mut RaftServer) -> Vec<String> {
        let mut applied = Vec::new();
        while server.last_applied < server.commit_index {
            if let Some(cmd) = Self::apply_log_entry(server) {
                applied.push(cmd);
            }
        }
        applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_raft_state() {
        let server = RaftSolver::create_raft_state("server1");
        assert_eq!(server.state, RaftState::Follower);
        assert_eq!(server.current_term, 0);
    }

    #[test]
    fn test_become_candidate() {
        let mut server = RaftSolver::create_raft_state("server1");
        RaftSolver::become_candidate(&mut server);
        assert_eq!(server.state, RaftState::Candidate);
        assert_eq!(server.current_term, 1);
    }

    #[test]
    fn test_append_to_log() {
        let mut server = RaftSolver::create_raft_state("server1");
        let idx = RaftSolver::append_to_log(&mut server, "cmd1");
        assert_eq!(idx, 1);
        assert_eq!(RaftSolver::get_last_log_index(&server), 1);
    }

    #[test]
    fn test_request_vote() {
        let mut server = RaftSolver::create_raft_state("server1");
        let granted = RaftSolver::request_vote(&mut server, 1, "candidate", 0, 0);
        assert!(granted);
        assert_eq!(server.voted_for, Some("candidate".to_string()));
    }

    #[test]
    fn test_election_timeout() {
        let mut server = RaftSolver::create_raft_state("server1");
        let timeout = RaftSolver::handle_election_timeout(&mut server, 1000);
        assert!(timeout);
        assert_eq!(server.state, RaftState::Candidate);
    }

    #[test]
    fn test_append_entries() {
        let mut server = RaftSolver::create_raft_state("server1");
        let result = RaftSolver::append_entries(&mut server, 1, "leader", 0, 0, &[(1, "cmd".to_string())], 0);
        assert!(result);
    }

    #[test]
    fn test_become_leader() {
        let mut server = RaftSolver::create_raft_state("server1");
        let peers = vec!["server1".to_string(), "server2".to_string(), "server3".to_string()];
        RaftSolver::become_leader(&mut server, &peers);
        assert_eq!(server.state, RaftState::Leader);
    }

    #[test]
    fn test_log_consistency_point() {
        let mut server1 = RaftSolver::create_raft_state("server1");
        let mut server2 = RaftSolver::create_raft_state("server2");
        RaftSolver::append_to_log(&mut server1, "cmd1");
        RaftSolver::append_to_log(&mut server2, "cmd1");
        let point = RaftSolver::get_log_consistency_point(&server1, &server2);
        assert_eq!(point, 1);
    }

    #[test]
    fn test_safety_invariants() {
        let server = RaftSolver::create_raft_state("server1");
        assert!(RaftSolver::check_safety_invariants(&server));
    }

    #[test]
    fn test_apply_log_entry() {
        let mut server = RaftSolver::create_raft_state("server1");
        RaftSolver::append_to_log(&mut server, "cmd1");
        server.commit_index = 1;
        let entry = RaftSolver::apply_log_entry(&mut server);
        assert_eq!(entry, Some("cmd1".to_string()));
    }
}
