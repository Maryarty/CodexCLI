//! Time as a fact the app is told (ADR-012, "time as a declared wake"): the shell reads the
//! wall clock and says it (`AppEv::Clock`); the app keeps it (`CodexApp::now`) and a feature
//! reads it (`AppView::now`, `AppFacts::now`). Nothing in the core reads a clock, and nothing
//! ticks on a schedule of its own: what wants to be woken says when (`Feature::wake_at`,
//! `Feature::thread_wake_at`), the app hands the nearest moment to the shell (`AppFx::wake`)
//! and the shell keeps one timer for it. A test says the time itself (the script's `clock`
//! step, an `AppEv::Clock` of a fake moment), so no test waits for a real second.

/// A moment on the wall clock: milliseconds since the Unix epoch, and the offset of the local
/// time zone from UTC at that moment (seconds east).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Now {
    millis: i64,
    utc_offset_secs: i32,
}

impl Now {
    /// The moment `millis` milliseconds after the epoch, in UTC.
    pub const fn at_millis(millis: i64) -> Self {
        Self {
            millis,
            utc_offset_secs: 0,
        }
    }

    /// The moment `secs` seconds after the epoch, in UTC.
    pub const fn at_secs(secs: i64) -> Self {
        Self::at_millis(secs.saturating_mul(1000))
    }

    /// The same moment seen in the zone `utc_offset_secs` east of UTC.
    pub const fn in_zone(mut self, utc_offset_secs: i32) -> Self {
        self.utc_offset_secs = utc_offset_secs;
        self
    }

    /// Milliseconds since the epoch.
    pub const fn millis(self) -> i64 {
        self.millis
    }

    /// Whole seconds since the epoch.
    pub const fn secs(self) -> i64 {
        self.millis.div_euclid(1000)
    }

    /// The local zone's offset from UTC, seconds east.
    pub const fn utc_offset_secs(self) -> i32 {
        self.utc_offset_secs
    }

    /// This moment `millis` milliseconds later, in the same zone.
    pub const fn plus_millis(self, millis: i64) -> Self {
        Self {
            millis: self.millis.saturating_add(millis),
            utc_offset_secs: self.utc_offset_secs,
        }
    }

    /// Whether the clock, at `self`, has reached `due`.
    pub const fn reached(self, due: Now) -> bool {
        self.millis >= due.millis
    }

    /// The start of the next whole minute: a screen that words ages in minutes is woken then.
    pub const fn next_minute(self) -> Self {
        let minute = self.millis.div_euclid(60_000);
        Self {
            millis: minute.saturating_add(1).saturating_mul(60_000),
            utc_offset_secs: self.utc_offset_secs,
        }
    }

    /// The earlier of two moments.
    pub const fn min(self, other: Now) -> Now {
        if other.millis < self.millis {
            other
        } else {
            self
        }
    }
}

/// The nearest moment the app wants the clock told, as of the end of a step: the moment, and
/// the app's own clock when it said so, so the shell sleeps the difference on its own clock
/// (a script that told the app another year does not make the shell sleep for it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wake {
    now: Now,
    at: Now,
}

impl Wake {
    /// The shortest wait a wake is given: a wake that is due already and still asked for (a
    /// feature whose intent did not clear it) is retried a second later, never at once.
    pub const MIN_WAIT_MILLIS: i64 = 1000;

    pub(crate) fn new(now: Now, at: Now) -> Self {
        let earliest = now.plus_millis(Self::MIN_WAIT_MILLIS);
        Self {
            now,
            at: if earliest.reached(at) { earliest } else { at },
        }
    }

    /// The moment the app wants to be told.
    pub fn at(&self) -> Now {
        self.at
    }

    /// How long after the step that moment is.
    pub fn after(&self) -> std::time::Duration {
        let millis = self.at.millis().saturating_sub(self.now.millis()).max(0);
        std::time::Duration::from_millis(u64::try_from(millis).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moment_counts_in_millis_and_secs() {
        let now = Now::at_secs(90).plus_millis(250).in_zone(3600);
        assert_eq!(now.millis(), 90_250);
        assert_eq!(now.secs(), 90);
        assert_eq!(now.utc_offset_secs(), 3600);
        assert!(now.reached(Now::at_secs(90)));
        assert!(!now.reached(Now::at_secs(91)));
    }

    #[test]
    fn the_next_minute_is_after_the_moment_even_on_the_boundary() {
        assert_eq!(Now::at_secs(59).next_minute(), Now::at_secs(60));
        assert_eq!(Now::at_secs(60).next_minute(), Now::at_secs(120));
        assert_eq!(Now::at_millis(-1).next_minute(), Now::at_secs(0));
    }

    #[test]
    fn a_wake_waits_the_gap_and_never_less_than_a_second() {
        let now = Now::at_secs(100);
        assert_eq!(
            Wake::new(now, Now::at_secs(130)).after(),
            std::time::Duration::from_secs(30)
        );
        assert_eq!(
            Wake::new(now, Now::at_secs(100)).after(),
            std::time::Duration::from_secs(1)
        );
        assert_eq!(
            Wake::new(now, Now::at_secs(5)).at(),
            Now::at_secs(101),
            "a moment already gone is asked for again a second later"
        );
    }

    #[test]
    fn the_earlier_moment_wins() {
        assert_eq!(Now::at_secs(5).min(Now::at_secs(3)), Now::at_secs(3));
        assert_eq!(Now::at_secs(3).min(Now::at_secs(5)), Now::at_secs(3));
    }
}
