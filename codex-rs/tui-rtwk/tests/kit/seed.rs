//! Seeds: the state a screen test starts from, in three kinds, each one call
//! on the builder (`Open`).
//!
//! - `entry(dir, name)`: a screen's debug-menu entry, the screen over the
//!   fixture script its directory registered (the way the debug menu and the
//!   `screen` scenario step open it). The entry exists when the slice
//!   registered it.
//! - `script(jsonl)`: the live chat, fed the script's steps as a server would
//!   (`notify`, `request`, and the directories' `screen` events), then its
//!   `key` and `resize` steps. A `seed` step is skipped (the live app is
//!   already in its session).
//! - `live()`: the live chat as the session left it, nothing fed.

use codex_tui_core::app::{AppEv, BackendEv};
use codex_tui_core::fixtures::script::{self, Step};
use codex_tui_core::screens::seed_ev;

use super::Screen;
use super::verdict::{Owner, missing};
use crate::common::{Harness, ms};

#[derive(Clone, Copy)]
pub(super) enum Seed {
    Live,
    Entry {
        dir: &'static str,
        entry: &'static str,
    },
    Script(&'static str),
}

/// A screen test's start: who owns the screen, the terminal size, the seed.
pub struct Open {
    pub(super) owner: Owner,
    pub(super) width: u16,
    pub(super) height: u16,
    pub(super) seed: Seed,
}

impl Screen {
    /// Starts describing a screen test; `owner` is the slice that builds what
    /// the test opens (it names the pending when the seed does not exist yet).
    pub fn build(owner: Owner) -> Open {
        Open {
            owner,
            width: crate::common::WIDTH,
            height: crate::common::HEIGHT,
            seed: Seed::Live,
        }
    }
}

impl Open {
    /// The terminal size of the oracle test (columns, rows).
    pub fn size(mut self, width: u16, height: u16) -> Open {
        self.width = width;
        self.height = height;
        self
    }

    /// A screen directory's debug entry.
    pub fn entry(mut self, dir: &'static str, entry: &'static str) -> Open {
        self.seed = Seed::Entry { dir, entry };
        self
    }

    /// The live chat, fed a JSONL script (`include_str!` of a file under
    /// `tests/oracle/fixtures/`).
    pub fn script(mut self, jsonl: &'static str) -> Open {
        self.seed = Seed::Script(jsonl);
        self
    }

    /// The live chat as it starts.
    pub fn live(mut self) -> Open {
        self.seed = Seed::Live;
        self
    }

    pub fn open(self) -> Screen {
        match self.seed {
            Seed::Live => Screen::from(self.live_harness()),
            Seed::Entry { dir, entry } => self.open_entry(dir, entry),
            Seed::Script(text) => self.open_script(text),
        }
    }

    fn live_harness(&self) -> Harness {
        let mut h = crate::common::start_sized(false, self.width, self.height);
        for _ in 0..200 {
            if h.fake.epoch().is_some() {
                break;
            }
            h.t.pump_for(ms(20));
            std::thread::sleep(ms(5));
        }
        h
    }

    fn open_entry(self, dir: &str, entry: &str) -> Screen {
        let dirs = codex_tui_rtwk::screens::dirs();
        let Some(d) = dirs.iter().find(|d| d.name == dir) else {
            missing(self.owner, &format!("no screen directory `{dir}`"));
        };
        if !(d.debug_entries)().iter().any(|e| e.name == entry) {
            missing(
                self.owner,
                &format!("no debug entry `{dir}/{entry}`: the slice has not registered this state"),
            );
        }
        let mut h = crate::common::start_sized(true, self.width, self.height);
        h.open_debug_row(dir, Some(entry));
        h.t.pump_for(ms(800));
        Screen::from(h)
    }

    fn open_script(self, text: &str) -> Screen {
        let script = match script::parse(text) {
            Ok(script) => script,
            Err(err) => panic!("script line {}: {}", err.line, err.message),
        };
        let mut h = self.live_harness();
        let Some(epoch) = h.fake.epoch() else {
            panic!("the live app never connected");
        };
        let mut inputs = Vec::new();
        for (line, step) in script.steps {
            match step {
                Step::Seed(_) => {}
                Step::Screen { dir, ev } => match seed_ev(&dir, ev) {
                    Ok(ev) => h.fake.inject(epoch, AppEv::Screen(ev)),
                    Err(err) => panic!("script line {line}: screen `{dir}`: {err:?}"),
                },
                Step::Notify { message } => {
                    h.fake
                        .inject(epoch, AppEv::Backend(BackendEv::Notification(message)));
                }
                Step::Request { message } => {
                    h.fake
                        .inject(epoch, AppEv::Backend(BackendEv::Request(message)));
                }
                Step::Key { keys } => inputs.push(script::Input::Key(keys)),
                Step::Resize { width, height } => {
                    inputs.push(script::Input::Resize { width, height });
                }
            }
        }
        h.settle();
        h.play_inputs(&inputs);
        Screen::from(h)
    }
}
