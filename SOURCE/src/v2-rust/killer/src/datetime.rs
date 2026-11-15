// src/v2-rust/killer/src/datetime.rs
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

    /// Civil date (year, month, day) from days since 1970-01-01 (proleptic Gregorian).
    fn civil_from_days(z: i64) -> (i64, i64, i64) {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (if m <= 2 { y + 1 } else { y }, m, d)
    }

    fn days_since_epoch(&self) -> i64 {
        self.seconds.div_euclid(86400)
    }

    /// Get year (0000-9999)
    pub fn year(&self) -> i32 {
        Self::civil_from_days(self.days_since_epoch()).0 as i32
    }

    /// Get month (1-12)
    pub fn month(&self) -> i32 {
        Self::civil_from_days(self.days_since_epoch()).1 as i32
    }

    /// Get day of month (1-31)
    pub fn day(&self) -> i32 {
        Self::civil_from_days(self.days_since_epoch()).2 as i32
    }

    /// Get day of year (1-366)
    pub fn day_of_year(&self) -> i64 {
        let days = self.days_since_epoch();
        let (y, _, _) = Self::civil_from_days(days);
        // days from 1970-01-01 to Jan 1 of year y
        let y1 = y - 1;
        let jan1 = (y - 1970) * 365 + (y1.div_euclid(4) - 1969 / 4) - (y1.div_euclid(100) - 1969 / 100)
            + (y1.div_euclid(400) - 1969 / 400);
        days - jan1 + 1
    }

    /// Get hour (0-23)
    pub fn hour(&self) -> i32 {
        (self.seconds.rem_euclid(86400) / 3600) as i32
    }

    /// Get minute (0-59)
    pub fn minute(&self) -> i32 {
        (self.seconds.rem_euclid(3600) / 60) as i32
    }

    /// Get second (0-59)
    pub fn second(&self) -> i32 {
        self.seconds.rem_euclid(60) as i32
    }

    /// Get millisecond (0-999)
    pub fn millisecond(&self) -> i32 {
        (self.nanos / 1_000_000) as i32
    }

    /// Get weekday (0=Monday, 6=Sunday)
    pub fn weekday(&self) -> i32 {
        // Unix epoch (Jan 1, 1970) was a Thursday (3)
        (self.days_since_epoch() + 3).rem_euclid(7) as i32
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

    if !(0..=9999).contains(&year) {
        return Err("Year out of range (0-9999)".to_string());
    }
    if !(1..=12).contains(&month) {
        return Err("Month out of range (1-12)".to_string());
    }
    let max_day = [31, if KillerDateTime::is_leap_year(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][(month - 1) as usize];
    if day < 1 || day > max_day {
        return Err("Day out of range for month".to_string());
    }
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return Err("Time out of range (HH 0-23, MM 0-59, SS 0-59)".to_string());
    }

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

#[cfg(test)]
mod parse_range_tests {
    use super::*;

    #[test]
    fn parse_datetime_rejects_out_of_range_fields() {
        for bad in ["2020-13-01", "2020-00-01", "2000000000-01-01", "2020-02-30", "2021-02-29",
                    "2020-01-00", "2020-01-32", "2020-01-01 24:00:00", "2020-01-01 00:60:00",
                    "2020-01-01 00:00:60", "-5-01-01", "10000-01-01"] {
            assert!(parse_datetime(bad).is_err(), "{bad} should be rejected");
        }
        assert!(parse_datetime("2020-02-29 23:59:59").is_ok());
        assert!(parse_datetime("1970-01-01").unwrap().seconds == 0);
    }
}

#[cfg(test)]
mod civil_tests {
    use super::*;

    fn ymd(secs: i64) -> (i32, i32, i32, i64, i32, i32) {
        let d = KillerDateTime { seconds: secs, nanos: 0 };
        (d.year(), d.month(), d.day(), d.day_of_year(), d.weekday(), d.hour())
    }

    #[test]
    fn known_timestamps() {
        assert_eq!(ymd(0), (1970, 1, 1, 1, 3, 0));
        assert_eq!(ymd(951_782_400), (2000, 2, 29, 60, 1, 0));
        assert_eq!(ymd(1_763_000_000), (2025, 11, 13, 317, 3, 02));
        assert_eq!(ymd(-86_400), (1969, 12, 31, 365, 2, 0));
        assert_eq!(ymd(-1), (1969, 12, 31, 365, 2, 23));
        assert_eq!(ymd(4_102_444_800), (2100, 1, 1, 1, 4, 0));
        assert_eq!(ymd(4_107_542_400), (2100, 3, 1, 60, 0, 0));
        assert_eq!(ymd(1_709_164_800), (2024, 2, 29, 60, 3, 0));
        assert_eq!(ymd(-2_208_988_800), (1900, 1, 1, 1, 0, 0));
    }

    #[test]
    fn negative_minute_second() {
        let d = KillerDateTime { seconds: -1, nanos: 0 };
        assert_eq!((d.minute(), d.second()), (59, 59));
    }
}
