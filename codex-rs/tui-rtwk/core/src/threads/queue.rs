//! What waits behind a running turn: the lines the queue key (Tab) set aside, each with what the
//! drain at the turn's end does with it (oracle `chatwidget/input_flow.rs`, `QueuedInputAction`
//! and `maybe_send_next_queued_input`; `bottom_pane/chat_composer/slash_input.rs`,
//! `queued_input_action`).
//!
//! A message and a literal paste start the next turn; a `/command` is dispatched again when its
//! turn comes (the screen runs it and reports what it opened); a `!shell` line runs as the
//! user's own shell command. The commands' rule for the next item is `QueueDrain`
//! (`commands::queued_command_drain_result`).

use std::ops::Deref;

use crate::commands::{QueueDrain, SlashCommand, queued_command_drain_result};
use crate::threads::UserMessage;

/// A line of the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Queued {
    /// A message for the model.
    Message(UserMessage),
    /// A message that is sent as it is: no slash or shell parsing (a `!` the user did not type
    /// in shell mode, a paste that merely starts with `/`).
    Literal(UserMessage),
    /// A line that starts with `/`: dispatched when its turn comes.
    Command(UserMessage),
    /// A `!` line of the composer's shell mode: run when its turn comes.
    Shell(UserMessage),
}

impl Queued {
    /// The kind the queue key gives `message` typed in the composer (`queued_input_action`):
    /// `shell` is the composer being in shell mode. A line led by a space is not a command.
    pub fn of(message: UserMessage, shell: bool) -> Queued {
        let raw = message.text.as_str();
        let slash = !raw.starts_with(' ') && raw.trim().starts_with('/');
        let message = message.trimmed();
        if message.text.starts_with('!') {
            if shell {
                Queued::Shell(message)
            } else {
                Queued::Literal(message)
            }
        } else if slash {
            Queued::Command(message)
        } else {
            Queued::Message(message)
        }
    }

    pub fn message(&self) -> &UserMessage {
        match self {
            Queued::Message(message)
            | Queued::Literal(message)
            | Queued::Command(message)
            | Queued::Shell(message) => message,
        }
    }

    /// The message as the composer gets it back (the edit-queued key, an interrupt).
    pub fn into_message(self) -> UserMessage {
        match self {
            Queued::Message(message)
            | Queued::Literal(message)
            | Queued::Command(message)
            | Queued::Shell(message) => message,
        }
    }
}

impl Deref for Queued {
    type Target = UserMessage;

    fn deref(&self) -> &UserMessage {
        self.message()
    }
}

impl From<UserMessage> for Queued {
    fn from(message: UserMessage) -> Self {
        Queued::Message(message)
    }
}

/// A queued command popped for the screen to run: the screen dispatches `line` as it would a
/// line typed now (`dispatch_queued`), then answers with what happened
/// (`ThreadIntent::QueueRan`); the queue waits for the answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueCommand {
    seq: u64,
    line: UserMessage,
}

impl DueCommand {
    pub(super) fn handed(seq: u64, line: UserMessage) -> Self {
        Self { seq, line }
    }

    /// Counts the commands of the thread: a change of it is a new command to run.
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// The line, as queued (it starts with `/`).
    pub fn line(&self) -> &UserMessage {
        &self.line
    }
}

/// What the screen reports of a queued command it ran (`submit_queued_slash_prompt`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ran {
    /// The line went to the model as a message (a path, a command that takes no arguments given
    /// some): a turn starts, the queue stops.
    Submitted,
    /// The line was dispatched: the command (none for an unknown name, whose error note the
    /// screen said, or a service tier) and the facts the rule of the next item reads.
    Dispatched(Dispatched),
}

/// The facts of a dispatched queued command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dispatched {
    pub command: Option<SlashCommand>,
    /// A sheet, dialog or prompt is open now (`!no_modal_or_popup_active`).
    pub modal: bool,
    /// `/cd` is open to this thread (`can_change_working_directory`).
    pub can_change_directory: bool,
    /// `/worktree` has a managed worktree to work on (`managed_worktree_available`).
    pub managed_worktree: bool,
}

impl Dispatched {
    /// What the queue does next (`queued_command_drain_result`): a turn running or about to start
    /// or an open popup stops it before the command's own rule is read; an unknown name or a
    /// service tier lets it go on.
    pub fn drain(&self, turn_pending: bool) -> QueueDrain {
        if turn_pending || self.modal {
            return QueueDrain::Stop;
        }
        match self.command {
            Some(command) => queued_command_drain_result(
                command,
                self.can_change_directory,
                self.managed_worktree,
            ),
            None => QueueDrain::Continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(text: &str) -> UserMessage {
        UserMessage::text(text)
    }

    #[test]
    fn the_queue_key_sorts_a_line_by_its_first_character() {
        assert_eq!(
            Queued::of(text("hello"), false),
            Queued::Message(text("hello"))
        );
        assert_eq!(
            Queued::of(text("/review x"), false),
            Queued::Command(text("/review x"))
        );
        assert_eq!(
            Queued::of(text("!echo hi"), true),
            Queued::Shell(text("!echo hi"))
        );
        assert_eq!(Queued::of(text("!"), true), Queued::Shell(text("!")));
        // a `!` the user did not type in shell mode is a message as it stands
        assert_eq!(
            Queued::of(text(" !echo hi"), false),
            Queued::Literal(text("!echo hi"))
        );
        // a line led by a space is not a command
        assert_eq!(
            Queued::of(text(" /model"), false),
            Queued::Message(text("/model"))
        );
    }

    #[test]
    fn a_popup_or_a_running_turn_stops_the_queue_before_the_rule() {
        let ran = Dispatched {
            command: Some(SlashCommand::Status),
            modal: false,
            can_change_directory: true,
            managed_worktree: true,
        };
        assert_eq!(ran.drain(false), QueueDrain::Continue);
        assert_eq!(ran.drain(true), QueueDrain::Stop);
        assert_eq!(
            Dispatched { modal: true, ..ran }.drain(false),
            QueueDrain::Stop
        );
        assert_eq!(
            Dispatched {
                command: None,
                ..ran
            }
            .drain(false),
            QueueDrain::Continue
        );
        assert_eq!(
            Dispatched {
                command: Some(SlashCommand::Model),
                ..ran
            }
            .drain(false),
            QueueDrain::Stop
        );
    }
}
