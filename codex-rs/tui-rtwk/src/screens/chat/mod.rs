//! `chat`: the view half of the slot of S10 (main chat screen), the `mock-chat` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_rtwk_core::screens::chat`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

mod home;

use rtwk::navigation::PageRoute;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::screens::chat::{ID, NAME};

use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::ui::seeded::Seeded;

pub mod strings;

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![DebugEntry {
        name: "streamed_answer",
        description: "a chat transcript with a streamed answer",
        script: include_str!("fixtures/streamed_answer.jsonl"),
        open: EntryOpen::Route(|seeded: &Seeded| {
            home::Chat {
                ui: seeded.thread_ui.get(seeded.core.session.active.as_ref()),
            }
            .into()
        }),
    }]
}

/// The `Navigator`'s home: the chat of the active thread.
pub fn home(app: &crate::app::UiApp) -> AnyWidget {
    home::Chat {
        ui: app.ui.thread_ui.get(app.core.session.active.as_ref()),
    }
    .into()
}

/// The route of this screen. The chat screen is the `Navigator`'s home; the
/// route exists for the debug menu and for tests.
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Screen.into())
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
