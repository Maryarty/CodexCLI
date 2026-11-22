//! The viewer's wall clock for the rows that print a time (the completion separator, oracle
//! `history_cell/separators.rs` with `clock_format.rs`): a host fact, as the oracle reads it once
//! from the machine. The detection itself (macOS locale, Windows short time, Linux `LC_TIME`) is
//! IO and belongs to the host; the model keeps its result.

use serde::Deserialize;

/// 12-hour or 24-hour time. Unknown preferences keep the oracle's 12-hour output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockFormat {
    #[default]
    TwelveHour,
    TwentyFourHour,
}

/// What the host knows about the viewer's clock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewerClock {
    pub format: ClockFormat,
    /// Seconds east of UTC the times are read in. None is the process's local zone, which keeps
    /// the zone's own daylight-saving changes; a fixed offset is for a script, so its labels do
    /// not depend on the machine the test runs on.
    pub utc_offset: Option<i32>,
}
