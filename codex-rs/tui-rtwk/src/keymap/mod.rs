//! The keymap in the view: Codex's `tui.keymap.*` (core's `Keymap`, resolved
//! over the oracle's defaults and held by the app, ADR-003) as the keys rtwk
//! speaks. In Flutter the bindings are a `Shortcuts` map built from one
//! configuration object that sits above the app (an `InheritedWidget`); here
//! the `Keymap` is provided above the app (`app/view.rs`), a widget reads it
//! with [`keymap`], and turns the action it serves into names for
//! `KeyBindings` ([`names`]), or asks whether a key press is the action's
//! ([`pressed`]) where the key reaches a handler before the focus tree does.
//! A footer or a hint takes its label from the same keymap ([`label`]).
//!
//! The keys are the oracle's names for them (`ctrl-x`, `esc`); rtwk's
//! `KeyCombo` is what the framework matches a key press against, so the quirks
//! of a terminal (`ctrl+j` arriving as Enter) are rtwk's, in one place.

use std::rc::Rc;

use rtwk::Cx;
use rtwk::focus::{KeyCombo, KeyPress};
use rtwk::prelude::{KeyCode, Mods as PressMods};
use rtwk::provider::watch;

use codex_tui_core::keymap::fixed::{self, FixedBinding};
use codex_tui_core::keymap::{ActionId, Binding, Key, Keymap, Mods, Stroke};

/// The keymap the app provides (`Provider::value_shared` over
/// `CodexApp::keymap`); the widget is rebuilt when the host replaces it (the
/// config is read once the session starts).
pub fn keymap<S: 'static>(cx: &mut Cx<'_, S>) -> Rc<Keymap> {
    watch::<Keymap, S>(cx)
}

/// The name rtwk's `KeyCombo::parse` reads for a stroke (`ctrl+shift+a`).
/// Shift with a character that has no case (`shift-?`) names the character:
/// a terminal reports the shifted character itself, and rtwk takes the press
/// of it for that key.
fn name_of(stroke: &Stroke) -> Option<String> {
    let mut mods = stroke.mods;
    let key = match stroke.key {
        Key::Char(' ') => "space".to_owned(),
        Key::Char(c) if c.is_ascii_uppercase() => {
            mods = mods.with(Mods::SHIFT);
            c.to_ascii_lowercase().to_string()
        }
        Key::Char(c) => {
            if !c.is_alphabetic() {
                mods = mods.without(Mods::SHIFT);
            }
            c.to_string()
        }
        Key::F(n) => format!("f{n}"),
        Key::Enter => "enter".to_owned(),
        Key::Tab => "tab".to_owned(),
        Key::BackTab => {
            mods = mods.with(Mods::SHIFT);
            "tab".to_owned()
        }
        Key::Backspace => "backspace".to_owned(),
        Key::Esc => "escape".to_owned(),
        Key::Delete => "delete".to_owned(),
        Key::Insert => "insert".to_owned(),
        Key::Up => "up".to_owned(),
        Key::Down => "down".to_owned(),
        Key::Left => "left".to_owned(),
        Key::Right => "right".to_owned(),
        Key::Home => "home".to_owned(),
        Key::End => "end".to_owned(),
        Key::PageUp => "pageup".to_owned(),
        Key::PageDown => "pagedown".to_owned(),
    };
    let mut name = String::new();
    for (flag, word) in [
        (Mods::CTRL, "ctrl"),
        (Mods::ALT, "alt"),
        (Mods::SHIFT, "shift"),
        (Mods::SUPER, "super"),
    ] {
        if mods.contains(flag) {
            name.push_str(word);
            name.push('+');
        }
    }
    name.push_str(&key);
    Some(name)
}

/// The combo of a stroke.
pub fn combo_of(stroke: &Stroke) -> Option<KeyCombo> {
    KeyCombo::parse(&name_of(stroke)?).ok()
}

/// The name a binding has for `KeyBindings`: a key (`ctrl+t`) or the keys
/// pressed one after another (`ctrl+x enter`).
pub fn name_of_binding(binding: &Binding) -> Option<String> {
    let names = binding
        .strokes()
        .iter()
        .map(name_of)
        .collect::<Option<Vec<_>>>()?;
    let name = names.join(" ");
    // A name rtwk does not read binds nothing (never a panic at startup).
    KeyCombo::parse_sequence(&name, None).ok().map(|_| name)
}

