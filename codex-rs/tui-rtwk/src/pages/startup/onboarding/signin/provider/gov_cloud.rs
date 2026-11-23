//! The GovCloud guidance (`onboarding/bedrock.rs`, `BedrockState::guidance`
//! and the guidance branch of `render`): shown once at launch when the
//! Bedrock authentication resolves to AWS GovCloud, until it is
//! acknowledged.
//!
//! In Flutter it is a `StatefulWidget` whose `State` holds the
//! `ScrollController` of the text. The text and the acknowledge row sit
//! together when they fit the pane; in a pane too short for them the row is
//! kept at the bottom with its hint, and the text scrolls above it (Up and
//! Down by one row, the wheel, a `SingleChildScrollView`). Enter or a tap
//! on the row acknowledges; the parent decides what that does (the
//! `acknowledge` callback, `onPressed`).

use std::rc::Rc;
use std::time::Duration;

use rtwk::AnyWidget;
use rtwk::binding::{AppBinding, InputListener};
use rtwk::focus::{Focus, FocusNode, KeyEventResult, KeyPress};
use rtwk::gestures::{GestureDetector, PointerEvent};
use rtwk::layout::{Column, CrossAxisAlignment, Flexible, MainAxisSize};
use rtwk::prelude::HitTestBehavior;
use rtwk::provider::read;
use rtwk::scroll::{ScrollController, SingleChildScrollView};
use rtwk::style::{Attrs, Span, Style};
use rtwk::{Cx, Handler, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};
use rtwk_core::input::{InputEvent, KeyKind};

use codex_tui_core::keymap::fixed;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::onboarding::Intent;

use super::super::{Line, Look, column_of, guarded_keys, rows_of, width_of};
use super::{sized, strings};
use crate::app::theme::Tokens;
use crate::app::view::Hs;
use crate::keymap::{event_is_fixed, pressed_fixed};
use crate::widgets::key_guard::KeyGuard;

/// The guidance; `acknowledge` runs on Enter or a tap on its row.
#[derive(Clone, PartialEq)]
pub struct GovCloudGuidance {
    pub acknowledge: Handler<()>,
}

pub struct GovCloudState {
    scroll: ScrollController,
    node: FocusNode,
}

pub enum Msg {
    Up,
    Down,
    Acknowledge,
    Quit,
}

impl StatefulWidget for GovCloudGuidance {
    type State = GovCloudState;

    fn create_state(&self) -> GovCloudState {
        GovCloudState {
            scroll: ScrollController::default(),
            node: FocusNode::new(),
        }
    }
}

impl_widget!(stateful eq GovCloudGuidance);

impl State for GovCloudState {
    type Widget = GovCloudGuidance;
    type Msg = Msg;

    fn init_state(&mut self, _: &GovCloudGuidance, cx: &mut Cx<'_, Self>) {
        let send = cx.local_sender();
        let clock = AppBinding::maybe_of(cx).map(|binding| binding.clock());
        let now: Rc<dyn Fn() -> Duration> =
            Rc::new(move || clock.as_ref().map_or(Duration::ZERO, |clock| clock.now()));
        let quit = {
            let send = send.clone();
            Rc::new(move || send(Msg::Quit))
        };
        // The quit keys (Ctrl+C, Ctrl+D, q) meet the guidance's node first;
        // nothing is held back, no key opened this screen.
        let guarded = guarded_keys(KeyGuard::default(), now, Rc::new(|| false), quit);
        self.node
            .set_on_key_event(Some(Rc::new(move |node, key: KeyPress| {
                if guarded(node, key.clone()) == KeyEventResult::Handled {
                    return KeyEventResult::Handled;
                }
                let msg = if pressed_fixed(fixed::CONFIRM_PREVIOUS, &key) {
                    Msg::Up
                } else if pressed_fixed(fixed::CONFIRM_NEXT, &key) {
                    Msg::Down
                } else if pressed_fixed(fixed::ENTER, &key) {
                    Msg::Acknowledge
                } else {
                    return KeyEventResult::Ignored;
                };
                send(msg);
                KeyEventResult::Handled
            })));
    }

