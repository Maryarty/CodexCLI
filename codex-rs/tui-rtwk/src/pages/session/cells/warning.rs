// cell: Item::Warning { .. } | Item::LocalWarning(_) | Item::StartupWarning(_)
//! A warning of the server in the detailed transcript (oracle
//! history_cell/notices.rs `new_warning_event`: a `PrefixedWrappedHistoryCell`
//! of `⚠ ` over the message, the rest of its rows indented two columns, the
//! `Attention` tone: bold, yellow on a dark background).
//!
//! Flutter: a `Row` of a two-column `SizedBox` for the mark and an `Expanded`
//! `Text`.

use rtwk::gestures::ExcludeSelection;
use rtwk::layout::{CrossAxisAlignment, Expanded, Row, SizedBox};
use rtwk::style::Attrs;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::app::theme::Tokens;

use super::dispatch::{CellEnv, Place, below};
use super::strings::Strings;
use super::text::Part;
use codex_tui_core::threads::Item;

/// The mark and its blank (oracle `"⚠ "` and `"  "`).
const MARK_COLUMNS: i32 = 2;

/// The message as the oracle's wrapping lays it out: a blank line of the message is dropped
/// (`adaptive_wrap` keeps the rows of its lines that have words).
fn displayed(message: &str) -> String {
    message
        .split('\n')
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The texts of the cell: the mark, a gutter, and the message.
pub fn texts(message: &str, strings: &Strings) -> Vec<Part> {
    vec![Part::gutter(strings.warning_mark), Part::text(message)]
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WarningCell {
    pub message: String,
}

impl_widget!(stateless eq WarningCell);

impl StatelessWidget for WarningCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let strings = super::strings::of(cx);
        let attention = tokens.warning().attrs(Attrs::BOLD);
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .child(ExcludeSelection::new(SizedBox::new(
                Some(MARK_COLUMNS),
                None,
                Some(
                    Text::new(strings.warning_mark)
                        .style(attention.clone())
                        .into(),
                ),
            )))
            .child(Expanded::new(
                1,
                Text::new(displayed(&self.message)).style(attention),
            ))
            .into()
    }
}

/// A warning: the server's, the client's own and a startup warning.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let message = match item {
        Item::Warning { message } => message.clone(),
        Item::LocalWarning(warning) => env.chat.local_warning(warning),
        Item::StartupWarning(warning) => env.chat.mcp_warning(warning),
        _ => return AnyWidget::default(),
    };
    below(place.separator, WarningCell { message }.into())
}
