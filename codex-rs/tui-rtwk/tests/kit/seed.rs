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
//!
//! Options on any seed: `early()` stops at the first frame (a loading state
//! before the data arrives), `keymap(..)` remaps keys. After opening,
//! `Screen::feed(jsonl)` applies more script steps to the open screen
//! (history prepended, a tail absorbed, a result arriving). An `entry` screen
//! plays its fixture in a model of its own: `feed` is for the live app.

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
    pub(super) early: bool,
    pub(super) keymap: Option<&'static [(&'static str, &'static str)]>,
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
            early: false,
            keymap: None,
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

    /// Stops at the screen's first frame: the state before the data arrives
    /// (`Loading sessions…` before the list). `Screen::wait_for(..)` runs on.
    pub fn early(mut self) -> Open {
        self.early = true;
        self
    }

    /// The user's key bindings: `(action, key)` pairs as the keymap config
    /// names them, `&[("list.move_down", "z")]`.
    pub fn keymap(mut self, bindings: &'static [(&'static str, &'static str)]) -> Open {
        self.keymap = Some(bindings);
        self
    }

    pub fn open(self) -> Screen {
        if self.keymap.is_some() {
            // The port has no keymap config yet (`src/keymap` is empty): the
            // test is written in full under this pending.
            missing(
                Owner::ReuseCarry,
                "no keymap seed: the port reads no key bindings from a config yet (the keymap slice); the test's remapped keys cannot be set",
            );
        }
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
        h.t.pump_for(ms(if self.early { 20 } else { 800 }));
        Screen {
            entry: true,
            ..Screen::from(h)
        }
    }

    fn open_script(self, text: &str) -> Screen {
        let mut h = self.live_harness();
        feed(&mut h, text);
        Screen::from(h)
    }
}

impl Screen {
    /// Applies more steps of a JSONL script to the open screen, as a server
    /// and the user would deliver them: `notify`, `request`, a directory's
    /// `screen` events, then its `key` and `resize` steps.
    pub fn feed(&mut self, jsonl: &str) -> &mut Screen {
        assert!(
            !self.entry,
            "feed works on a script() or live() screen: a debug entry's screen plays its own fixture, the app's events do not reach it"
        );
        feed(&mut self.h, jsonl);
        self
    }
}

fn feed(h: &mut Harness, text: &str) {
    let script = match script::parse(text) {
        Ok(script) => script,
        Err(err) => panic!("script line {}: {}", err.line, err.message),
    };
    for _ in 0..200 {
        if h.fake.epoch().is_some() {
            break;
        }
        h.t.pump_for(ms(20));
        std::thread::sleep(ms(5));
    }
    let Some(epoch) = h.fake.epoch() else {
        panic!("the app never connected");
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
}