/// The names of an action's bindings, single keys first, then chords: what
/// `KeyBindings::new` takes, each paired with the action's handler.
pub fn names(keymap: &Keymap, id: ActionId) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for name in keymap.bindings(id).iter().filter_map(name_of_binding) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// The pairs of keys and what they do, with a key two pairs name the way
/// `KeyBindings` reads them (`shift+?` and `?` are one key) left to the first
/// of them: it panics on a key bound twice, and a user's keymap may bind two
/// actions to one key (the order of the pairs is the order of the actions the
/// screen asks first).
pub fn first_of_each<T>(pairs: Vec<(String, T)>) -> Vec<(String, T)> {
    let mut seen = std::collections::HashSet::new();
    pairs
        .into_iter()
        .filter(|(name, _)| {
            KeyCombo::parse_sequence(name, None).is_ok_and(|keys| {
                seen.insert(
                    keys.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" "),
                )
            })
        })
        .collect()
}

/// `names` of an action, each paired with `handler`.
pub fn bindings<H: Clone>(keymap: &Keymap, id: ActionId, handler: &H) -> Vec<(String, H)> {
    names(keymap, id)
        .into_iter()
        .map(|name| (name, handler.clone()))
        .collect()
}

/// Whether a key press is one of the action's single keys (the chords of an
/// action are `KeyBindings`' to match). A hardwired action (`keymap::fixed` of
/// core: the keys Codex has no setting for) is read the same way, by its fixed
/// bindings.
pub fn pressed(keymap: &Keymap, id: ActionId, key: &KeyPress) -> bool {
    if let Some(fixed) = fixed::bindings_of(id) {
        return fixed.iter().any(|binding| holds(binding, key));
    }
    keymap
        .singles(id)
        .iter()
        .filter_map(combo_of)
        .any(|combo| combo.accepts_press(key))
}

fn press_mods(mods: Mods) -> PressMods {
    [
        (Mods::CTRL, PressMods::CTRL),
        (Mods::ALT, PressMods::ALT),
        (Mods::SHIFT, PressMods::SHIFT),
        (Mods::SUPER, PressMods::SUPER),
    ]
    .into_iter()
    .filter(|(flag, _)| mods.contains(*flag))
    .fold(PressMods::NONE, |all, (_, mods)| all | mods)
}

fn press_code(key: Key) -> Option<KeyCode> {
    Some(match key {
        Key::Char(_) => KeyCode::Char,
        Key::Enter => KeyCode::Enter,
        Key::Tab | Key::BackTab => KeyCode::Tab,
        Key::Backspace => KeyCode::Backspace,
        Key::Esc => KeyCode::Escape,
        Key::Delete => KeyCode::Delete,
        Key::Insert => KeyCode::Insert,
        Key::Up => KeyCode::Up,
        Key::Down => KeyCode::Down,
        Key::Left => KeyCode::Left,
        Key::Right => KeyCode::Right,
        Key::Home => KeyCode::Home,
        Key::End => KeyCode::End,
        Key::PageUp => KeyCode::PageUp,
        Key::PageDown => KeyCode::PageDown,
        Key::F(n) => match n {
            1 => KeyCode::F1,
            2 => KeyCode::F2,
            3 => KeyCode::F3,
            4 => KeyCode::F4,
            5 => KeyCode::F5,
            6 => KeyCode::F6,
            7 => KeyCode::F7,
            8 => KeyCode::F8,
            9 => KeyCode::F9,
            10 => KeyCode::F10,
            11 => KeyCode::F11,
            12 => KeyCode::F12,
            _ => return None,
        },
    })
}

fn holds(binding: &FixedBinding, key: &KeyPress) -> bool {
    let held = [
        (PressMods::CTRL, Mods::CTRL),
        (PressMods::ALT, Mods::ALT),
        (PressMods::SHIFT, Mods::SHIFT),
        (PressMods::SUPER, Mods::SUPER),
    ]
    .into_iter()
    .filter(|(flag, _)| key.mods.contains(*flag))
    .fold(Mods::NONE, |all, (_, mods)| all.with(mods));
    let same_key = match binding.key {
        Key::Char(c) => key.ch == Some(c),
        other => press_code(other) == Some(key.code),
    };
    same_key && binding.mods.accepts(held)
}

/// The press a stroke stands for (a test's key, the key the field's own editing is written
/// for).
pub fn press_of(stroke: &Stroke) -> Option<KeyPress> {
    let mods = press_mods(stroke.mods);
    Some(match stroke.key {
        Key::Char(c) => KeyPress::char(c).with_mods(mods),
        Key::BackTab => KeyPress::code(KeyCode::Tab).with_mods(mods | PressMods::SHIFT),
        key => KeyPress::code(press_code(key)?).with_mods(mods),
    })
}

