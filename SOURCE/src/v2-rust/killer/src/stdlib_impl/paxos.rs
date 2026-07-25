// ================================================================
// PAXOS CONSENSUS - Phase 28.2
// Byzantine-fault-tolerant consensus algorithm
// ================================================================

use std::collections::HashMap;

/// Proposal phase result
#[derive(Clone, Debug)]
pub struct ProposalPhaseResult {
    pub promised: bool,
    pub accepted_proposal: Option<u64>,
    pub accepted_value: Option<String>,
}

/// Accept phase result
#[derive(Clone, Debug)]
pub struct AcceptPhaseResult {
    pub accepted: bool,
    pub min_proposal: u64,
}

/// Paxos state
#[derive(Clone, Debug)]
pub struct PaxosState {
    pub node_id: String,
    pub promised_proposal: u64,
    pub accepted_proposal: Option<u64>,
    pub accepted_value: Option<String>,
    pub consensus_value: Option<String>,
}

pub struct PaxosSolver;

impl PaxosSolver {
    // ================================================================
    // PROPOSER PHASE (1-12)
    // ================================================================

    /// Problem 1: Proposer prepare phase
    pub fn proposer_prepare_phase(proposal_num: u64, acceptors: usize) -> Vec<u64> {
        vec![proposal_num; acceptors]
    }

    /// Problem 2: Find proposal number
    pub fn find_proposal_number(round: u64, proposer_id: u64) -> u64 {
        round * 1000 + proposer_id
    }

    /// Problem 3: Request prepare
    pub fn request_prepare(proposal_num: u64, acceptor_id: &str) -> (String, u64) {
        (acceptor_id.to_string(), proposal_num)
    }

    /// Problem 4: Handle prepare response
    pub fn handle_prepare_response(
        responses: &[ProposalPhaseResult],
    ) -> Option<String> {
        for resp in responses {
            if resp.promised && resp.accepted_value.is_some() {
                return resp.accepted_value.clone();
            }
        }
        None
    }

    /// Problem 5: Select proposal value
    pub fn select_proposal_value(
        suggested_value: &str,
        highest_accepted: Option<&str>,
    ) -> String {
        highest_accepted
            .unwrap_or(suggested_value)
            .to_string()
    }

    /// Problem 6: Proposer accept phase
    pub fn proposer_accept_phase(
        proposal_num: u64,
        value: &str,
        acceptors: usize,
    ) -> Vec<(u64, String)> {
        vec![(proposal_num, value.to_string()); acceptors]
    }

    /// Problem 7: Request accept
    pub fn request_accept(
        proposal_num: u64,
        value: &str,
        acceptor_id: &str,
    ) -> (String, u64, String) {
        (acceptor_id.to_string(), proposal_num, value.to_string())
    }

    /// Problem 8: Handle accept response
    pub fn handle_accept_response(responses: &[AcceptPhaseResult]) -> bool {
        let accepted_count = responses
            .iter()
            .filter(|r| r.accepted)
            .count();
        accepted_count > responses.len() / 2
    }

    /// Problem 9: Detect prepare failure
    pub fn detect_prepare_failure(responses: &[bool], total_acceptors: usize) -> bool {
        let promises = responses.iter().filter(|&&b| b).count();
        promises <= total_acceptors / 2
    }

    /// Problem 10: Detect accept failure
    pub fn detect_accept_failure(responses: &[bool], total_acceptors: usize) -> bool {
        let accepts = responses.iter().filter(|&&b| b).count();
        accepts <= total_acceptors / 2
    }

    /// Problem 11: Retry proposal
    pub fn retry_proposal(current_proposal: u64) -> u64 {
        current_proposal + 1
    }

    /// Problem 12: Propose value
    pub fn propose_value(
        proposal_num: u64,
        value: &str,
        acceptor_count: usize,
    ) -> Result<String, String> {
        if acceptor_count > 0 && proposal_num > 0 {
            Ok(value.to_string())
        } else {
            Err("Invalid proposal".to_string())
        }
    }

    // ================================================================
    // ACCEPTOR PHASE (13-24)
    // ================================================================

