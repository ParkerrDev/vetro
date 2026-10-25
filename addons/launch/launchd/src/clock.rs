//! Seconds since the epoch, to and from the UTC times Launch Library writes
//! ("2026-10-19T10:30:00Z", sometimes with a fraction or an offset).

use std::time::{SystemTime, UNIX_EPOCH};

const SECONDS_PER_DAY: i64 = 86_400;

/// The time now, in whole seconds since the epoch (0 if the clock is before it).
#[must_use]
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
        .unwrap_or(0)
}

/// Whole seconds from a float, where it is a time at all.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "checked to lie within i64's range first"
)]
pub fn whole_seconds(value: f64) -> Option<i64> {
    // 2^62 seconds is far past any date a launch has
    const LIMIT: f64 = 4.611_686_018_427_388e18;
    (value.is_finite() && value.abs() < LIMIT).then(|| value.trunc() as i64)
}

fn number(text: &str, from: usize, to: usize) -> Option<i64> {
    let digits = text.get(from..to)?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Days from 1970-01-01 to a civil date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    let shifted = if month <= 2 {
        year.checked_sub(1)?
    } else {
        year
    };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted.checked_sub(era.checked_mul(400)?)?;
    let month_index = if month > 2 {
        month.checked_sub(3)?
    } else {
        month.checked_add(9)?
    };
    let day_of_year = month_index
        .checked_mul(153)?
        .checked_add(2)?
        .checked_div(5)?
        .checked_add(day)?
        .checked_sub(1)?;
    let day_of_era = year_of_era
        .checked_mul(365)?
        .checked_add(year_of_era.checked_div(4)?)?
        .checked_sub(year_of_era.checked_div(100)?)?
        .checked_add(day_of_year)?;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

/// The offset from UTC at the end of a time, in seconds: "Z", "+HH:MM" or
/// "-HH:MM", after an optional fraction of a second.
fn offset(tail: &str) -> Option<i64> {
    let rest = tail.strip_prefix('.').map_or(tail, |fraction| {
        fraction.trim_start_matches(|c: char| c.is_ascii_digit())
    });
    if rest == "Z" || rest.is_empty() {
        return Some(0);
    }
    let sign = match rest.get(..1)? {
        "+" => 1,
        "-" => -1,
        _ => return None,
    };
    let hours = number(rest, 1, 3)?;
    let minutes = number(rest, 4, 6)?;
    hours
        .checked_mul(3600)?
        .checked_add(minutes.checked_mul(60)?)?
        .checked_mul(sign)
}

/// Seconds since the epoch for an ISO 8601 date and time, or None.
#[must_use]
pub fn parse(text: &str) -> Option<i64> {
    let (year, month, day) = (
        number(text, 0, 4)?,
        number(text, 5, 7)?,
        number(text, 8, 10)?,
    );
    let (hour, minute, second) = (
        number(text, 11, 13)?,
        number(text, 14, 16)?,
        number(text, 17, 19)?,
    );
    let separators = [(4, '-'), (7, '-'), (10, 'T'), (13, ':'), (16, ':')];
    if !separators
        .iter()
        .all(|&(at, want)| text.chars().nth(at) == Some(want))
    {
        return None;
    }
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    let clock = hour
        .checked_mul(3600)?
        .checked_add(minute.checked_mul(60)?)?
        .checked_add(second)?;
    days.checked_mul(SECONDS_PER_DAY)?
        .checked_add(clock)?
        .checked_sub(offset(text.get(19..)?)?)
}

/// A civil date from days since 1970-01-01 (the inverse of `days_from_civil`).
fn civil_from_days(days: i64) -> Option<(i64, i64, i64)> {
    let shifted = days.checked_add(719_468)?;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.checked_sub(era.checked_mul(146_097)?)?;
    let year_of_era = day_of_era
        .checked_sub(day_of_era.checked_div(1460)?)?
        .checked_add(day_of_era.checked_div(36_524)?)?
        .checked_sub(day_of_era.checked_div(146_096)?)?
        .checked_div(365)?;
    let day_of_year = day_of_era.checked_sub(
        year_of_era
            .checked_mul(365)?
            .checked_add(year_of_era.checked_div(4)?)?
            .checked_sub(year_of_era.checked_div(100)?)?,
    )?;
    let month_index = day_of_year
        .checked_mul(5)?
        .checked_add(2)?
        .checked_div(153)?;
    let day = day_of_year
        .checked_sub(
            month_index
                .checked_mul(153)?
                .checked_add(2)?
                .checked_div(5)?,
        )?
        .checked_add(1)?;
    let month = if month_index < 10 {
        month_index.checked_add(3)?
    } else {
        month_index.checked_sub(9)?
    };
    let year = year_of_era
        .checked_add(era.checked_mul(400)?)?
        .checked_add(i64::from(month <= 2))?;
    Some((year, month, day))
}

/// "2026-10-19T10:30:00Z" for seconds since the epoch.
#[must_use]
pub fn format(seconds: i64) -> Option<String> {
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let clock = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days)?;
    let (hour, minute, second) = (
        clock.checked_div(3600)?,
        clock.rem_euclid(3600).checked_div(60)?,
        clock.rem_euclid(60),
    );
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

#[cfg(test)]
mod tests {
    use super::{format, parse};

    #[test]
    fn reads_launch_library_times() {
        assert_eq!(parse("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse("2026-10-19T10:30:00Z"), Some(1_792_405_800));
        assert_eq!(parse("2026-10-19T10:30:00.123Z"), Some(1_792_405_800));
        assert_eq!(parse("2026-10-19T03:30:00-07:00"), Some(1_792_405_800));
        assert_eq!(parse("2024-02-29T12:00:00+00:00"), Some(1_709_208_000));
    }

    #[test]
    fn refuses_what_is_not_a_time() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("soon"), None);
        assert_eq!(parse("2026-13-01T00:00:00Z"), None);
        assert_eq!(parse("2026-10-19 10:30:00Z"), None);
        assert_eq!(parse("2026-10-19T10:30:00Q"), None);
    }

    #[test]
    fn writes_what_it_reads() {
        for seconds in [0, 1_792_405_800, 1_709_208_000, 951_782_400] {
            let text = format(seconds).unwrap_or_default();
            assert_eq!(parse(&text), Some(seconds), "{text}");
        }
        assert_eq!(
            format(1_792_405_800).as_deref(),
            Some("2026-10-19T10:30:00Z")
        );
    }
}
