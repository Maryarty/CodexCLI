//! A message another task sent to this thread (oracle history_cell/
//! delegated_message.rs): `• Sent by Codex from task <id>` and the Markdown
//! body indented by two columns. The chat shows the redrawable transcript,
//! whose lists are spaced uniformly (`ListSpacing::Uniform`); the scrollback
//! spacing is not a view of the chat.
//!
//! Flutter: a `Row` of the bullet and an `Expanded` `Column` of the
//! attribution and the `MarkdownView`.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::Attrs;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};
use rtwk_rich::markdown::{ListSpacing, MarkdownTheme, MarkdownView};

use codex_tui_core::threads::DelegatedMessage;

use crate::app::theme::Tokens;

/// The bullet and its blank (oracle `initial_indent("• ")`, indent `"  "`).
const BULLET_COLUMNS: i32 = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelegatedCell {
    pub message: Arc<DelegatedMessage>,
}

impl_widget!(stateless eq DelegatedCell);

impl StatelessWidget for DelegatedCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let theme = MarkdownTheme {
            list_spacing: ListSpacing::Uniform,
            ..tokens.transcript_markdown(cx)
        };
        let attribution = strings
            .delegated_attribution
            .replace("{}", &self.message.source);
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(SizedBox::new(
                Some(BULLET_COLUMNS),
                None,
                Some(
                    Text::new(strings.bullet)
                        .style(tokens.dim().attrs(Attrs::DIM))
                        .into(),
                ),
            ))
            .child(Expanded::new(
                1,
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .child(Text::new(attribution))
                    .child(MarkdownView::new(self.message.markdown.as_str()).theme(theme)),
            ))
            .into()
    }
}
