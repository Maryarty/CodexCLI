//! `popovers`: the view half of the slot of S12 (composer popovers), the `mock-chat` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_rtwk_core::screens::popovers`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_rtwk_core::screens::popovers::{ID, NAME};

use crate::screens::DebugEntry;
use crate::screens::DebugOpen;
use crate::screens::EntryOpen;
use crate::ui::seeded::Seeded;

pub mod strings;

/// What the debug menu lists for this directory: the real chat composer on
/// the data a script seeds (files, skills, plugins, history, tiers).
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![DebugEntry {
        name: "composer_popups",
        description: "the chat composer and its popups",
        script: include_str!("fixtures/composer_popups.jsonl"),
        open: EntryOpen::Route(|seeded: &Seeded| crate::screens::chat::seeded(seeded)),
    }]
}

/// The part as a widget, placed by the chat screen's subtree.
pub fn part() -> AnyWidget {
    Screen.into()
}

/// Opens the empty form of this slot (debug menu), as a page.
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
