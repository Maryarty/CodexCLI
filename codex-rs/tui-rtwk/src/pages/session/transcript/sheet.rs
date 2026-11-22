//! The transcript sheet (oracle `pager_overlay/transcript.rs`, Ctrl+T): the
//! whole transcript with its reasoning and warnings, full screen over the
//! chat, read with the pager's keys.
//!
//! Flutter's full-screen route of a `Column`: the title row, the same
//! transcript list the chat shows (`TranscriptPane`, detailed), the status
//! row (the news and the select key), two hint rows. The scroll state is the
//! thread's `Follow` controller, shared with the chat so the reader's place
//! survives the sheet; the document is the thread model's, read through the
//! provider.

use std::rc::Rc;

use rtwk::focus::Focus;
use rtwk::foundation::ListenableBuilder;
use rtwk::layout::{Column, CrossAxisAlignment, Expanded};
use rtwk::navigation::Navigator;
use rtwk::style::Style;
use rtwk::text::{Text, TextOverflow};
use rtwk::{AnyWidget, Cx, Rebuild, State, StatefulWidget, impl_widget};

use super::pager::{self, Msg};
use super::strings::{self, Strings};
use crate::pages::session::transcript_pane::TranscriptPane;
use crate::widgets::thread_ui::Follow;

/// The sheet over the transcript of the thread below it. `highlight` is the
/// browse mode's: a prompt is marked, the hints add the keys to edit it.
#[derive(Clone, PartialEq)]
pub struct TranscriptSheet {
    pub follow: Follow,
    pub highlight: bool,
}

/// Widget state: none (R14); the scroll is the `Follow` controller's.
pub struct SheetState;

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
        match msg {
            Msg::Rows(rows) => w.follow.scroll_by(rows),
            Msg::Pages(pages) => w.follow.page(pages),
            Msg::HalfPages(pages) => w.follow.half_page(pages),
            Msg::Top => w.follow.jump_to_first(),
            Msg::Bottom => w.follow.jump_to_latest(),
            // closing is the sheet's; what the opener does with it (the chat
            // clears its `detailed` flag) is the opener's
            Msg::Close | Msg::Action => {
                Navigator::maybe_pop(cx);
            }
        }
        Rebuild::No
    }

    fn build(&self, w: &TranscriptSheet, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let table = strings::of(cx);
        let dim = crate::app::theme::Tokens::of(cx).dim();
        let column = Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(pager::header(table.sheet_title, table, dim.clone()))
            .child(Expanded::new(1, TranscriptPane(w.follow.clone(), true)))
            .child(status(w.follow.clone(), table, dim.clone()))
            .child(pager::hint_row(&navigation(table), table, dim.clone()))
            .child(pager::hint_row(&second(w.highlight, table), table, dim));
        let mut focus = Focus::new(column).autofocus();
        for (key, msg) in pager::bindings(false) {
            focus = focus.bind(key, cx.send(msg));
        }
        focus.into()
    }
}

fn navigation(table: &'static Strings) -> Vec<pager::Pair> {
    pager::navigation(table)
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
