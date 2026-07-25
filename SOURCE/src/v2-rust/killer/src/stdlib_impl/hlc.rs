// ================================================================
// HYBRID LOGICAL CLOCKS - Phase 28.3
// Causality tracking across distributed systems
// ================================================================

use std::collections::HashMap;

/// HLC timestamp with logical component
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct HLCTimestamp {
    pub wall_time: u64,
    pub logical_clock: u32,
}

/// Causality relationship
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CausalityRelation {
    Happens_Before,
    Happens_After,
    Concurrent,
}

pub struct HLCSolver;

impl HLCSolver {
    // ================================================================
    // CLOCK MANAGEMENT (1-12)
    // ================================================================

    /// Problem 1: Create HLC timestamp
    pub fn create_hlc_timestamp(wall_time: u64, logical_clock: u32) -> HLCTimestamp {
        HLCTimestamp {
            wall_time,
            logical_clock,
        }
    }

    /// Problem 2: Initialize HLC
    pub fn initialize_hlc() -> HLCTimestamp {
        HLCTimestamp {
            wall_time: 0,
            logical_clock: 0,
        }
    }

    /// Problem 3: Get wall time
    pub fn get_wall_time(ts: &HLCTimestamp) -> u64 {
        ts.wall_time
    }

    /// Problem 4: Get logical clock
    pub fn get_logical_clock(ts: &HLCTimestamp) -> u32 {
        ts.logical_clock
    }

    /// Problem 5: Increment logical clock
    pub fn increment_logical_clock(ts: &mut HLCTimestamp) {
        if ts.logical_clock < u32::MAX {
            ts.logical_clock += 1;
        }
    }

    /// Problem 6: Update on local event
    pub fn update_on_local_event(current: &mut HLCTimestamp, now: u64) {
        if now > current.wall_time {
            current.wall_time = now;
            current.logical_clock = 0;
        } else {
            Self::increment_logical_clock(current);
        }
    }

    /// Problem 7: Send timestamp
    pub fn send_timestamp(ts: &HLCTimestamp) -> String {
        format!("{}:{}", ts.wall_time, ts.logical_clock)
    }

