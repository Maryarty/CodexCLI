//! The `/debug-config` cell (oracle `debug_config.rs` `render_debug_config_lines`, drawn as a
//! `PlainHistoryCell`): the typed command in the accent, the section titles bold, the notes
//! dim, one row per line the host gave, with no bullet in front. In Flutter it is a `Column` of
//! `Text.rich` in the history list; the words are the host's, the styles the feature's
//! (`Style`).

use rtwk::layout::{Column, CrossAxisAlignment, MainAxisSize};
use rtwk::style::{Span, Style};
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::features::small_cmds::{Note, Row, Style as RowStyle};

use crate::app::theme::Tokens;

/// The cell of a `/debug-config` note; every other note of the feature is a line of words.
pub fn of(note: &Note) -> Option<AnyWidget> {
    let Note::DebugConfig(rows) = note else {
        return None;
    };
    Some(DebugConfigCell { rows: rows.clone() }.into())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugConfigCell {
    rows: Vec<Row>,
}

impl_widget!(stateless eq DebugConfigCell);

impl StatelessWidget for DebugConfigCell {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = Tokens::of(cx);
        let mut column = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch);
        for row in &self.rows {
            let style = match row.style {
                RowStyle::Plain => Style::new(),
                RowStyle::Bold => tokens.bold(),
                RowStyle::Dim => tokens.dim(),
                // the oracle's magenta, the theme's accent (`status_line.rs`)
                RowStyle::Magenta => tokens.accent(),
            };
            column = column.child(Text::rich(vec![Span::styled(row.text.clone(), style)]));
        }
        column.into()
    }
}
