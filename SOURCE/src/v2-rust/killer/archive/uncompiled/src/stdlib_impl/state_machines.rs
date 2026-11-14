// ================================================================
// CONSENSUS-BASED STATE MACHINES - Phase 28.5
// Replicated deterministic state with snapshots
// ================================================================

use std::collections::HashMap;

/// Command to apply to state machine
#[derive(Clone, Debug)]
pub struct Command {
    pub index: u64,
    pub term: u64,
    pub data: String,
}

/// State machine state
#[derive(Clone, Debug)]
pub struct StateMachineState {
    pub state_id: String,
    pub current_value: String,
    pub last_applied: u64,
    pub last_term: u64,
}

/// Snapshot information
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub snapshot_id: String,
    pub version: u64,
    pub state: StateMachineState,
    pub created_at: u64,
}

pub struct ConsensusStateMachineSolver;

impl ConsensusStateMachineSolver {
    // ================================================================
    // STATE MACHINE BASICS (1-12)
    // ================================================================

    /// Problem 1: Create state machine
    pub fn create_state_machine(state_id: &str) -> StateMachineState {
        StateMachineState {
            state_id: state_id.to_string(),
            current_value: "".to_string(),
            last_applied: 0,
            last_term: 0,
        }
    }

    /// Problem 2: Get state value
    pub fn get_state_value(state: &StateMachineState) -> String {
        state.current_value.clone()
    }

    /// Problem 3: Update state value
    pub fn update_state_value(state: &mut StateMachineState, value: &str) {
        state.current_value = value.to_string();
    }

    /// Problem 4: Get last applied index
    pub fn get_last_applied_index(state: &StateMachineState) -> u64 {
        state.last_applied
    }

    /// Problem 5: Set last applied index
    pub fn set_last_applied_index(state: &mut StateMachineState, index: u64) {
        state.last_applied = index;
    }

    /// Problem 6: Validate state consistency
    pub fn validate_state_consistency(state: &StateMachineState) -> bool {
        !state.state_id.is_empty() && state.last_applied > 0
    }

    /// Problem 7: Reset state machine
    pub fn reset_state_machine(state: &mut StateMachineState) {
        state.current_value = "".to_string();
        state.last_applied = 0;
        state.last_term = 0;
    }

    /// Problem 8: Get state metadata
    pub fn get_state_metadata(state: &StateMachineState) -> (u64, u64) {
        (state.last_applied, state.last_term)
    }

    /// Problem 9: Clone state
    pub fn clone_state(state: &StateMachineState) -> StateMachineState {
        state.clone()
    }

    /// Problem 10: Compare states
    pub fn compare_states(state1: &StateMachineState, state2: &StateMachineState) -> bool {
        state1.current_value == state2.current_value
            && state1.last_applied == state2.last_applied
            && state1.last_term == state2.last_term
    }

    /// Problem 11: Serialize state
    pub fn serialize_state(state: &StateMachineState) -> String {
        format!(
            "{}|{}|{}|{}",
            state.state_id, state.current_value, state.last_applied, state.last_term
        )
    }

    /// Problem 12: Deserialize state
    pub fn deserialize_state(s: &str) -> Option<StateMachineState> {
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() == 4 {
            Some(StateMachineState {
                state_id: parts[0].to_string(),
                current_value: parts[1].to_string(),
                last_applied: parts[2].parse().ok()?,
                last_term: parts[3].parse().ok()?,
            })
        } else {
            None
        }
    }

    // ================================================================
    // COMMAND LOGS (13-24)
    // ================================================================

    /// Problem 13: Create command
    pub fn create_command(index: u64, term: u64, data: &str) -> Command {
        Command {
            index,
            term,
            data: data.to_string(),
        }
    }

    /// Problem 14: Apply command
    pub fn apply_command(
        state: &mut StateMachineState,
        cmd: &Command,
    ) -> Result<(), String> {
        if cmd.index > state.last_applied {
            state.current_value = cmd.data.clone();
            state.last_applied = cmd.index;
            state.last_term = cmd.term;
            Ok(())
        } else {
            Err("Command out of order".to_string())
        }
    }

    /// Problem 15: Apply commands in batch
    pub fn apply_commands_in_batch(
        state: &mut StateMachineState,
        commands: &[Command],
    ) -> Result<(), String> {
        for cmd in commands {
            Self::apply_command(state, cmd)?;
        }
        Ok(())
    }

