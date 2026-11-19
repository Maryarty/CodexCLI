//! The warnings viewer: one warning at a time, its full text in a scrolling
//! body, the viewer's own footer. Oracle: `bottom_pane/warnings_view.rs` (keys,
//! decisions) and `warnings_view_render.rs` (layout, footer).

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize, SizedBox};
use rtwk::overlay::Dialog;
use rtwk::scroll::SingleChildScrollView;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::screens::dialogs::{PageIntent, Warnings};

use super::dialog::{Msg, SmallDialogState, ctrl_o, f2};
use super::strings::Strings;
use crate::app::theme::Tokens;

/// `WARNING_PANEL_CONTENT_ROWS` less the header and its blank row.
const BODY_ROWS: i32 = 8;

pub(super) fn build(
    warnings: &Warnings,
    s: &Strings,
    cx: &mut Cx<'_, SmallDialogState>,
) -> Focus {
    let tokens = Tokens::of(cx);
    let page = warnings.page();
    let title = match page {
        Some(entry) => s
            .warnings_title(warnings.current + 1, warnings.entries.len(), &entry.source)
            .to_string(),
        None => s.warnings_title_empty.to_string(),
    };
    let details = page.map_or(s.no_warnings, |entry| entry.details.as_str());
    // Keyed by the page: each page's body scrolls from its top.
    let body = AnyWidget::from(
        SingleChildScrollView::new(Text::new(details.to_string())).autofocus(true),
    )
    .with_key(warnings.current);
    let hint = [s.hint_keep, s.hint_dismiss, s.hint_copy, s.hint_pages].join(s.hint_separator);
    let column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::new(String::new()))
        .child(SizedBox::new(None, Some(BODY_ROWS), Some(body)))
        .child(Text::new(String::new()))
        .child(Text::line(hint).style(tokens.dim()));
    Focus::new(Dialog::new(column).title(title))
        .can_request_focus(false)
        .bind(KeyPress::ESC, cx.send(Msg::Close))
        .bind(f2(), cx.send(Msg::Close))
        .bind('k', cx.send(Msg::Page(PageIntent::KeepAndNext)))
        .bind(KeyPress::LEFT, cx.send(Msg::Page(PageIntent::Previous)))
        .bind(KeyPress::RIGHT, cx.send(Msg::Page(PageIntent::Next)))
        .bind(ctrl_o(), cx.send(Msg::Page(PageIntent::Copy)))
}
