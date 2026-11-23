//! The read-only notice in place of the editor when another process holds the writer.

use rtwk::gestures::GestureDetector;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Row,
};
use rtwk::style::Span;
use rtwk::widgets::Text;
use codex_tui_core::keymap::actions::GLOBAL_OPEN_TRANSCRIPT;
use rtwk::{AnyWidget, Cx, LayoutBuilder};

use crate::app::theme::Tokens;

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
            .padding(EdgeInsets::only(2, 1, 2, 1))
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

/// The hints under the card, wrapped at the word like the oracle's
/// `footer_lines`; the transcript entry names the key the keymap binds and is
/// left out while none is.
pub(super) fn footer(
    cx: &mut Cx<'_, ChatState>,
    strings: &Strings,
    tokens: &Tokens,
) -> AnyWidget {
    let keymap = crate::keymap::keymap(cx);
    let transcript = crate::keymap::label(&keymap, GLOBAL_OPEN_TRANSCRIPT);
    let mut entries = vec![
        (strings.writer_retry_key.to_owned(), strings.writer_retry),
        ("f".to_owned(), strings.writer_fork),
        ("←/esc".to_owned(), strings.writer_center),
        ("⌃c/q".to_owned(), strings.writer_exit),
    ];
    if let Some(key) = transcript {
        entries.push((key, strings.writer_transcript));
    }
    let key_style = tokens.bold();
    let dim = tokens.dim();
    let mut spans = vec![Span::styled(strings.writer_pad, dim.clone())];
    for (index, (key, label)) in entries.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(strings.writer_gap, dim.clone()));
        }
        spans.push(Span::styled(key, key_style.clone()));
        spans.push(Span::styled(strings.writer_pad, dim.clone()));
        spans.push(Span::styled(label, dim.clone()));
    }
    rtwk::layout::Padding::new(EdgeInsets::only(2, 0, 0, 0), Text::rich(spans)).into()
}