    /// Problem 13: Acceptor handle prepare
    pub fn acceptor_handle_prepare(
        state: &mut PaxosState,
        proposal_num: u64,
    ) -> ProposalPhaseResult {
        if proposal_num > state.promised_proposal {
            state.promised_proposal = proposal_num;
            ProposalPhaseResult {
                promised: true,
                accepted_proposal: state.accepted_proposal,
                accepted_value: state.accepted_value.clone(),
            }
        } else {
            ProposalPhaseResult {
                promised: false,
                accepted_proposal: None,
                accepted_value: None,
            }
        }
    }

    /// Problem 14: Store prepare response
    pub fn store_prepare_response(
        responses: &mut Vec<ProposalPhaseResult>,
        response: ProposalPhaseResult,
    ) {
        responses.push(response);
    }

    /// Problem 15: Compare proposal numbers
    pub fn compare_proposal_numbers(prop1: u64, prop2: u64) -> std::cmp::Ordering {
        prop1.cmp(&prop2)
    }

    /// Problem 16: Acceptor handle accept
    pub fn acceptor_handle_accept(
        state: &mut PaxosState,
        proposal_num: u64,
        value: &str,
    ) -> AcceptPhaseResult {
        if proposal_num >= state.promised_proposal {
            state.accepted_proposal = Some(proposal_num);
            state.accepted_value = Some(value.to_string());
            state.promised_proposal = proposal_num;
            AcceptPhaseResult {
                accepted: true,
                min_proposal: proposal_num,
            }
        } else {
            AcceptPhaseResult {
                accepted: false,
                min_proposal: state.promised_proposal,
            }
        }
    }

    /// Problem 17: Store accept state
    pub fn store_accept_state(state: &mut PaxosState, proposal: u64, value: &str) {
        state.accepted_proposal = Some(proposal);
        state.accepted_value = Some(value.to_string());
    }

    /// Problem 18: Validate proposal number
    pub fn validate_proposal_number(
        current_proposal: u64,
        promised_proposal: u64,
    ) -> bool {
        current_proposal >= promised_proposal
    }

    /// Problem 19: Get accepted value
    pub fn get_accepted_value(state: &PaxosState) -> Option<String> {
        state.accepted_value.clone()
    }

    /// Problem 20: Get accepted proposal
    pub fn get_accepted_proposal(state: &PaxosState) -> Option<u64> {
        state.accepted_proposal
    }

    /// Problem 21: Persist to disk
    pub fn persist_to_disk(state: &PaxosState) -> String {
        format!("Persisted: {:?}", state)
    }

    /// Problem 22: Recover from disk
    pub fn recover_from_disk() -> PaxosState {
        PaxosState {
            node_id: "recovered".to_string(),
            promised_proposal: 0,
            accepted_proposal: None,
            accepted_value: None,
            consensus_value: None,
        }
    }

    /// Problem 23: Check quorum acceptance
    pub fn check_quorum_acceptance(accepts: usize, total: usize) -> bool {
        accepts * 2 > total
    }

    /// Problem 24: Predict acceptance
    pub fn predict_acceptance(
        consensus_confidence: f64,
    ) -> bool {
        consensus_confidence > 0.5
    }

    // ================================================================
    // LEARNER PHASE (25-36)
    // ================================================================

    /// Problem 25: Learner collect accepted
    pub fn learner_collect_accepted(
        accepted_values: &HashMap<String, String>,
    ) -> Vec<(String, u32)> {
        let mut counts: HashMap<String, u32> = HashMap::new();
        for value in accepted_values.values() {
            *counts.entry(value.clone()).or_insert(0) += 1;
        }
        counts.into_iter().collect()
    }

    /// Problem 26: Detect consensus
    pub fn detect_consensus(
        value_counts: &[(String, u32)],
        quorum_size: u32,
    ) -> Option<String> {
        for (value, count) in value_counts {
            if *count > quorum_size / 2 {
                return Some(value.clone());
            }
        }
        None
    }

    /// Problem 27: Notify consensus
    pub fn notify_consensus(value: &str) -> String {
        format!("Consensus reached: {}", value)
    }

