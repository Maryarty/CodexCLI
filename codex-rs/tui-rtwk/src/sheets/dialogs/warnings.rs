//! The warnings viewer: one warning at a time, its full text in a scrolling
//! body, the viewer's own footer. Oracle: `bottom_pane/warnings_view.rs` (keys,
//! decisions) and `warnings_view_render.rs` (layout, footer).

use rtwk::focus::{Focus, KeyPress};
use rtwk::layout::{
    BoxConstraints, BoxWidget, Column, ConstrainedBox, CrossAxisAlignment, EdgeInsets, Flexible,
    MainAxisSize, SizedBox, UNBOUNDED,
};
use rtwk::overlay::Dialog;
use rtwk::scroll::SingleChildScrollView;
use rtwk::style::Theme;
use rtwk::text::segmenter_of;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder};

use codex_tui_core::screens::dialogs::{PageIntent, Warnings};

use super::dialog::{Msg, SmallDialogState, ctrl_o, f2};
use super::strings::Strings;
use crate::app::theme::Tokens;

/// `WARNING_PANEL_CONTENT_ROWS` less the header and its blank row.
const BODY_ROWS: i32 = 8;

pub(super) fn build(warnings: &Warnings, s: &Strings, cx: &mut Cx<'_, SmallDialogState>) -> Focus {
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
    let body =
        AnyWidget::from(SingleChildScrollView::new(Text::new(details.to_string())).autofocus(true))
            .with_key(warnings.current);
    let hints = [s.hint_keep, s.hint_dismiss, s.hint_copy, s.hint_pages];
    let separator = s.hint_separator;
    let dim = tokens.dim();
    let seg = segmenter_of(cx);
    // The footer shows the hints that fit the width, in order; one that does not fit is
    // left out and the next is tried (`warnings_view_render.rs`).
    let hint = LayoutBuilder::new(move |_, constraints| {
        let mut shown: Vec<&str> = Vec::new();
        for hint in hints {
            shown.push(hint);
            if seg.width(&shown.join(separator)) as i32 > constraints.max_width {
                shown.pop();
            }
        }
        Text::line(shown.join(separator)).style(dim.clone()).into()
    });
    // The panel (`render_menu_surface`): a blank row on top, the title, a blank row, the
    // body, a blank row, all inset two cells on the panel colour; the hint row is under it.
    // The body is at most the oracle's body rows, fewer where the room is short (its body
    // is `Constraint::Min(0)` above the footer row).
    let surface = tokens.prompt_surface(cx, false);
    let content = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::line(title).style(tokens.bold()))
        .child(Text::new(String::new()))
        .child(Flexible::new(
            1,
            ConstrainedBox::new(
                BoxConstraints::new(0, UNBOUNDED, 0, BODY_ROWS),
                SizedBox::new(None, Some(BODY_ROWS), Some(body)),
            ),
        ));
    let panel = BoxWidget::new()
        .style(surface)
        .padding(EdgeInsets::only(2, 1, 2, 1))
        .child(content);
    let column = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Flexible::new(1, panel))
        // The footer row is opaque: what the composer's footer drew there is covered.
        .child(
            BoxWidget::new()
                .style(Theme::of(cx).background.clone())
                .padding(EdgeInsets::only(2, 0, 0, 0))
                .child(hint),
        );
    Focus::new(Dialog::bare(column))
        .can_request_focus(false)
        .bind(KeyPress::ESC, cx.send(Msg::Close))
        .bind(f2(), cx.send(Msg::Close))
        .bind('k', cx.send(Msg::Page(PageIntent::KeepAndNext)))
        .bind(KeyPress::LEFT, cx.send(Msg::Page(PageIntent::Previous)))
        .bind(KeyPress::RIGHT, cx.send(Msg::Page(PageIntent::Next)))
        .bind(ctrl_o(), cx.send(Msg::Page(PageIntent::Copy)))
}
