//! Input a plain xterm cannot send, for the tests of the oracle that hand the
//! widget a `KeyEvent` or a `MouseEvent` of any kind.
//!
//! `rtwk_test::Tester` already speaks the terminal's side of it: `press`
//! (keys as bindings name them), `mouse` (any kind, button and modifiers),
//! `click_button`. What it leaves to the test is the kitty keyboard protocol
//! where a key has no bytes of its own, so the kit adds, in one place:
//!
//! - `keys("super+space cmd+c shift+enter")`: a key with no bytes of its own
//!   on an xterm (Super, Shift+Enter, Ctrl+Shift+letter) is sent as a
//!   terminal with the kitty protocol on sends it (`CSI code;mods u`).
//! - `key_repeat(key)` / `key_release(key)`: the key held down, and let go
//!   (kitty event types 2 and 3), with the modifiers the key names.
//! - `right_click(x, y)`, `shift_click(x, y)`, `ctrl_click(x, y)`,
//!   `triple_click(x, y)`, `pointer_move(x, y)`: the mouse as the oracle's
//!   `MouseEvent`s have it (the left button alone was in the kit before).

use rtwk::focus::KeyCombo;
use rtwk_core::input::{KeyCode, Mods, MouseKind};
use rtwk_test::keys::{KeysError, key_bytes};

use super::Screen;

/// The kitty protocol's event types (`CSI code;mods:event u`).
#[derive(Clone, Copy)]
pub(super) enum KeyKind {
    Repeat = 2,
    Release = 3,
}

/// The bytes of `key` (one key, named as bindings name it) as the kitty
/// protocol reports the event `kind` of it.
pub(super) fn event_bytes(key: &str, kind: KeyKind) -> String {
    let combo = match KeyCombo::parse(key) {
        Ok(combo) => combo,
        Err(err) => panic!("key `{key}`: {err}"),
    };
    let mods = combo.modifiers.0 + 1;
    let event = kind as u8;
    let tilde = |n: u8| format!("\x1b[{n};{mods}:{event}~");
    let arrow = |fin: char| format!("\x1b[1;{mods}:{event}{fin}");
    let unicode = |code: u32| format!("\x1b[{code};{mods}:{event}u");
    match combo.code {
        KeyCode::Char => unicode(u32::from(combo.char.unwrap_or(' '))),
        KeyCode::Enter => unicode(13),
        KeyCode::Escape => unicode(27),
        KeyCode::Tab => unicode(9),
        KeyCode::Backspace => unicode(127),
        KeyCode::Up => arrow('A'),
        KeyCode::Down => arrow('B'),
        KeyCode::Right => arrow('C'),
        KeyCode::Left => arrow('D'),
        KeyCode::Home => arrow('H'),
        KeyCode::End => arrow('F'),
        KeyCode::Insert => tilde(2),
        KeyCode::Delete => tilde(3),
        KeyCode::PageUp => tilde(5),
        KeyCode::PageDown => tilde(6),
        other => panic!("key `{key}`: no kitty bytes for {other:?} in the kit"),
    }
}

/// `ctrl+j` and `ctrl+m`: on an xterm the bytes of Enter, so no terminal without the kitty
/// protocol can tell them apart (`ctrl+h` and the others are the delete key and the like to the
/// oracle's tests, and stay the bytes a terminal sends).
fn shares_legacy_bytes(key: &str) -> bool {
    let Ok(combo) = KeyCombo::parse(key) else {
        return false;
    };
    combo.code == KeyCode::Char
        && combo.modifiers == Mods::CTRL
        && matches!(combo.char, Some('j' | 'm'))
}

