//! A key press read as the keymap reads it (oracle `key_event_to_config_key_spec` and the
//! keypress inspector's raw event): where a `KeyCode` may be matched beside the parent module
//! (ADR-006). The `/keymap` editor's capture view and inspector use it; their words are theirs.

use rtwk::focus::KeyPress;
use rtwk::prelude::{KeyCode, Mods};

use codex_tui_core::keymap::{Key, Mods as StrokeMods, Stroke};

/// The press of `key` as a stroke, whatever the config can store of it (the debug view names
/// what it detected); none for a key that is no stroke (a modifier alone).
pub fn stroke_of(key: &KeyPress) -> Option<Stroke> {
    let mut mods = StrokeMods::NONE;
    for (flag, stroke) in [
        (Mods::CTRL, StrokeMods::CTRL),
        (Mods::ALT, StrokeMods::ALT),
        (Mods::SHIFT, StrokeMods::SHIFT),
        (Mods::SUPER, StrokeMods::SUPER),
    ] {
        if key.mods.contains(flag) {
            mods = mods.with(stroke);
        }
    }
    let k = match key.code {
        KeyCode::Char => {
            let ch = key.ch?;
            if ch.is_ascii_uppercase() {
                mods = mods.with(StrokeMods::SHIFT);
                Key::Char(ch.to_ascii_lowercase())
            } else {
                Key::Char(ch)
            }
        }
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Escape => Key::Esc,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::F1 => Key::F(1),
        KeyCode::F2 => Key::F(2),
        KeyCode::F3 => Key::F(3),
        KeyCode::F4 => Key::F(4),
        KeyCode::F5 => Key::F(5),
        KeyCode::F6 => Key::F(6),
        KeyCode::F7 => Key::F(7),
        KeyCode::F8 => Key::F(8),
        KeyCode::F9 => Key::F(9),
        KeyCode::F10 => Key::F(10),
        KeyCode::F11 => Key::F(11),
        KeyCode::F12 => Key::F(12),
        // A modifier, lock or media key pressed alone is no stroke (kitty reports them).
        KeyCode::Modifier | KeyCode::Lock | KeyCode::Media => return None,
    };
    Some(Stroke::new(k, mods).canonical())
}

/// Why a key press cannot be stored in the config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A modifier the config has no name for.
    Modifiers,
    /// A character that is not printable ASCII.
    Printable,
    /// A key the config cannot name.
    Unsupported,
}

/// The key as `tui.keymap` stores it, or why it cannot be (`key_event_to_config_key_spec`).
pub fn spec_of(key: &KeyPress) -> Result<String, Refusal> {
    let known = Mods::CTRL | Mods::ALT | Mods::SHIFT | Mods::SUPER;
    if key.mods.0 & !known.0 != 0 {
        return Err(Refusal::Modifiers);
    }
    if key.code == KeyCode::Char
        && let Some(ch) = key.ch
        && ch != '-'
        && (!ch.is_ascii() || ch.is_ascii_control())
    {
        return Err(Refusal::Printable);
    }
    match stroke_of(key) {
        Some(stroke) if stroke.key != Key::Insert => Ok(stroke.spec()),
        _ => Err(Refusal::Unsupported),
    }
}

/// Esc with no modifier: the capture view's cancel.
pub fn is_escape(key: &KeyPress) -> bool {
    key.code == KeyCode::Escape && key.mods == Mods::NONE
}

/// The fixed controls of the workspace's modal divider preview.
pub enum ResizeKey {
    Finish,
    Step(i32),
}

pub fn resize_key(key: &KeyPress) -> Option<ResizeKey> {
    Some(match (key.code, key.mods, key.ch) {
        (KeyCode::Escape | KeyCode::Enter, Mods::NONE, _) => ResizeKey::Finish,
        (KeyCode::Left, Mods::SHIFT, _) => ResizeKey::Step(-10),
        (KeyCode::Right, Mods::SHIFT, _) => ResizeKey::Step(10),
        (KeyCode::Left, Mods::NONE, _) | (_, Mods::NONE, Some('h')) => ResizeKey::Step(-1),
        (KeyCode::Right, Mods::NONE, _) | (_, Mods::NONE, Some('l')) => ResizeKey::Step(1),
        _ => return None,
    })
}

/// The previous/next button of a confirmation controlled by the arrow keys.
pub fn horizontal_focus(key: &KeyPress) -> Option<bool> {
    match (key.code, key.mods) {
        (KeyCode::Left, Mods::NONE) => Some(false),
        (KeyCode::Right, Mods::NONE) => Some(true),
        _ => None,
    }
}

/// Ctrl+C: the key typed, or the key it is on a US layout (a Cyrillic layout's Ctrl+с is Ctrl+C,
/// as `keymap::holds` reads a press).
pub fn is_ctrl_c(key: &KeyPress) -> bool {
    key.code == KeyCode::Char
        && (key.ch == Some('c') || key.layout_char() == Some('c'))
        && key.mods == Mods::CTRL
}

/// The code of a key press as the inspector's raw event names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Raw {
    /// A character: the inspector prints it quoted.
    Char(char),
    /// Any other key: `F(8)`, `Esc`, `BackTab`, `Up`.
    Named(String),
}

/// `F(8)`, `Char('x')`: the code of `key` (`KeyEvent::code`).
pub fn raw_code(key: &KeyPress) -> Raw {
    let shift = key.mods.contains(Mods::SHIFT);
    Raw::Named(match key.code {
        KeyCode::Char => return Raw::Char(key.ch.unwrap_or(' ')),
        KeyCode::Tab if shift => "BackTab".to_owned(),
        KeyCode::Escape => "Esc".to_owned(),
        KeyCode::F1 => "F(1)".to_owned(),
        KeyCode::F2 => "F(2)".to_owned(),
        KeyCode::F3 => "F(3)".to_owned(),
        KeyCode::F4 => "F(4)".to_owned(),
        KeyCode::F5 => "F(5)".to_owned(),
        KeyCode::F6 => "F(6)".to_owned(),
        KeyCode::F7 => "F(7)".to_owned(),
        KeyCode::F8 => "F(8)".to_owned(),
        KeyCode::F9 => "F(9)".to_owned(),
        KeyCode::F10 => "F(10)".to_owned(),
        KeyCode::F11 => "F(11)".to_owned(),
        KeyCode::F12 => "F(12)".to_owned(),
        other => format!("{other:?}"),
    })
}

/// The modifiers of `key` as the inspector's raw event names them (`ctrl|alt`), in its order;
/// None for a press without any.
pub fn raw_mods(key: &KeyPress) -> Option<String> {
    let names: Vec<&str> = [
        (Mods::CTRL, "ctrl"),
        (Mods::ALT, "alt"),
        (Mods::SHIFT, "shift"),
        (Mods::SUPER, "SUPER"),
    ]
    .into_iter()
    .filter(|(flag, _)| key.mods.contains(*flag))
    .map(|(_, name)| name)
    .collect();
    (!names.is_empty()).then(|| names.join("|"))
}
