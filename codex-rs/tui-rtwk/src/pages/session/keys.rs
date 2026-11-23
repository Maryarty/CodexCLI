//! The chat's key actions: the catalogue's `open_transcript`, `find_transcript`, `open_warnings`,
//! `toggle_voice`, `toggle_side_conversation` and `toggle_raw_output`, and the hardwired
//! `cycle_collaboration_mode` (`keymap::fixed`), each with the keys of the keymap the user's
//! `tui.keymap` resolved over the oracle's defaults (`RuntimeKeymap::defaults`: `app`, `chat`,
//! `composer` groups). Only the actions of the chat's hub role are here: those that open a screen
//! or start history search. The rest (copy, clear terminal, external editor, interrupt of a
//! running turn, ...) belong to the slices that own what they act on.

use rtwk::focus::KeyPress;

use codex_tui_core::commands::Target;
use codex_tui_core::keymap::actions::{
    CHAT_TOGGLE_VOICE, GLOBAL_FIND_TRANSCRIPT, GLOBAL_OPEN_TRANSCRIPT, GLOBAL_OPEN_WARNINGS,
    GLOBAL_TOGGLE_RAW_OUTPUT, GLOBAL_TOGGLE_SIDE_CONVERSATION,
};
use codex_tui_core::keymap::{ActionId, Keymap, fixed};

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
    /// `toggle_side_conversation`: Ctrl+/.
    ToggleSide,
    /// `toggle_raw_output`: Alt+R (the same as `/raw`).
    ToggleRaw,
    /// `cycle_collaboration_mode`: Shift+Tab, Plan and Default in turn.
    CycleMode,
}

impl Action {
    /// The screen the action opens, if it opens one.
    pub fn target(self) -> Option<Target> {
        match self {
            Action::FindTranscript => Some(Target::Pager),
            Action::OpenWarnings => Some(Target::Dialogs),
            Action::ToggleVoice => Some(Target::Voice),
            Action::OpenTranscript | Action::ToggleSide | Action::ToggleRaw | Action::CycleMode => {
                None
            }
        }
    }

    /// The keymap action whose keys do it.
    pub fn id(self) -> ActionId {
        match self {
            Action::OpenTranscript => GLOBAL_OPEN_TRANSCRIPT,
            Action::FindTranscript => GLOBAL_FIND_TRANSCRIPT,
            Action::OpenWarnings => GLOBAL_OPEN_WARNINGS,
            Action::ToggleVoice => CHAT_TOGGLE_VOICE,
            Action::ToggleSide => GLOBAL_TOGGLE_SIDE_CONVERSATION,
            Action::ToggleRaw => GLOBAL_TOGGLE_RAW_OUTPUT,
            Action::CycleMode => fixed::CYCLE_COLLABORATION_MODE,
        }
    }
}

/// The keys of the actions, in the oracle's order. Without the kitty protocol Ctrl+/ is sent as
/// the C0 byte 0x1f (rtwk reads Ctrl+_) and Ctrl+7 shares it (`keymap.rs:730-735`, `2317`): the
/// side conversation has those two as well.
pub fn presses(keymap: &Keymap) -> Vec<(Action, KeyPress)> {
    use Action::{
        CycleMode, FindTranscript, OpenTranscript, OpenWarnings, ToggleRaw, ToggleSide, ToggleVoice,
    };
    let mut keys = Vec::new();
    for action in [
        OpenTranscript,
        FindTranscript,
        OpenWarnings,
        ToggleVoice,
        ToggleSide,
    ] {
        keys.extend(
            crate::keymap::presses(keymap, action.id())
                .into_iter()
                .map(|key| (action, key)),
        );
    }
    keys.extend(
        crate::keymap::presses(keymap, fixed::SIDE_CONVERSATION_BYTES)
            .into_iter()
            .map(|key| (ToggleSide, key)),
    );
    for action in [ToggleRaw, CycleMode] {
        keys.extend(
            crate::keymap::presses(keymap, action.id())
                .into_iter()
                .map(|key| (action, key)),
        );
    }
    keys
}
