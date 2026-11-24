//! The reasoning summary of the detailed transcript (oracle history_cell/
//! messages.rs `ReasoningSummaryCell`): `• ` and the Markdown body, every span
//! dim and italic over its own style (`span.patch_style(dim().italic())`).
//! Flutter: a `Row` of the bullet `Text` and an `Expanded` `DefaultTextStyle`
//! (dim, italic) over the `MarkdownView`. The oracle renders this body with
//! its generic renderer: a `:codex-file-citation` directive stays literal here.

use rtwk::layout::{CrossAxisAlignment, Expanded, Row, SizedBox};
use rtwk::style::{Attrs, Style};
use rtwk::text::DefaultTextStyle;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use super::dispatch::{CellEnv, Place, below};
use crate::app::theme::Tokens;
use codex_tui_core::threads::Item;

/// The bullet and its blank (oracle `prefix_lines(.., "• ", "  ")`).
const BULLET_COLUMNS: i32 = 2;

/// One reasoning summary: the Markdown body the core projected.
#[derive(Clone, Debug, PartialEq)]
pub struct ReasoningCell {
    pub summary: String,
}

impl_widget!(stateless eq ReasoningCell);

impl StatelessWidget for ReasoningCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let theme = tokens.transcript_markdown(cx);
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
                DefaultTextStyle::new(
                    Style::new().attrs(Attrs::DIM | Attrs::ITALIC),
                    crate::widgets::markdown::themed(
                        crate::widgets::markdown::view(&self.summary),
                        theme,
                    ),
                ),
            ))
            .into()
    }
}

/// The model's reasoning summary. A block with nothing to say (its parts empty or `<!-- -->`) draws nothing, not even the blank row above it (oracle: an empty layout takes no separator).
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Reasoning { .. } = item else {
        return AnyWidget::default();
    };
    let summary = item.reasoning_text(env.show_raw_reasoning);
    if summary.trim().is_empty() {
        return AnyWidget::default();
    }
    below(
        place.separator,
        ReasoningCell {
            summary: summary.to_owned(),
        }
        .into(),
    )
}
