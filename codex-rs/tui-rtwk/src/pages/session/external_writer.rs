//! The read-only notice in place of the editor when another process holds the writer.

use rtwk::gestures::GestureDetector;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Row,
};
use rtwk::style::Span;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, LayoutBuilder};

use crate::app::theme::Tokens;
use crate::widgets::page_keys;

use super::session_page::{ChatMsg, ChatState};
use super::strings::Strings;

pub(super) fn notice(
    cx: &mut Cx<'_, ChatState>,
    strings: &'static Strings,
    tokens: &Tokens,
) -> AnyWidget {
    let panel_style = tokens.prompt_surface(cx, false);
    let bold = tokens.bold();
    let dim = tokens.dim();
    let key_style = tokens.info();
    let retry = cx.map(|_| ChatMsg::RetryWriter);
    LayoutBuilder::new(move |_, room| {
        let retry = GestureDetector::new()
            .child(Text::rich(vec![
                Span::styled(strings.writer_retry_key, key_style.clone()),
                Span::styled(strings.writer_retry_detail, dim.clone()),
            ]))
            .on_tap(retry.clone());
        let title = Text::new(strings.read_only_title).style(bold.clone());
        let header: AnyWidget = if room.max_width >= 60 {
            Row::new()
                .spacing(1)
                .child(Expanded::new(1, title))
                .child(retry)
                .into()
        } else {
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(title)
                .child(retry)
                .into()
        };
        let body = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(header)
            .child(Text::new(strings.read_only_hint).style(dim.clone()));
        BoxWidget::new()
            .style(panel_style.clone())
            .padding(EdgeInsets::all(1))
            .child(
                Row::new()
                    .spacing(2)
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .child(Text::line("🔒"))
                    .child(Expanded::new(1, body)),
            )
            .into()
    })
    .into()
}

pub(super) fn footer(strings: &Strings, tokens: &Tokens) -> AnyWidget {
    rtwk::layout::Padding::new(
        EdgeInsets::only(2, 0, 0, 0),
        page_keys::footer_with(
            [
                (strings.writer_retry_key, strings.writer_retry),
                ("f", strings.writer_fork),
                ("←/esc", strings.writer_center),
                ("⌃c/q", strings.writer_exit),
                ("⌃t", strings.writer_transcript),
            ],
            tokens.bold(),
            tokens.dim(),
            "   ",
        ),
    )
    .into()
}
