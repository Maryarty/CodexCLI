//! The transcript sheet (oracle `pager_overlay/transcript.rs`, Ctrl+T): the
//! whole transcript with its reasoning and warnings, full screen over the
//! chat, read with the pager's keys, and the prompts browsed with Esc.
//!
//! Flutter's full-screen route of a `Column`: the title row, the same
//! transcript list the chat shows (`TranscriptPane`, detailed), the status
//! row (the news and the select key, or the browsing keys), two hint rows.
//! The scroll state is the thread's `Follow` controller, shared with the chat
//! so the reader's place survives the sheet; the mode is the thread's `Modes`
//! controller; the document is the thread model's, read through the provider.

use std::rc::Rc;

use rtwk::focus::{Focus, KeyPress};
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize};
use rtwk::navigation::Navigator;
use rtwk::prelude::KeyCode;
use rtwk::provider::read;
use rtwk::style::{Span, Style};
use rtwk::text::{Text, TextOverflow, segmenter_of};
use rtwk::{AnyWidget, Cx, LayoutBuilder, Rebuild, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::backtrack::Targets;
use codex_tui_core::threads::ThreadModel;

use super::pager;
use super::strings::{self, Strings};
use crate::app::debug::seeded::Seeded;
use crate::pages::session::transcript_pane::TranscriptPane;
use crate::widgets::thread_ui::{Follow, Modes};

/// The sheet over the transcript of the thread below it.
#[derive(Clone, PartialEq)]
pub struct TranscriptSheet {
    pub follow: Follow,
    pub modes: Modes,
}

/// The sheet of the active thread inside a seeded debug entry.
pub fn seeded(seeded: &Seeded<'_>) -> AnyWidget {
    let ui = seeded.thread_ui.get(seeded.core.session().active.as_ref());
    TranscriptSheet {
        follow: ui.follow.clone(),
        modes: ui.modes.clone(),
    }
    .into()
}

/// Widget state: none (R14); the scroll is the `Follow` controller's, the
/// mode the `Modes` controller's.
pub struct SheetState;

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Pager(pager::Msg),
    Esc,
    Older,
    Newer,
    Confirm,
}

impl StatefulWidget for TranscriptSheet {
    type State = SheetState;

    fn create_state(&self) -> SheetState {
        SheetState
    }
}

impl_widget!(stateful eq TranscriptSheet);

impl State for SheetState {
    type Widget = TranscriptSheet;
    type Msg = Msg;

    fn update(&mut self, msg: Msg, w: &TranscriptSheet, cx: &mut Cx<'_, Self>) -> Rebuild {
        let thread = read::<ThreadModel, _>(cx);
        let targets = Targets::of(thread.transcript(), thread.can_load_history());
        match msg {
            Msg::Pager(pager::Msg::Rows(rows)) => w.follow.scroll_by(rows),
            Msg::Pager(pager::Msg::Pages(pages)) => w.follow.page(pages),
            Msg::Pager(pager::Msg::HalfPages(pages)) => w.follow.half_page(pages),
            Msg::Pager(pager::Msg::Top) => w.follow.jump_to_first(),
            Msg::Pager(pager::Msg::Bottom) => w.follow.jump_to_latest(),
            // closing is the sheet's; what the opener does with it (the chat
            // clears its `detailed` flag) is the opener's
            Msg::Pager(pager::Msg::Close | pager::Msg::Action) => {
                w.modes.reset();
                Navigator::maybe_pop(cx);
            }
            // one Esc starts browsing, the next ends it (`overlay_forward_event`)
            Msg::Esc => {
                w.modes.esc(Some(thread.id()), targets);
            }
            Msg::Older if w.modes.browsing() => {
                w.modes.older(targets.count);
            }
            Msg::Newer if w.modes.browsing() => {
                w.modes.newer(targets.count);
            }
            // the prompt chosen: the sheet closes; the edit itself is the
            // thread's intent (`ThreadIntent::EditPrompt`), the chat's to send
            Msg::Confirm => {
                if w.modes.confirm(Some(thread.id()), targets.count).is_some() {
                    Navigator::maybe_pop(cx);
                }
            }
            Msg::Older | Msg::Newer => {}
        }
        Rebuild::No
    }

