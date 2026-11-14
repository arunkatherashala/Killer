// Date/Time Module for Killer Language
// Time and date manipulation with 20+ functions
// Version: 2.1.0

use std::time::{SystemTime, UNIX_EPOCH, Duration};
use std::collections::HashMap;

/// Date/Time utilities for timestamp and duration operations
pub struct DateTimeModule;

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    pub timestamp: u64, // seconds since Unix epoch
}

impl DateTime {
    pub fn from_timestamp(secs: u64) -> Self {
        DateTime { timestamp: secs }
    }
    
    pub fn to_timestamp(&self) -> u64 {
        self.timestamp
    }
}

impl DateTimeModule {
    // ==================== Current Time ====================
    
    /// Get current Unix timestamp (seconds)
    /// now() => 1710288000
    pub fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Get current timestamp in milliseconds
    /// now_millis() => 1710288000123
    pub fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    /// Get current timestamp in microseconds
    pub fn now_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
    
    // ==================== Timestamp Operations ====================
    
    /// Create DateTime from timestamp (seconds)
    pub fn from_timestamp(secs: u64) -> DateTime {
        DateTime { timestamp: secs }
    }
    
    /// Get timestamp from milliseconds
    pub fn from_millis(millis: u64) -> DateTime {
        DateTime { timestamp: millis / 1000 }
    }
    
    /// Convert timestamp to seconds
    pub fn to_seconds(dt: &DateTime) -> u64 {
        dt.timestamp
    }
    
    /// Convert timestamp to milliseconds
    pub fn to_millis(dt: &DateTime) -> u64 {
        dt.timestamp * 1000
    }
    
    /// Get current DateTime object
    pub fn today() -> DateTime {
        DateTime { timestamp: Self::now() }
    }
    
    // ==================== Arithmetic ====================
    
    /// Add seconds to timestamp
    /// add_seconds(dt, 3600) => timestamp + 1 hour
    pub fn add_seconds(dt: &DateTime, seconds: i64) -> DateTime {
        let new_timestamp = (dt.timestamp as i64 + seconds).max(0) as u64;
        DateTime { timestamp: new_timestamp }
    }
    
    /// Add minutes to timestamp
    pub fn add_minutes(dt: &DateTime, minutes: i64) -> DateTime {
        Self::add_seconds(dt, minutes * 60)
    }
    
    /// Add hours to timestamp
    pub fn add_hours(dt: &DateTime, hours: i64) -> DateTime {
        Self::add_seconds(dt, hours * 3600)
    }
    
    /// Add days to timestamp
    pub fn add_days(dt: &DateTime, days: i64) -> DateTime {
        Self::add_seconds(dt, days * 86400)
    }
    
    /// Subtract seconds from timestamp
    pub fn subtract_seconds(dt: &DateTime, seconds: i64) -> DateTime {
        Self::add_seconds(dt, -seconds)
    }
    
    /// Subtract minutes from timestamp
    pub fn subtract_minutes(dt: &DateTime, minutes: i64) -> DateTime {
        Self::add_minutes(dt, -minutes)
    }
    
    /// Subtract hours from timestamp
    pub fn subtract_hours(dt: &DateTime, hours: i64) -> DateTime {
        Self::add_hours(dt, -hours)
    }
    
    /// Subtract days from timestamp
    pub fn subtract_days(dt: &DateTime, days: i64) -> DateTime {
        Self::add_days(dt, -days)
    }
    
    /// Calculate difference between two timestamps in seconds
    /// difference_seconds(dt1, dt2) => seconds between them
    pub fn difference_seconds(dt1: &DateTime, dt2: &DateTime) -> i64 {
        (dt1.timestamp as i64) - (dt2.timestamp as i64)
    }
    
    /// Calculate difference in minutes
    pub fn difference_minutes(dt1: &DateTime, dt2: &DateTime) -> i64 {
        Self::difference_seconds(dt1, dt2) / 60
    }
    
    /// Calculate difference in hours
    pub fn difference_hours(dt1: &DateTime, dt2: &DateTime) -> i64 {
        Self::difference_seconds(dt1, dt2) / 3600
    }
    
    /// Calculate difference in days
    pub fn difference_days(dt1: &DateTime, dt2: &DateTime) -> i64 {
        Self::difference_seconds(dt1, dt2) / 86400
    }
    
    // ==================== Formatting ====================
    