    fn update(&mut self, msg: Msg, w: &GovCloudGuidance, cx: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Up | Msg::Down => {
                if self.scroll.has_clients() {
                    let position = self.scroll.position();
                    let to = position.pixels() + if matches!(msg, Msg::Up) { -1 } else { 1 };
                    position.jump_to(to);
                }
            }
            Msg::Acknowledge => cx.emit(&w.acknowledge, ()),
            // Leaving from a step cancels the launch (`should_exit`).
            Msg::Quit => {
                let hs = read::<Hs, _>(cx);
                cx.emit(&hs.screen, ScreenIntent::Onboarding(Intent::Exit));
            }
        }
        Rebuild::No
    }

    fn build(&self, _: &GovCloudGuidance, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let strings = strings::of(cx);
        let tokens = Tokens::of(cx);
        let look = Look {
            dim: Style::new().attrs(Attrs::DIM),
            key: tokens.bold(),
            cyan: tokens.info(),
            green: tokens.success(),
            red: tokens.error(),
        };
        let [before, link, after] = strings.gov_cloud_text;
        let text: Vec<Line> = vec![
            vec![Span::new(strings.gov_cloud_title).bold()],
            Vec::new(),
            vec![
                Span::new(before),
                Span::styled(link, look.cyan.clone())
                    .underline()
                    .link(strings.gov_cloud_url.to_owned()),
                Span::new(after),
            ],
        ];
        let acknowledge: Line = vec![Span::styled(strings.acknowledge, look.cyan.clone())];
        let press: Line = vec![
            Span::styled(strings.press, look.dim.clone()),
            Span::styled(strings.enter, look.key.clone()),
            Span::styled(strings.press_to_continue, look.dim.clone()),
        ];
        // Where the pane is too short: the acknowledge row, and under it
        // the keys, kept at the bottom.
        let [up, slash, down, scroll_words, enter] = strings.gov_cloud_scroll;
        let keys: Line = vec![
            Span::new(up),
            Span::styled(slash, look.dim.clone()),
            Span::new(down),
            Span::styled(scroll_words, look.dim),
            Span::new(enter),
        ];
        let full_footer: Vec<Line> = vec![Vec::new(), acknowledge.clone(), Vec::new(), press];
        let tap: Handler<PointerEvent> = cx.map(|_| Msg::Acknowledge);
        let scroll = self.scroll.clone();
        let focus = self.node.clone();
        let body = LayoutBuilder::new(move |cx, constraints| {
            let width = width_of(constraints.max_width);
            let height = usize::try_from(constraints.max_height).unwrap_or(0);
            let row = |lines: Vec<Line>, cx: &mut Cx<'_, _>| -> AnyWidget {
                GestureDetector::new()
                    .behavior(HitTestBehavior::Opaque)
                    .on_tap(tap.clone())
                    .child(sized(cx, lines, width))
                    .into()
            };
            let fits = rows_of(cx, &text, width) + rows_of(cx, &full_footer, width) <= height;
            if fits {
                return Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(sized(cx, text.clone(), width))
                    .child(row(full_footer.clone(), cx))
                    .into();
            }
            let mut footer = vec![acknowledge.clone()];
            if height > 2 {
                footer.push(keys.clone());
            }
            Column::new()
                .main_axis_size(MainAxisSize::Max)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(Flexible::new(
                    1,
                    SingleChildScrollView::new(column_of(&text)).controller(scroll.clone()),
                ))
                .child(row(footer, cx))
                .into()
        });
        let focused = Focus::new(body).focus_node(focus).autofocus();
        // A held Enter does not acknowledge again (`KeyEventKind::Press`):
        // repeats of the other keys are swallowed, the arrows scroll.
        InputListener::with_raw_key_repeats(focused, move |event| {
            let InputEvent::Key(key) = event else {
                return false;
            };
            let arrow = event_is_fixed(fixed::CONFIRM_PREVIOUS, key)
                || event_is_fixed(fixed::CONFIRM_NEXT, key);
            key.kind == KeyKind::Repeat && !arrow
        })
        .into()
    }
}
