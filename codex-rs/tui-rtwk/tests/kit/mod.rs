//! The screen-test kit: black-box tests of a screen against the oracle.
//!
//! A test seeds a screen (`Screen::build(owner).size(w, h).entry(..)` /
//! `.script(..)` / `.live()` then `.open()`), presses keys or uses the mouse,
//! and compares the rendered rows with the oracle's insta snapshot at the
//! oracle's own size. Nothing else is reachable from a test: not core's model,
//! not the widgets. A test written this way survives any rework of core or of
//! the view. The recipe: `docs/rtwk-port/refs/screen-tests.md`.
//!
//! A test binary that uses the kit has `mod common; mod kit;` at its root.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

#[allow(clippy::duplicate_mod)] // one helper, a module in each binary
#[path = "../oracle_snap/mod.rs"]
mod oracle_snap;

mod compare;
mod seed;
mod verdict;

#[allow(unused_imports)]
pub use compare::{normalize_labels, snap_size};
#[allow(unused_imports)]
pub use verdict::{Owner, missing, na, pending, todo};

use rtwk_core::input::{Mods, MouseKind};

use crate::common::{Harness, ms};

/// The screen under test.
pub struct Screen {
    h: Harness,
}

impl From<Harness> for Screen {
    fn from(h: Harness) -> Screen {
        Screen { h }
    }
}

impl Screen {
    // ---- keys ----------------------------------------------------------

    /// Presses keys named as bindings name them, apart by spaces: `down`,
    /// `enter`, `esc`, `ctrl+c`, `shift+tab`, `alt+b`, `A`, `down down enter`.
    pub fn keys(&mut self, keys: &str) -> &mut Screen {
        self.h.press(keys);
        self
    }

    /// Types text as a keyboard does (each char a key press).
    pub fn type_text(&mut self, text: &str) -> &mut Screen {
        self.h.t.type_text(text);
        self.h.t.pump_for(ms(100));
        self
    }

    /// Pastes text as a terminal with bracketed paste does.
    pub fn paste(&mut self, text: &str) -> &mut Screen {
        self.h.t.paste(text);
        self.h.t.pump_for(ms(100));
        self
    }

    // ---- mouse (0-based cell column x, row y) ---------------------------

    pub fn click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.h.t.click(x, y);
        self.h.t.pump_for(ms(100));
        self
    }

    pub fn double_click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.h.t.click(x, y);
        self.h.t.click(x, y);
        self.h.t.pump_for(ms(100));
        self
    }

    /// Clicks the first cell of the first occurrence of `text` on the screen
    /// (columns counted in chars: ASCII before it).
    pub fn click_text(&mut self, text: &str) -> &mut Screen {
        let (x, y) = self.find(text);
        self.click(x, y)
    }

    pub fn double_click_text(&mut self, text: &str) -> &mut Screen {
        let (x, y) = self.find(text);
        self.double_click(x, y)
    }

    pub fn wheel_up(&mut self, x: u16, y: u16, notches: u16) -> &mut Screen {
        self.wheel(MouseKind::WheelUp, x, y, notches)
    }

    pub fn wheel_down(&mut self, x: u16, y: u16, notches: u16) -> &mut Screen {
        self.wheel(MouseKind::WheelDown, x, y, notches)
    }

    fn wheel(&mut self, kind: MouseKind, x: u16, y: u16, notches: u16) -> &mut Screen {
        for _ in 0..notches {
            self.h.t.mouse(kind, x, y, 0, Mods::NONE);
        }
        self.h.t.pump_for(ms(100));
        self
    }

    fn find(&self, text: &str) -> (u16, u16) {
        for (y, row) in self.h.t.lines().iter().enumerate() {
            if let Some(byte) = row.find(text) {
                let x = row[..byte].chars().count();
                return (
                    u16::try_from(x).unwrap_or(u16::MAX),
                    u16::try_from(y).unwrap_or(u16::MAX),
                );
            }
        }
        panic!("no `{text}` on the screen:\n{}", self.h.t.screen());
    }

    // ---- terminal and time ----------------------------------------------

    pub fn resize(&mut self, width: u16, height: u16) -> &mut Screen {
        self.h.t.resize(width, height);
        self.h.t.pump_for(ms(100));
        self
    }

    /// Lets `millis` pass (frames run on the fake clock).
    pub fn wait(&mut self, millis: u64) -> &mut Screen {
        self.h.t.pump_for(ms(millis));
        self
    }

    /// Waits until `text` is on the screen (real time for the backend thread);
    /// panics with the screen when it never shows.
    pub fn wait_for(&mut self, text: &str) -> &mut Screen {
        let text = text.to_string();
        self.h.wait_for(&format!("`{text}`"), |s| s.contains(&text));
        self
    }

    // ---- reading ----------------------------------------------------------

    /// The rows, trailing blanks of each cut.
    pub fn rows(&self) -> Vec<String> {
        self.h
            .t
            .lines()
            .iter()
            .map(|r| r.trim_end().to_string())
            .collect()
    }

    /// The screen as text, for a failure message.
    pub fn text(&self) -> String {
        self.h.t.screen()
    }

    // ---- comparing with the oracle ------------------------------------------

    /// The whole screen equals the oracle snapshot `name` (its test name, the
    /// tail after `__tests__`, or its whole file stem).
    pub fn assert_snap(&self, name: &str) {
        compare::assert_snap(&self.rows(), name);
    }

    /// The snapshot is one region of the oracle's screen: its rows equal ours
    /// from screen row `row`, each without its first `col` columns.
    pub fn assert_snap_region(&self, name: &str, row: usize, col: usize) {
        compare::assert_snap_region(&self.rows(), name, row, col);
    }

    /// The whole screen equals the `--- section ---` of a multi-state snapshot.
    pub fn assert_snap_section(&self, name: &str, section: &str) {
        compare::assert_snap_section(&self.rows(), name, section);
    }

    // ---- an oracle assertion's literal ------------------------------------------

    /// The oracle test asserts the screen holds `text` (copy its literal).
    pub fn assert_contains(&self, text: &str) {
        assert!(
            self.text().contains(text),
            "the screen has no `{text}`:\n{}",
            self.text()
        );
    }

    /// The oracle test asserts the screen holds no `text`.
    pub fn assert_absent(&self, text: &str) {
        assert!(
            !self.text().contains(text),
            "the screen still has `{text}`:\n{}",
            self.text()
        );
    }

    /// The oracle test asserts row `row` (0-based) reads exactly `text`.
    pub fn assert_row(&self, row: usize, text: &str) {
        let rows = self.rows();
        let got = rows.get(row).map(String::as_str).unwrap_or("");
        pretty_assertions::assert_eq!(text, got, "row {row} of the screen:\n{}", self.text());
    }
}
