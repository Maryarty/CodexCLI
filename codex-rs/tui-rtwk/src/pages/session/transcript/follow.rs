//! What the chat shows of its transcript's reader (oracle
//! `transcript_view/follow_control.rs`, `composer_gap.rs`, `footer.rs`): the
//! row reserved above the composer, with the "back to bottom" control in it
//! while the reader is away from the last row, and the footer's hint for the
//! way back.
//!
//! Flutter's `ListenableBuilder` over the reader's controller (`Follow`) and,
//! for the control, a `MouseRegion` (hover) over a `GestureDetector` (tap)
//! over a `Text` the width picks the spelling of (`LayoutBuilder` +
//! `segmenter_of`, as `widgets/dots.rs`). Nothing here takes the focus: the
//! composer keeps its caret.

use std::rc::Rc;

use rtwk::editing::TextEditingController;
use rtwk::focus::KeyPress;
use rtwk::foundation::{Listenable, ListenableBuilder};
use rtwk::gestures::{GestureDetector, MouseRegion, PointerEvent};
use rtwk::layout::{Center, Column, Expanded, Flexible, MainAxisSize, Row, SizedBox};
use rtwk::prelude::KeyCode;
use rtwk::provider::watch;
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::{Text, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::composer::trigger;

use crate::app::theme;
use crate::widgets::thread_ui::{Follow, TextSelection};

use super::copy::Shown;
use super::copy_feedback::CopyFeedback;
use super::strings::{self, Strings};

/// The row above the composer, there while the window has the room (the oracle
/// reserves it, so the composer does not move when the control appears; it is
/// a flexible child of the bottom pane, the first thing a short window clips):
/// the control while the reader is away, or the result of the last copy (the oracle's
/// `composer_gap.rs`: it wins over the control). With `status_above` (the working line
/// or the previews are above), a blank row precedes the control to set it
/// apart.
pub fn gap<S>(cx: &mut Cx<'_, S>, follow: Follow, status_above: bool) -> AnyWidget {
    let table = strings::of(cx);
    let tokens = theme::Tokens::of(cx);
    // the user message's surface, in the accent
    let surface = tokens.prompt_surface(cx, true);
    let style = match tokens.accent().fg {
        Some(fg) => surface.fg(fg),
        None => surface,
    };
    let copied = watch::<Shown, _>(cx);
    let listenable = Rc::new(follow.listenable());
    let builder = ListenableBuilder::new(listenable, move |_, _| {
        let blank: AnyWidget = SizedBox::new(None, Some(1), None).into();
        let plain = if follow.away() {
            let control = FollowControl {
                follow: follow.clone(),
                unseen: follow.unseen(),
                table,
                style: style.clone(),
            };
            let row: AnyWidget =
                SizedBox::new(None, Some(1), Some(Center::new(control).into())).into();
            if status_above {
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .child(blank)
                    .child(row)
                    .into()
            } else {
                row
            }
        } else {
            blank
        };
        match &copied.0 {
            Some(feedback) => CopyFeedback {
                feedback: feedback.clone(),
                lead: status_above,
                fallback: plain,
            }
            .into(),
            None => plain,
        }
    });
    Flexible::new(1, builder).into()
}

/// The control: a label in the surface of the user's messages, the widest
/// that fits the row; reversed and bold under the pointer; a tap returns to
/// the last row.
#[derive(Clone)]
struct FollowControl {
    follow: Follow,
    unseen: bool,
    table: &'static Strings,
    style: Style,
}

impl PartialEq for FollowControl {
    fn eq(&self, other: &Self) -> bool {
        self.follow == other.follow
            && self.unseen == other.unseen
            && std::ptr::eq(self.table, other.table)
            && self.style == other.style
    }
}

struct FollowControlState {
    hovered: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Msg {
    Hover(bool),
    Tap,
}

impl StatefulWidget for FollowControl {
    type State = FollowControlState;

    fn create_state(&self) -> FollowControlState {
        FollowControlState { hovered: false }
    }
}

impl_widget!(stateful eq FollowControl);

impl State for FollowControlState {
    type Widget = FollowControl;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &FollowControl, _: &mut Cx<'_, Self>) -> Rebuild {
        match msg {
            Msg::Hover(on) => {
                self.hovered = on;
                Rebuild::Yes
            }
            Msg::Tap => {
                w.follow.jump_to_latest();
                Rebuild::No
            }
        }
    }

    fn build(&self, w: &FollowControl, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let labels = if w.unseen {
            w.table.follow_new
        } else {
            w.table.follow_back
        };
        let style = if self.hovered {
            w.style.clone().attrs(Attrs::REVERSE | Attrs::BOLD)
        } else {
            w.style.clone()
        };
        let enter = cx.map(|_: PointerEvent| Msg::Hover(true));
        let exit = cx.map(|_: PointerEvent| Msg::Hover(false));
        let tap = cx.map(|_: PointerEvent| Msg::Tap);
        LayoutBuilder::new(move |cx, constraints| {
            let segmenter = segmenter_of(cx);
            let label = labels
                .into_iter()
                .find(|label| {
                    i32::try_from(segmenter.width(label)).unwrap_or(i32::MAX)
                        <= constraints.max_width
                })
                .unwrap_or("");
            // a blank label draws nothing and takes no click
            if label.is_empty() {
                return SizedBox::new(Some(0), Some(0), None).into();
            }
            MouseRegion::new()
                .on_enter(enter.clone())
                .on_exit(exit.clone())
                .child(
                    GestureDetector::new()
                        .on_tap(tap.clone())
                        .child(Text::line(label).style(style.clone())),
                )
                .into()
        })
        .into()
    }
}

/// The footer while something is selected (the keys of the selection), or the reader is away
/// (the keys that return): `keys` (the footer of the page) otherwise. `enter`: an Enter on the
/// empty composer returns too. The news is named first, then in short, then dropped, as the
/// width allows (`footer.rs`).
pub fn footer<S>(
    cx: &mut Cx<'_, S>,
    follow: Follow,
    selection: TextSelection,
    enter: bool,
    keys: AnyWidget,
) -> AnyWidget {
    let table = strings::of(cx);
    let tokens = theme::Tokens::of(cx);
    let (key_style, dim) = (tokens.bold(), tokens.dim());
    let listenable = Rc::new(<dyn Listenable>::merge([
        Some(follow.listenable()),
        Some(selection.listenable()),
    ]));
    ListenableBuilder::new(listenable, move |_, _| {
        let selected = selection.held();
        if !follow.away() && !selected {
            return keys.clone();
        }
        let unseen = follow.unseen();
        let (key_style, dim) = (key_style.clone(), dim.clone());
        let hint: AnyWidget = LayoutBuilder::new(move |cx, constraints| {
            let segmenter = segmenter_of(cx);
            let navigation = |lead: &'static str| -> Vec<Span> {
                let mut spans = Vec::new();
                if !lead.is_empty() {
                    spans.push(Span::styled(lead, dim.clone()));
                    spans.push(Span::styled(table.footer_joiner, dim.clone()));
                }
                spans.push(Span::styled(
                    if enter {
                        table.latest_keys_enter
                    } else {
                        table.latest_keys
                    },
                    key_style.clone(),
                ));
                spans.push(Span::styled(table.latest_action, dim.clone()));
                spans
            };
            // what the keys of a selection do: pairs of keys and their action
            let hint = |pairs: &[(&'static str, &'static str)]| -> Vec<Span> {
                let mut spans = Vec::new();
                for (index, (keys, action)) in pairs.iter().enumerate() {
                    if index > 0 {
                        spans.push(Span::styled(table.footer_joiner, dim.clone()));
                    }
                    spans.push(Span::styled(*keys, key_style.clone()));
                    if !action.is_empty() {
                        spans.push(Span::styled(*action, dim.clone()));
                    }
                }
                spans
            };
            let candidates: Vec<Vec<Span>> = if selected {
                // the widest hint, led by the news while the reader is away and has some
                // (`status_line_with_navigation`), then the hints alone
                let mut all = Vec::new();
                if unseen {
                    let mut news = vec![
                        Span::styled(table.new_activity, dim.clone()),
                        Span::styled(table.footer_joiner, dim.clone()),
                    ];
                    news.extend(hint(table.selection_hints[0]));
                    all.push(news);
                }
                all.extend(table.selection_hints.iter().map(|pairs| hint(pairs)));
                all
            } else if unseen {
                vec![
                    navigation(table.new_activity),
                    navigation(table.new_short),
                    navigation(""),
                ]
            } else {
                vec![navigation("")]
            };
            let fits = |spans: &Vec<Span>| {
                let cells: usize = spans.iter().map(|span| segmenter.width(&span.text)).sum();
                i32::try_from(cells).unwrap_or(i32::MAX) <= constraints.max_width
            };
            // none fits: the shortest, clipped
            let mut chosen = candidates
                .iter()
                .find(|spans| fits(spans))
                .or(candidates.last())
                .cloned()
                .unwrap_or_default();
            // an empty composer adds the key of its shortcuts when it fits
            if enter && !selected {
                let shortcuts = [
                    Span::styled(table.footer_joiner, dim.clone()),
                    Span::styled(table.shortcuts_key, key_style.clone()),
                    Span::styled(table.shortcuts_action, dim.clone()),
                ];
                let both: Vec<Span> = chosen.iter().cloned().chain(shortcuts).collect();
                if fits(&both) {
                    chosen = both;
                }
            }
            Text::rich(chosen).into()
        })
        .into();
        // two cells in, as the footer's own row
        Row::new()
            .child(SizedBox::new(Some(2), None, None))
            .child(Expanded::new(1, hint))
            .into()
    })
    .into()
}

/// The pages PageUp (-1) and PageDown (1) turn the transcript, wherever the
/// caret is (`handle_scroll_key`; the field would move its caret): none for
/// another key, or while the text before the caret opens the popup of
/// suggestions, whose list the keys are.
pub fn pager_key(key: &KeyPress, composer: &TextEditingController) -> Option<i8> {
    let pages = match key.code {
        KeyCode::PageUp => -1,
        KeyCode::PageDown => 1,
        _ => return None,
    };
    if !key.mods.is_empty() {
        return None;
    }
    let text = composer.text();
    let selection = composer.selection();
    let before = text.get(..selection.extent_offset).unwrap_or("");
    (selection.is_collapsed() && trigger(before).is_none()).then_some(pages)
}
