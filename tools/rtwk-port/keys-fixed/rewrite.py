#!/usr/bin/env python3 -I
"""adr013-keys: the hardwired `Key::` strokes of pages and sheets -> named actions of
`core::keymap::fixed`. Rerunnable (a replacement already done is skipped). Run from the repo root:
    python3 -I tools/rtwk-port/keys-fixed/rewrite.py
Then `acargo check --tests`. What it does not do (the import prompt's key guard, the hint
helpers) is hand-edited; see the commit."""
import pathlib

ROOT = pathlib.Path("codex-rs/tui-rtwk")


def sub(path, pairs):
    p = ROOT / path
    text = p.read_text()
    for old, new in pairs:
        if old in text:
            text = text.replace(old, new)
    p.write_text(text)


# the shortcut overlay's table: Chord/Pair of strokes -> Fixed/FixedPair of actions
sub("src/sheets/dialogs/strings.rs", [
    ("Keys::Chord(Stroke::new(Key::Char('v'), Mods::CTRL))", "Keys::Fixed(PASTE_IMAGE)"),
    ("""Keys::Pair(
                        Stroke::new(Key::Left, Mods::ALT),
                        Stroke::new(Key::Right, Mods::ALT),
                    )""", "Keys::FixedPair(SUBAGENT_PREVIOUS, SUBAGENT_NEXT)"),
    ("Keys::Chord(Stroke::new(Key::Char('c'), Mods::CTRL))", "Keys::Fixed(QUIT_OR_INTERRUPT)"),
    ("""Keys::Pair(
                        Stroke::new(Key::PageUp, Mods::NONE),
                        Stroke::new(Key::PageDown, Mods::NONE),
                    )""", "Keys::FixedPair(TRANSCRIPT_PAGE_UP, TRANSCRIPT_PAGE_DOWN)"),
    ("Keys::Chord(Stroke::new(Key::Char(' '), Mods::CTRL))", "Keys::Fixed(SELECTION_START)"),
    ("""Keys::Pair(
                        Stroke::new(Key::Home, Mods::CTRL),
                        Stroke::new(Key::End, Mods::CTRL),
                    )""", "Keys::FixedPair(TRANSCRIPT_TOP, TRANSCRIPT_LATEST)"),
    ("""Keys::Pair(
                        Stroke::new(Key::Char(','), Mods::ALT.with(Mods::SHIFT)),
                        Stroke::new(Key::Char('.'), Mods::ALT.with(Mods::SHIFT)),
                    )""", "Keys::FixedPair(TRANSCRIPT_TOP_ALT, TRANSCRIPT_LATEST_ALT)"),
    ("""    /// A key the oracle fixes, spelt as the platform spells a hint (`\\u{2303}v`).
    Chord(Stroke),
    /// Two fixed keys, `a / b`.
    Pair(Stroke, Stroke),""",
     """    /// A key the oracle fixes (a named action of `keymap::fixed`), spelt as the platform spells a
    /// hint (`\\u{2303}v`).
    Fixed(ActionId),
    /// Two fixed keys, `a / b`.
    FixedPair(ActionId, ActionId),"""),
    ("use codex_tui_core::keymap::{ActionId, Key, Mods, Stroke};",
     """use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::fixed::{
    PASTE_IMAGE, QUIT_OR_INTERRUPT, SELECTION_START, SUBAGENT_NEXT, SUBAGENT_PREVIOUS,
    TRANSCRIPT_LATEST, TRANSCRIPT_LATEST_ALT, TRANSCRIPT_PAGE_DOWN, TRANSCRIPT_PAGE_UP,
    TRANSCRIPT_TOP, TRANSCRIPT_TOP_ALT,
};"""),
])
sub("src/sheets/dialogs/shortcuts.rs", [
    ("""                        Keys::Chord(stroke) => spell(&stroke),
                        Keys::Pair(a, b) => {
                            format!("{}{}{}", spell(&a), s.shortcut_either, spell(&b))
                        }""",
     """                        Keys::Fixed(id) => spell(&stroke_for(id)?),
                        Keys::FixedPair(a, b) => format!(
                            "{}{}{}",
                            spell(&stroke_for(a)?),
                            s.shortcut_either,
                            spell(&stroke_for(b)?)
                        ),"""),
])