    /// Problem 8: Parse received timestamp
    pub fn parse_received_timestamp(s: &str) -> Option<HLCTimestamp> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() == 2 {
            let wall_time = parts[0].parse::<u64>().ok()?;
            let logical_clock = parts[1].parse::<u32>().ok()?;
            Some(HLCTimestamp {
                wall_time,
                logical_clock,
            })
        } else {
            None
        }
    }

    /// Problem 9: Merge on receive
    pub fn merge_on_receive(local: &mut HLCTimestamp, received: &HLCTimestamp, now: u64) {
        let max_wall = local.wall_time.max(received.wall_time);
        if max_wall > now {
            local.wall_time = max_wall;
            local.logical_clock = if local.wall_time == received.wall_time {
                local.logical_clock.max(received.logical_clock) + 1
            } else {
                0
            };
        } else {
            local.wall_time = now;
            local.logical_clock = 0;
        }
    }

    /// Problem 10: Compare timestamps
    pub fn compare_timestamps(ts1: &HLCTimestamp, ts2: &HLCTimestamp) -> std::cmp::Ordering {
        ts1.cmp(ts2)
    }

    /// Problem 11: Format HLC for display
    pub fn format_hlc(ts: &HLCTimestamp) -> String {
        format!("HLC({}, {})", ts.wall_time, ts.logical_clock)
    }

    /// Problem 12: Validate HLC timestamp
    pub fn validate_hlc_timestamp(ts: &HLCTimestamp) -> bool {
        ts.wall_time > 0 || ts.logical_clock == 0
    }

    // ================================================================
    // CAUSALITY TRACKING (13-24)
    // ================================================================

    /// Problem 13: Detect causal relationship
    pub fn detect_causal_relationship(
        ts1: &HLCTimestamp,
        ts2: &HLCTimestamp,
    ) -> CausalityRelation {
        if ts1 < ts2 {
            CausalityRelation::Happens_Before
        } else if ts1 > ts2 {
            CausalityRelation::Happens_After
        } else {
            CausalityRelation::Concurrent
        }
    }

    /// Problem 14: Check happens before
    pub fn check_happens_before(ts1: &HLCTimestamp, ts2: &HLCTimestamp) -> bool {
        ts1 < ts2
    }

    /// Problem 15: Check concurrency
    pub fn check_concurrency(ts1: &HLCTimestamp, ts2: &HLCTimestamp) -> bool {
        ts1 != ts2 && !(ts1 < ts2) && !(ts1 > ts2)
    }

    /// Problem 16: Build causal chain
    pub fn build_causal_chain(events: &[HLCTimestamp]) -> Vec<usize> {
        let mut chain = vec![0];
        for i in 1..events.len() {
            if events[chain[chain.len() - 1]] < events[i] {
                chain.push(i);
            }
        }
        chain
    }

    /// Problem 17: Track event dependencies
    pub fn track_event_dependencies(
        events: &[(String, HLCTimestamp)],
    ) -> HashMap<String, Vec<String>> {
        let mut deps = HashMap::new();
        for i in 0..events.len() {
            let mut predecessors = Vec::new();
            for j in 0..i {
                if events[j].1 < events[i].1 {
                    predecessors.push(events[j].0.clone());
                }
            }
            deps.insert(events[i].0.clone(), predecessors);
        }
        deps
    }

    /// Problem 18: Detect causal cycles
    pub fn detect_causal_cycles(
        dependencies: &HashMap<String, Vec<String>>,
    ) -> bool {
        for (node, deps) in dependencies.iter() {
            for dep in deps {
                if dependencies
                    .get(dep)
                    .map(|d| d.contains(node))
                    .unwrap_or(false)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Problem 19: Linearize concurrent events
    pub fn linearize_concurrent_events(
        events: &[HLCTimestamp],
    ) -> Vec<usize> {
        let mut sorted: Vec<_> = (0..events.len()).collect();
        sorted.sort_by(|&a, &b| events[a].cmp(&events[b]));
        sorted
    }

    /// Problem 20: Get transitive closure
    pub fn get_transitive_closure(
        direct_deps: &HashMap<String, Vec<String>>,
    ) -> HashMap<String, Vec<String>> {
        let mut closure = direct_deps.clone();
        for node in direct_deps.keys() {
            let mut reachable = std::collections::HashSet::new();
            let mut to_visit = direct_deps.get(node).cloned().unwrap_or_default();

            while !to_visit.is_empty() {
                let current = to_visit.pop().unwrap();
                if reachable.insert(current.clone()) {
                    if let Some(deps) = direct_deps.get(&current) {
                        to_visit.extend(deps.clone());
                    }
                }
            }
            closure.insert(node.clone(), reachable.into_iter().collect());
        }
        closure
    }

    /// Problem 21: Merge causality tracking
    pub fn merge_causality_tracking(
        local_deps: &HashMap<String, Vec<String>>,
        received_deps: &HashMap<String, Vec<String>>,
    ) -> HashMap<String, Vec<String>> {
        let mut merged = local_deps.clone();
        for (key, deps) in received_deps.iter() {
            let entry = merged.entry(key.clone()).or_insert_with(Vec::new);
            for dep in deps {
                if !entry.contains(dep) {
                    entry.push(dep.clone());
                }
            }
        }
        merged
    }

    /// Problem 22: Verify causal consistency
    pub fn verify_causal_consistency(deps: &HashMap<String, Vec<String>>) -> bool {
        !Self::detect_causal_cycles(deps)
    }

    /// Problem 23: Extract causal history
    pub fn extract_causal_history(
        event: &str,
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut history = vec![event.to_string()];
        let mut to_visit = dependencies
            .get(event)
            .cloned()
            .unwrap_or_default();

        while !to_visit.is_empty() {
            let current = to_visit.pop().unwrap();
            if !history.contains(&current) {
                history.push(current.clone());
                if let Some(deps) = dependencies.get(&current) {
                    to_visit.extend(deps.clone());
                }
            }
        }
        history
    }

    /// Problem 24: Compute causal frontier
    pub fn compute_causal_frontier(
        events: &[HLCTimestamp],
    ) -> Vec<HLCTimestamp> {
        let mut frontier = Vec::new();
        for ts in events {
            let is_frontier = events.iter().all(|other| {
                other == ts || !(ts < other)
            });
            if is_frontier {
                frontier.push(*ts);
            }
        }
        frontier
    }

    // ================================================================
    // GAP HANDLING (25-36)
    // ================================================================

    /// Problem 25: Detect time gap
    pub fn detect_time_gap(ts1: &HLCTimestamp, ts2: &HLCTimestamp) -> bool {
        ts2.wall_time > ts1.wall_time + 1000
    }

    /// Problem 26: Handle clock skew
    pub fn handle_clock_skew(
        local_ts: &mut HLCTimestamp,
        received_ts: &HLCTimestamp,
        now: u64,
        max_skew: u64,
    ) -> bool {
        if now > received_ts.wall_time && now - received_ts.wall_time > max_skew {
            return false;
        }
        Self::merge_on_receive(local_ts, received_ts, now);
        true
    }

    /// Problem 27: Fill clock gaps
    pub fn fill_clock_gaps(
        ts1: &HLCTimestamp,
        ts2: &HLCTimestamp,
    ) -> Vec<HLCTimestamp> {
        let mut gap = vec![*ts1];
        let mut current = *ts1;
        while current < *ts2 {
            current.logical_clock += 1;
            gap.push(current);
        }
        gap
    }

    /// Problem 28: Detect monotonicity violations
    pub fn detect_monotonicity_violations(timestamps: &[HLCTimestamp]) -> bool {
        for i in 1..timestamps.len() {
            if timestamps[i-1] > timestamps[i] {
                return true;
            }
        }
        false
    }

    /// Problem 29: Handle backwards time jump
    pub fn handle_backwards_time_jump(
        local: &mut HLCTimestamp,
        now: u64,
    ) -> bool {
        if now < local.wall_time {
            local.logical_clock += 1;
            true
        } else {
            local.wall_time = now;
            local.logical_clock = 0;
            false
        }
    }

    /// Problem 30: Reconcile divergent clocks
    pub fn reconcile_divergent_clocks(
        clocks: &[HLCTimestamp],
    ) -> HLCTimestamp {
        clocks.iter().max().copied().unwrap_or_default()
    }

    /// Problem 31: Detect clock divergence
    pub fn detect_clock_divergence(ts1: &HLCTimestamp, ts2: &HLCTimestamp) -> u64 {
        (ts1.wall_time as i64 - ts2.wall_time as i64).abs() as u64
    }

    /// Problem 32: Synchronize clock domains
    pub fn synchronize_clock_domains(
        local: &mut HLCTimestamp,
        peers: &[HLCTimestamp],
    ) {
        if let Some(max_peer) = peers.iter().max() {
            if max_peer > local {
                *local = *max_peer;
                local.logical_clock += 1;
            }
        }
    }

    /// Problem 33: Apply clock adjustment
    pub fn apply_clock_adjustment(
        ts: &mut HLCTimestamp,
        adjustment: i64,
    ) {
        if adjustment > 0 {
            ts.wall_time = ts.wall_time.saturating_add(adjustment as u64);
        } else if adjustment < 0 {
            ts.wall_time = ts.wall_time.saturating_sub((-adjustment) as u64);
        }
    }

    /// Problem 34: Estimate clock drift
    pub fn estimate_clock_drift(
        ts1: &HLCTimestamp,
        ts2: &HLCTimestamp,
        elapsed: u64,
    ) -> f64 {
        if elapsed == 0 {
            return 0.0;
        }
        let clock_diff = (ts2.wall_time as i64 - ts1.wall_time as i64).abs() as f64;
        (clock_diff / elapsed as f64) - 1.0
    }

    /// Problem 35: Predict future timestamp
    pub fn predict_future_timestamp(
        ts: &HLCTimestamp,
        drift: f64,
        future_ms: u64,
    ) -> HLCTimestamp {
        let adjusted = (ts.wall_time as f64 * (1.0 + drift)) as u64;
        HLCTimestamp {
            wall_time: adjusted + future_ms,
            logical_clock: 0,
        }
    }

    /// Problem 36: Detect NTP sync failures
    pub fn detect_ntp_sync_failures(
        clock_deltas: &[i64],
        threshold: i64,
    ) -> bool {
        clock_deltas.iter().any(|&d| d.abs() > threshold)
    }

    // ================================================================
    // TIMESTAMP ORDERING (37-50)
    // ================================================================

    /// Problem 37: Create total order
    pub fn create_total_order(
        events: &[(String, HLCTimestamp)],
    ) -> Vec<String> {
        let mut sorted = events.to_vec();
        sorted.sort_by_key(|(_name, ts)| *ts);
        sorted.into_iter().map(|(name, _)| name).collect()
    }

    /// Problem 38: Get event before
    pub fn get_events_before(
        events: &[(String, HLCTimestamp)],
        ts: &HLCTimestamp,
    ) -> Vec<String> {
        events
            .iter()
            .filter(|(_, event_ts)| event_ts < ts)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Problem 39: Get events after
    pub fn get_events_after(
        events: &[(String, HLCTimestamp)],
        ts: &HLCTimestamp,
    ) -> Vec<String> {
        events
            .iter()
            .filter(|(_, event_ts)| event_ts > ts)
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Problem 40: Get concurrent events
    pub fn get_concurrent_events(
        events: &[(String, HLCTimestamp)],
        ts: &HLCTimestamp,
    ) -> Vec<String> {
        events
            .iter()
            .filter(|(_, event_ts)| event_ts != ts && !(event_ts < ts) && !(event_ts > ts))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Problem 41: Sort by timestamp
    pub fn sort_by_timestamp(
        mut events: Vec<(String, HLCTimestamp)>,
    ) -> Vec<(String, HLCTimestamp)> {
        events.sort_by_key(|(_name, ts)| *ts);
        events
    }

    /// Problem 42: Find timestamp gaps
    pub fn find_timestamp_gaps(
        events: &[HLCTimestamp],
    ) -> Vec<(HLCTimestamp, HLCTimestamp)> {
        let mut gaps = Vec::new();
        for i in 1..events.len() {
            if events[i].wall_time > events[i - 1].wall_time + 1 {
                gaps.push((events[i - 1], events[i]));
            }
        }
        gaps
    }

    /// Problem 43: Extract timestamp range
    pub fn extract_timestamp_range(
        events: &[HLCTimestamp],
    ) -> Option<(HLCTimestamp, HLCTimestamp)> {
        if events.is_empty() {
            return None;
        }
        events.iter().min().and_then(|min| {
            events.iter().max().map(|max| (*min, *max))
        })
    }

    /// Problem 44: Check ordering consistency
    pub fn check_ordering_consistency(
        events: &[HLCTimestamp],
    ) -> bool {
        for i in 1..events.len() {
            if events[i] < events[i - 1] {
                return false;
            }
        }
        true
    }

    /// Problem 45: Reorder based on dependency
    pub fn reorder_based_on_dependency(
        events: &[(String, HLCTimestamp)],
        dependencies: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut ordered = Vec::new();
        let mut completed = std::collections::HashSet::new();

        for (name, _) in events {
            if let Some(deps) = dependencies.get(name) {
                if deps.iter().all(|d| completed.contains(d)) {
                    ordered.push(name.clone());
                    completed.insert(name.clone());
                }
            } else {
                ordered.push(name.clone());
                completed.insert(name.clone());
            }
        }
        ordered
    }

    /// Problem 46: Compact timestamp sequence
    pub fn compact_timestamp_sequence(
        events: &[HLCTimestamp],
    ) -> Vec<HLCTimestamp> {
        let mut compact = Vec::new();
        for ts in events {
            if compact.is_empty() || compact[compact.len() - 1] != *ts {
                compact.push(*ts);
            }
        }
        compact
    }

    /// Problem 47: Expand timestamp range
    pub fn expand_timestamp_range(
        start: &HLCTimestamp,
        end: &HLCTimestamp,
        step: u32,
    ) -> Vec<HLCTimestamp> {
        let mut expanded = Vec::new();
        let mut current = *start;
        while current <= *end {
            expanded.push(current);
            current.logical_clock += step;
        }
        expanded
    }

    /// Problem 48: Assign sequence numbers
    pub fn assign_sequence_numbers(
        events: &[(String, HLCTimestamp)],
    ) -> HashMap<String, u64> {
        let mut sequenced = HashMap::new();
        for (i, (name, _)) in events.iter().enumerate() {
            sequenced.insert(name.clone(), i as u64);
        }
        sequenced
    }

    /// Problem 49: Detect out-of-order delivery
    pub fn detect_out_of_order_delivery(
        sequence_numbers: &[u64],
    ) -> bool {
        for i in 1..sequence_numbers.len() {
            if sequence_numbers[i] < sequence_numbers[i - 1] {
                return true;
            }
        }
        false
    }

    /// Problem 50: Build version vector
    pub fn build_version_vector(
        events: &[(String, HLCTimestamp)],
    ) -> HashMap<String, u64> {
        let mut vectors = HashMap::new();
        for (name, ts) in events {
            *vectors
                .entry(name.clone())
                .or_insert(0) = ts.wall_time;
        }
        vectors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_hlc() {
        let ts = HLCSolver::create_hlc_timestamp(1000, 5);
        assert_eq!(ts.wall_time, 1000);
        assert_eq!(ts.logical_clock, 5);
    }

    #[test]
    fn test_update_on_local_event() {
        let mut ts = HLCTimestamp {
            wall_time: 1000,
            logical_clock: 0,
        };
        HLCSolver::update_on_local_event(&mut ts, 1005);
        assert_eq!(ts.wall_time, 1005);
        assert_eq!(ts.logical_clock, 0);
    }

    #[test]
    fn test_merge_on_receive() {
        let mut local = HLCTimestamp {
            wall_time: 1000,
            logical_clock: 0,
        };
        let received = HLCTimestamp {
            wall_time: 1100,
            logical_clock: 3,
        };
        HLCSolver::merge_on_receive(&mut local, &received, 1050);
        assert_eq!(local.wall_time, 1100);
    }

    #[test]
    fn test_causal_relationship() {
        let ts1 = HLCTimestamp { wall_time: 100, logical_clock: 0 };
        let ts2 = HLCTimestamp { wall_time: 200, logical_clock: 0 };
        let rel = HLCSolver::detect_causal_relationship(&ts1, &ts2);
        assert_eq!(rel, CausalityRelation::Happens_Before);
    }

    #[test]
    fn test_detect_time_gap() {
        let ts1 = HLCTimestamp { wall_time: 100, logical_clock: 0 };
        let ts2 = HLCTimestamp { wall_time: 2000, logical_clock: 0 };
        assert!(HLCSolver::detect_time_gap(&ts1, &ts2));
    }

    #[test]
    fn test_linearize_concurrent_events() {
        let events = vec![
            HLCTimestamp { wall_time: 50, logical_clock: 0 },
            HLCTimestamp { wall_time: 100, logical_clock: 0 },
            HLCTimestamp { wall_time: 75, logical_clock: 0 },
        ];
        let order = HLCSolver::linearize_concurrent_events(&events);
        assert_eq!(order.len(), 3);
    }

    #[test]
    fn test_extract_causal_history() {
        let mut deps = HashMap::new();
        deps.insert("b".to_string(), vec!["a".to_string()]);
        let history = HLCSolver::extract_causal_history("b", &deps);
        assert!(history.contains(&"a".to_string()));
    }

    #[test]
    fn test_sort_by_timestamp() {
        let events = vec![
            ("a".to_string(), HLCTimestamp { wall_time: 100, logical_clock: 0 }),
            ("b".to_string(), HLCTimestamp { wall_time: 50, logical_clock: 0 }),
        ];
        let sorted = HLCSolver::sort_by_timestamp(events);
        assert_eq!(sorted[0].0, "b");
    }

    #[test]
    fn test_check_ordering_consistency() {
        let events = vec![
            HLCTimestamp { wall_time: 100, logical_clock: 0 },
            HLCTimestamp { wall_time: 200, logical_clock: 0 },
        ];
        assert!(HLCSolver::check_ordering_consistency(&events));
    }

    #[test]
    fn test_estimate_clock_drift() {
        let ts1 = HLCTimestamp { wall_time: 1000, logical_clock: 0 };
        let ts2 = HLCTimestamp { wall_time: 1000, logical_clock: 0 };
        let drift = HLCSolver::estimate_clock_drift(&ts1, &ts2, 100);
        assert_eq!(drift, -1.0);
    }

    #[test]
    fn test_build_version_vector() {
        let events = vec![
            ("node1".to_string(), HLCTimestamp { wall_time: 100, logical_clock: 0 }),
            ("node2".to_string(), HLCTimestamp { wall_time: 200, logical_clock: 0 }),
        ];
        let vectors = HLCSolver::build_version_vector(&events);
        assert_eq!(vectors.len(), 2);
    }
}
