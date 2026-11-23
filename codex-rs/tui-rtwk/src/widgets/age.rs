//! The relative age of a timestamp, one for every page that shows it
//! (oracle: `format_relative_time`); each page words it with its own strings.

/// An age.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Age {
    Missing,
    Now,
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
}

pub fn age(clock: i64, ts: Option<i64>) -> Age {
    let Some(ts) = ts else {
        return Age::Missing;
    };
    let seconds = (clock - ts).max(0);
    if seconds == 0 {
        return Age::Now;
    }
    if seconds < 60 {
        return Age::Seconds(seconds);
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return Age::Minutes(minutes);
    }
    let hours = minutes / 60;
    if hours < 24 {
        return Age::Hours(hours);
    }
    Age::Days(hours / 24)
}
