"""Moves the `KeyCode` reading of the `/keymap` editor (capture view, inspector) into
`codex-rs/tui-rtwk/src/keymap/press.rs`, where ADR-006 (`adr006_keycode_only_in_keymap`) allows it.
Needs an edit of `src/keymap/mod.rs` (one `pub mod press;` line), so it is for whoever owns that
directory to run: python3 -I tools/rtwk-port/w2-settings/keymap_press.py   (from the repo root).
Rerunnable: it does nothing once `src/keymap/press.rs` exists. Then run clippy and the rules test."""
import os
import sys

CRATE = "codex-rs/tui-rtwk"
EDITOR = f"{CRATE}/src/features/settings/keymap_editor/"
PRESS = f"{CRATE}/src/keymap/press.rs"

if os.path.exists(PRESS):
    sys.exit("already moved")


def read(path):
    with open(path) as f:
        return f.read()


def write(path, text):
    with open(path, "w") as f:
        f.write(text)


def between(text, start, end):
    a = text.index(start)
    return a, text.index(end, a)


def edit(path, pairs):
    text = read(path)
    for old, new in pairs:
        if old not in text:
            sys.exit(f"{path}: not found: {old[:70]}")
        text = text.replace(old, new, 1)
    write(path, text)


RAW = """/// The code of a key press as the inspector's raw event names it.
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

/// The modifiers of `key` as the inspector's raw event names them, in its order.
pub fn raw_mods(key: &KeyPress) -> Vec<&'static str> {
    [
        (Mods::CTRL, "ctrl"),
        (Mods::ALT, "alt"),
        (Mods::SHIFT, "shift"),
        (Mods::SUPER, "SUPER"),
    ]
    .into_iter()
    .filter(|(flag, _)| key.mods.contains(*flag))
    .map(|(_, name)| name)
    .collect()
}
"""

# what debug.rs keeps of the raw event: its words
RAW_OF = """/// `code=F(8), modifiers=none, kind=Press`: the event as the oracle prints it.
fn raw_of(key: &KeyPress, s: &Strings) -> String {
    let code = match press::raw_code(key) {
        press::Raw::Char(ch) => s.raw_char(ch),
        press::Raw::Named(name) => name,
    };
    let mods = press::raw_mods(key);
    let mods = if mods.is_empty() {
        s.modifiers_none.to_owned()
    } else {
        mods.join("|")
    };
    s.raw_event(&code, &mods)
}

"""

cap = read(EDITOR + "capture.rs")
dbg = read(EDITOR + "debug.rs")

sa, sb = between(cap, "/// The press of `key` as a stroke", "/// The key as `tui.keymap` stores it")
_, se = between(cap, "/// The key as `tui.keymap` stores it", "/// The sheet's instructions for what is left")
stroke_fn = cap[sa:sb]
ra, rb = between(dbg, "/// `code=F(8), modifiers=none", "impl InspectorState")

press = (
    "//! A key press read as the keymap reads it (oracle `key_event_to_config_key_spec` and the\n"
    "//! keypress inspector's raw event): where a `KeyCode` may be matched beside the parent module\n"
    "//! (ADR-006). The `/keymap` editor's capture view and inspector use it; their words are theirs.\n\n"
    "use rtwk::focus::KeyPress;\n"
    "use rtwk::prelude::{KeyCode, Mods};\n\n"
    "use codex_tui_core::keymap::{Key, Mods as StrokeMods, Stroke};\n\n"
    + stroke_fn
    + """/// Why a key press cannot be stored in the config.
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

/// Ctrl+C.
pub fn is_ctrl_c(key: &KeyPress) -> bool {
    key.code == KeyCode::Char && key.ch == Some('c') && key.mods == Mods::CTRL
}

"""
    + RAW
)
write(PRESS, press)

edit(
    EDITOR + "capture.rs",
    [
        (cap[sa:se], "/// What the capture view says of a key the config cannot store.\n"
         "pub fn refusal_words(refusal: Refusal, s: &Strings) -> String {\n"
         "    match refusal {\n        Refusal::Modifiers => s.key_modifiers,\n"
         "        Refusal::Printable => s.key_printable,\n"
         "        Refusal::Unsupported => s.key_unsupported,\n    }\n    .to_owned()\n}\n\n"),
        ("use rtwk::prelude::{KeyCode, Mods};\n", ""),
        ("use codex_tui_core::keymap::{Key, Mods as StrokeMods, Stroke};\n\n", ""),
        ("use crate::app::theme::Tokens;", "use crate::app::theme::Tokens;\nuse crate::keymap::press::{self, Refusal};"),
        ("        match spec_of(&key, s) {", "        match press::spec_of(&key).map_err(|refusal| refusal_words(refusal, s)) {"),
    ],
)
text = read(EDITOR + "capture.rs")
a = text.index("        let escape = key.code")
b = text.index("            cx.emit(&w.on_done, Done::Cancel);")
text = text[:a] + "        if press::is_escape(&key) || press::is_ctrl_c(&key) {\n" + text[b:]
write(EDITOR + "capture.rs", text)

text = read(EDITOR + "debug.rs")
text = text.replace(dbg[ra:rb], RAW_OF)
write(EDITOR + "debug.rs", text)
edit(
    EDITOR + "debug.rs",
    [
        ("use rtwk::prelude::{KeyCode, Mods};\n", ""),
        ("use super::capture::{spec_of, stroke_of};", "use super::capture::refusal_words;"),
        ("use crate::keymap::{keymap, stroke_label};", "use crate::keymap::press::{self, is_ctrl_c, stroke_of};\nuse crate::keymap::{keymap, stroke_label};"),
        ("if key.code == KeyCode::Char && key.ch == Some('c') && key.mods == Mods::CTRL {", "if is_ctrl_c(&key) {"),
        ("config_key: spec_of(key, s),", "config_key: press::spec_of(key).map_err(|refusal| refusal_words(refusal, s)),"),
    ],
)
edit(EDITOR + "mod.rs", [("pub use self::capture::{Capture, Done, Mode, spec_of, stroke_of};", "pub use self::capture::{Capture, Done, Mode};")])
edit(f"{CRATE}/src/keymap/mod.rs", [("use std::rc::Rc;\n", "pub mod press;\n\nuse std::rc::Rc;\n")])
print("moved: now cargo fmt, clippy and the rules test")
