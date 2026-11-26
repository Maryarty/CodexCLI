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

mod asserts;
mod background;
mod compare;
mod elicitation;
mod env;
mod expect;
mod find;
mod grid;
mod handles;
mod hold;
mod input;
mod labels;
mod lists;
mod markdown_gutter;
mod palette;
mod probe;
mod replies;
mod seed;
mod settings;
mod side;
mod sparkle;
mod startup;
mod states;
mod tail;
mod usage;
mod verdict;
mod view;
mod worktrees;

#[allow(unused_imports)]
pub use asserts::{assert_differs, assert_same, native_status};
#[allow(unused_imports)]
pub use compare::{
    Want, assert_snap as assert_snap_on, snap_cuts, snap_frames, snap_rows, snap_size, snap_states,
};
#[allow(unused_imports)]
pub use env::{ColorDepth, Install, REPLAYED, replay_turn};
#[allow(unused_imports)]
pub use expect::{Ask, Turn, assert_rows_are};
#[allow(unused_imports)]
pub use grid::{Grid, check_style, color_name, modifier_name};
#[allow(unused_imports)]
pub use input::event_bytes_of;
#[allow(unused_imports)]
pub use palette::{DARK, LIGHT, contrast, luminance};
#[allow(unused_imports)]
pub use settings::Renderers;
#[allow(unused_imports)]
pub use side::SIDE_THREAD;
#[allow(unused_imports)]
pub use sparkle::{Dots, split_frames};
#[allow(unused_imports)]
pub use states::image_file_sized;

#[allow(unused_imports)]
pub use labels::{normalize as normalize_rows, normalize_key_names, normalize_labels};
#[allow(unused_imports)]
pub use replies::{Page, agent_turn, turns_of};
#[allow(unused_imports)]
pub use verdict::{Owner, missing, na, pending, todo};
#[allow(unused_imports)]
pub use worktrees::{Checkout, Held};

use rtwk_core::input::{Mods, MouseKind};

use crate::common::Harness;

/// How far apart a typist's keys are (the composer's paste-burst detector
/// takes keys under 8 ms apart for a paste).
use crate::common::KEY_PACE_MS;

fn to_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

/// The screen under test.
pub struct Screen {
    h: Harness,
    /// Opened from a debug entry: the screen has a model of its own, which
    /// the app's events do not reach (`feed` cannot change it).
    entry: bool,
    /// Columns at the left the oracle's rows do not have (`Open::markdown`).
    gutter: usize,
    /// How many history page requests the test has answered (`kit/replies.rs`).
    answered_pages: usize,
}

impl From<Harness> for Screen {
    fn from(h: Harness) -> Screen {
        Screen {
            h,
            entry: false,
            gutter: 0,
            answered_pages: 0,
        }
    }
}

/// A tiny PNG `name` in a directory of this test process; returns its path.
/// The wall clock in unix seconds, as the app reads it: a shard that seeds a `clock` step at
/// "now" takes it from here, not from the view's API.
pub fn unix_now() -> i64 {
    codex_tui_rtwk::services::clock::unix_now()
}

