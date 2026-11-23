//! The bottom pane's activity slots above the composer: the Working row
//! (`../activity.rs`), the pending preview (`pending`) and the background-terminal
//! line (`terminals`), the oracle's `status_indicator_widget.rs`,
//! `bottom_pane/pending_input_preview.rs` and `bottom_pane/unified_exec_footer.rs`.

pub mod entries;
mod pending;
pub mod strings;
mod terminals;

pub(super) use pending::{Pending, PendingInput};
pub(super) use terminals::TerminalsLine;

use rtwk::style::{Attrs, Style};

/// The oracle's `.dim()`: the terminal's dim attribute on the cell's own colour.
pub(super) fn dim() -> Style {
    Style::new().attrs(Attrs::DIM)
}

/// The key in the hints, `.bold()` on the cell's own colour.
pub(super) fn bold() -> Style {
    Style::new().attrs(Attrs::BOLD)
}
