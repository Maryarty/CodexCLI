// cell: Item::Delegated(_)
//! A message another task sent to this thread (oracle history_cell/
//! delegated_message.rs): `• Sent by Codex from task <id>` and the Markdown
//! body indented by two columns. The chat shows the redrawable transcript,
//! whose lists are spaced the way rtwk's `MarkdownView` spaces them (the
//! oracle's uniform spacing is gap G73, item 2); the scrollback spacing is not
//! a view of the chat.
//!
//! Flutter: a `Row` of the bullet and an `Expanded` `Column` of the
//! attribution and the `MarkdownView`.

use std::sync::Arc;

use rtwk::layout::{Column, CrossAxisAlignment, Expanded, MainAxisSize, Row, SizedBox};
use rtwk::style::Attrs;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::threads::DelegatedMessage;

use super::dispatch::{CellEnv, Place, below};
use crate::app::theme::Tokens;
use codex_tui_core::threads::Item;

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
        let theme = tokens.transcript_markdown(cx);
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
                    .child(crate::widgets::markdown::themed(
                        crate::widgets::markdown::view(&self.message.markdown),
                        theme,
                    )),
            ))
            .into()
    }
}

/// A message another task delegated to this thread.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, _env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Delegated(message) = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        DelegatedCell {
            message: message.clone(),
        }
        .into(),
    )
}
