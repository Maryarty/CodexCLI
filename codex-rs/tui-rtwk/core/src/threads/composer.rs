//! What the composer of a thread may do (oracle `bottom_pane` `set_composer_input_enabled`,
//! `blocks_direct_input`, `ChatWidget::is_shutting_down`): one read-only fact the page draws
//! its placeholder and its hint from, instead of reading the writer, the provider error and
//! the parent flag itself.

use super::{ThreadModel, Writer};

/// The state of a thread's composer, strongest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComposerState {
    /// The app is closing: nothing is taken (`Facts::shutting_down`).
    ShuttingDown,
    /// Another process writes the thread; this one only shows it.
    ReadOnly,
    /// The thread's parent controls it: only the commands `parent_owned_takes` allows go.
    ParentOwned,
    /// This process holds no writer, or the server refused the thread for its model provider.
    Disabled,
    /// Text, commands and shell lines are taken.
    Open,
}

impl ComposerState {
    /// Whether a typed line is taken at all (a parent-owned thread still takes commands).
    pub fn takes_text(self) -> bool {
        matches!(self, Self::Open)
    }

    /// Whether a slash command can go (`parent_owned_takes` narrows the parent-owned case).
    pub fn takes_commands(self) -> bool {
        matches!(self, Self::Open | Self::ParentOwned)
    }
}

/// The composer state of `model`; `shutting_down` is the app fact (`Facts::shutting_down`).
pub fn composer_state(model: &ThreadModel, shutting_down: bool) -> ComposerState {
    if shutting_down {
        ComposerState::ShuttingDown
    } else if model.read_only() {
        ComposerState::ReadOnly
    } else if model.slots.chat.parent_owned {
        ComposerState::ParentOwned
    } else if model.writer == Writer::Detached || model.slots.chat.provider_error.is_some() {
        ComposerState::Disabled
    } else {
        ComposerState::Open
    }
}
