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
mod grid;
mod labels;
mod seed;
mod verdict;

#[allow(unused_imports)]
pub use compare::{Want, assert_snap as assert_snap_on, snap_frames, snap_rows, snap_size};
#[allow(unused_imports)]
pub use grid::{Grid, check_style, color_name, modifier_name};
#[allow(unused_imports)]
pub use labels::{normalize as normalize_rows, normalize_key_names, normalize_labels};
#[allow(unused_imports)]
pub use verdict::{Owner, missing, na, pending, todo};

use rtwk_core::input::{Mods, MouseKind};

use crate::common::Harness;

fn to_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The bytes of a repeated key in the kitty keyboard protocol
/// (`CSI code;modifiers:event u`, event 2 is a repeat).
fn repeat_bytes(key: &str) -> String {
    match key {
        "up" => "\x1b[1;1:2A".to_string(),
        "down" => "\x1b[1;1:2B".to_string(),
        "right" => "\x1b[1;1:2C".to_string(),
        "left" => "\x1b[1;1:2D".to_string(),
        "enter" => "\x1b[13;1:2u".to_string(),
        "esc" => "\x1b[27;1:2u".to_string(),
        "tab" => "\x1b[9;1:2u".to_string(),
        one if one.chars().count() == 1 => {
            let code = one.chars().next().map_or(0, u32::from);
            format!("\x1b[{code};1:2u")
        }
        other => panic!("key_repeat(`{other}`): one char, or up down left right enter esc tab"),
    }
}

/// The screen under test.
pub struct Screen {
    h: Harness,
    /// Opened from a debug entry: the screen has a model of its own, which
    /// the app's events do not reach (`feed` cannot change it).
    entry: bool,
}

impl From<Harness> for Screen {
    fn from(h: Harness) -> Screen {
        Screen { h, entry: false }
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
        self.h.advance(100);
        self
    }

    /// Pastes text as a terminal with bracketed paste does.
    pub fn paste(&mut self, text: &str) -> &mut Screen {
        self.h.t.paste(text);
        self.h.advance(100);
        self
    }

    // ---- mouse (0-based cell column x, row y) ---------------------------

