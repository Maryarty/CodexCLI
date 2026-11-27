//! `chat`: the view half of the slot of S10 (main chat screen), the `mock-chat` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::chat`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

mod actions;
mod activity;
pub mod backtrack;
mod banners;
pub(crate) mod blossom;
pub mod cells;
pub mod composer;
mod external_writer;
pub(crate) mod footer;
mod hook_reveal;
mod key_actions;
mod keys;
mod live;
pub mod open;
pub mod popovers;
mod popup;
mod session_page;
mod slice;
pub mod status;
mod status_surfaces;
pub mod transcript;
mod transcript_activity;
mod transcript_header;
mod transcript_pane;
mod window_title;

use rtwk::navigation::PageRoute;
use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::chat::{ID, NAME};
use codex_tui_core::{ids::ThreadId, threads::ThreadModel};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;

mod side_pane;
pub mod strings;

/// The draft before the first session has no server thread yet.
fn thread_address(thread: &ThreadModel) -> Option<ThreadId> {
    let id = thread.id();
    (!id.as_str().is_empty()).then(|| id.clone())
}

/// Only the selected chat presents terminal-wide controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Presentation {
    #[default]
    Solo,
    Focused,
    Passive,
}

impl Presentation {
    fn focused(self) -> bool {
        self != Self::Passive
    }
}

/// What the debug menu lists for this directory: a screen per state worth
/// opening, on the model a fixture script makes.
pub fn debug_entries() -> Vec<DebugEntry> {
    let mut entries = vec![DebugEntry {
        name: "streamed_answer",
        description: "a chat transcript with a streamed answer",
        script: include_str!("fixtures/streamed_answer.jsonl"),
        open: EntryOpen::Route(|seeded: &Seeded| {
            session_page::Chat {
                ui: seeded.thread_ui.get(seeded.core.session().active.as_ref()),
                presentation: Presentation::Solo,
            }
            .into()
        }),
    }];
    entries.extend(footer::entries::debug_entries());
    entries.extend(status::entries::debug_entries());
    entries
}

/// The chat of the active thread inside a seeded debug entry (the popovers
/// entry runs the real composer on a model a fixture script made).
pub fn seeded(seeded: &Seeded<'_>) -> AnyWidget {
    session_page::Chat {
        ui: seeded.thread_ui.get(seeded.core.session().active.as_ref()),
        presentation: Presentation::Solo,
    }
    .into()
}

/// The `Navigator`'s home: the chat of the active thread.
pub fn home(app: &crate::app::UiApp) -> AnyWidget {
    if let Some(pair) = side_pane::Dock::of(app) {
        return pair.keyed();
    }
    session_page::Chat {
        ui: app.ui.thread_ui.get(app.core.session().active.as_ref()),
        presentation: Presentation::Solo,
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
