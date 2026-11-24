//! The keypress inspector of `/keymap` (oracle `keymap_setup/debug.rs`): the key the terminal
//! sent as the app detects it, as the config would store it, as the event arrived, and the
//! actions that have it. Esc is inspected; Ctrl+C closes. After three seconds with no key it
//! says what a terminal that sends nothing means. In Flutter it is a `Focus` over a `Column`
//! with an `AnimationController` for the three seconds.

use std::rc::Rc;
use std::time::Duration;

use rtwk::Handler;
use rtwk::animation::{AnimationController, AnimationStatus};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::layout::{BoxWidget, Column, CrossAxisAlignment, Expanded, MainAxisSize, Row};
use rtwk::overlay::Dialog;
use rtwk::provider::select;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextWrap};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::keymap::{self, KeyMatch, Source};
use codex_tui_core::keymap::KeymapSpec;

use super::capture::refusal_words;
use super::strings::{self, Strings};
use crate::app::theme::Tokens;
use crate::keymap::press::{self, is_ctrl_c, stroke_of};
use crate::keymap::{keymap, stroke_label};

/// How long the inspector waits for a first key before it says more (`MISSING_KEY_HINT_DELAY`).
pub const MISSING_KEY_HINT_DELAY: Duration = Duration::from_secs(3);

/// What the last key was.
#[derive(Clone, Debug, PartialEq)]
struct Report {
    detected: String,
    config_key: Result<String, String>,
    raw: String,
    matches: Vec<KeyMatch>,
}

/// The inspector.
#[derive(Clone, Debug, PartialEq)]
pub struct Inspector {
    /// Ctrl+C: the opener closes the inspector.
    pub on_close: Handler<()>,
}

pub struct InspectorState {
    node: FocusNode,
    clock: Option<AnimationController>,
    report: Option<Report>,
    delayed: bool,
}

#[derive(Clone, Debug)]
pub enum Msg {
    Key(KeyPress),
    Delay,
}

impl StatefulWidget for Inspector {
    type State = InspectorState;

    fn create_state(&self) -> InspectorState {
        InspectorState {
            node: FocusNode::new(),
            clock: None,
            report: None,
            delayed: false,
        }
    }
}

impl_widget!(stateful eq Inspector);

/// `code=F(8), modifiers=none, kind=Press`: the event as the oracle prints it.
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

impl InspectorState {
    fn inspect(&mut self, key: &KeyPress, cx: &mut Cx<'_, Self>) {
        let s = strings::of(cx);
        let live = keymap(cx);
        let spec = select::<FeatureStates, KeymapSpec, _>(cx, |states| {
            states.settings.facts.keymap_spec.clone()
        });
        let stroke = stroke_of(key);
        self.report = Some(Report {
            detected: stroke
                .as_ref()
                .map_or_else(|| raw_of(key, s), stroke_label),
            config_key: press::spec_of(key).map_err(|refusal| refusal_words(refusal, s)),
            raw: raw_of(key, s),
            matches: stroke
                .map(|stroke| keymap::matching(&live, &spec, &stroke))
                .unwrap_or_default(),
        });
    }
}

impl State for InspectorState {
    type Widget = Inspector;
    type Msg = Msg;

    fn init_state(&mut self, _: &Inspector, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        self.node.set_on_key_event(Some(Rc::new(move |_, key| {
            send(Msg::Key(key));
            KeyEventResult::Handled
        })));
        let clock = AnimationController::new(cx).with_duration(MISSING_KEY_HINT_DELAY);
        clock.add_status_listener(cx.map(|status: AnimationStatus| {
            let _ = status;
            Msg::Delay
        }));
        clock.forward(Some(0.0), cx);
        self.clock = Some(clock);
    }

    fn dispose(&mut self) {
        self.node.set_on_key_event(None);
        if let Some(clock) = self.clock.take() {
            clock.dispose();
        }
    }

    fn update(&mut self, msg: Msg, w: &Inspector, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Key(key) => {
                if is_ctrl_c(&key) {
                    cx.emit(&w.on_close, ());
                    return Rebuild::No;
                }
                self.inspect(&key, cx);
            }
            Msg::Delay => {
                if self
                    .clock
                    .as_ref()
                    .is_some_and(AnimationController::is_completed)
                {
                    self.delayed = true;
                }
            }
        }
        Rebuild::Yes
    }

    fn build(&self, _: &Inspector, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let s = strings::of(cx);
        let tokens = Tokens::of(cx);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::line(s.debug_title).style(tokens.bold()))
            .child(
                Text::new(s.debug_intro)
                    .style(tokens.dim())
                    .wrap(TextWrap::Word),
            )
            .child(
                Text::new(if self.delayed && self.report.is_none() {
                    s.debug_tip_delayed
                } else {
                    s.debug_tip
                })
                .style(tokens.dim())
                .wrap(TextWrap::Word),
            )
            .child(Text::new(String::new()));
        let Some(report) = &self.report else {
            column = column.child(Text::line(s.debug_waiting).style(tokens.info()));
            return self.frame(column, &tokens, cx);
        };
        column = column.child(Text::rich(vec![
            Span::styled(s.debug_detected.to_owned(), tokens.dim()),
            Span::styled(report.detected.clone(), tokens.info()),
        ]));
        column = match &report.config_key {
            Ok(key) => column.child(Text::rich(vec![
                Span::styled(s.debug_config_key.to_owned(), tokens.dim()),
                Span::styled(key.clone(), tokens.info()),
            ])),
            Err(error) => column.child(hung(
                s.debug_config_key,
                &s.with(s.debug_unsupported, error).to_string(),
                tokens.dim(),
            )),
        };
        column = column
            .child(hung(s.debug_raw, &report.raw, tokens.dim()))
            .child(Text::new(String::new()))
            .child(Text::line(s.debug_assigned).style(tokens.dim()));
        if report.matches.is_empty() {
            column = column.child(Text::line(s.debug_none).style(tokens.dim()));
        } else {
            for matched in &report.matches {
                let line = s
                    .matched(
                        matched.context,
                        matched.action,
                        &matched.label,
                        matched.description,
                        source_word(s, matched.source),
                    )
                    .to_string();
                column = column.child(hung("  - ", &line, tokens.dim()));
            }
        }
        self.frame(column, &tokens, cx)
    }
}

impl InspectorState {
    fn frame(&self, column: Column, tokens: &Tokens, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let surface = tokens.prompt_surface(cx, false);
        Focus::new(Dialog::bare(BoxWidget::new().style(surface).child(column)))
            .focus_node(self.node.clone())
            .autofocus()
            .into()
    }
}

/// A line of text that wraps under its lead (`textwrap`'s initial and subsequent indent).
fn hung(lead: &str, text: &str, style: Style) -> Row {
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::line(lead.to_owned()).style(style.clone()))
        .child(Expanded::new(
            1,
            Text::new(text.to_owned()).style(style).wrap(TextWrap::Word),
        ))
}
