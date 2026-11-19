//! `import`: the view half of the slot of S06 (import from other agents), the `mock-pre` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::import`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

use rtwk::navigation::PageRoute;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::import::{ID, NAME};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;

pub mod strings;

/// What the debug menu lists for this directory: nothing yet. A slice adds
/// one entry per state worth opening (a screen on made-up data).
pub fn debug_entries() -> Vec<DebugEntry> {
    Vec::new()
}

/// The route of the import step: a step of the startup page, not a page of
/// its own.
pub fn route() -> PageRoute<()> {
    crate::pages::startup::onboarding::route(crate::pages::startup::onboarding::Step::Import)
}

/// Opens the empty form of this slot (debug menu).
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Screen.into())
}

/// The empty screen widget of this slot.
#[derive(Clone, Debug, PartialEq)]
pub struct Screen;

/// Widget state: ids, drafts and controllers only (R14).
#[derive(Debug)]
pub struct ScreenState;

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn build(&self, _: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        let shared = crate::l10n::strings::of(cx);
        Text::new(shared.empty_slot(NAME, ID).to_string()).into()
    }
}
