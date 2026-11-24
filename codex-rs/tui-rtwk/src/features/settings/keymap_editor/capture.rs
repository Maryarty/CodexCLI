//! The capture of a key for `/keymap` (oracle `keymap_setup/capture.rs`): one key, a literal
//! chord of two, or the key that follows the leader, read raw (the list's own keys do not
//! apply here). In Flutter it is a `Focus` with an `onKeyEvent` that takes every key, over a
//! `Column` of the instructions. What the key is as the config writes it (`spec_of`) is the
//! oracle's `key_event_to_config_key_spec`; what to do with it is the opener's (`Done`).

use rtwk::Handler;
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding,
};
use rtwk::overlay::Dialog;
use rtwk::prelude::{KeyCode, Mods};
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::keymap::{Key, Mods as StrokeMods, Stroke};

use super::strings::{self, Strings};
use crate::app::theme::Tokens;

/// What the capture takes (`KeymapCaptureMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Single,
    Chord,
    /// The key that follows the configured leader.
    Leader,
}

impl Mode {
    /// The mode that captured `key` (the conflict sheet asks for the same again).
    pub fn of_key(key: &str) -> Mode {
        if key.starts_with("leader ") {
            Mode::Leader
        } else if key.contains(' ') {
            Mode::Chord
        } else {
            Mode::Single
        }
    }
}

/// What the capture came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    /// The key (or the chord, or `leader <key>`) as the config writes it.
    Key(String),
    /// Esc, or Ctrl+C.
    Cancel,
}

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
    };
    Some(Stroke::new(k, mods).canonical())
}

/// The key as `tui.keymap` stores it, or why it cannot be (`key_event_to_config_key_spec`).
pub fn spec_of(key: &KeyPress, s: &Strings) -> Result<String, String> {
    let known = Mods::CTRL | Mods::ALT | Mods::SHIFT | Mods::SUPER;
    if key.mods.0 & !known.0 != 0 {
        return Err(s.key_modifiers.to_owned());
    }
    if key.code == KeyCode::Char
        && let Some(ch) = key.ch
        && ch != '-'
        && (!ch.is_ascii() || ch.is_ascii_control())
    {
        return Err(s.key_printable.to_owned());
    }
    match stroke_of(key) {
        Some(stroke) if stroke.key != Key::Insert => Ok(stroke.spec()),
        _ => Err(s.key_unsupported.to_owned()),
    }
}

/// The sheet's instructions for what is left to press.
fn instructions(s: &Strings, mode: Mode, first: Option<&str>) -> String {
    match (mode, first) {
        (Mode::Single, _) => s.capture_single.to_owned(),
        (Mode::Chord, None) => s.capture_chord_first.to_owned(),
        (Mode::Chord, Some(first)) => s.with(s.capture_chord_second, first).to_string(),
        (Mode::Leader, _) => s.capture_leader.to_owned(),
    }
}

/// The capture view.
#[derive(Clone, Debug, PartialEq)]
pub struct Capture {
    pub context: String,
    pub action: String,
    /// What the action is bound to now, as the menu says it.
    pub current: String,
    pub mode: Mode,
    pub on_done: Handler<Done>,
}

pub struct CaptureState {
    node: FocusNode,
    first: Option<String>,
    error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Key(KeyPress),
}

impl StatefulWidget for Capture {
    type State = CaptureState;

    fn create_state(&self) -> CaptureState {
        CaptureState {
            node: FocusNode::new(),
            first: None,
            error: None,
        }
    }
}

impl_widget!(stateful eq Capture);

impl State for CaptureState {
    type Widget = Capture;
    type Msg = Msg;

    fn init_state(&mut self, _: &Capture, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        self.node.set_on_key_event(Some(std::rc::Rc::new(move |_, key| {
            send(Msg::Key(key));
            KeyEventResult::Handled
        })));
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
    }

    fn update(&mut self, msg: Msg, w: &Capture, cx: &mut Cx<'_, Self>) -> Rebuild {
        let Msg::Key(key) = msg;
        let escape = key.code == KeyCode::Escape && key.mods == Mods::NONE;
        let interrupt = key.code == KeyCode::Char
            && key.ch == Some('c')
            && key.mods == Mods::CTRL;
        if escape || interrupt {
            cx.emit(&w.on_done, Done::Cancel);
            return Rebuild::No;
        }
        let s = strings::of(cx);
        match spec_of(&key, s) {
            Ok(spec) => {
                self.error = None;
                let done = match w.mode {
                    Mode::Single => spec,
                    Mode::Leader => s.leader_key(&spec),
                    Mode::Chord => {
                        let Some(first) = self.first.take() else {
                            self.first = Some(spec);
                            return Rebuild::Yes;
                        };
                        format!("{first} {spec}")
                    }
                };
                cx.emit(&w.on_done, Done::Key(done));
            }
            Err(error) => self.error = Some(error),
        }
        Rebuild::Yes
    }

    fn build(&self, w: &Capture, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let blank = || Text::new(String::new());
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(blank())
            .child(Text::line(s.capture_title).style(tokens.bold()))
            .child(
                Text::rich(vec![
                    Span::styled(s.capture_action.to_owned(), tokens.dim()),
                    Span::new(super::lists::label_of(&w.action)),
                    Span::new(s.id_gap()),
                    Span::styled(s.action_id(&w.context, &w.action), tokens.dim()),
                ])
                .wrap(TextWrap::Word),
            )
            .child(
                Text::rich(vec![
                    Span::styled(s.capture_current.to_owned(), tokens.dim()),
                    Span::styled(w.current.clone(), tokens.accent()),
                ])
                .wrap(TextWrap::Word),
            )
            .child(
                Text::new(instructions(s, w.mode, self.first.as_deref()))
                    .style(tokens.dim())
                    .wrap(TextWrap::Word),
            );
        if let Some(error) = &self.error {
            column = column.child(blank()).child(
                Text::new(s.capture_failed(error))
                    .style(tokens.error())
                    .wrap(TextWrap::Word),
            );
        }
        let column = column.child(blank());
        let surface: Style = tokens.prompt_surface(cx, false);
        Focus::new(Dialog::bare(
            BoxWidget::new()
                .style(surface)
                .child(Padding::new(EdgeInsets::symmetric(2, 0), column)),
        ))
        .focus_node(self.node.clone())
        .autofocus()
        .into()
    }
}
