//! The reasoning summary of the detailed transcript (oracle history_cell/
//! messages.rs `ReasoningSummaryCell`): `• ` and the Markdown body, every span
//! dim and italic over its own style (`span.patch_style(dim().italic())`).
//! Flutter: a `Row` of the bullet `Text` and an `Expanded` `MarkdownView`; the
//! dimming is the `MarkdownTheme`'s (a `DefaultTextStyle` above the view does
//! not reach its paragraphs: gap G71).

use rtwk::layout::{CrossAxisAlignment, Expanded, Row, SizedBox};
use rtwk::style::{Attrs, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};
use rtwk_rich::markdown::{MarkdownTheme, MarkdownView};

use crate::app::theme::Tokens;

/// The bullet and its blank (oracle `prefix_lines(.., "• ", "  ")`).
const BULLET_COLUMNS: i32 = 2;

/// One reasoning summary: the Markdown body the core projected.
#[derive(Clone, Debug, PartialEq)]
pub struct ReasoningCell {
    pub summary: String,
}

impl_widget!(stateless eq ReasoningCell);

/// Every styled part of the document dim and italic over its own style.
fn dim_italic(theme: MarkdownTheme) -> MarkdownTheme {
    let over = |style: &Style| style.merge(&Style::new().attrs(Attrs::DIM | Attrs::ITALIC));
    MarkdownTheme {
        heading1: over(&theme.heading1),
        heading2: over(&theme.heading2),
        heading3: over(&theme.heading3),
        heading4: over(&theme.heading4),
        heading5: over(&theme.heading5),
        heading6: over(&theme.heading6),
        emphasis: over(&theme.emphasis),
        strong: over(&theme.strong),
        strikethrough: over(&theme.strikethrough),
        code: over(&theme.code),
        link: over(&theme.link),
        image: over(&theme.image),
        quote: over(&theme.quote),
        marker: over(&theme.marker),
        ..theme
    }
}

impl StatelessWidget for ReasoningCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let theme = dim_italic(tokens.transcript_markdown(cx));
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            // Two columns: a text's trailing blank takes no room of its own.
            .child(SizedBox::new(
                Some(BULLET_COLUMNS),
                None,
                Some(Text::new(strings.bullet).style(tokens.dim()).into()),
            ))
            .child(Expanded::new(
                1,
                MarkdownView::new(self.summary.as_str()).theme(theme),
            ))
            .into()
    }
}
