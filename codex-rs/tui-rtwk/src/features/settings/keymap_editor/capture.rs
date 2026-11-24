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
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::keymap::press::{self, Refusal};

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

/// What the capture view says of a key the config cannot store.
pub fn refusal_words(refusal: Refusal, s: &Strings) -> String {
    match refusal {
        Refusal::Modifiers => s.key_modifiers,
        Refusal::Printable => s.key_printable,
        Refusal::Unsupported => s.key_unsupported,
    }
    .to_owned()
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
        if press::is_escape(&key) || press::is_ctrl_c(&key) {
            cx.emit(&w.on_done, Done::Cancel);
            return Rebuild::No;
        }
        let s = strings::of(cx);
        match press::spec_of(&key).map_err(|refusal| refusal_words(refusal, s)) {
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