impl Screen {
    /// One key as a terminal sends it: its own bytes where it has them, else
    /// the kitty protocol's (`super+space`, `shift+enter`).
    pub(super) fn press_key(&mut self, key: &str) {
        if shares_legacy_bytes(key) {
            // `ctrl+j` is the byte 0x0a on an xterm, which is Enter: a test that binds the chord
            // means the key itself, so it goes as a terminal with the kitty protocol sends it.
            for bytes in key_bytes(key, true).unwrap_or_else(|err| panic!("key `{key}`: {err}")) {
                self.h.t.input(&bytes);
            }
            return;
        }
        match key_bytes(key, false) {
            Ok(_) => self.h.t.press(key),
            Err(KeysError::NoBytes { .. }) => {
                let bytes = key_bytes(key, true).unwrap_or_else(|err| panic!("key `{key}`: {err}"));
                for bytes in bytes {
                    self.h.t.input(&bytes);
                }
            }
            Err(err) => panic!("key `{key}`: {err}"),
        }
    }

    /// Delivers a key as the terminal reports it held down (the kitty
    /// protocol's repeat event): `k`, `down`, `ctrl+d`, `enter`, `esc`. One
    /// press of it is `keys`.
    pub fn key_repeat(&mut self, key: &str) -> &mut Screen {
        self.key_event(key, KeyKind::Repeat)
    }

    /// Delivers the release of a key (the kitty protocol's release event):
    /// `cmd+c`, `down`. The terminal reports it only with the protocol on;
    /// the press is `keys`.
    pub fn key_release(&mut self, key: &str) -> &mut Screen {
        self.key_event(key, KeyKind::Release)
    }

    fn key_event(&mut self, key: &str, kind: KeyKind) -> &mut Screen {
        let bytes = event_bytes(key, kind);
        self.h.t.input(bytes.as_bytes());
        self.h.advance(100);
        self
    }

    // ---- mouse: more than the left button ---------------------------------

    /// Presses and releases the right button at (x, y).
    pub fn right_click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.h.t.click_button(x, y, 2);
        self.h.advance(100);
        self
    }

    /// The parts of a right click, for a test that looks between them.
    pub fn right_down(&mut self, x: u16, y: u16) -> &mut Screen {
        self.mouse_event(MouseKind::Down, 2, x, y, Mods::NONE)
    }

    pub fn right_up(&mut self, x: u16, y: u16) -> &mut Screen {
        self.mouse_event(MouseKind::Up, 2, x, y, Mods::NONE)
    }

    /// A left click with Shift held (extends a selection).
    pub fn shift_click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.click_with(x, y, Mods::SHIFT)
    }

    /// A left click with Ctrl held.
    pub fn ctrl_click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.click_with(x, y, Mods::CTRL)
    }

    /// A left click with `mods` held.
    pub fn click_with(&mut self, x: u16, y: u16, mods: Mods) -> &mut Screen {
        self.h.t.mouse(MouseKind::Down, x, y, 0, mods);
        self.h.t.mouse(MouseKind::Up, x, y, 0, mods);
        self.h.advance(100);
        self
    }

    /// Three left clicks at one cell, in a row (selects a line).
    pub fn triple_click(&mut self, x: u16, y: u16) -> &mut Screen {
        self.h.t.click(x, y);
        self.h.t.click(x, y);
        self.h.t.click(x, y);
        self.h.advance(100);
        self
    }

    /// The pointer moves to (x, y) with no button held (a terminal reports it
    /// only while it tracks all motion).
    pub fn pointer_move(&mut self, x: u16, y: u16) -> &mut Screen {
        self.mouse_event(MouseKind::Move, 0, x, y, Mods::NONE)
    }

    /// Any mouse report: `kind` with `button` (0 left, 1 middle, 2 right) and
    /// `mods` at (x, y).
    pub fn mouse_event(
        &mut self,
        kind: MouseKind,
        button: u8,
        x: u16,
        y: u16,
        mods: Mods,
    ) -> &mut Screen {
        self.h.t.mouse(kind, x, y, button, mods);
        self.h.advance(100);
        self
    }
}

/// `event_bytes` for a test of the kit: `release` false is a repeat.
pub fn event_bytes_of(key: &str, release: bool) -> String {
    let kind = if release {
        KeyKind::Release
    } else {
        KeyKind::Repeat
    };
    event_bytes(key, kind)
}
