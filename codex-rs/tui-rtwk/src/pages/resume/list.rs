//! The picker's list: one line per saved thread of the model's `rows`. The
//! skeleton's minimal view (the resume slice replaces it with the oracle's
//! picker); it shows that a screen reads a seeded model through the same
//! provider as the live one.

use rtwk::layout::Column;
use rtwk::provider::select;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, StatelessWidget, impl_widget};

use codex_tui_core::screens::ScreenStates;
use codex_tui_core::screens::resume::Row;

use crate::app::theme;
use crate::pages::resume::strings;

#[derive(Clone, Debug, PartialEq)]
pub struct List;

impl_widget!(stateless eq List);

impl StatelessWidget for List {
    fn build(&self, cx: &mut Cx<'_>) -> AnyWidget {
        let tokens = theme::Tokens::of(cx);
        let title = strings::of(cx).title;
        let saved = select::<ScreenStates, Vec<Row>, _>(cx, |states| states.resume.rows.clone());
        let mut column = Column::new().child(Text::new(title).style(tokens.accent()));
        for row in &saved {
            column = column.child(Text::new(format!(
                "{}  {}  {}",
                row.started, row.preview, row.model_provider
            )));
        }
        column.into()
    }
}
