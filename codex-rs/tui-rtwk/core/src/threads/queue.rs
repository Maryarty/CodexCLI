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

use super::{
    Item, Note, ThreadFx, ThreadModel, apply_appended, give_back, is_start_turn, merge_messages,
};
use crate::commands::{QueueDrain, SlashCommand, queued_command_drain_result};
use crate::screens::{self, ScreenThreadIntent};
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

/// One chat intent on a thread this process writes: the queue's five (Tab, the screen's report of
/// a command it ran, what stopped the queue is over, the hold and the edit key) are applied here
/// (the views' door stays `chat::ThreadIntent`; the queue is the thread's own, not a feature's:
/// ADR-012 "thread lifecycle state"); any other intent is given back, untouched, to the caller
/// (`Err`), so an intent that is nobody's is never dropped here.
pub(super) fn apply(
    model: &mut ThreadModel,
    intent: ScreenThreadIntent,
    fx: &mut ThreadFx<'_>,
) -> Result<(), ScreenThreadIntent> {
    use screens::chat::ThreadIntent as Chat;
    let ScreenThreadIntent::Chat(intent) = intent else {
        return Err(intent);
    };
    match intent {
        // Tab: the line waits behind the running turn.
        Chat::Queue(message) => enqueue(model, message, fx),
        // The screen ran the queued command it was handed: the rule of the next item reads what
        // it opened (`submit_queued_slash_prompt`).
        Chat::QueueRan(ran) => {
            if model.due.take().is_none() {
                return Ok(());
            }
            let busy = model.is_running() || model.outgoing.any(is_start_turn);
            let stop = match ran {
                Ran::Submitted => true,
                Ran::Dispatched(ran) => ran.drain(busy) == QueueDrain::Stop,
            };
            if !stop {
                send_next(model, fx);
            }
        }
        // What stopped the queue is over (the sheet closed, a settings pick was applied, a goal
        // was set): the next item goes on (`maybe_send_next_queued_input`).
        Chat::QueueResume => {
            if !model.is_running() && !model.outgoing.any(is_start_turn) {
                send_next(model, fx);
            }
        }
        // A settings pick that was to run before what waits went on failed: nothing goes out
        // until the user sends again (`retain_input_after_failed_permission_selection`).
        Chat::HoldQueue => {
            model.recovered |= !model.queue.is_empty()
                || !model.rejected_steers.is_empty()
                || !model.pending_steers.is_empty();
        }
        // The edit-queued key: the newest line that waits goes back to the composer
        // (`pop_latest_queued_composer_state`).
        Chat::EditQueued => {
            let message = model
                .queue
                .pop()
                .map(Queued::into_message)
                .or_else(|| model.rejected_steers.pop());
            if let Some(message) = message {
                give_back(model, vec![message], true);
            }
            model.recovered &= !model.queue.is_empty() || !model.rejected_steers.is_empty();
        }
        other => return Err(ScreenThreadIntent::Chat(other)),
    }
    Ok(())
}

/// A message to the queue, behind the running turn; with none running and
/// nothing older waiting it goes out at once.
pub(super) fn enqueue(model: &mut ThreadModel, message: Queued, fx: &mut ThreadFx<'_>) {
    let busy = model.is_running() || model.outgoing.any(is_start_turn);
    model.queue.push(message);
    model.last_submit = Some(fx.mint());
    if !busy {
        send_next(model, fx);
    }
}

/// The next queued message becomes the next turn (one per turn end); a queued command is
/// handed to the screen and a shell line runs, after which the queue goes on or stops by the
/// command's rule (`maybe_send_next_queued_input`).
pub(super) fn send_next(model: &mut ThreadModel, fx: &mut ThreadFx<'_>) {
    // a refused thread keeps what waits (oracle `suppress_queue_autosend`); a command the screen
    // runs is waited for
    if model.slots.chat.mcp.is_starting()
        || model.safety.blocks_input()
        || model.slots.chat.provider_error.is_some()
        || model.recovered
        || model.due.is_some()
    {
        return;
    }
    // a start said in this step goes first: what waits stays in the queue, for the next turn end
    if fx.start_said() {
        return;
    }
    // Steers the server refused go first, as one message
    // (`pop_next_queued_user_message`).
    if !model.rejected_steers.is_empty() {
        let merged = merge_messages(std::mem::take(&mut model.rejected_steers));
        if let Err(merged) = fx.start_turn(merged) {
            model.rejected_steers.push(merged);
        }
        return;
    }
    while !model.queue.is_empty() {
        match model.queue.remove(0) {
            Queued::Message(message) => {
                if let Err(message) = fx.start_turn(message) {
                    model.queue.insert(0, Queued::Message(message));
                }
                return;
            }
            Queued::Literal(message) => {
                if let Err(message) = fx.start_turn(message) {
                    model.queue.insert(0, Queued::Literal(message));
                }
                return;
            }
            // `submit_queued_slash_prompt`: the screen dispatches it; the queue goes on when it
            // reports (`ThreadIntent::QueueRan`).
            Queued::Command(line) => {
                model.due_seq += 1;
                model.due = Some(DueCommand::handed(model.due_seq, line));
                return;
            }
            // `submit_queued_shell_prompt`: a bare `!` says how to use it and the queue goes
            // on; a command runs, and the turn it makes stops the queue.
            Queued::Shell(line) => {
                let command = line.text.strip_prefix('!').unwrap_or(&line.text).trim();
                if command.is_empty() {
                    apply_appended(
                        model,
                        None,
                        Item::Note(Note::from(crate::commands::Notice::ShellHelp)),
                    );
                    continue;
                }
                let intent = screens::chat::ThreadIntent::Shell(command.to_string());
                screens::apply_thread_intent(
                    &mut model.slots,
                    ScreenThreadIntent::Chat(intent),
                    false,
                    fx,
                );
                return;
            }
        }
    }
}
