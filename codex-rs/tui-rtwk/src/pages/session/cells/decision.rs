// cell: Item::Decision(_) | Item::Review(_)
//! What an answer to an approval request or the auto-reviewer's verdict left (oracle
//! `new_approval_decision_cell`, the plain permissions line), and a line `/auto-review` or the
//! reviewer's paths added. The cells are the approvals sheet's (`sheets::approvals`); this file
//! is the transcript's entry for them.

use rtwk::AnyWidget;

use codex_tui_core::threads::Item;

use super::dispatch::{CellEnv, Place, below};
use crate::sheets::approvals::{DecisionCell, ReviewLineCell};

/// A decision or a review line.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, _env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    match item {
        Item::Decision(decided) => below(
            place.separator,
            DecisionCell {
                decided: decided.clone(),
            }
            .into(),
        ),
        Item::Review(line) => below(
            place.separator,
            ReviewLineCell { line: line.clone() }.into(),
        ),
        _ => AnyWidget::default(),
    }
}
