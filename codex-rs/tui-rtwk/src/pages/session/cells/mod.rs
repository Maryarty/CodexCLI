//! `cells`: the view half of the slot of S11 (streaming and history cells), the `mock-chat` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::cells`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::cells::{ID, NAME};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;

pub mod agent;
pub mod citation;
pub mod collab;
pub mod computer_activity;
pub mod decision;
pub mod delegated;
pub mod deprecation;
pub mod disclosure;
pub mod dispatch;
pub mod error;
pub mod exec;
pub mod generated_image;
pub mod hook;
pub mod interaction;
pub mod markdown;
pub mod note;
pub mod notice;
pub mod one_row;
pub mod patch;
pub mod plan;
pub mod plan_update;
pub mod raw;
pub mod reasoning;
pub mod request_input;
pub mod safety;
pub mod separator;
pub mod session_info;
pub mod status;
pub mod strings;
pub mod text;
pub mod tool_call;
pub mod tool_rows;
pub mod update_available;
pub mod user;
pub mod viewed_image;
pub mod warning;
pub mod web;

pub use dispatch::{CellEnv, Place, cell};

/// The list of the kinds of transcript item, one line per `Item` pattern, handed to `callback` as
/// `pattern => file;` entries: `file` is the module `cells/<file>.rs` whose `view(item, env,
/// place)` builds the cell of the items the pattern matches. The match the callback makes is
/// exhaustive, so an `Item` no line names does not compile; a cell is one line here (and its
/// file). What a feature draws for a note of its own is `FeatureView::cell` (reached through
/// `note`), never a line.
macro_rules! cell_list {
    ($callback:ident) => {
        $callback! {
            Item::User(_) => user;
            Item::Agent { .. } => agent;
            Item::Tool(_) => tool_rows;
            Item::Reasoning { .. } => reasoning;
            Item::Patch(_) => patch;
            Item::Plan { .. } => plan;
            Item::Web(_) => web;
            Item::ImageView { .. } => viewed_image;
            Item::Notice(_) => notice;
            Item::Note(_) => note;
            Item::PlanUpdate(_) => plan_update;
            Item::Hook(_) => hook;
            Item::Collab(_) => collab;
            Item::Delegated(_) => delegated;
            Item::Generated(_) => generated_image;
            Item::Error { .. } | Item::HighLoad => error;
            Item::Warning { .. } | Item::LocalWarning(_) | Item::StartupWarning(_) => warning;
            Item::Deprecation { .. } => deprecation;
            Item::Decision(_) | Item::Review(_) => decision;
            Item::Status { .. } => status;
        }
    };
}

use cell_list;

/// What the debug menu lists for this directory: nothing yet. A slice adds
/// one entry per state worth opening (a screen on made-up data).
pub fn debug_entries() -> Vec<DebugEntry> {
    Vec::new()
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