    /// Problem 16: Get command history
    pub fn get_command_history(
        commands: &[Command],
        start_index: u64,
    ) -> Vec<Command> {
        commands
            .iter()
            .filter(|c| c.index >= start_index)
            .cloned()
            .collect()
    }

    /// Problem 17: Validate command ordering
    pub fn validate_command_ordering(commands: &[Command]) -> bool {
        for i in 1..commands.len() {
            if commands[i].index <= commands[i - 1].index {
                return false;
            }
        }
        true
    }

    /// Problem 18: Store command
    pub fn store_command(
        log: &mut Vec<Command>,
        cmd: Command,
    ) {
        log.push(cmd);
    }

    /// Problem 19: Retrieve command
    pub fn retrieve_command(log: &[Command], index: u64) -> Option<Command> {
        log.iter().find(|c| c.index == index).cloned()
    }

    /// Problem 20: Truncate log from index
    pub fn truncate_log_from_index(log: &mut Vec<Command>, start_index: u64) {
        log.retain(|c| c.index < start_index);
    }

    /// Problem 21: Compact log
    pub fn compact_log(log: &mut Vec<Command>, keep_indexes: &[u64]) {
        log.retain(|c| keep_indexes.contains(&c.index));
    }

    /// Problem 22: Get log length
    pub fn get_log_length(log: &[Command]) -> u64 {
        log.len() as u64
    }

    /// Problem 23: Verify log consistency
    pub fn verify_log_consistency(log: &[Command]) -> bool {
        Self::validate_command_ordering(log)
    }

    /// Problem 24: Replay log to state
    pub fn replay_log_to_state(
        state: &mut StateMachineState,
        log: &[Command],
    ) -> Result<(), String> {
        for cmd in log {
            Self::apply_command(state, cmd)?;
        }
        Ok(())
    }

    // ================================================================
    // SNAPSHOTS (25-36)
    // ================================================================

    /// Problem 25: Create snapshot
    pub fn create_snapshot(
        state: &StateMachineState,
        version: u64,
        now: u64,
    ) -> Snapshot {
        Snapshot {
            snapshot_id: format!("snap_{}", version),
            version,
            state: state.clone(),
            created_at: now,
        }
    }

    /// Problem 26: Store snapshot
    pub fn store_snapshot(
        snapshots: &mut HashMap<u64, Snapshot>,
        snap: Snapshot,
    ) {
        snapshots.insert(snap.version, snap);
    }

    /// Problem 27: Retrieve snapshot
    pub fn retrieve_snapshot(
        snapshots: &HashMap<u64, Snapshot>,
        version: u64,
    ) -> Option<Snapshot> {
        snapshots.get(&version).cloned()
    }

    /// Problem 28: Get latest snapshot
    pub fn get_latest_snapshot(snapshots: &HashMap<u64, Snapshot>) -> Option<Snapshot> {
        snapshots
            .values()
            .max_by_key(|s| s.version)
            .cloned()
    }

    /// Problem 29: Load state from snapshot
    pub fn load_state_from_snapshot(snap: &Snapshot) -> StateMachineState {
        snap.state.clone()
    }

    /// Problem 30: Restore from snapshot
    pub fn restore_from_snapshot(
        state: &mut StateMachineState,
        snap: &Snapshot,
    ) {
        *state = snap.state.clone();
    }

    /// Problem 31: Delete old snapshots
    pub fn delete_old_snapshots(
        snapshots: &mut HashMap<u64, Snapshot>,
        keep_versions: usize,
    ) -> usize {
        let total_before = snapshots.len();
        let to_keep = snapshots
            .keys()
            .copied()
            .collect::<Vec<_>>();
        let mut sorted_keys = to_keep;
        sorted_keys.sort_by(|a, b| b.cmp(a));

        let to_delete: Vec<u64> = sorted_keys.into_iter().skip(keep_versions).collect();

        for version in &to_delete {
            snapshots.remove(version);
        }
        total_before - snapshots.len()
    }

    /// Problem 32: Verify snapshot integrity
    pub fn verify_snapshot_integrity(snap: &Snapshot) -> bool {
        !snap.snapshot_id.is_empty()
            && snap.version > 0
            && !snap.state.state_id.is_empty()
    }

    /// Problem 33: Incrementally snapshot
    pub fn incrementally_snapshot(
        full_snap: &Snapshot,
        commands: &[Command],
    ) -> Snapshot {
        let mut new_state = full_snap.state.clone();
        for cmd in commands {
            if cmd.index > new_state.last_applied {
                new_state.current_value = cmd.data.clone();
                new_state.last_applied = cmd.index;
            }
        }
        Snapshot {
            snapshot_id: format!("snap_{}", full_snap.version + 1),
            version: full_snap.version + 1,
            state: new_state,
            created_at: 0,
        }
    }

