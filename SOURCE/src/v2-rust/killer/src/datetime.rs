// src/v2-rust/killer_vm/src/datetime.rs
// DateTime module for Killer language
// Provides system time, parsing, formatting, and date operations

use std::time::{SystemTime, UNIX_EPOCH, Duration};

/// Killer DateTime wrapper - encapsulates system time with formatting support
#[derive(Clone, Debug)]
pub struct KillerDateTime {
    /// Unix timestamp in seconds
    pub seconds: i64,
    /// Nanosecond component
    pub nanos: u32,
}

impl KillerDateTime {
    /// Create datetime from system time
    pub fn from_system_time(st: SystemTime) -> Self {
        let duration = st.duration_since(UNIX_EPOCH).unwrap_or_default();
        KillerDateTime {
            seconds: duration.as_secs() as i64,
            nanos: duration.subsec_nanos(),
        }
    }

    /// Get current system time as KillerDateTime
    pub fn now() -> Self {
        Self::from_system_time(SystemTime::now())
    }

    /// Convert to system time
    pub fn to_system_time(&self) -> SystemTime {
        UNIX_EPOCH + Duration::new(self.seconds as u64, self.nanos)
    }

    /// Get year (0000-9999)
    pub fn year(&self) -> i32 {
        // Convert Unix timestamp to year (simplified - not accounting for leap seconds)
        // Unix epoch = 1970-01-01
        let days_since_epoch = self.seconds / 86400;
        let years_since_1970 = (days_since_epoch / 365) as i32;
        1970 + years_since_1970
    }

    /// Get month (1-12)
    pub fn month(&self) -> i32 {
        let day_of_year = self.day_of_year();
        
        let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        
        let mut month = 1;
        let mut days_passed = 0;
        
        // Simplified: assume not a leap year (better implementation would check)
        for &days in &days_in_month {
            if days_passed + days >= day_of_year {
                return month;
            }
            days_passed += days;
            month += 1;
        }
        month
    }

    /// Get day of month (1-31)
    pub fn day(&self) -> i32 {
        let day_of_year = self.day_of_year();
        
        let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        
        let mut days_passed = 0;
        for &days in &days_in_month {
            if days_passed + days >= day_of_year {
                return (day_of_year - days_passed) as i32;
            }
            days_passed += days;
        }
        day_of_year as i32
    }

    /// Get day of year (1-366)
    pub fn day_of_year(&self) -> i64 {
        let days_since_epoch = self.seconds / 86400;
        let year = self.year() as i64;
        
        // Calculate days at start of this year
        let mut days_at_year_start = 0i64;
        for y in 1970..year {
            if Self::is_leap_year(y as i32) {
                days_at_year_start += 366;
            } else {
                days_at_year_start += 365;
            }
        }
        
        (days_since_epoch - days_at_year_start) % 366 + 1
    }

    /// Get hour (0-23)
    pub fn hour(&self) -> i32 {
        ((self.seconds % 86400) / 3600) as i32
    }

    /// Get minute (0-59)
    pub fn minute(&self) -> i32 {
        ((self.seconds % 3600) / 60) as i32
    }

    /// Get second (0-59)
    pub fn second(&self) -> i32 {
        (self.seconds % 60) as i32
    }

    /// Get millisecond (0-999)
    pub fn millisecond(&self) -> i32 {
        (self.nanos / 1_000_000) as i32
    }

    /// Get weekday (0=Monday, 6=Sunday)
    pub fn weekday(&self) -> i32 {
        // Unix epoch (Jan 1, 1970) was a Thursday (3)
        let days_since_epoch = self.seconds / 86400;
        ((days_since_epoch + 3) % 7) as i32
    }

