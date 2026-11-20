//! The prompt surface is shared by the body and attachment labels.

use codex_tui_core::threads::UserPrompt;
use rtwk::AnyWidget;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Padding, Row,
};
use rtwk::style::{Span, Style};
use rtwk::widgets::Text;
use rtwk_rich::markdown::{MarkdownTheme, MarkdownView, MdBlock, MdInline, parse_markdown};

use crate::app::theme::Tokens;

use super::strings::Strings;

pub(super) fn panel(
    prompt: &UserPrompt,
    surface: Style,
    tokens: &Tokens,
    strings: &Strings,
    joins_previous: bool,
    markdown: &MarkdownTheme,
) -> BoxWidget {
    let text = prompt.text.trim_end();
    let mut content = Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch);
    if !prompt.images.is_empty() {
        let labels = prompt.images.iter().map(|number| {
            Text::new(strings.transcript_label(strings.image_label, &number.to_string()))
                .style(tokens.info())
        });
        content = content.child(Padding::new(
            EdgeInsets::only(2, 0, 0, i32::from(!text.is_empty())),
            Column::new()
                .main_axis_size(MainAxisSize::Min)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .children(labels.map(Into::into)),
        ));
    }
    if !text.is_empty() {
        let body: AnyWidget = if prompt.elements.is_empty() && has_markup(text) {
            MarkdownView::new(text).theme(markdown.clone()).into()
        } else if prompt.elements.is_empty() {
            Text::new(text.to_owned()).into()
        } else {
            let mut spans = Vec::with_capacity(prompt.elements.len() * 2 + 1);
            let mut cursor = 0;
            for range in &prompt.elements {
                let Some(element) = text.get(range.clone()) else {
                    break;
                };
                spans.push(Span::new(text[cursor..range.start].to_owned()));
                spans.push(Span::styled(element.to_owned(), tokens.info()));
                cursor = range.end;
            }
            spans.push(Span::new(text[cursor..].to_owned()));
            Text::rich(spans).into()
        };
        content = content.child(
            Row::new()
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .child(super::actions::prompt(
                    strings.prompt,
                    tokens.history_prompt_mark(),
                ))
                .child(Expanded::new(1, body)),
        );
    }
    BoxWidget::new()
        .style(surface)
        .padding(EdgeInsets::only(0, i32::from(!joins_previous), 1, 1))
        .child(content)
}

fn has_markup(text: &str) -> bool {
    parse_markdown(text)
        .iter()
        .any(|block| match block.as_ref() {
            MdBlock::Paragraph(inlines) => inlines.iter().any(|inline| {
                !matches!(
                    inline,
                    MdInline::Text(_) | MdInline::SoftBreak | MdInline::HardBreak
                )
            }),
            _ => true,
        })
}
