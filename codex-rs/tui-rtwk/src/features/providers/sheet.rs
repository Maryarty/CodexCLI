//! The sheet of the model providers (oracle `app/provider.rs`): in Flutter a
//! `showModalBottomSheet` over a nested `Navigator` whose home is the list and whose routes are
//! the form (the details are a full-screen pager, a dialog over the sheet). The sheet says it
//! wants the list (`Interest::List`) for as long as it is open: the feature reads it when the
//! sheet opens, and a late answer after it closed is dropped.

use rtwk::AnyWidget;
use rtwk::navigation::Navigator;
use rtwk::overlay::DialogOptions;

use codex_tui_core::features::providers::Interest as Wanted;

use super::list::Home;
use crate::widgets::interest::Interest;
use crate::widgets::sheet::bottom;

/// The sheet: a navigator whose home is the list, each page at the bottom edge.
pub fn sheet() -> AnyWidget {
    let pages = Navigator::new().home(bottom(Home)).back_key(None);
    Interest::new(Wanted::List, pages).into()
}

/// The options the sheet is shown with: the bottom pane, built under the providers of the page
/// that asks.
pub fn options() -> DialogOptions {
    DialogOptions {
        use_root_navigator: false,
        ..crate::sheets::sheet_options()
    }
}
