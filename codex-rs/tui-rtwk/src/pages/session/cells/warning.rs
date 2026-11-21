//! A warning of the server in the detailed transcript (oracle
//! history_cell/notices.rs `new_warning_event`: a `PrefixedWrappedHistoryCell`
//! of `⚠ ` over the message, the rest of its rows indented two columns, the
//! `Attention` tone: bold, yellow on a dark background).
//!
//! Flutter: a `Row` of a two-column `SizedBox` for the mark and an `Expanded`
//! `Text`.

use rtwk::layout::{CrossAxisAlignment, Expanded, Row, SizedBox};
use rtwk::style::Attrs;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use crate::app::theme::Tokens;

/// The mark and its blank (oracle `"⚠ "` and `"  "`).
const MARK_COLUMNS: i32 = 2;

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
            .child(SizedBox::new(
                Some(MARK_COLUMNS),
                None,
                Some(
                    Text::new(strings.warning_mark)
                        .style(attention.clone())
                        .into(),
                ),
            ))
            .child(Expanded::new(
                1,
                Text::new(self.message.clone()).style(attention),
            ))
            .into()
    }
}