    pub fn click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.h.t.click(x, y);
        self.h.advance(100);
        self
    }

    pub fn double_click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.h.t.click(x, y);
        self.h.t.click(x, y);
        self.h.advance(100);
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

    /// Presses the left button at (x, y), moves to (x2, y2) with it held, and
    /// releases there: a drag gesture.
    pub fn drag(&mut self, x: u16, y: u16, x2: u16, y2: u16) -> &mut Screen {
        self.mouse_down(x, y).mouse_drag(x2, y2).mouse_up(x2, y2)
    }

    /// The parts of a drag, for a test that looks between them.
    pub fn mouse_down(&mut self, x: u16, y: u16) -> &mut Screen {
        self.mouse_at(MouseKind::Down, x, y)
    }

    pub fn mouse_drag(&mut self, x: u16, y: u16) -> &mut Screen {
        self.mouse_at(MouseKind::Drag, x, y)
    }

    pub fn mouse_up(&mut self, x: u16, y: u16) -> &mut Screen {
        self.mouse_at(MouseKind::Up, x, y)
    }

    fn mouse_at(&mut self, kind: MouseKind, x: u16, y: u16) -> &mut Screen {
        self.h.t.mouse(kind, x, y, 0, Mods::NONE);
        self.h.advance(100);
        self
    }

    /// Delivers a key as the terminal reports it held down (the kitty
    /// protocol's repeat event): `k`, `down`, `enter`, `esc`, `tab`. One
    /// press of it is `keys`.
    pub fn key_repeat(&mut self, key: &str) -> &mut Screen {
        let bytes = repeat_bytes(key);
        self.h.t.input(bytes.as_bytes());
        self.h.advance(100);
        self
    }

    fn wheel(&mut self, kind: MouseKind, x: u16, y: u16, notches: u16) -> &mut Screen {
        for _ in 0..notches {
            self.h.t.mouse(kind, x, y, 0, Mods::NONE);
        }
        self.h.advance(100);
        self
    }

    fn find(&self, text: &str) -> (u16, u16) {
        match self.grid().locate(text) {
            Some((x, _, y)) => (to_u16(x), to_u16(y)),
            None => panic!("no `{text}` on the screen:\n{}", self.h.t.screen()),
        }
    }

    pub(crate) fn grid(&self) -> Grid {
        self.h.t.with_screen(Grid::of)
    }

    // ---- terminal and time ----------------------------------------------

    pub fn resize(&mut self, width: u16, height: u16) -> &mut Screen {
        self.h.t.resize(width, height);
        self.h.advance(100);
        self
    }

    /// Lets `millis` pass (frames run on the fake clock).
    pub fn wait(&mut self, millis: u64) -> &mut Screen {
        self.h.advance(millis);
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

    /// What the app wrote to the terminal as the window title (OSC 0 or 2),
    /// the last one; `None` when it wrote none.
    pub fn title(&self) -> Option<String> {
        title_in(&self.h.t.written())
    }

    /// A copy of the screen's rows now, to compare a later screen with
    /// (`assert_same_rows`, `assert_rows_differ`).
    pub fn shot(&self) -> Shot {
        Shot(self.rows())
    }

    // ---- comparing with the oracle ------------------------------------------

    /// Starts a comparison with the oracle snapshot `name` (its test name or
    /// whole file stem), refined by `.frame(..)` and `.at(..)`, run by
    /// `.assert()`. The rows must equal the oracle's; where the snapshot is a
    /// `Buffer` dump its cell styles must too, and where it has a `Cursor:`
    /// line the cursor.
    pub fn snap<'a>(&'a self, name: &'a str) -> Snap<'a> {
        Snap {
            screen: self,
            name,
            frame: "",
            at: None,
        }
    }

    /// The whole screen equals the oracle snapshot `name`.
    pub fn assert_snap(&self, name: &str) {
        self.snap(name).assert();
    }

    /// The snapshot is one region of the oracle's screen: its rows equal ours
    /// from screen row `row`, each without its first `col` columns.
    pub fn assert_snap_region(&self, name: &str, row: usize, col: usize) {
        self.snap(name).at(row, col).assert();
    }

    /// The whole screen equals one frame of a snapshot the oracle joined of
    /// several: `--- x ---`, `---- x ----`, `N columns:`, `width=N..`, or a
    /// label above a `Buffer {` dump; `section` is the header text without the
    /// dashes or the colon (`loading`, `124 columns`), `name#2` for the second
    /// frame of the same header.
    pub fn assert_snap_section(&self, name: &str, section: &str) {
        self.snap(name).frame(section).assert();
    }

    // ---- an oracle assertion's literal ------------------------------------------

    /// The oracle test asserts the screen holds `text` (copy its literal).
    pub fn assert_contains(&self, text: &str) {
        self.grid().assert_contains(text);
    }

    /// The oracle test asserts the screen holds no `text`.
    pub fn assert_absent(&self, text: &str) {
        self.grid().assert_absent(text);
    }

    /// The oracle test asserts row `row` (0-based) reads exactly `text`.
    pub fn assert_row(&self, row: usize, text: &str) {
        self.grid().assert_row(row, text);
    }

    /// The oracle asserts some row reads exactly `text` (trailing blanks
    /// aside).
    pub fn assert_has_row(&self, text: &str) {
        self.grid().assert_has_row(text);
    }

    /// The oracle asserts row `row` starts with `text`.
    pub fn assert_row_starts_with(&self, row: usize, text: &str) {
        self.grid().assert_row_starts_with(row, text);
    }

    /// The oracle asserts row `row` ends with `text` (trailing blanks aside).
    pub fn assert_row_ends_with(&self, row: usize, text: &str) {
        self.grid().assert_row_ends_with(row, text);
    }

    /// The oracle asserts the fragments appear on the screen in this order
    /// (each after the one before it, reading rows top to bottom).
    pub fn assert_in_order(&self, fragments: &[&str]) {
        self.grid().assert_in_order(fragments);
    }

    /// The oracle asserts `text` starts at column `x` of row `y` (cells).
    pub fn assert_text_at(&self, x: usize, y: usize, text: &str) {
        self.grid().assert_text_at(x, y, text);
    }

    // ---- styles, cursor, links, title: what rows do not show -----------------------

    /// The oracle asserts the style of the cell at column `x`, row `y`; `spec`
    /// as the oracle reads: `"fg=Cyan bold"`, `"bg=Rgb(55,57,66) !dim"`,
    /// `"reversed"`, `"plain"` (see `check_style`).
    pub fn assert_style(&self, x: usize, y: usize, spec: &str) {
        self.grid().assert_style(x, y, spec);
    }

    /// The oracle asserts the style of every cell of the first `text` on the
    /// screen.
    pub fn assert_text_style(&self, text: &str, spec: &str) {
        self.grid().assert_text_style(text, spec);
    }

    /// The oracle asserts the cursor is shown at column `x`, row `y`.
    pub fn assert_cursor(&self, x: usize, y: usize) {
        self.grid().assert_cursor(x, y);
    }

    /// The oracle asserts the cursor is hidden.
    pub fn assert_cursor_hidden(&self) {
        self.grid().assert_cursor_hidden();
    }

    /// The oracle asserts every cell of the first `text` links to `url`
    /// (OSC 8).
    pub fn assert_link(&self, text: &str, url: &str) {
        self.grid().assert_link(text, url);
    }

    /// The oracle asserts no cell of the first `text` has a link.
    pub fn assert_no_link(&self, text: &str) {
        self.grid().assert_no_link(text);
    }

    /// The oracle asserts the window title (OSC 0/2) the app last wrote.
    pub fn assert_title(&self, title: &str) {
        let got = self.title();
        assert!(
            got.as_deref() == Some(title),
            "the window title is {got:?}, the oracle's is {title:?}"
        );
    }

    // ---- a screen against the screen before ------------------------------------------

    /// The oracle compares the screen with one it saw before: all rows equal.
    pub fn assert_same_rows(&self, before: &Shot) {
        pretty_assertions::assert_eq!(
            before.0.join("\n"),
            self.rows().join("\n"),
            "the screen now (right) differs from the screen then (left)"
        );
    }

    /// As `assert_same_rows` for rows `rows` only (0-based, end excluded).
    pub fn assert_same_rows_in(&self, before: &Shot, rows: std::ops::Range<usize>) {
        let now = self.rows();
        let cut = |v: &[String]| {
            v.iter()
                .skip(rows.start)
                .take(rows.len())
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        };
        pretty_assertions::assert_eq!(
            cut(&before.0),
            cut(&now),
            "rows {rows:?} now (right) differ from then (left)"
        );
    }

    /// The oracle asserts the screen changed.
    pub fn assert_rows_differ(&self, before: &Shot) {
        assert!(
            before.0 != self.rows(),
            "the screen did not change:\n{}",
            self.text()
        );
    }
}