pub fn image_file(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("codex-kit-images-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("image dir");
    let path = dir.join(name);
    std::fs::write(&path, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").expect("write the image");
    path
}

impl Screen {
    // ---- keys ----------------------------------------------------------

    /// Presses keys named as bindings name them, apart by spaces: `down`,
    /// `enter`, `esc`, `ctrl+c`, `shift+tab`, `alt+b`, `A`, `down down enter`.
    ///
    /// A key with no bytes of its own on an xterm (`super+space`, `shift+enter`,
    /// `ctrl+shift+k`) goes as the kitty keyboard protocol sends it.
    ///
    /// The keys are a person's: `KEY_PACE_MS` apart, so the composer's paste
    /// detector reads `a b enter` as typing (use `type_burst` for a paste).
    pub fn keys(&mut self, keys: &str) -> &mut Screen {
        for key in keys.split_whitespace() {
            self.press_key(key);
            self.h.t.pump_for(crate::common::ms(KEY_PACE_MS));
        }
        self.h.advance(100);
        self
    }

    /// Types text as a person does: a key press per char, `KEY_PACE_MS`
    /// apart (slower than a paste burst), then a moment to settle.
    pub fn type_text(&mut self, text: &str) -> &mut Screen {
        let mut utf8 = [0u8; 4];
        for ch in text.chars() {
            self.h.t.type_text(ch.encode_utf8(&mut utf8));
            self.h.t.pump_for(crate::common::ms(KEY_PACE_MS));
        }
        self.h.advance(100);
        self
    }

    /// Types text as a terminal without bracketed paste delivers a paste: a
    /// key press per char, a millisecond apart, and nothing waits after the
    /// last (a held key is flushed only when the test lets time pass). The
    /// app draws one frame per `frame_gap` (16.7 ms), so what the screen
    /// shows right after is the state at the first key, not the held one;
    /// a test reads the screen after `wait`.
    pub fn type_burst(&mut self, text: &str) -> &mut Screen {
        let mut utf8 = [0u8; 4];
        for ch in text.chars() {
            self.h.t.type_text(ch.encode_utf8(&mut utf8));
            self.h.t.pump_for(crate::common::ms(1));
        }
        self
    }

    /// Attaches a PNG at the caret, as the oracle's `attach_image(path)`: a
    /// terminal drops a dragged file on the prompt as its path, and the
    /// composer attaches an image file as `[Image #N]` (with a space after
    /// it, which is taken back, as `attach_image` adds none). The file is a
    /// tiny PNG `name` in a directory of this test process; returns its
    /// path.
    pub fn attach_image(&mut self, name: &str) -> std::path::PathBuf {
        let path = image_file(name);
        self.paste(&path.to_string_lossy());
        self.keys("backspace");
        path
    }

    /// The messages the app has started turns with (what the backend was
    /// asked), oldest first.
    pub fn turns(&self) -> Vec<codex_tui_core::threads::UserMessage> {
        self.h
            .fake
            .commands()
            .into_iter()
            .filter_map(|c| match c {
                codex_tui_core::app::Command::StartTurn { message, .. } => Some(message),
                _ => None,
            })
            .collect()
    }

    /// Every command the app has given the backend, oldest first (the oracle's ops and
    /// `AppEvent`s that leave the widget).
    pub fn commands(&self) -> Vec<codex_tui_core::app::Command> {
        self.h.fake.commands()
    }

    /// The terminal reports that its window lost the focus (`CSI O`): the app notifies from now
    /// on (the oracle's `pending_notification` is sent when the terminal is not focused).
    pub fn focus_out(&mut self) -> &mut Screen {
        self.h.t.input(b"\x1b[O");
        self
    }

    /// The notifications the app asked the terminal to show (OSC 9: `title: body`), oldest
    /// first; the terminal needs `Open::notifications`.
    pub fn notifications(&self) -> Vec<String> {
        self.h
            .t
            .written()
            .split("\u{1b}]9;")
            .skip(1)
            .filter_map(|rest| {
                let end = rest.find(['\u{7}', '\u{1b}'])?;
                Some(rest[..end].to_owned())
            })
            .collect()
    }

    /// The pages the app asked the browser to open (`AppEvent::OpenUrlInBrowser`), oldest first:
    /// the app's own `Command::OpenUrl` and a feature's `Ask::OpenUrl` effect.
    pub fn opened_urls(&self) -> Vec<String> {
        use codex_tui_core::app::effects::Ask;
        self.h
            .fake
            .commands()
            .into_iter()
            .filter_map(|c| match c {
                codex_tui_core::app::Command::OpenUrl { url, .. } => Some(url),
                codex_tui_core::app::Command::Effect {
                    ask: Ask::OpenUrl { url },
                    ..
                } => Some(url),
                _ => None,
            })
            .collect()
    }

    /// The add-credits nudges the app asked the backend for (the fake leaves them unanswered),
    /// oldest first.
    pub fn nudges(&self) -> Vec<codex_tui_core::app::Command> {
        self.h
            .fake
            .commands()
            .into_iter()
            .filter(|c| matches!(c, codex_tui_core::app::Command::SendAddCreditsNudge { .. }))
            .collect()
    }

    /// The server answers the request `command` made with `result` (its JSON).
    pub fn answer_request(
        &mut self,
        command: &codex_tui_core::app::Command,
        result: serde_json::Value,
    ) {
        let (ticket, reply) =
            codex_tui_core::protocol::reply(command, Ok(result)).expect("a command with a reply");
        let epoch = self.h.fake.epoch().expect("attached");
        self.h
            .fake
            .inject(epoch, codex_tui_core::app::AppEv::Reply(ticket, reply));
        self.h.advance(100);
    }

    /// The host answers the effect `command` asked of it (`Command::Effect`: the clipboard, the
    /// git lists of `/review`) with `done`.
    pub fn answer_effect(
        &mut self,
        command: &codex_tui_core::app::Command,
        done: codex_tui_core::app::effects::Done,
    ) {
        let codex_tui_core::app::Command::Effect { request, .. } = command else {
            panic!("not an effect: {command:?}");
        };
        let epoch = self.h.fake.epoch().expect("attached");
        self.h.fake.inject(
            epoch,
            codex_tui_core::app::AppEv::Effect {
                request: *request,
                done,
            },
        );
        self.h.advance(100);
    }

    /// The host answers the external editor (Ctrl+G) that was given `draft` with `text`: the
    /// effect `Ask::Editor` of that draft is read from the commands and answered.
    pub fn answer_editor(&mut self, draft: &str, text: &str) {
        use codex_tui_core::app::effects::{Ask, Done};
        let ask = self
            .commands()
            .into_iter()
            .find(|c| {
                matches!(c, codex_tui_core::app::Command::Effect { ask: Ask::Editor { draft: d, .. }, .. } if d == draft)
            })
            .unwrap_or_else(|| panic!("the draft `{draft}` did not go to the editor"));
        self.answer_effect(&ask, Done::Editor(Ok(text.into())));
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
        let grid = self.h.t.with_screen(Grid::of);
        if self.gutter == 0 {
            grid
        } else {
            grid.without_left(self.gutter)
        }
    }

    // ---- terminal and time ----------------------------------------------

    pub fn resize(&mut self, width: u16, height: u16) -> &mut Screen {
        // A `markdown()` screen speaks of the renderer's width: the cell's
        // gutter and free column come on top.
        let width = if self.gutter > 0 {
            width + markdown_gutter::GUTTER + markdown_gutter::RESERVE
        } else {
            width
        };
        // A terminal asked for the size it has is not resized (the oracle's
        // `Buffer::empty(area)` of that size changes nothing either).
        let (now_width, now_height) = self
            .h
            .t
            .with_screen(|screen| (screen.width(), screen.height()));
        if (usize::from(width), usize::from(height)) != (now_width, now_height) {
            self.h.t.resize(width, height);
        }
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

    /// Waits until `ready` holds of the screen's cells (`Harness::wait_until`):
    /// for what the app applies from another thread (an answer of the shell),
    /// where `wait` would be a guess at how long that takes.
    pub fn wait_until(&mut self, what: &str, ready: impl Fn(&Grid) -> bool) -> &mut Screen {
        self.h
            .wait_until(what, |h| h.t.with_screen(|s| ready(&Grid::of(s))));
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

    /// Whether the cell at column `x`, row `y` is drawn reversed (the oracle's selection and
    /// the marked prompt are).
    pub fn reversed(&self, x: u16, y: u16) -> bool {
        self.h
            .t
            .cell(usize::from(x), usize::from(y))
            .attrs
            .intersects(rtwk_core::style::Attrs::REVERSE)
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
            labels: &[],
            state: None,
            cut: None,
            states: None,
            at: None,
            dialog: false,
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

    /// The frame names of the oracle snapshot `name` (what `assert_snap_section`
    /// takes), in order: a bare label such as `before scroll:` is one.
    pub fn snap_frame_names(name: &str) -> Vec<String> {
        oracle_snap::frames(&oracle_snap::find(name))
            .into_iter()
            .map(|f| f.name)
            .collect()
    }

    /// The composer's text (`ComposerInput::composer_text` of the oracle),
    /// read from the composer's rows (see `Grid::composer_text` for what it
    /// can and cannot tell): `""` while the placeholder shows.
    pub fn composer_text(&self) -> String {
        self.grid().composer_text()
    }

    /// The oracle test asserts the composer's text is exactly `text`
    /// (`assert_eq!(chat.bottom_pane.composer_text(), ..)`).
    pub fn assert_composer(&self, text: &str) {
        self.grid().assert_composer(text);
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

    /// The oracle asserts an attribute (`dim`, `bold`, `italic`, `!dim`, ...)
    /// of the cell at `(x, y)` as its buffer holds it. Read from the painted
    /// surface, not the terminal: the presenter writes a blank without the
    /// attributes that do not show on it (DIM, BOLD, ITALIC), where the
    /// oracle's `Buffer` keeps them (G46).
    pub fn assert_painted(&self, x: usize, y: usize, spec: &str) {
        use rtwk::style::Attrs;
        let frame = self.h.t.frame_loop();
        let attrs = frame.styles().attrs_of(frame.surface().style(x, y));
        for token in spec.split_whitespace() {
            let (negate, name) = match token.strip_prefix('!') {
                Some(name) => (true, name),
                None => (false, token),
            };
            let attr = match name {
                "dim" => Attrs::DIM,
                "bold" => Attrs::BOLD,
                "italic" => Attrs::ITALIC,
                other => panic!("assert_painted reads dim, bold, italic; not `{other}`"),
            };
            assert_eq!(
                attrs.intersects(attr),
                !negate,
                "cell ({x}, {y}) against `{token}`:\n{}",
                self.h.t.screen()
            );
        }
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

    /// The oracle compares a measured frame (cells and styles it read off the grid) with the one
    /// it measured before: all equal.
    pub fn assert_same_frame<T: PartialEq>(&self, before: &T, now: &T, why: &str) {
        assert!(before == now, "{why}: the frame moved:\n{}", self.text());
    }

    /// The oracle asserts a measured frame holds `count` > 0 cells of `what`.
    pub fn assert_drawn(&self, count: usize, what: &str) {
        assert!(count > 0, "no {what} drawn:\n{}", self.text());
    }

    /// The first `count` rows of the oracle's snapshot `name` equal our rows from `from` (trailing
    /// blanks aside); `blink` ignores a leading `•`/`◦` (a spinner glyph that follows the clock).
    pub fn assert_snap_rows_from(&self, name: &str, from: usize, count: usize, blink: bool) {
        let ours = self.rows();
        let glyph: &[char] = if blink { &['•', '◦'] } else { &[] };
        for (at, want) in snap_rows(name, "").iter().take(count).enumerate() {
            let got = ours.get(from + at).map_or("", |r| r.trim_end());
            pretty_assertions::assert_eq!(
                got.trim_start_matches(glyph),
                want.trim_end().trim_start_matches(glyph),
                "{name}: row {at}\n{}",
                self.text()
            );
        }
    }

    /// The oracle drew the chat widget alone (no history, no gap row under the transcript): its
    /// snapshot `name` starts at the working line, and its rows, blanks aside, are the app's
    /// bottom rows from that line on. The spinner glyph and the seconds of the first row follow
    /// the clock.
    pub fn assert_snap_rows_blanks_aside(&self, name: &str) {
        let glyph: &[char] = &['•', '◦'];
        let kept = |rows: Vec<String>| -> Vec<String> {
            rows.iter()
                .map(|row| row.trim_end().to_string())
                .filter(|row| !row.is_empty())
                .collect()
        };
        let head = |row: &str| {
            let row = row.trim_start_matches(glyph);
            row.split_once(" (")
                .map_or(row, |(head, _)| head)
                .to_string()
        };
        let (want, ours) = (kept(snap_rows(name, "")), kept(self.rows()));
        let first = head(&want[0]);
        let from = ours
            .iter()
            .position(|row| head(row) == first)
            .unwrap_or_else(|| panic!("{name}: no `{first}` row:\n{}", self.text()));
        let got = &ours[from..(from + want.len()).min(ours.len())];
        let timeless = |rows: &[String]| -> Vec<String> {
            rows.iter()
                .enumerate()
                .map(|(at, row)| if at == 0 { head(row) } else { row.clone() })
                .collect()
        };
        let want = timeless(&want);
        assert_rows_are(
            &timeless(got),
            &want.iter().map(String::as_str).collect::<Vec<_>>(),
        );
    }

    /// As `assert_snap_rows_from` for the rows `rows` of the screen against the same rows of the
    /// snapshot, but for the rows in `except` (the port's own words there).
    pub fn assert_snap_rows_except(
        &self,
        name: &str,
        rows: std::ops::Range<usize>,
        except: &[usize],
    ) {
        let ours = self.rows();
        let want = snap_rows(name, "");
        for at in rows.filter(|at| !except.contains(at)) {
            pretty_assertions::assert_eq!(
                ours[at].trim_end(),
                want[at].trim_end(),
                "{name}: row {at}\n{}",
                self.text()
            );
        }
    }

    /// The first `count` rows of the screen equal the rows `at..at + count` of the oracle's
    /// snapshot `name` (one that joins several states by a blank row with no header).
    pub fn assert_snap_state_rows(&self, name: &str, at: usize, count: usize) {
        let ours = self.rows();
        let want = snap_rows(name, "");
        for row in 0..count {
            pretty_assertions::assert_eq!(
                ours[row].trim_end(),
                want[at + row].trim_end(),
                "{name} from row {at}, row {row}:\n{}",
                self.text()
            );
        }
    }

    /// The oracle presses `keys` `times` times and asserts `text` showed on some row on the way.
    pub fn assert_passes_by(&mut self, keys: &str, times: usize, text: &str) {
        let mut seen = false;
        for _ in 0..times {
            self.keys(keys);
            seen |= self.rows().iter().any(|row| row.contains(text));
        }
        assert!(seen, "`{text}` never showed:\n{}", self.text());
    }

    /// The oracle asserts the screen's words, wrapped or not, hold `text`: the rows' words joined
    /// by single spaces.
    pub fn assert_contains_unwrapped(&self, text: &str) {
        let joined = self
            .rows()
            .iter()
            .flat_map(|row| row.split_whitespace())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            joined.contains(text),
            "no `{text}` in the screen's words:\n{}",
            self.text()
        );
    }

    /// The oracle asserts a token with no spaces (`token`) runs over rows from the one that starts
    /// with its first piece: the rows' pieces join back to it, and the first piece does not end at
    /// a hyphen or a slash.
    pub fn assert_token_wraps_whole(&self, token: &str) {
        let rows = self.rows();
        let first = rows
            .iter()
            .position(|row| row.trim().starts_with(&token[..token.len().min(8)]))
            .unwrap_or_else(|| panic!("no `{token}` row on the screen:\n{}", self.text()));
        let shown: Vec<&str> = rows[first..]
            .iter()
            .map(|row| row.trim())
            .take_while(|row| !row.is_empty())
            .collect();
        pretty_assertions::assert_eq!(shown.concat(), token, "{}", self.text());
        assert!(
            !shown[0].ends_with('-') && !shown[0].ends_with('/'),
            "the token is split at a hyphen or a slash: {shown:?}"
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
    labels: &'a [&'a str],
    at: Option<(usize, usize)>,
    dialog: bool,
    state: Option<(usize, usize)>,
    states: Option<(&'a [usize], usize)>,
    cut: Option<usize>,
}

impl<'a> Snap<'a> {
    /// The snapshot is one frame of several (`assert_snap_section`'s header).
    pub fn frame(mut self, frame: &'a str) -> Snap<'a> {
        self.frame = frame;
        self
    }

    /// The snapshot is one the test glued of several renders under labels of its own
    /// (`Compact` / `Full`, `Full height:` / `Clipped:`, `Resume 118x30`, `empty (48 columns):`):
    /// `labels` are the lines the oracle test put between them, the text of its own
    /// `format!`/`join`; `frame(..)` then names one by its label (a colon aside). Nothing else in
    /// the snapshot is a header.
    pub fn labelled(mut self, labels: &'a [&'a str]) -> Snap<'a> {
        self.labels = labels;
        self
    }

    /// The snapshot is one the test glued of states of one screen with a blank row and no header
    /// (`format!("{initial}\n\n{scrolled}")`), each `height` rows: this compares state `index`
    /// (from 0, in the order the test wrote them).
    pub fn state(mut self, height: usize, index: usize) -> Snap<'a> {
        self.state = Some((height, index));
        self
    }

    /// The snapshot is one the test glued of bodies of different heights with a blank row and no
    /// header (`panel(..)` bodies joined by `"\n\n"`): `heights` are the bodies' rows in the order
    /// the test wrote them (a body's own blank rows included); this compares state `index`.
    pub fn states(mut self, heights: &'a [usize], index: usize) -> Snap<'a> {
        self.states = Some((heights, index));
        self
    }

    /// The snapshot is one the test glued of renders with `join("\n---\n")` and no header
    /// (`rate_limit_reset_popup_states`): this compares state `index` (from 0, in the order the
    /// test wrote them), the lines between the bare `---` rules; `dialog()` reads it as a dialog.
    pub fn cut(mut self, index: usize) -> Snap<'a> {
        self.cut = Some(index);
        self
    }

    /// The snapshot is a region of the oracle's screen that starts at screen
    /// row `row` and column `col` of ours.
    pub fn at(mut self, row: usize, col: usize) -> Snap<'a> {
        self.at = Some((row, col));
        self
    }

    /// The frame is a view the oracle drew into the whole area (spare rows
    /// above the footer on its last row); ours is a dialog as tall as its
    /// content: the rows are compared from the blank row above its first text
    /// on, one blank row above the footer.
    pub fn dialog(mut self) -> Snap<'a> {
        self.dialog = true;
        self
    }

    pub fn assert(self) {
        let grid = self.screen.grid();
        let want = compare::Want {
            name: self.name,
            frame: self.frame,
            at: self.at,
        };
        if let Some(index) = self.cut {
            compare::assert_cut(&grid, &want, index, self.dialog);
        } else if let Some((heights, index)) = self.states {
            compare::assert_states(&grid, &want, heights, index);
        } else if let Some((height, index)) = self.state {
            compare::assert_state(&grid, &want, height, index);
        } else if !self.labels.is_empty() {
            compare::assert_labelled(&grid, &want, self.labels, self.dialog);
        } else if self.dialog {
            compare::assert_dialog(&grid, &want);
        } else {
            compare::assert_snap(&grid, &want);
        }
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
