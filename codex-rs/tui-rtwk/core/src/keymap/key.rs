//! A key as the keymap config names it: one stroke (`ctrl-t`, `page-down`) and
//! a binding of one stroke or a two-stroke chord (`ctrl-x enter`).
//!
//! Carried from the oracle's `config/src/tui_keymap.rs` (the canonical
//! spelling: modifiers in the order `ctrl-alt-shift-super`, key names `esc`,
//! `page-up`, `space`, `minus`) and `tui/src/key_hint.rs` (`normalized_parts`,
//! `display_label`). No crossterm and no rtwk: the view maps a `Stroke` to
//! its own key type.

use std::fmt;

/// The highest function key a config may bind (`config` `MAX_FUNCTION_KEY`).
pub const MAX_FUNCTION_KEY: u8 = 24;

/// The key of a stroke.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    Char(char),
    F(u8),
    Enter,
    Tab,
    /// Shift+Tab as a terminal reports it.
    BackTab,
    Backspace,
    Esc,
    Delete,
    Insert,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

/// The modifiers held with a key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Mods(u8);

impl Mods {
    pub const NONE: Mods = Mods(0);
    pub const CTRL: Mods = Mods(1);
    pub const ALT: Mods = Mods(2);
    pub const SHIFT: Mods = Mods(4);
    pub const SUPER: Mods = Mods(8);

    pub const fn contains(self, other: Mods) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn with(self, other: Mods) -> Mods {
        Mods(self.0 | other.0)
    }

    #[must_use]
    pub const fn without(self, other: Mods) -> Mods {
        Mods(self.0 & !other.0)
    }

    /// Ctrl and Alt together, which a terminal on some layouts sends for
    /// AltGr (`key_hint::is_altgr`, the Windows spelling).
    pub const fn is_altgr(self) -> bool {
        self.contains(Mods::CTRL) && self.contains(Mods::ALT)
    }
}

/// One key event a binding can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Stroke {
    pub key: Key,
    pub mods: Mods,
}

impl Stroke {
    pub const fn new(key: Key, mods: Mods) -> Stroke {
        Stroke { key, mods }
    }

    /// The stroke a canonical spec names: `ctrl-alt-shift-super-<key>`.
    /// Config deserialization already normalizes (lower case, aliases); this
    /// parser stays strict, but takes an upper-case letter as itself because
    /// the built-in defaults hold `A` beside `shift-a` (`parse_keybinding`).
    pub fn parse(spec: &str) -> Option<Stroke> {
        let mut parts = spec.split('-');
        let mut mods = Mods::NONE;
        let mut key_name = None;
        for part in parts.by_ref() {
            match part {
                "ctrl" => mods = mods.with(Mods::CTRL),
                "alt" => mods = mods.with(Mods::ALT),
                "shift" => mods = mods.with(Mods::SHIFT),
                "super" => mods = mods.with(Mods::SUPER),
                other => {
                    key_name = Some(other.to_string());
                    break;
                }
            }
        }
        let mut key_name = key_name?;
        for trailing in parts {
            key_name.push('-');
            key_name.push_str(trailing);
        }
        let key = match key_name.as_str() {
            "enter" => Key::Enter,
            "tab" => Key::Tab,
            "backspace" => Key::Backspace,
            "esc" => Key::Esc,
            "delete" => Key::Delete,
            "insert" => Key::Insert,
            "up" => Key::Up,
            "down" => Key::Down,
            "left" => Key::Left,
            "right" => Key::Right,
            "home" => Key::Home,
            "end" => Key::End,
            "page-up" => Key::PageUp,
            "page-down" => Key::PageDown,
            "space" => Key::Char(' '),
            "minus" => Key::Char('-'),
            other => {
                let mut chars = other.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if c.is_ascii() && !c.is_ascii_control() => Key::Char(c),
                    (Some('f'), Some(_)) => {
                        let number = other[1..].parse::<u8>().ok()?;
                        if !(1..=MAX_FUNCTION_KEY).contains(&number) {
                            return None;
                        }
                        Key::F(number)
                    }
                    _ => return None,
                }
            }
        };
        Some(Stroke { key, mods })
    }

    /// The canonical spec (`Stroke::parse` reads it back).
    pub fn spec(&self) -> String {
        let mut out = String::new();
        for (m, name) in [
            (Mods::CTRL, "ctrl"),
            (Mods::ALT, "alt"),
            (Mods::SHIFT, "shift"),
            (Mods::SUPER, "super"),
        ] {
            if self.mods.contains(m) {
                out.push_str(name);
                out.push('-');
            }
        }
        match self.key {
            Key::Char(' ') => out.push_str("space"),
            Key::Char('-') => out.push_str("minus"),
            Key::Char(c) => out.push(c),
            Key::F(n) => out.push_str(&format!("f{n}")),
            Key::Enter => out.push_str("enter"),
            Key::Tab => out.push_str("tab"),
            Key::BackTab => out.push_str("shift-tab"),
            Key::Backspace => out.push_str("backspace"),
            Key::Esc => out.push_str("esc"),
            Key::Delete => out.push_str("delete"),
            Key::Insert => out.push_str("insert"),
            Key::Up => out.push_str("up"),
            Key::Down => out.push_str("down"),
            Key::Left => out.push_str("left"),
            Key::Right => out.push_str("right"),
            Key::Home => out.push_str("home"),
            Key::End => out.push_str("end"),
            Key::PageUp => out.push_str("page-up"),
            Key::PageDown => out.push_str("page-down"),
        }
        out
    }

    /// What two strokes are compared by (`KeyBinding::normalized_parts`,
    /// `normalize_chord_binding`): Ctrl+5 is Ctrl+] and Ctrl+7 is Ctrl+/, the
    /// one key a terminal sends for both.
    pub fn normalized(&self) -> (Key, Mods) {
        match (self.key, self.mods) {
            (Key::Char('5'), Mods::CTRL) => (Key::Char(']'), Mods::CTRL),
            (Key::Char('7'), Mods::CTRL) => (Key::Char('/'), Mods::CTRL),
            (key, mods) => (key, mods),
        }
    }

    /// The stroke as the keymap stores a chord's keys.
    pub fn canonical(&self) -> Stroke {
        let (key, mods) = self.normalized();
        Stroke { key, mods }
    }

    /// Whether `other` is the same key for the keymap's duplicate checks.
    pub fn same_key(&self, other: &Stroke) -> bool {
        self.normalized() == other.normalized()
    }

    /// Whether the stroke types a character on its own (no Ctrl, Alt or
    /// Super): the keys a text field takes (`is_plain_text_key_event`).
    pub fn is_plain_text(&self) -> bool {
        matches!(self.key, Key::Char(_))
            && !self.mods.contains(Mods::CTRL)
            && !self.mods.contains(Mods::ALT)
            && !self.mods.contains(Mods::SUPER)
    }

    /// The name a hint shows (`KeyBinding::display_label`, the glyph form of
    /// macOS and of the oracle's tests): `⌃t`, `⇧↑`, `esc`, `pgdn`.
    pub fn label(&self) -> String {
        if self.key == Key::BackTab {
            return Stroke::new(Key::Tab, self.mods.with(Mods::SHIFT)).label();
        }
        let mut out = String::new();
        for (m, glyph) in [
            (Mods::CTRL, "⌃"),
            (Mods::SHIFT, "⇧"),
            (Mods::ALT, "⌥"),
            (Mods::SUPER, "⌘"),
        ] {
            if self.mods.contains(m) {
                out.push_str(glyph);
            }
        }
        match self.key {
            Key::Char(' ') => out.push_str("space"),
            Key::Char(c) => out.push(c.to_ascii_lowercase()),
            Key::F(n) => out.push_str(&format!("f{n}")),
            Key::Enter => out.push_str("enter"),
            Key::Tab | Key::BackTab => out.push_str("tab"),
            Key::Backspace => out.push_str("backspace"),
            Key::Esc => out.push_str("esc"),
            Key::Delete => out.push_str("del"),
            Key::Insert => out.push_str("insert"),
            Key::Up => out.push('↑'),
            Key::Down => out.push('↓'),
            Key::Left => out.push('←'),
            Key::Right => out.push('→'),
            Key::Home => out.push_str("home"),
            Key::End => out.push_str("end"),
            Key::PageUp => out.push_str("pgup"),
            Key::PageDown => out.push_str("pgdn"),
        }
        out
    }
}

