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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ages_follow_the_oracle() {
        assert_eq!(age(100, None), Age::Missing);
        assert_eq!(age(100, Some(100)), Age::Now);
        assert_eq!(age(100, Some(58)), Age::Seconds(42));
        assert_eq!(age(10_000, Some(10_000 - 35 * 60)), Age::Minutes(35));
        assert_eq!(age(100_000, Some(100_000 - 7200)), Age::Hours(2));
        assert_eq!(age(1_000_000, Some(0)), Age::Days(11));
    }
}