    fn build(&self, w: &TranscriptSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let table = strings::of(cx);
        let tokens = crate::app::theme::Tokens::of(cx);
        let (dim, bold, accent) = (tokens.dim(), tokens.bold(), tokens.accent());
        let (follow, modes) = (w.follow.clone(), w.modes.clone());
        let bottom = ListenableBuilder::new(modes.listenable(), move |_, _| {
            let browsing = modes.browsing();
            let hints = second(modes.selected().is_some(), table);
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .child(if browsing {
                    browsing_footer(table, bold.clone(), accent.clone(), dim.clone())
                } else {
                    status(follow.clone(), table, dim.clone())
                })
                .child(pager::hint_row(
                    &pager::navigation(table),
                    table,
                    dim.clone(),
                ))
                .child(pager::hint_row(&hints, table, dim.clone()))
                .into()
        });
        let column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(pager::header(table.sheet_title, table, tokens.dim()))
            .child(Expanded::new(1, TranscriptPane(w.follow.clone(), true)))
            .child(bottom);
        let mut focus = Focus::new(column).autofocus();
        for (key, msg) in pager::bindings(false) {
            focus = focus.bind(key, cx.send(Msg::Pager(msg)));
        }
        let plain = KeyPress::code;
        for (key, msg) in [
            (plain(KeyCode::Escape), Msg::Esc),
            (plain(KeyCode::Left), Msg::Older),
            (KeyPress::char('h'), Msg::Older),
            (plain(KeyCode::Right), Msg::Newer),
            (KeyPress::char('l'), Msg::Newer),
            (plain(KeyCode::Enter), Msg::Confirm),
        ] {
            focus = focus.bind(key, cx.send(msg));
        }
        focus.into()
    }
}

/// The second hint row: close, find, browse, and with a prompt marked the
/// keys that edit it (`render_hints`).
fn second(highlight: bool, table: &'static Strings) -> Vec<pager::Pair> {
    let mut pairs = vec![
        (vec![table.key_close], table.hint_close),
        (vec![table.key_find], table.hint_find),
        (vec![table.key_esc], table.hint_browse),
    ];
    if highlight {
        pairs.push((vec![table.key_right], table.hint_edit_next));
        pairs.push((vec![table.key_enter], table.hint_edit_message));
    }
    pairs
}

/// The status row (`status_line_with_navigation`): the news the reader has
/// not seen, then the select key.
fn status(follow: Follow, table: &'static Strings, style: Style) -> AnyWidget {
    let listenable = follow.listenable();
    ListenableBuilder::new(Rc::new(listenable), move |_, _| {
        let news = if follow.unseen() {
            table.status_news
        } else {
            ""
        };
        Text::line(format!("{news}{}", table.status_select))
            .style(style.clone())
            .overflow(TextOverflow::Clip)
            .into()
    })
    .into()
}

/// The status row while prompts are browsed: the first spelling that fits
/// the row, whole to bare (`prompt_navigation_footer`'s `first_fitting_line`).
fn browsing_footer(table: &'static Strings, key: Style, accent: Style, dim: Style) -> AnyWidget {
    LayoutBuilder::new(move |cx, constraints| {
        let segmenter = segmenter_of(cx);
        let [whole, short] = table.browsing_label;
        let line = |label: &'static str, hints: &[(&'static str, &'static str)]| -> Vec<Span> {
            let mut spans = vec![Span::styled(label, accent.clone())];
            for (keys, action) in hints {
                spans.push(Span::styled(table.hint_joiner, dim.clone()));
                spans.push(Span::styled(*keys, key.clone()));
                if !action.is_empty() {
                    spans.push(Span::styled(table.hint_gap, dim.clone()));
                    spans.push(Span::styled(*action, dim.clone()));
                }
            }
            spans
        };
        let mut full = vec![
            (table.browsing_scroll_keys, table.browsing_scroll),
            (table.browsing_prompt_keys, table.browsing_prompts),
        ];
        let mut compact = vec![(table.browsing_both_keys, "")];
        for hints in [&mut full, &mut compact] {
            hints.push((table.browsing_details_key, table.browsing_details));
            hints.push((table.browsing_rewind_key, table.browsing_rewind));
            hints.push((table.key_esc, table.browsing_back));
        }
        let rewind = (table.browsing_rewind_key, table.browsing_rewind);
        let back = (table.key_esc, table.browsing_back);
        let candidates = [
            line(whole, &full),
            line(short, &full),
            line(short, &compact),
            line(
                short,
                &[(table.browsing_both_keys, ""), rewind, (table.key_esc, "")],
            ),
            line(short, &[rewind, back]),
            line(short, &[back]),
            line(short, &[(table.key_esc, "")]),
            line(short, &[]),
        ];
        let fits = |spans: &Vec<Span>| {
            let cells: usize = spans.iter().map(|span| segmenter.width(&span.text)).sum();
            i32::try_from(cells).unwrap_or(i32::MAX) <= constraints.max_width
        };
        let chosen = candidates
            .iter()
            .find(|spans| fits(spans))
            .cloned()
            .unwrap_or_default();
        Text::rich_line(chosen).overflow(TextOverflow::Clip).into()
    })
    .into()
}