    /// Problem 34: Merge snapshots
    pub fn merge_snapshots(snap1: &Snapshot, snap2: &Snapshot) -> Snapshot {
        let merged_state = if snap1.version > snap2.version {
            snap1.state.clone()
        } else {
            snap2.state.clone()
        };
        Snapshot {
            snapshot_id: format!("snap_merged_{}", snap1.version.max(snap2.version)),
            version: snap1.version.max(snap2.version),
            state: merged_state,
            created_at: 0,
        }
    }

    /// Problem 35: Get snapshot size estimate
    pub fn get_snapshot_size_estimate(snap: &Snapshot) -> usize {
        snap.snapshot_id.len()
            + snap.state.state_id.len()
            + snap.state.current_value.len()
            + 16
    }

    /// Problem 36: List all snapshots
    pub fn list_all_snapshots(snapshots: &HashMap<u64, Snapshot>) -> Vec<u64> {
        let mut versions: Vec<_> = snapshots.keys().copied().collect();
        versions.sort();
        versions
    }

    // ================================================================
    // QUERIES (37-50)
    // ================================================================

    /// Problem 37: Query state
    pub fn query_state(state: &StateMachineState, key: &str) -> Option<String> {
        if key == "value" {
            Some(state.current_value.clone())
        } else {
            None
        }
    }

    /// Problem 38: Execute read-only query
    pub fn execute_read_only_query(
        state: &StateMachineState,
        _query: &str,
    ) -> Result<String, String> {
        Ok(state.current_value.clone())
    }

    /// Problem 39: Execute conditional query
    pub fn execute_conditional_query(
        state: &StateMachineState,
        condition: &str,
    ) -> bool {
        if condition == "exists" {
            !state.current_value.is_empty()
        } else {
            false
        }
    }

    /// Problem 40: Get state snapshot at index
    pub fn get_state_snapshot_at_index(
        states: &HashMap<u64, StateMachineState>,
        index: u64,
    ) -> Option<StateMachineState> {
        states.get(&index).cloned()
    }

    /// Problem 41: Compute state hash
    pub fn compute_state_hash(state: &StateMachineState) -> u64 {
        let mut hash = 5381u64;
        for byte in state.current_value.as_bytes() {
            hash = ((hash << 5).wrapping_add(hash)).wrapping_add(*byte as u64);
        }
        hash
    }

    /// Problem 42: Verify state against hash
    pub fn verify_state_against_hash(
        state: &StateMachineState,
        expected_hash: u64,
    ) -> bool {
        Self::compute_state_hash(state) == expected_hash
    }

    /// Problem 43: Get state diff
    pub fn get_state_diff(
        state1: &StateMachineState,
        state2: &StateMachineState,
    ) -> Vec<String> {
        let mut diffs = Vec::new();
        if state1.current_value != state2.current_value {
            diffs.push(format!(
                "value: {} -> {}",
                state1.current_value, state2.current_value
            ));
        }
        if state1.last_applied != state2.last_applied {
            diffs.push(format!(
                "last_applied: {} -> {}",
                state1.last_applied, state2.last_applied
            ));
        }
        diffs
    }

    /// Problem 44: Apply state patch
    pub fn apply_state_patch(
        state: &mut StateMachineState,
        patch: &[(String, String)],
    ) {
        for (key, value) in patch {
            if key == "value" {
                state.current_value = value.clone();
            }
        }
    }

    /// Problem 45: Extract state subset
    pub fn extract_state_subset(
        state: &StateMachineState,
        fields: &[&str],
    ) -> HashMap<String, String> {
        let mut subset = HashMap::new();
        for field in fields {
            match *field {
                "value" => {
                    subset.insert("value".to_string(), state.current_value.clone());
                }
                "last_applied" => {
                    subset.insert("last_applied".to_string(), state.last_applied.to_string());
                }
                _ => {}
            }
        }
        subset
    }

    /// Problem 46: Merge state updates
    pub fn merge_state_updates(
        base: &mut StateMachineState,
        updates: &[StateMachineState],
    ) {
        for update in updates {
            if update.last_applied > base.last_applied {
                base.current_value = update.current_value.clone();
                base.last_applied = update.last_applied;
                base.last_term = update.last_term;
            }
        }
    }

