//! `transcript`: the view half of the slot of S10 (transcript view), the `mock-chat` task's to fill. It
//! holds the empty form the skeleton registers: a screen widget, its opener
//! and the debug-menu entries. Its model part, events, intents and RPCs are
//! core's `codex_tui_core::screens::transcript`. Nothing outside these two
//! directories (bar their test files) needs an edit when it grows.

use rtwk::widgets::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::screens::transcript::{ID, NAME};

use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;

pub mod copy;
mod copy_feedback;
pub mod follow;
pub mod history;
pub mod pager;
pub mod selection;
pub mod sheet;
pub mod strings;
pub mod turn_details;

/// A mode of the transcript: the marker type `Mode` of a file of this directory, one line in
/// `mode_list!`. The state a mode keeps is its own (its page or sheet, `thread_ui::Input` says
/// which one the reader holds); the list is what the directory assembles from the files.
pub trait TranscriptMode {
    /// The mode's name: `pager`, `sheet`, `select`, `copy`.
    const NAME: &'static str;

    /// The entries of the debug menu the mode has (its screens on made-up data).
    fn debug_entries() -> Vec<DebugEntry> {
        Vec::new()
    }
}

/// The list of the transcript's modes, handed to `callback` as `file;` entries: `file` is the
/// module `transcript/<file>.rs` with a `Mode` marker that implements [`TranscriptMode`]. A mode
/// is one line here (and its file).
macro_rules! mode_list {
    ($callback:ident) => {
        $callback! {
            pager;
            sheet;
            selection;
            copy;
        }
    };
}

macro_rules! mode_dirs {
    ($($file:ident;)*) => {
        /// What the debug menu lists for this directory: the entries of the modes, in list order
        /// (the static pager over the document a fixture script gives the model, the sheet over a
        /// transcript).
        pub fn debug_entries() -> Vec<DebugEntry> {
            let mut entries = Vec::new();
            $(entries.extend(<$file::Mode as TranscriptMode>::debug_entries());)*
            entries
        }

        /// The names of the modes, in list order.
        pub fn modes() -> Vec<&'static str> {
            vec![$(<$file::Mode as TranscriptMode>::NAME,)*]
        }
    };
}

mode_list!(mode_dirs);

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