/// The presses of every single key of an action, for the code that binds presses to a `Focus`
/// (`names` is the same for `KeyBindings`): a hardwired action's own keys, or the keymap's.
pub fn presses(keymap: &Keymap, id: ActionId) -> Vec<KeyPress> {
    match fixed::bindings_of(id) {
        Some(keys) => keys
            .iter()
            .filter_map(|binding| press_of(&Stroke::new(binding.key, binding.mods.pressed())))
            .collect(),
        None => keymap.singles(id).iter().filter_map(press_of).collect(),
    }
}

/// Whether a key press is one of a hardwired action's keys (`pressed` without a keymap, for the
/// code that holds none: a fixed action has no configured keys).
pub fn pressed_fixed(id: ActionId, key: &KeyPress) -> bool {
    fixed::bindings_of(id).is_some_and(|keys| keys.iter().any(|binding| holds(binding, key)))
}

/// The press of a hardwired action's first key (`Edit::canonical`: the key the field's own
/// editing answers to for an action).
pub fn press_for(id: ActionId) -> Option<KeyPress> {
    let first = fixed::bindings_of(id)?.first()?;
    press_of(&Stroke::new(first.key, first.mods.pressed()))
}

/// The stroke of a hardwired action's first key (a picker prop that names a key by stroke: the
/// /plugins browser's toggle and hot keys); none for an action of the catalogue.
pub fn stroke_for(id: ActionId) -> Option<Stroke> {
    let first = fixed::bindings_of(id)?.first()?;
    Some(Stroke::new(first.key, first.mods.pressed()))
}

/// The name a hint shows for a stroke, as the oracle's `display_label` prints
/// it on this platform (`key_hint.rs`: glyphs on macOS, `^` on Linux, words
/// elsewhere; Backspace is `delete` on macOS, where crossterm names it so):
/// `⌃t`, `⇧tab`, `esc`, `pgdn`.
pub fn stroke_label(stroke: &Stroke) -> String {
    let (key, mods) = match stroke.key {
        Key::BackTab => (Key::Tab, stroke.mods.with(Mods::SHIFT)),
        key => (key, stroke.mods),
    };
    let mut out = String::new();
    for (flag, glyph, word) in [
        (
            Mods::CTRL,
            "\u{2303}",
            if cfg!(target_os = "linux") {
                "^"
            } else {
                "ctrl"
            },
        ),
        (Mods::SHIFT, "\u{21e7}", "shift"),
        (Mods::ALT, "\u{2325}", "alt"),
        (Mods::SUPER, "\u{2318}", "super"),
    ] {
        if !mods.contains(flag) {
            continue;
        }
        if cfg!(target_os = "macos") {
            out.push_str(glyph);
        } else {
            out.push_str(word);
            if word != "^" {
                out.push('+');
            }
        }
    }
    match key {
        Key::Char(' ') => out.push_str("space"),
        Key::Char(c) => out.push(c.to_ascii_lowercase()),
        Key::F(n) => out.push_str(&format!("f{n}")),
        Key::Enter => out.push_str("enter"),
        Key::Tab | Key::BackTab => out.push_str("tab"),
        Key::Backspace if cfg!(target_os = "macos") => out.push_str("delete"),
        Key::Backspace => out.push_str("backspace"),
        Key::Esc => out.push_str("esc"),
        Key::Delete => out.push_str("delete"),
        Key::Insert => out.push_str("insert"),
        Key::Up => out.push('\u{2191}'),
        Key::Down => out.push('\u{2193}'),
        Key::Left => out.push('\u{2190}'),
        Key::Right => out.push('\u{2192}'),
        Key::Home => out.push_str("home"),
        Key::End => out.push_str("end"),
        Key::PageUp => out.push_str("pgup"),
        Key::PageDown => out.push_str("pgdn"),
    }
    out
}

