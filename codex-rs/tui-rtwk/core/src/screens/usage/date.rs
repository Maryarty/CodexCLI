//! The calendar day and the RFC 3339 instant of the reports, without a date
//! crate: core takes no dependency beyond the protocol (`tests/rules/layers.rs`
//! `CORE_DEPS`). The oracle uses chrono's `NaiveDate`; this carries the few
//! methods its analytics code calls, under the same names, so the carried
//! code (normalize, tokens, the activity chart) reads the same.
//! Own function over carry (the person, 2026-11-18): civil-day arithmetic is
//! the proleptic Gregorian calendar of Howard Hinnant's `days_from_civil`.

use std::str::FromStr;

/// A day of the proleptic Gregorian calendar; ordered by date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NaiveDate {
    /// Days since 1970-01-01; the order and the arithmetic are on this.
    days: i64,
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn month_length(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        _ => 28,
    }
}

impl NaiveDate {
    pub fn from_ymd_opt(year: i32, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) || day == 0 || day > month_length(year, month) {
            return None;
        }
        Some(Self {
            days: days_from_civil(i64::from(year), i64::from(month), i64::from(day)),
        })
    }

    /// `YYYY-MM-DD`, the way the reports spell a day (chrono's `%Y-%m-%d`).
    pub fn parse_ymd(text: &str) -> Option<Self> {
        let mut parts = text.split('-');
        let (year, month, day) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return None;
        }
        Self::from_ymd_opt(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)
    }

    pub fn from_unix_secs(secs: i64) -> Self {
        Self {
            days: secs.div_euclid(86_400),
        }
    }

    pub fn ymd(self) -> (i32, u32, u32) {
        let (year, month, day) = civil_from_days(self.days);
        (year as i32, month as u32, day as u32)
    }

    pub fn year(self) -> i32 {
        self.ymd().0
    }

    pub fn month(self) -> u32 {
        self.ymd().1
    }

    pub fn day(self) -> u32 {
        self.ymd().2
    }

    /// 0 for Sunday: chrono's `weekday().num_days_from_sunday()`.
    pub fn weekday_from_sunday(self) -> u32 {
        // 1970-01-01 was a Thursday.
        (self.days + 4).rem_euclid(7) as u32
    }

    pub fn plus_days(self, days: i64) -> Self {
        Self {
            days: self.days + days,
        }
    }

    pub fn minus_days(self, days: i64) -> Self {
        self.plus_days(-days)
    }

    pub fn checked_sub_days(self, days: u64) -> Option<Self> {
        i64::try_from(days).ok().map(|days| self.minus_days(days))
    }

    /// `self`, the next day, and so on: chrono's `iter_days`.
    pub fn iter_days(self) -> impl Iterator<Item = NaiveDate> {
        (0..).map(move |offset| self.plus_days(offset))
    }

    /// Days from `earlier` to `self`.
    pub fn days_since(self, earlier: NaiveDate) -> i64 {
        self.days - earlier.days
    }
}

impl FromStr for NaiveDate {
    type Err = ();

    fn from_str(text: &str) -> Result<Self, ()> {
        Self::parse_ymd(text).ok_or(())
    }
}

impl<'de> serde::Deserialize<'de> for NaiveDate {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse_ymd(&text).ok_or_else(|| serde::de::Error::custom("a date as YYYY-MM-DD"))
    }
}

impl std::fmt::Display for NaiveDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (year, month, day) = self.ymd();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

/// The Unix seconds of an RFC 3339 instant (`2026-01-09T10:20:30Z`, a
/// fraction, `+02:00`): chrono's `DateTime::parse_from_rfc3339(..).timestamp()`.
pub fn rfc3339_unix(text: &str) -> Option<i64> {
    let (date, rest) = text.split_once(['T', 't', ' '])?;
    let day = NaiveDate::parse_ymd(date)?;
    let at = rest.find(['Z', 'z', '+']).or_else(|| rest.rfind('-'))?;
    let (clock, offset) = rest.split_at(at);
    let mut parts = clock.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = parts.next()?.split('.').next()?.parse().ok()?;
    if parts.next().is_some() || hours > 23 || minutes > 59 || seconds > 60 {
        return None;
    }
    let shift = match offset.as_bytes().first()? {
        b'Z' | b'z' => 0,
        sign @ (b'+' | b'-') => {
            let (h, m) = offset[1..].split_once(':')?;
            let secs = h.parse::<i64>().ok()? * 3600 + m.parse::<i64>().ok()? * 60;
            if *sign == b'+' { secs } else { -secs }
        }
        _ => return None,
    };
    Some(day.days * 86_400 + hours * 3600 + minutes * 60 + seconds - shift)
}
