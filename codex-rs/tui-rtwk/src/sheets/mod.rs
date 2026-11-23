//! Modal sheet content (`show_dialog` / bottom-sheet bodies) and the
//! presenter that opens the request dialog.

pub mod approvals;
pub mod dialogs;
pub mod pets;
pub mod presenter;
pub mod select;
pub mod voice;

use rtwk::layout::Alignment;
use rtwk::overlay::DialogOptions;

/// The options every modal sheet is shown with (`show_dialog`): placed at
/// the bottom edge where the oracle shows its bottom pane, no dimming, shown
/// at once; Esc and the sheet's own keys close it, not the barrier. A sheet
/// the oracle shows elsewhere keeps `DialogOptions::default()` (centred).
pub fn sheet_options() -> DialogOptions {
    DialogOptions {
        alignment: Alignment::BOTTOM_CENTER,
        barrier_alpha: Some(0),
        barrier_dismissible: false,
        fade_in: std::time::Duration::ZERO,
        ..DialogOptions::default()
    }
}