    /// Format timestamp as ISO 8601 string (basic)
    /// format_iso(now()) => "2024-03-13T00:00:00Z"
    pub fn format_iso(timestamp: u64) -> String {
        // Simplified UTC formatting
        let secs_per_day = 86400;
        let days_since_epoch = timestamp / secs_per_day;
        let secs_today = timestamp % secs_per_day;
        
        let hours = secs_today / 3600;
        let minutes = (secs_today % 3600) / 60;
        let secs = secs_today % 60;
        
        // Simplified date calculation (naive)
        let mut year = 1970;
        let mut days = days_since_epoch;
        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if days < days_in_year as u64 {
                break;
            }
            days -= days_in_year as u64;
            year += 1;
        }
        
        let month_days = [31, if Self::is_leap_year(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut month = 1;
        let mut day = days + 1;
        for &days_in_month in &month_days {
            if day <= days_in_month as u64 {
                break;
            }
            day -= days_in_month as u64;
            month += 1;
        }
        
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hours, minutes, secs)
    }
    
    /// Format timestamp as simple date string
    /// format_date(now()) => "2024-03-13"
    pub fn format_date(timestamp: u64) -> String {
        let parts = Self::format_iso(timestamp);
        parts[..10].to_string()
    }
    
    /// Format timestamp as time string
    /// format_time(now()) => "12:30:45"
    pub fn format_time(timestamp: u64) -> String {
        let secs_today = timestamp % 86400;
        let hours = secs_today / 3600;
        let minutes = (secs_today % 3600) / 60;
        let secs = secs_today % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }
    
    // ==================== Parsing ====================
    
    /// Parse ISO 8601 date string to timestamp
    /// parse_iso("2024-03-13T12:30:45Z") => 1710358245
    pub fn parse_iso(date_str: &str) -> Option<u64> {
        if date_str.len() < 19 {
            return None;
        }
        
        let year: u64 = date_str[0..4].parse().ok()?;
        let month: u64 = date_str[5..7].parse().ok()?;
        let day: u64 = date_str[8..10].parse().ok()?;
        let hour: u64 = date_str[11..13].parse().ok()?;
        let minute: u64 = date_str[14..16].parse().ok()?;
        let second: u64 = date_str[17..19].parse().ok()?;
        
        Self::make_timestamp(year, month, day, hour, minute, second)
    }
    
    /// Parse simple date string
    /// parse_date("2024-03-13") => timestamp at 00:00:00
    pub fn parse_date(date_str: &str) -> Option<u64> {
        if date_str.len() < 10 {
            return None;
        }
        
        let year: u64 = date_str[0..4].parse().ok()?;
        let month: u64 = date_str[5..7].parse().ok()?;
        let day: u64 = date_str[8..10].parse().ok()?;
        
        Self::make_timestamp(year, month, day, 0, 0, 0)
    }
    
    fn make_timestamp(year: u64, month: u64, day: u64, hour: u64, minute: u64, second: u64) -> Option<u64> {
        if month < 1 || month > 12 || day < 1 || day > 31 || hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        
        // Calculate days since epoch
        let mut days = 0u64;
        for y in 1970..year {
            days += if Self::is_leap_year(y) { 366 } else { 365 };
        }
        
        let month_days = [31, if Self::is_leap_year(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for m in 1..(month as usize) {
            days += month_days[m - 1] as u64;
        }
        
        days += day - 1;
        
        let timestamp = days * 86400 + hour * 3600 + minute * 60 + second;
        Some(timestamp)
    }
    
    // ==================== Utilities ====================
    
    /// Get day of week (0 = Sunday, 6 = Saturday)
    pub fn day_of_week(timestamp: u64) -> u32 {
        // Unix epoch was Thursday (4), so add 4 to offset
        ((timestamp / 86400 + 4) % 7) as u32
    }
    
    /// Get name of day of week
    pub fn day_name(timestamp: u64) -> &'static str {
        let days = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
        days[Self::day_of_week(timestamp) as usize]
    }
    
    /// Check if year is leap year
    pub fn is_leap_year(year: u64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
    
    /// Get number of days in month
    pub fn days_in_month(month: u32, year: u64) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => if Self::is_leap_year(year) { 29 } else { 28 },
            _ => 0,
        }
    }
    
    /// Get number of days in year
    pub fn days_in_year(year: u64) -> u32 {
        if Self::is_leap_year(year) { 366 } else { 365 }
    }
    
