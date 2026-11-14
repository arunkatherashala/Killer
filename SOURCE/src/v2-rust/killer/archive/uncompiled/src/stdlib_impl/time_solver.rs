// ================================================================
// TIME & SCHEDULING SOLVER - Phase 21.5
// Time operations, date/time calculations, scheduling, timers
// ================================================================

use std::time::{SystemTime, Duration, UNIX_EPOCH};
use std::fmt;

/// Time representation (milliseconds since epoch)
pub type Timestamp = u64;

/// Durations and intervals
pub type TimeDelta = Duration;

/// Time & Scheduling Operations Solver
pub struct TimeSolver;

impl TimeSolver {
    // ================================================================
    // CURRENT TIME (1-10)
    // ================================================================

    /// Problem 1: Get current Unix timestamp (seconds)
    pub fn unix_timestamp_seconds() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Problem 2: Get current Unix timestamp (milliseconds)
    pub fn unix_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Problem 3: Get current Unix timestamp (microseconds)
    pub fn unix_timestamp_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }

    /// Problem 4: Get current Unix timestamp (nanoseconds)
    pub fn unix_timestamp_nanos() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Problem 5: Current time as ISO 8601 string (approximation)
    pub fn iso_8601_now() -> String {
        let secs = Self::unix_timestamp_seconds();
        // Simplified: actual ISO8601 needs proper date/time calculation
        format!("1970-01-01T00:00:{}Z", secs)
    }

    /// Problem 6: Elapsed time since epoch in days
    pub fn days_since_epoch() -> u64 {
        Self::unix_timestamp_seconds() / 86400
    }

    /// Problem 7: Elapsed time since epoch in hours
    pub fn hours_since_epoch() -> u64 {
        Self::unix_timestamp_seconds() / 3600
    }

    /// Problem 8: Elapsed time since epoch in minutes
    pub fn minutes_since_epoch() -> u64 {
        Self::unix_timestamp_seconds() / 60
    }

    // ================================================================
    // TIME CALCULATIONS (9-25)
    // ================================================================

    /// Problem 9: Duration between two timestamps (seconds)
    pub fn duration_seconds(start: u64, end: u64) -> i64 {
        end as i64 - start as i64
    }

    /// Problem 10: Duration between two timestamps (milliseconds)
    pub fn duration_millis(start: u64, end: u64) -> i64 {
        (end as i64 - start as i64) * 1000
    }

    /// Problem 11: Add seconds to timestamp
    pub fn add_seconds(ts: u64, seconds: u64) -> u64 {
        ts + seconds
    }

    /// Problem 12: Add minutes to timestamp
    pub fn add_minutes(ts: u64, minutes: u64) -> u64 {
        ts + minutes * 60
    }

    /// Problem 13: Add hours to timestamp
    pub fn add_hours(ts: u64, hours: u64) -> u64 {
        ts + hours * 3600
    }

    /// Problem 14: Add days to timestamp
    pub fn add_days(ts: u64, days: u64) -> u64 {
        ts + days * 86400
    }

    /// Problem 15: Subtract seconds from timestamp
    pub fn subtract_seconds(ts: u64, seconds: u64) -> u64 {
        ts.saturating_sub(seconds)
    }

    /// Problem 16: Subtract minutes from timestamp
    pub fn subtract_minutes(ts: u64, minutes: u64) -> u64 {
        ts.saturating_sub(minutes * 60)
    }

    /// Problem 17: Subtract hours from timestamp
    pub fn subtract_hours(ts: u64, hours: u64) -> u64 {
        ts.saturating_sub(hours * 3600)
    }

    /// Problem 18: Subtract days from timestamp
    pub fn subtract_days(ts: u64, days: u64) -> u64 {
        ts.saturating_sub(days * 86400)
    }

    /// Problem 19: Is leap year (simple calculation)
    pub fn is_leap_year(year: u32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    /// Problem 20: Days in month
    pub fn days_in_month(month: u32, year: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if Self::is_leap_year(year) { 29 } else { 28 },
            _ => 0,
        }
    }

    // ================================================================
    // SCHEDULING & TIMERS (21-35)
    // ================================================================

    /// Problem 21: Check if deadline passed
    pub fn deadline_passed(deadline: u64, now: u64) -> bool {
        now >= deadline
    }

    /// Problem 22: Time until deadline (seconds)
    pub fn time_until_deadline(deadline: u64, now: u64) -> i64 {
        deadline as i64 - now as i64
    }

    /// Problem 23: Percentage of deadline elapsed
    pub fn deadline_progress(start: u64, deadline: u64, now: u64) -> f64 {
        if deadline <= start { return 0.0; }
        ((now - start) as f64) / ((deadline - start) as f64)
    }

    /// Problem 24: Round-robin scheduler - next slot
    pub fn round_robin_next_slot(current_slot: u32, num_tasks: u32) -> u32 {
        (current_slot + 1) % num_tasks
    }

    /// Problem 25: Rate limiter - tokens available
    pub fn rate_limiter_tokens(rate: f64, capacity: f64, time_elapsed: f64, tokens_used: f64) -> f64 {
        let tokens_generated = rate * time_elapsed;
        ((tokens_used - tokens_generated).max(0.0)).min(capacity)
    }

    /// Problem 26: Exponential backoff delay
    pub fn exponential_backoff(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
        let delay = base_delay_ms * (2u64.pow(attempt));
        delay.min(max_delay_ms)
    }

    /// Problem 27: Jittered backoff (with random component)
    pub fn jittered_backoff(attempt: u32, base_delay_ms: u64, max_delay_ms: u64) -> u64 {
        let delay = Self::exponential_backoff(attempt, base_delay_ms, max_delay_ms);
        let jitter_factor = 0.1 + ((attempt % 10) as f64) * 0.05;
        ((delay as f64) * jitter_factor) as u64
    }

    /// Problem 28: Check if time is within window
    pub fn in_time_window(now: u64, window_start: u64, window_end: u64) -> bool {
        now >= window_start && now < window_end
    }

    /// Problem 29: Batch timeout - should flush?
    pub fn batch_should_flush(batch_size: usize, max_size: usize, time_elapsed_ms: u64, max_wait_ms: u64) -> bool {
        batch_size >= max_size || time_elapsed_ms >= max_wait_ms
    }

    /// Problem 30: Sliding window - time window in seconds
    pub fn sliding_window_contains(event_time: u64, window_end: u64, window_length_sec: u64) -> bool {
        event_time >= window_end.saturating_sub(window_length_sec)
    }

    // ================================================================
    // TIMER UTILITIES (31-40)
    // ================================================================

    /// Problem 31: Format seconds to MM:SS
    pub fn format_duration_mm_ss(total_seconds: u64) -> String {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    /// Problem 32: Format seconds to HH:MM:SS
    pub fn format_duration_hh_mm_ss(total_seconds: u64) -> String {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    /// Problem 33: Parse HH:MM:SS to seconds
    pub fn parse_duration_hh_mm_ss(s: &str) -> u64 {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 { return 0; }
        
        let hours = parts[0].parse::<u64>().unwrap_or(0);
        let minutes = parts[1].parse::<u64>().unwrap_or(0);
        let seconds = parts[2].parse::<u64>().unwrap_or(0);
        
        hours * 3600 + minutes * 60 + seconds
    }

    /// Problem 34: Convert seconds to human-readable duration
    pub fn human_readable_duration(seconds: u64) -> String {
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else if seconds < 86400 {
            let hours = seconds / 3600;
            let mins = (seconds % 3600) / 60;
            format!("{}h {}m", hours, mins)
        } else {
            let days = seconds / 86400;
            let hours = (seconds % 86400) / 3600;
            format!("{}d {}h", days, hours)
        }
    }

    /// Problem 35: Timestamp to days since date
    pub fn days_since_date(ts: u64, ref_date_ts: u64) -> i64 {
        let diff_sec = (ts as i64) - (ref_date_ts as i64);
        diff_sec / 86400
    }

    /// Problem 36: Average time between events
    pub fn average_interval(total_time_ms: u64, num_events: u32) -> f64 {
        if num_events == 0 { return 0.0; }
        (total_time_ms as f64) / (num_events as f64)
    }

    /// Problem 37: Event rate (events per second)
    pub fn event_rate(num_events: u32, time_ms: u64) -> f64 {
        if time_ms == 0 { return 0.0; }
        (num_events as f64) * 1000.0 / (time_ms as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp() {
        let ts = TimeSolver::unix_timestamp_seconds();
        assert!(ts > 1_600_000_000); // After Sept 2020
    }

    #[test]
    fn test_time_calculations() {
        let start = 1000u64;
        let end = 2700u64; // 27 minutes later
        
        let duration = TimeSolver::duration_seconds(start, end);
        assert_eq!(duration, 1700);
    }

    #[test]
    fn test_format_duration() {
        let formatted = TimeSolver::format_duration_hh_mm_ss(3661);
        assert_eq!(formatted, "01:01:01");
    }

    #[test]
    fn test_backoff() {
        let delay1 = TimeSolver::exponential_backoff(0, 100, 10000);
        let delay2 = TimeSolver::exponential_backoff(1, 100, 10000);
        assert!(delay2 > delay1);
    }
}