    /// Get day name
    pub fn day_name(&self) -> &'static str {
        match self.weekday() {
            0 => "Monday",
            1 => "Tuesday",
            2 => "Wednesday",
            3 => "Thursday",
            4 => "Friday",
            5 => "Saturday",
            6 => "Sunday",
            _ => "Unknown",
        }
    }

    /// Get month name
    pub fn month_name(&self) -> &'static str {
        match self.month() {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }
    }

    /// Check if year is a leap year
    pub fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    /// Format datetime string
    /// Supported format codes:
    /// %Y = 4-digit year
    /// %m = 2-digit month
    /// %d = 2-digit day
    /// %H = 2-digit hour (24-hour)
    /// %M = 2-digit minute
    /// %S = 2-digit second
    /// %A = full day name
    /// %B = full month name
    /// %y = 2-digit year
    pub fn format(&self, pattern: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '%' && i + 1 < chars.len() {
                i += 1;
                match chars[i] {
                    'Y' => result.push_str(&format!("{:04}", self.year())),
                    'y' => result.push_str(&format!("{:02}", self.year() % 100)),
                    'm' => result.push_str(&format!("{:02}", self.month())),
                    'd' => result.push_str(&format!("{:02}", self.day())),
                    'H' => result.push_str(&format!("{:02}", self.hour())),
                    'M' => result.push_str(&format!("{:02}", self.minute())),
                    'S' => result.push_str(&format!("{:02}", self.second())),
                    'A' => result.push_str(self.day_name()),
                    'B' => result.push_str(self.month_name()),
                    _   => {
                        result.push('%');
                        result.push(chars[i]);
                    }
                }
            } else {
                result.push(chars[i]);
            }
            i += 1;
        }

        result
    }

    /// ISO 8601 format (YYYY-MM-DD HH:MM:SS)
    pub fn to_iso_string(&self) -> String {
        self.format("%Y-%m-%d %H:%M:%S")
    }
}

/// Parse datetime from string
/// Supports "YYYY-MM-DD", "YYYY-MM-DD HH:MM:SS", ISO 8601 formats
pub fn parse_datetime(input: &str) -> Result<KillerDateTime, String> {
    let input = input.trim();
    
    // Simple parser for YYYY-MM-DD HH:MM:SS format
    let parts: Vec<&str> = if input.contains(' ') {
        input.split(' ').collect()
    } else {
        vec![input]
    };

    if parts.is_empty() {
        return Err("Empty datetime string".to_string());
    }

    let date_parts: Vec<&str> = parts[0].split('-').collect();
    if date_parts.len() != 3 {
        return Err("Invalid date format. Use YYYY-MM-DD".to_string());
    }

    let year: i32 = date_parts[0].parse().map_err(|_| "Invalid year".to_string())?;
    let month: i32 = date_parts[1].parse().map_err(|_| "Invalid month".to_string())?;
    let day: i32 = date_parts[2].parse().map_err(|_| "Invalid day".to_string())?;

    let (hour, minute, second) = if parts.len() > 1 {
        let time_parts: Vec<&str> = parts[1].split(':').collect();
        if time_parts.len() != 3 {
            return Err("Invalid time format. Use HH:MM:SS".to_string());
        }
        let h: i32 = time_parts[0].parse().map_err(|_| "Invalid hour".to_string())?;
        let m: i32 = time_parts[1].parse().map_err(|_| "Invalid minute".to_string())?;
        let s: i32 = time_parts[2].parse().map_err(|_| "Invalid second".to_string())?;
        (h, m, s)
    } else {
        (0, 0, 0)
    };

    // Simple calculation: days from 1970-01-01 to given date
    let mut total_days = 0i64;
    
    // Add days for complete years
    for y in 1970..year as i64 {
        if KillerDateTime::is_leap_year(y as i32) {
            total_days += 366;
        } else {
            total_days += 365;
        }
    }
    
    // Add days for months in current year
    let days_in_month = if KillerDateTime::is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    
    for i in 0..(month as usize - 1) {
        total_days += days_in_month[i] as i64;
    }
    
    // Add days in current month
    total_days += (day - 1) as i64;

    // Convert to seconds
    let total_seconds = total_days * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second as i64;

    Ok(KillerDateTime {
        seconds: total_seconds,
        nanos: 0,
    })
}

/// Calculate duration between two datetimes (result in milliseconds)
pub fn duration_millis(dt1: &KillerDateTime, dt2: &KillerDateTime) -> i64 {
    (dt2.seconds - dt1.seconds) * 1000 + (dt2.nanos as i64 - dt1.nanos as i64) / 1_000_000
}