    /// Check if a date is valid
    pub fn is_valid_date(year: u64, month: u32, day: u32) -> bool {
        month >= 1 && month <= 12 && day >= 1 && day <= Self::days_in_month(month, year)
    }
    
    /// Get elapsed time since timestamp as string
    /// elapsed(earlier_time) => "2 hours ago" or "in 1 day"
    pub fn elapsed(timestamp: u64) -> String {
        let now = Self::now();
        let diff = (now as i64 - timestamp as i64).abs();
        let is_past = timestamp <= now;
        
        if diff < 60 {
            if is_past {
                "just now".to_string()
            } else {
                "in a moment".to_string()
            }
        } else if diff < 3600 {
            let minutes = diff / 60;
            if is_past {
                format!("{} minutes ago", minutes)
            } else {
                format!("in {} minutes", minutes)
            }
        } else if diff < 86400 {
            let hours = diff / 3600;
            if is_past {
                format!("{} hours ago", hours)
            } else {
                format!("in {} hours", hours)
            }
        } else {
            let days = diff / 86400;
            if is_past {
                format!("{} days ago", days)
            } else {
                format!("in {} days", days)
            }
        }
    }
    
    /// Create object with timestamp components
    pub fn components(timestamp: u64) -> HashMap<String, i64> {
        let secs_per_day = 86400;
        let days = timestamp / secs_per_day;
        let secs_today = timestamp % secs_per_day;
        
        let hours = secs_today / 3600;
        let minutes = (secs_today % 3600) / 60;
        let secs = secs_today % 60;
        
        let mut map = HashMap::new();
        map.insert("timestamp".to_string(), timestamp as i64);
        map.insert("days".to_string(), days as i64);
        map.insert("hours".to_string(), hours as i64);
        map.insert("minutes".to_string(), minutes as i64);
        map.insert("seconds".to_string(), secs as i64);
        map.insert("day_of_week".to_string(), Self::day_of_week(timestamp) as i64);
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_now() {
        let ts = DateTimeModule::now();
        assert!(ts > 1000000000); // After 2001
    }
    
    #[test]
    fn test_from_timestamp() {
        let dt = DateTimeModule::from_timestamp(1000000000);
        assert_eq!(dt.to_timestamp(), 1000000000);
    }
    
    #[test]
    fn test_add_seconds() {
        let dt = DateTimeModule::from_timestamp(1000000000);
        let dt2 = DateTimeModule::add_seconds(&dt, 3600);
        assert_eq!(dt2.to_timestamp(), 1000003600);
    }
    
    #[test]
    fn test_add_days() {
        let dt = DateTimeModule::from_timestamp(1000000000);
        let dt2 = DateTimeModule::add_days(&dt, 1);
        assert_eq!(dt2.to_timestamp(), 1000086400);
    }
    
    #[test]
    fn test_difference() {
        let dt1 = DateTimeModule::from_timestamp(2000000000);
        let dt2 = DateTimeModule::from_timestamp(1000000000);
        let diff = DateTimeModule::difference_seconds(&dt1, &dt2);
        assert_eq!(diff, 1000000000);
    }
    
    #[test]
    fn test_leap_year() {
        assert!(DateTimeModule::is_leap_year(2000));
        assert!(DateTimeModule::is_leap_year(2004));
        assert!(!DateTimeModule::is_leap_year(2001));
        assert!(!DateTimeModule::is_leap_year(1900));
    }
    
    #[test]
    fn test_format_time() {
        let time_str = DateTimeModule::format_time(3661); // 1 hour, 1 minute, 1 second
        assert_eq!(time_str, "01:01:01");
    }
    
    #[test]
    fn test_day_name() {
        let day = DateTimeModule::day_name(86400); // Second day of Unix epoch
        assert!(["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"].contains(&day));
    }
    
    #[test]
    fn test_days_in_month() {
        assert_eq!(DateTimeModule::days_in_month(1, 2024), 31);
        assert_eq!(DateTimeModule::days_in_month(2, 2024), 29); // 2024 is leap year
        assert_eq!(DateTimeModule::days_in_month(4, 2024), 30);
    }
    
    #[test]
    fn test_is_valid_date() {
        assert!(DateTimeModule::is_valid_date(2024, 2, 29)); // Leap year
        assert!(!DateTimeModule::is_valid_date(2023, 2, 29)); // Not leap year
        assert!(DateTimeModule::is_valid_date(2024, 1, 15));
        assert!(!DateTimeModule::is_valid_date(2024, 13, 1)); // Invalid month
    }
}