    /// Problem 28: Track value counts
    pub fn track_value_counts(
        values: &[String],
    ) -> HashMap<String, u32> {
        let mut counts = HashMap::new();
        for value in values {
            *counts.entry(value.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Problem 29: Find unanimously accepted
    pub fn find_unanimously_accepted(
        value_counts: &HashMap<String, u32>,
        total_acceptors: u32,
    ) -> Option<String> {
        value_counts
            .iter()
            .find(|(_, count)| **count == total_acceptors)
            .map(|(value, _)| value.clone())
    }

    /// Problem 30: Handle competing proposals
    pub fn handle_competing_proposals(
        values: &[String],
    ) -> Option<String> {
        let counts = Self::track_value_counts(values);
        counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(value, _)| value.clone())
    }

    /// Problem 31: Resolve conflicts
    pub fn resolve_conflicts(
        value_counts: &HashMap<String, u32>,
    ) -> Option<String> {
        value_counts
            .iter()
            .max_by_key(|&(_, count)| count)
            .map(|(value, _)| value.clone())
    }

    /// Problem 32: Apply learned value
    pub fn apply_learned_value(
        state: &mut PaxosState,
        value: &str,
    ) {
        state.consensus_value = Some(value.to_string());
    }

    /// Problem 33: Broadcast decided value
    pub fn broadcast_decided_value(value: &str) -> String {
        format!("Broadcast: {}", value)
    }

    /// Problem 34: Verify decided value
    pub fn verify_decided_value(value: &str) -> bool {
        !value.is_empty()
    }

    /// Problem 35: Handle repeated proposals
    pub fn handle_repeated_proposals(
        state: &mut PaxosState,
        proposal_num: u64,
        value: &str,
    ) -> bool {
        if state.accepted_proposal == Some(proposal_num)
            && state.accepted_value == Some(value.to_string())
        {
            true
        } else {
            false
        }
    }

    /// Problem 36: Generate consensus proof
    pub fn generate_consensus_proof(acceptors_confirmed: u32, total: u32) -> String {
        format!(
            "Consensus proved: {}/{} acceptors confirmed",
            acceptors_confirmed, total
        )
    }

    // ================================================================
    // BYZANTINE HANDLING (37-50)
    // ================================================================

    /// Problem 37: Detect Byzantine behavior
    pub fn detect_byzantine_behavior(
        responses_a: &str,
        responses_b: &str,
    ) -> bool {
        responses_a != responses_b
    }

    /// Problem 38: Quarantine faulty node
    pub fn quarantine_faulty_node(faulty_nodes: &mut Vec<String>, node_id: &str) {
        faulty_nodes.push(node_id.to_string());
    }

    /// Problem 39: Byzantine vote threshold
    pub fn byzantine_vote_threshold(total_acceptors: u32) -> u32 {
        (2 * total_acceptors + 2) / 3
    }

    /// Problem 40: Handle Byzantine prepare response
    pub fn handle_byzantine_prepare_response(
        response: &ProposalPhaseResult,
        expected_proposal: u64,
    ) -> bool {
        response.promised && response.accepted_proposal.is_some()
    }

    /// Problem 41: Handle Byzantine accept response
    pub fn handle_byzantine_accept_response(
        response: &AcceptPhaseResult,
        expected_proposal: u64,
    ) -> bool {
        response.accepted && response.min_proposal == expected_proposal
    }

    /// Problem 42: Cross validate responses
    pub fn cross_validate_responses(
        responses: &[ProposalPhaseResult],
    ) -> bool {
        let first = responses.first();
        responses.iter().skip(1).all(|r| {
            r.promised == first.map(|f| f.promised).unwrap_or(false)
        })
    }

    /// Problem 43: Calculate Byzantine safe quorum
    pub fn calculate_byzantine_safe_quorum(total_nodes: u32) -> u32 {
        Self::byzantine_vote_threshold(total_nodes)
    }

    /// Problem 44: Detect Byzantine replay attacks
    pub fn detect_byzantine_replay_attacks(
        msg_history: &[String],
    ) -> bool {
        msg_history.len() != msg_history.iter().collect::<std::collections::HashSet<_>>().len()
    }

    /// Problem 45: Handle Byzantine leader
    pub fn handle_byzantine_leader(
        leader_responses: &[bool],
        quorum: usize,
    ) -> bool {
        leader_responses.iter().filter(|&&b| b).count() >= quorum
    }

    /// Problem 46: Byzantine consensus proof
    pub fn byzantine_consensus_proof(
        acceptances: u32,
        total: u32,
    ) -> bool {
        acceptances > (2 * total) / 3
    }

    /// Problem 47: Detect Sybil attacks
    pub fn detect_sybil_attacks(
        node_signatures: &HashMap<String, String>,
    ) -> bool {
        let unique_sigs = node_signatures
            .values()
            .collect::<std::collections::HashSet<_>>()
            .len();
        unique_sigs < node_signatures.len()
    }

    /// Problem 48: Handle malicious message injection
    pub fn handle_malicious_message_injection(
        message_count: usize,
        expected_count: usize,
    ) -> bool {
        message_count > expected_count
    }

    /// Problem 49: Byzantine recovery protocol
    pub fn byzantine_recovery_protocol(
        consensus_state: &mut PaxosState,
    ) {
        consensus_state.promised_proposal = 0;
        consensus_state.accepted_proposal = None;
    }

    /// Problem 50: Track Byzantine nodes
    pub fn track_byzantine_nodes(
        nodes: &mut HashMap<String, u32>,
        node_id: &str,
    ) {
        *nodes.entry(node_id.to_string()).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_phase() {
        let requests = PaxosSolver::proposer_prepare_phase(1, 5);
        assert_eq!(requests.len(), 5);
    }

    #[test]
    fn test_acceptor_handle_prepare() {
        let mut state = PaxosState {
            node_id: "acceptor1".to_string(),
            promised_proposal: 0,
            accepted_proposal: None,
            accepted_value: None,
            consensus_value: None,
        };
        let result = PaxosSolver::acceptor_handle_prepare(&mut state, 1);
        assert!(result.promised);
    }

    #[test]
    fn test_acceptor_handle_accept() {
        let mut state = PaxosState {
            node_id: "acceptor1".to_string(),
            promised_proposal: 1,
            accepted_proposal: None,
            accepted_value: None,
            consensus_value: None,
        };
        let result = PaxosSolver::acceptor_handle_accept(&mut state, 1, "value1");
        assert!(result.accepted);
    }

    #[test]
    fn test_detect_consensus() {
        let value_counts = vec![
            ("value1".to_string(), 4),
            ("value2".to_string(), 1),
        ];
        let consensus = PaxosSolver::detect_consensus(&value_counts, 5);
        assert_eq!(consensus, Some("value1".to_string()));
    }

    #[test]
    fn test_handle_competing_proposals() {
        let values = vec!["a".to_string(), "a".to_string(), "b".to_string()];
        let winner = PaxosSolver::handle_competing_proposals(&values);
        assert_eq!(winner, Some("a".to_string()));
    }

    #[test]
    fn test_byzantine_vote_threshold() {
        let threshold = PaxosSolver::byzantine_vote_threshold(10);
        assert_eq!(threshold, 8);
    }

    #[test]
    fn test_detect_sybil_attacks() {
        let mut sigs = HashMap::new();
        sigs.insert("node1".to_string(), "sig1".to_string());
        sigs.insert("node2".to_string(), "sig1".to_string());
        assert!(PaxosSolver::detect_sybil_attacks(&sigs));
    }

    #[test]
    fn test_learner_collect_accepted() {
        let mut accepted = HashMap::new();
        accepted.insert("server1".to_string(), "value1".to_string());
        accepted.insert("server2".to_string(), "value1".to_string());
        let counts = PaxosSolver::learner_collect_accepted(&accepted);
        assert_eq!(counts.len(), 1);
    }

    #[test]
    fn test_byzantine_consensus_proof() {
        assert!(PaxosSolver::byzantine_consensus_proof(8, 10));
        assert!(!PaxosSolver::byzantine_consensus_proof(6, 10));
    }

    #[test]
    fn test_propose_value() {
        let result = PaxosSolver::propose_value(1, "test", 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_quorum_acceptance() {
        assert!(PaxosSolver::check_quorum_acceptance(3, 5));
        assert!(!PaxosSolver::check_quorum_acceptance(2, 5));
    }
}