    /// Problem 47: Validate state equality
    pub fn validate_state_equality(
        state1: &StateMachineState,
        state2: &StateMachineState,
    ) -> bool {
        state1.current_value == state2.current_value
            && state1.last_applied == state2.last_applied
    }

    /// Problem 48: Check state sanity
    pub fn check_state_sanity(state: &StateMachineState) -> bool {
        state.last_applied > 0 && !state.state_id.is_empty()
    }

    /// Problem 49: Get state version
    pub fn get_state_version(state: &StateMachineState) -> String {
        format!("v{}", state.last_applied)
    }

    /// Problem 50: Compare state versions
    pub fn compare_state_versions(
        state1: &StateMachineState,
        state2: &StateMachineState,
    ) -> std::cmp::Ordering {
        state1.last_applied.cmp(&state2.last_applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_state_machine() {
        let state = ConsensusStateMachineSolver::create_state_machine("app1");
        assert_eq!(state.state_id, "app1");
    }

    #[test]
    fn test_apply_command() {
        let mut state = ConsensusStateMachineSolver::create_state_machine("app1");
        let cmd = ConsensusStateMachineSolver::create_command(1, 1, "value1");
        assert!(ConsensusStateMachineSolver::apply_command(&mut state, &cmd).is_ok());
        assert_eq!(state.current_value, "value1");
    }

    #[test]
    fn test_create_snapshot() {
        let state = ConsensusStateMachineSolver::create_state_machine("app1");
        let snap = ConsensusStateMachineSolver::create_snapshot(&state, 1, 1000);
        assert_eq!(snap.version, 1);
    }

    #[test]
    fn test_restore_from_snapshot() {
        let mut original = ConsensusStateMachineSolver::create_state_machine("app1");
        original.current_value = "test_value".to_string();
        let snap = ConsensusStateMachineSolver::create_snapshot(&original, 1, 1000);

        let mut restored = ConsensusStateMachineSolver::create_state_machine("app2");
        ConsensusStateMachineSolver::restore_from_snapshot(&mut restored, &snap);
        assert_eq!(restored.current_value, "test_value");
    }

    #[test]
    fn test_command_ordering() {
        let cmds = vec![
            ConsensusStateMachineSolver::create_command(1, 1, "a"),
            ConsensusStateMachineSolver::create_command(2, 1, "b"),
        ];
        assert!(ConsensusStateMachineSolver::validate_command_ordering(&cmds));
    }

    #[test]
    fn test_get_state_diff() {
        let mut state1 = ConsensusStateMachineSolver::create_state_machine("app1");
        let mut state2 = ConsensusStateMachineSolver::create_state_machine("app1");
        state1.current_value = "value1".to_string();
        state2.current_value = "value2".to_string();
        let diff = ConsensusStateMachineSolver::get_state_diff(&state1, &state2);
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_serialize_deserialize() {
        let mut state = ConsensusStateMachineSolver::create_state_machine("app1");
        state.current_value = "test".to_string();
        let serialized = ConsensusStateMachineSolver::serialize_state(&state);
        let deserialized = ConsensusStateMachineSolver::deserialize_state(&serialized).unwrap();
        assert_eq!(deserialized.state_id, "app1");
    }

    #[test]
    fn test_store_and_retrieve_snapshot() {
        let mut snapshots = HashMap::new();
        let state = ConsensusStateMachineSolver::create_state_machine("app1");
        let snap = ConsensusStateMachineSolver::create_snapshot(&state, 1, 1000);
        ConsensusStateMachineSolver::store_snapshot(&mut snapshots, snap.clone());
        let retrieved = ConsensusStateMachineSolver::retrieve_snapshot(&snapshots, 1).unwrap();
        assert_eq!(retrieved.version, 1);
    }

    #[test]
    fn test_apply_batch_commands() {
        let mut state = ConsensusStateMachineSolver::create_state_machine("app1");
        let cmds = vec![
            ConsensusStateMachineSolver::create_command(1, 1, "a"),
            ConsensusStateMachineSolver::create_command(2, 1, "b"),
        ];
        assert!(ConsensusStateMachineSolver::apply_commands_in_batch(&mut state, &cmds).is_ok());
        assert_eq!(state.current_value, "b");
    }

    #[test]
    fn test_compute_state_hash() {
        let state = ConsensusStateMachineSolver::create_state_machine("app1");
        let hash = ConsensusStateMachineSolver::compute_state_hash(&state);
        assert!(hash > 0);
    }
}