impl fmt::Display for Stroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.spec())
    }
}

/// What a binding is: one stroke, or a prefix and the stroke that completes
/// it (`ctrl-x enter`, `leader b`, which the keymap expands with the leader
/// key as the prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Binding {
    Single(Stroke),
    Chord { prefix: Stroke, completion: Stroke },
}

impl Binding {
    /// The stroke that starts the binding.
    pub fn first(&self) -> Stroke {
        match self {
            Binding::Single(stroke) => *stroke,
            Binding::Chord { prefix, .. } => *prefix,
        }
    }

    /// The single stroke, if the binding is one.
    pub fn single(&self) -> Option<Stroke> {
        match self {
            Binding::Single(stroke) => Some(*stroke),
            Binding::Chord { .. } => None,
        }
    }

    /// The strokes in the order they are pressed.
    pub fn strokes(&self) -> Vec<Stroke> {
        match self {
            Binding::Single(stroke) => vec![*stroke],
            Binding::Chord { prefix, completion } => vec![*prefix, *completion],
        }
    }

    /// The hint's label (`ShortcutHint::display_label`): the strokes' labels
    /// apart by a space.
    pub fn label(&self) -> String {
        match self {
            Binding::Single(stroke) => stroke.label(),
            Binding::Chord { prefix, completion } => {
                format!("{} {}", prefix.label(), completion.label())
            }
        }
    }

    pub fn same_key(&self, other: &Binding) -> bool {
        match (self, other) {
            (Binding::Single(a), Binding::Single(b)) => a.same_key(b),
            (
                Binding::Chord {
                    prefix: a,
                    completion: x,
                },
                Binding::Chord {
                    prefix: b,
                    completion: y,
                },
            ) => a.same_key(b) && x.same_key(y),
            _ => false,
        }
    }
}

/// Why a spec is not a binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecError {
    /// A stroke the parser does not know.
    Stroke,
    /// More than two strokes, or a `leader` not followed by one key.
    Shape,
}

/// What a spec means before the leader keys are known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spec {
    Binding(Binding),
    /// `leader <key>`: one binding for each leader key.
    Leader(Stroke),
}

/// Reads one spec: a stroke, `prefix completion`, or `leader completion`.
pub fn parse_spec(spec: &str) -> Result<Spec, SpecError> {
    let strokes: Vec<&str> = spec.split_whitespace().collect();
    match strokes.as_slice() {
        [one] => Stroke::parse(one)
            .map(|stroke| Spec::Binding(Binding::Single(stroke)))
            .ok_or(SpecError::Stroke),
        ["leader", completion] => Stroke::parse(completion)
            .map(Spec::Leader)
            .ok_or(SpecError::Stroke),
        [prefix, completion] => {
            let prefix = Stroke::parse(prefix).ok_or(SpecError::Stroke)?;
            let completion = Stroke::parse(completion).ok_or(SpecError::Stroke)?;
            Ok(Spec::Binding(Binding::Chord { prefix, completion }))
        }
        _ => Err(SpecError::Shape),
    }
}
