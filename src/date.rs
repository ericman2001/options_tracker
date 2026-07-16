//! Minimal date utilities built on `std::time` only — no external date crate.
//!
//! Dates are stored throughout the application as ISO 8601 `YYYY-MM-DD` strings
//! (SQLite has no native date type; zero-padded ISO text sorts chronologically).
//! These helpers derive "today" and days-to-expiration from the system clock so
//! we can drive a non-blocking reconciliation alert without pulling in `chrono`
//! or `time`.
//!
//! Note: the civil date produced here is **UTC-based**. That is acceptable for a
//! best-effort, informational expiration alert; it is not used for anything that
//! requires local-timezone precision.

use std::time::{SystemTime, UNIX_EPOCH};

const SECONDS_PER_DAY: i64 = 86_400;

/// Returns the current UTC civil date formatted as zero-padded `YYYY-MM-DD`.
///
/// The result compares directly (lexicographically) against stored `expiration`
/// strings. If the system clock is set before the Unix epoch we clamp to day 0
/// (1970-01-01) rather than panicking.
pub fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(SECONDS_PER_DAY);
    let (y, m, d) = civil_from_days(days);
    format_ymd(y, m, d)
}

/// Formats a `(year, month, day)` triple as zero-padded `YYYY-MM-DD`.
pub fn format_ymd(year: i64, month: u32, day: u32) -> String {
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Converts a Unix day count (days since 1970-01-01) to a `(year, month, day)`
/// civil date using Howard Hinnant's `civil_from_days` algorithm (integer math
/// only, valid across the whole proleptic Gregorian range).
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01 so leap days fall at the end of the era.
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Converts a `(year, month, day)` civil date to a Unix day count (days since
/// 1970-01-01) using Howard Hinnant's `days_from_civil` algorithm. This is the
/// inverse of [`civil_from_days`].
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let m = month as i64;
    let d = day as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Parses an ISO 8601 `YYYY-MM-DD` string into a Unix day count. Returns `None`
/// if the string is not exactly `YYYY-MM-DD` with numeric parts.
pub fn parse_unix_day(date: &str) -> Option<i64> {
    if date.len() != 10 {
        return None;
    }
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse::<i64>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    let day = parts[2].parse::<u32>().ok()?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some(days_from_civil(year, month, day))
}

/// Number of days in a given month, accounting for leap years. Returns 0 for an
/// out-of-range month.
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Proleptic Gregorian leap-year test.
fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days-to-expiration: number of days from `today` (ISO) until `expiration`
/// (ISO). Negative when the expiration is in the past, `0` when it is today.
/// Returns `None` if either string cannot be parsed.
pub fn days_to_expiration(today: &str, expiration: &str) -> Option<i64> {
    Some(parse_unix_day(expiration)? - parse_unix_day(today)?)
}

/// Renders days-to-expiration as a short human label: `EXPIRED`, `expires today`,
/// or `N days`.
pub fn format_dte(dte: i64) -> String {
    match dte {
        d if d < 0 => "EXPIRED".to_string(),
        0 => "expires today".to_string(),
        1 => "1 day".to_string(),
        d => format!("{} days", d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_from_days_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        // 2000-03-01 is 11017 days after the epoch.
        assert_eq!(civil_from_days(11_017), (2000, 3, 1));
        // 2024-02-29 (leap day).
        assert_eq!(civil_from_days(19_782), (2024, 2, 29));
        assert_eq!(civil_from_days(19_783), (2024, 3, 1));
    }

    #[test]
    fn days_from_civil_known_dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        assert_eq!(days_from_civil(2024, 2, 29), 19_782);
    }

    #[test]
    fn round_trip_civil_days() {
        for z in [-3652, 0, 1, 999, 11_017, 19_782, 40_000, 50_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
    }

    #[test]
    fn format_ymd_zero_pads() {
        assert_eq!(format_ymd(2024, 1, 5), "2024-01-05");
        assert_eq!(format_ymd(999, 12, 31), "0999-12-31");
    }

    #[test]
    fn parse_unix_day_valid_and_invalid() {
        assert_eq!(parse_unix_day("1970-01-01"), Some(0));
        assert_eq!(parse_unix_day("2024-02-29"), Some(19_782));
        assert_eq!(parse_unix_day("2024-2-9"), None);
        assert_eq!(parse_unix_day("2024/02/09"), None);
        assert_eq!(parse_unix_day("not-a-date"), None);
        assert_eq!(parse_unix_day("2024-13-01"), None);
    }

    #[test]
    fn parse_unix_day_rejects_calendar_invalid_days() {
        // Day-vs-month validity, not just 1..=31.
        assert_eq!(parse_unix_day("2024-02-31"), None);
        assert_eq!(parse_unix_day("2024-04-31"), None);
        assert_eq!(parse_unix_day("2023-02-29"), None); // 2023 is not a leap year
        assert_eq!(parse_unix_day("2024-02-29"), Some(19_782)); // leap day is valid
        assert_eq!(parse_unix_day("2024-01-00"), None);
        assert_eq!(
            parse_unix_day("2024-04-30"),
            Some(days_from_civil(2024, 4, 30))
        );
    }

    #[test]
    fn dte_signs_and_labels() {
        assert_eq!(days_to_expiration("2024-01-01", "2024-01-10"), Some(9));
        assert_eq!(days_to_expiration("2024-01-10", "2024-01-10"), Some(0));
        assert_eq!(days_to_expiration("2024-01-10", "2024-01-01"), Some(-9));
        assert_eq!(format_dte(-1), "EXPIRED");
        assert_eq!(format_dte(0), "expires today");
        assert_eq!(format_dte(1), "1 day");
        assert_eq!(format_dte(5), "5 days");
    }

    #[test]
    fn today_is_well_formed() {
        let t = today();
        assert_eq!(t.len(), 10);
        assert!(parse_unix_day(&t).is_some());
    }
}