/// The label of a binding: its keys' labels apart by a space (`\u{2303}x b`).
pub fn binding_label(binding: &Binding) -> String {
    binding
        .strokes()
        .iter()
        .map(stroke_label)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The label of an action's first binding, as a hint shows it (`\u{2303}t`,
/// `esc`, `\u{2303}x b`); none while the action is unbound.
pub fn label(keymap: &Keymap, id: ActionId) -> Option<String> {
    keymap.bindings(id).first().map(binding_label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_tui_core::keymap::KeymapSpec;
    use codex_tui_core::keymap::actions::{
        AGENTS_RESUME, GLOBAL_OPEN_TRANSCRIPT, LIST_ACCEPT, LIST_CANCEL, LIST_MOVE_DOWN,
    };
    use rtwk::prelude::KeyCode;

    fn with(table: &str, action: &str, specs: &[&str]) -> Keymap {
        let spec = KeymapSpec::default().with(table, action, specs);
        Keymap::resolve_unchecked(&spec)
    }

    #[test]
    fn the_defaults_are_the_oracles_keys_in_rtwk_names() {
        let keymap = Keymap::defaults();
        assert_eq!(names(&keymap, LIST_ACCEPT), ["enter"]);
        assert_eq!(names(&keymap, LIST_CANCEL), ["escape"]);
        assert!(names(&keymap, LIST_MOVE_DOWN).contains(&"ctrl+n".to_owned()));
    }

    #[test]
    fn a_configured_key_and_a_chord_are_named_for_the_framework() {
        let keymap = with("global", "open_transcript", &["ctrl-x ctrl-t", "f6"]);
        assert_eq!(
            names(&keymap, GLOBAL_OPEN_TRANSCRIPT),
            ["f6", "ctrl+x ctrl+t"]
        );
        assert_eq!(
            label(&keymap, GLOBAL_OPEN_TRANSCRIPT).as_deref(),
            Some("f6")
        );
    }

    #[test]
    fn a_key_named_twice_is_left_to_the_first_of_its_pairs() {
        let pairs = vec![
            ("?".to_owned(), 1),
            ("f9".to_owned(), 2),
            ("shift+?".to_owned(), 3),
            ("f9".to_owned(), 4),
            ("ctrl+x b".to_owned(), 5),
            ("ctrl+x c".to_owned(), 6),
        ];
        assert_eq!(
            first_of_each(pairs),
            [
                ("?".to_owned(), 1),
                ("f9".to_owned(), 2),
                ("ctrl+x b".to_owned(), 5),
                ("ctrl+x c".to_owned(), 6),
            ]
        );
    }

    #[test]
    fn a_hint_is_labelled_the_way_this_platform_prints_it() {
        let ctrl_x_b = Binding::Chord {
            prefix: Stroke::new(Key::Char('x'), Mods::CTRL),
            completion: Stroke::new(Key::Char('b'), Mods::NONE),
        };
        let ctrl = if cfg!(target_os = "macos") {
            "\u{2303}"
        } else if cfg!(target_os = "linux") {
            "^"
        } else {
            "ctrl+"
        };
        assert_eq!(binding_label(&ctrl_x_b), format!("{ctrl}x b"));
        assert_eq!(
            stroke_label(&Stroke::new(Key::PageDown, Mods::NONE)),
            "pgdn"
        );
        let shift_tab = stroke_label(&Stroke::new(Key::BackTab, Mods::NONE));
        assert_eq!(
            shift_tab,
            if cfg!(target_os = "macos") {
                "\u{21e7}tab"
            } else {
                "shift+tab"
            }
        );
    }

    #[test]
    fn a_key_press_is_matched_by_rtwks_combo() {
        let keymap = with("list", "move_down", &["z", "ctrl-j"]);
        assert!(pressed(&keymap, LIST_MOVE_DOWN, &KeyPress::char('z')));
        assert!(!pressed(&keymap, LIST_MOVE_DOWN, &KeyPress::char('j')));
        assert!(pressed(
            &keymap,
            LIST_MOVE_DOWN,
            &KeyPress::char('j').with_mods(rtwk::prelude::Mods::CTRL)
        ));
        let none = with("list", "accept", &[]);
        assert!(!pressed(
            &none,
            LIST_ACCEPT,
            &KeyPress::code(KeyCode::Enter)
        ));
        assert!(names(&none, LIST_ACCEPT).is_empty());
    }

    #[test]
    fn a_shifted_symbol_names_the_symbol() {
        let stroke = Stroke::new(Key::Char('?'), Mods::SHIFT);
        assert_eq!(name_of(&stroke).as_deref(), Some("?"));
        let keymap = with("agents", "resume", &["?", "shift-?"]);
        assert_eq!(names(&keymap, AGENTS_RESUME), ["?"]);
        assert_eq!(
            name_of(&Stroke::new(Key::BackTab, Mods::NONE)).as_deref(),
            Some("shift+tab")
        );
        assert_eq!(
            name_of(&Stroke::new(Key::Char('A'), Mods::CTRL)).as_deref(),
            Some("ctrl+shift+a")
        );
    }
}
