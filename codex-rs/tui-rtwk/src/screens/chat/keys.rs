//! The chat's key actions and their oracle default chords
//! (`keymap.rs` `RuntimeKeymap::defaults`: `app`, `chat`, `composer`
//! groups). Only the actions of the chat's hub role are here: those that
//! open a screen or start history search. The rest (copy, clear terminal,
//! external editor, interrupt of a running turn, ...) belong to the slices
//! that own what they act on. The user's rebinding (`/keymap`) will replace
//! `defaults` with the configured keymap; the actions stay.

use rtwk::focus::KeyPress;
use rtwk::prelude::{KeyCode, Mods};

use codex_tui_rtwk_core::commands::Target;

/// What a key of the chat does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// `open_transcript`: Ctrl+T.
    OpenTranscript,
    /// `find_transcript`: F3.
    FindTranscript,
    /// `open_warnings`: F2.
    OpenWarnings,
    /// `toggle_voice`: F8.
    ToggleVoice,
    /// `history_search_previous`: Ctrl+R.
    SearchPrevious,
    /// `history_search_next`: Ctrl+S.
    SearchNext,
    /// `toggle_side_conversation`: Ctrl+/.
    ToggleSide,
}

impl Action {
    /// The screen the action opens, if it opens one.
    pub fn target(self) -> Option<Target> {
        match self {
            Action::OpenTranscript | Action::FindTranscript => Some(Target::Pager),
            Action::OpenWarnings => Some(Target::Dialogs),
            Action::ToggleVoice => Some(Target::Voice),
            Action::SearchPrevious | Action::SearchNext | Action::ToggleSide => None,
        }
    }
}

/// The default chords, in the oracle's order (`ToggleSide` has its aliases).
pub fn defaults() -> Vec<(Action, KeyPress)> {
    let ctrl = |c: char| KeyPress::char(c).with_mods(Mods::CTRL);
    vec![
        (Action::OpenTranscript, ctrl('t')),
        (Action::FindTranscript, KeyPress::code(KeyCode::F3)),
        (Action::OpenWarnings, KeyPress::code(KeyCode::F2)),
        (Action::ToggleVoice, KeyPress::code(KeyCode::F8)),
        (Action::SearchPrevious, ctrl('r')),
        (Action::SearchNext, ctrl('s')),
        (Action::ToggleSide, ctrl('/')),
        // `keymap.rs:730-735`, `2317`: without the kitty protocol Ctrl+/ is
        // sent as the C0 byte 0x1f (rtwk reads Ctrl+_), and Ctrl+7 shares it.
        (Action::ToggleSide, ctrl('_')),
        (Action::ToggleSide, ctrl('7')),
    ]
}
