// cell: Item::Deprecation { .. }
//! A deprecation notice in the detailed transcript (oracle history_cell/
//! notices.rs `DeprecationNoticeCell`: no display lines, so the chat shows
//! nothing; `transcript_lines` is a red `⚠ ` (bold) and the summary, then the
//! details dim, wrapped four columns short of the width and flush left).
//!
//! Flutter: a `Column` of a `Text.rich` and a `Padding` (right 4) around the
//! details `Text`.

use rtwk::layout::{Column, CrossAxisAlignment, EdgeInsets, MainAxisSize, Padding};
use rtwk::style::{Attrs, Span};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use codex_tui_core::threads::Item;

/// The texts of the cell: the summary row (the mark is part of it), then the details.
pub fn texts(summary: &str, details: Option<&str>, strings: &Strings) -> Vec<String> {
    let mut texts = vec![[strings.warning_mark, summary].concat()];
    texts.extend(details.map(str::to_owned));
    texts
}

/// Columns the oracle keeps free right of the details (`width - 4`).
const DETAILS_MARGIN: i32 = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeprecationCell {
    pub summary: String,
    pub details: Option<String>,
}

impl_widget!(stateless eq DeprecationCell);

impl StatelessWidget for DeprecationCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .child(Text::rich(vec![
                Span::styled(strings.warning_mark, tokens.error().attrs(Attrs::BOLD)),
                Span::styled(self.summary.clone(), tokens.error()),
            ]));
        if let Some(details) = &self.details {
            column = column.child(Padding::new(
                EdgeInsets::only(0, 0, DETAILS_MARGIN, 0),
                Text::new(details.clone()).style(tokens.dim().attrs(Attrs::DIM)),
            ));
        }
        column.into()
    }
}

/// A deprecation notice (the detailed transcript's).
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, _env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Deprecation { summary, details } = item else {
        return AnyWidget::default();
    };
    below(
        place.separator,
        DeprecationCell {
            summary: summary.clone(),
            details: details.clone(),
        }
        .into(),
    )
}
