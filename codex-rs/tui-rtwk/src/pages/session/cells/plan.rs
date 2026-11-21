//! Markdown cells beside the agent's answer: the proposed plan (oracle
//! history_cell/plans.rs `ProposedPlanCell`) and the reasoning summary of the
//! detailed transcript (messages.rs `new_reasoning_summary_block`). Flutter: a
//! `Column` of a header `Text.rich` and a `Container(color)` around the
//! `MarkdownView`; the dim italic summary is a `DefaultTextStyle` over it.

use rtwk::AnyWidget;
use rtwk::layout::{
    BoxWidget, Column, CrossAxisAlignment, EdgeInsets, Expanded, MainAxisSize, Row,
};
use rtwk::style::{Attrs, Span, Style};
use rtwk::text::DefaultTextStyle;
use rtwk::widgets::Text;
use rtwk_rich::markdown::{MarkdownTheme, MarkdownView};

use crate::app::theme::Tokens;

use super::strings::Strings;

/// `• Proposed Plan`, a blank row, then the plan on the prompt surface with a
/// blank row above and below and two columns on each side.
pub fn proposed_plan(
    text: &str,
    surface: Style,
    markdown: &MarkdownTheme,
    tokens: &Tokens,
    strings: &Strings,
) -> AnyWidget {
    let body: AnyWidget = if text.trim().is_empty() {
        Text::new(strings.empty_plan)
            .style(tokens.dim().attrs(Attrs::DIM | Attrs::ITALIC))
            .into()
    } else {
        MarkdownView::new(super::markdown::unwrap_markdown_fences(text).as_ref())
            .theme(markdown.clone())
            .into()
    };
    Column::new()
        .main_axis_size(MainAxisSize::Min)
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .child(Text::rich(vec![
            Span::styled(strings.bullet, tokens.dim()),
            Span::styled(strings.proposed_plan, tokens.bold()),
        ]))
        .child(Text::new(strings.gap))
        .child(
            BoxWidget::new()
                .style(surface)
                .padding(EdgeInsets::only(2, 1, 2, 1))
                .child(body),
        )
        .into()
}

/// The reasoning summary as the detailed transcript shows it: `• ` and the
/// Markdown, dim and italic.
pub fn reasoning(
    summary: &str,
    markdown: &MarkdownTheme,
    tokens: &Tokens,
    strings: &Strings,
) -> AnyWidget {
    Row::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .child(Text::new(strings.bullet).style(tokens.dim()))
        .child(Expanded::new(
            1,
            DefaultTextStyle::new(
                Style::new().attrs(Attrs::DIM | Attrs::ITALIC),
                MarkdownView::new(summary).theme(markdown.clone()),
            ),
        ))
        .into()
}
