//! Waiting for the app to apply what another thread sent it.
//!
//! The shell runs on a tokio thread and hands its answers to the app through
//! `AppCx::subscribe` / `AppCx::channel`: an item is applied at the next
//! flush after it was sent, and how many frames that takes depends on the
//! machine. A test that pumps a fixed span of the fake clock after a key or an
//! event, then reads the screen, measures its own luck. The test names what
//! it waits for instead (a screen text, a page, a counter of the app) and
//! `wait_until` runs frames on the fake clock, between them a real pause for
//! the other threads, until that holds, with a real deadline.
//!
//! One file, a module in each test binary that has a live app (`common`,
//! `shell_agents`): a binary implements `Pumped` for its harness.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::time::{Duration, Instant};

use rtwk_test::Tester;

/// A live app on an in-memory terminal, with other threads that answer it.
pub trait Pumped {
    fn tester(&mut self) -> &mut Tester;

    /// Gives the other threads a turn (a real pause on a shared runtime).
    fn pause(&mut self);
}

/// The slice of fake time of one frame round.
const FRAME: Duration = Duration::from_millis(20);

/// How long the others may take, in real time: far above any loaded machine,
/// so a panic means the thing never happened, not that it was slow.
const DEADLINE: Duration = Duration::from_secs(30);

/// Runs frames until `ready` holds (it is asked after each frame round, with
/// the harness, so it reads the screen, a page of the tree or the app).
/// Panics with `what` and the screen when `DEADLINE` of real time passes.
pub fn wait_until<S: Pumped>(s: &mut S, what: &str, mut ready: impl FnMut(&mut S) -> bool) {
    let started = Instant::now();
    loop {
        s.tester().pump_for(FRAME);
        if ready(s) {
            return;
        }
        assert!(
            started.elapsed() < DEADLINE,
            "never saw {what} in {DEADLINE:?}; the screen:\n{}",
            s.tester().screen()
        );
        s.pause();
    }
}
