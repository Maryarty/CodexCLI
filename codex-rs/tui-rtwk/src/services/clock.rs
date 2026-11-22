//! The clock the hosts read (R11: the clock is an input; only services read
//! it): the wall clock as unix seconds, and deadlines on the runtime's clock
//! (a load that gives up when its budget is spent: the usage page's chat
//! listing and estimates).

use std::time::Duration;

use tokio::time::Instant;

/// Seconds since the unix epoch, UTC; 0 for a clock set before it.
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0))
}

/// The instant `after` from now, for `tokio::time::timeout_at`.
pub fn deadline_in(after: Duration) -> Instant {
    Instant::now() + after
}

/// Whether `deadline` has come.
pub fn passed(deadline: Instant) -> bool {
    Instant::now() >= deadline
}