/// The screen's rows at one moment (`Screen::shot`).
pub struct Shot(Vec<String>);

/// A comparison with an oracle snapshot (`Screen::snap`).
pub struct Snap<'a> {
    screen: &'a Screen,
    name: &'a str,
    frame: &'a str,
    at: Option<(usize, usize)>,
}

impl<'a> Snap<'a> {
    /// The snapshot is one frame of several (`assert_snap_section`'s header).
    pub fn frame(mut self, frame: &'a str) -> Snap<'a> {
        self.frame = frame;
        self
    }

    /// The snapshot is a region of the oracle's screen that starts at screen
    /// row `row` and column `col` of ours.
    pub fn at(mut self, row: usize, col: usize) -> Snap<'a> {
        self.at = Some((row, col));
        self
    }

    pub fn assert(self) {
        compare::assert_snap(
            &self.screen.grid(),
            &compare::Want {
                name: self.name,
                frame: self.frame,
                at: self.at,
            },
        );
    }
}

/// The window title in what an app wrote: the text of the last `ESC ] 0 ;` or
/// `ESC ] 2 ;` sequence, up to its BEL or ST.
pub fn title_in(written: &str) -> Option<String> {
    let mut found = None;
    for start in ["\x1b]0;", "\x1b]2;"] {
        for (at, _) in written.match_indices(start) {
            let rest = &written[at + start.len()..];
            let end = rest.find(['\x07', '\x1b']).unwrap_or(rest.len());
            if found.as_ref().is_none_or(|(prev, _)| at > *prev) {
                found = Some((at, rest[..end].to_string()));
            }
        }
    }
    found.map(|(_, t)| t)
}
