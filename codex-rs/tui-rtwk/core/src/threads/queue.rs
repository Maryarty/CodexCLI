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

use super::{Item, Note, ThreadFx, ThreadModel, apply_appended, give_back, merge_messages};
use crate::commands::{QueueDrain, SlashCommand, queued_command_drain_result};
use crate::ids::ThreadId;
use crate::rpc::Address;
use crate::screens::{self, ScreenThreadIntent};
use crate::threads::UserMessage;

mod modal;

pub(crate) use modal::ModalKind;
pub use modal::{CopyChoice, ModalOperation, ModalReceipt, ModalToken, StatusLineChoice};

/// A line of the queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Queued {
    /// A message for the model.
    Message(UserMessage),
    /// Submitted before a disconnect; only a matching receipt can remove it automatically.
    Unconfirmed {
        message: Box<UserMessage>,
        client_id: String,
        submitted: crate::ids::Ticket,
    },
    /// A message that is sent as it is: no slash or shell parsing (a `!` the user did not type
    /// in shell mode, a paste that merely starts with `/`), and the text as it stood in the
    /// composer when it differs from the message (a `[Pasted Content 1006 chars]` placeholder
    /// the queue row keeps, `queue_user_message_with_options`). The label is a thin pointer: a
    /// queued item travels in the `Err` of the thread's intents (`result_large_err`).
    Literal(UserMessage, Option<Box<String>>),
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
                Queued::Literal(message, None)
            }
        } else if slash {
            Queued::Command(message)
        } else {
            Queued::Message(message)
        }
    }

    /// The line as the composer showed it: a literal paste keeps its placeholder.
    #[must_use]
    pub fn shown_as(self, composer_text: &str) -> Queued {
        match self {
            Queued::Literal(message, _) => {
                let label = composer_text.trim();
                let label = (label != message.text).then(|| Box::new(label.to_owned()));
                Queued::Literal(message, label)
            }
            other => other,
        }
    }

    pub fn message(&self) -> &UserMessage {
        match self {
            Queued::Message(message)
            | Queued::Literal(message, _)
            | Queued::Command(message)
            | Queued::Shell(message) => message,
            Queued::Unconfirmed { message, .. } => message,
        }
    }

    /// The text of the queue's row: what the user saw in the composer, else the message.
    pub fn shown(&self) -> &str {
        match self {
            Queued::Literal(_, Some(label)) => label,
            _ => &self.message().text,
        }
    }

    /// The message as the composer gets it back (the edit-queued key, an interrupt).
    pub fn into_message(self) -> UserMessage {
        match self {
            Queued::Message(message)
            | Queued::Literal(message, _)
            | Queued::Command(message)
            | Queued::Shell(message) => message,
            Queued::Unconfirmed { message, .. } => *message,
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
    operation: QueueOperation,
    line: UserMessage,
    phase: Phase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Issued,
    AwaitingModelPicker,
    ModelPickerOpen,
    ApplyingModel,
}

/// A queued command belongs to one thread and one generation of its queue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueOperation {
    thread: ThreadId,
    seq: u64,
}

impl QueueOperation {
    pub(crate) fn modal(thread: ThreadId, seq: u64) -> Self {
        Self { thread, seq }
    }

    pub fn thread(&self) -> &ThreadId {
        &self.thread
    }
}

/// Receipts for the operation issued by the queue owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueueEvent {
    Ran(QueueRan),
    PickerOpened(QueueOperation),
    PickerClosed(QueueOperation),
    PickerAccepted(QueueOperation),
    ModelApplied(QueueOperation),
    ModalOpening(ModalOpening),
    ModalOwnerLeft(ModalOwnerLeft),
    ModalOpened(ModalOperation),
    ModalFinished(ModalReceipt),
    ModalApplying(ModalApplying),
    ModalAcknowledged(ModalAcknowledged),
}

/// Issued only by the queue owner before asking the UI to show a modal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalOpening {
    operation: ModalOperation,
    address: Address,
}

/// The app observed the displayed conversation change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalOwnerLeft(());

/// The choice was consumed and its effect now owns the hold until it settles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalApplying(ModalOperation);

/// The effect of a native choice reached its terminal result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalAcknowledged(ModalOperation);

impl QueueEvent {
    pub(crate) fn opening_modal(operation: ModalOperation, address: Address) -> Self {
        Self::ModalOpening(ModalOpening { operation, address })
    }

    pub(crate) fn modal_owner_left() -> Self {
        Self::ModalOwnerLeft(ModalOwnerLeft(()))
    }

    pub(crate) fn applying_modal(operation: ModalOperation) -> Self {
        Self::ModalApplying(ModalApplying(operation))
    }

    pub(crate) fn acknowledged_modal(operation: ModalOperation) -> Self {
        Self::ModalAcknowledged(ModalAcknowledged(operation))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModalPhase {
    Opening(Address, ModalKind),
    Open(Address, ModalKind),
    Applying {
        address: Address,
        kind: ModalKind,
        owner_left: bool,
    },
    /// Its dialog may still close the original hold.
    OwnerLeft,
}

pub(crate) fn has_live_modal(model: &ThreadModel) -> bool {
    model.queue_modals.values().any(|phase| {
        !matches!(
            phase,
            ModalPhase::OwnerLeft
                | ModalPhase::Applying {
                    owner_left: true,
                    ..
                }
        )
    })
}

pub(crate) fn modal_is_visible(
    model: &ThreadModel,
    operation: &ModalOperation,
    address: Address,
) -> bool {
    model.id() == operation.thread()
        && matches!(model.queue_modals.get(&operation.operation().seq),
            Some(ModalPhase::Opening(issued, kind) | ModalPhase::Open(issued, kind))
                if *issued == address && *kind == operation.kind())
}

pub(crate) fn modal_is_open(
    model: &ThreadModel,
    operation: &ModalOperation,
    address: Address,
) -> bool {
    model.id() == operation.thread()
        && model.queue_modals.get(&operation.operation().seq)
            == Some(&ModalPhase::Open(address, operation.kind()))
}

pub(crate) fn modal_is_applying(
    model: &ThreadModel,
    operation: &ModalOperation,
    address: Address,
) -> bool {
    model.id() == operation.thread()
        && model.queue_modals.get(&operation.operation().seq)
            == Some(&ModalPhase::Applying {
                address,
                kind: operation.kind(),
                owner_left: false,
            })
}

fn holds_picker(due: &DueCommand, operation: &QueueOperation) -> bool {
    due.operation == *operation
        && matches!(
            due.phase,
            Phase::AwaitingModelPicker | Phase::ModelPickerOpen
        )
}

pub(crate) fn can_apply_model(model: &ThreadModel, operation: &QueueOperation) -> bool {
    model.has_writer() && has_model_picker(model, operation)
}

pub(crate) fn has_model_picker(model: &ThreadModel, operation: &QueueOperation) -> bool {
    model
        .due
        .as_ref()
        .is_some_and(|due| holds_picker(due, operation))
}

/// A report can advance only the operation that issued it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueRan {
    operation: QueueOperation,
    outcome: Report,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Report {
    Ran(Ran),
    OpeningModelPicker,
}

impl DueCommand {
    pub(super) fn handed(thread: ThreadId, seq: u64, line: UserMessage) -> Self {
        Self {
            operation: QueueOperation { thread, seq },
            line,
            phase: Phase::Issued,
        }
    }

    pub(super) fn issued(&self) -> bool {
        self.phase == Phase::Issued
    }

    /// A new generation is a new command for the screen to run.
    pub fn seq(&self) -> u64 {
        self.operation.seq
    }

    pub fn operation(&self) -> &QueueOperation {
        &self.operation
    }

    pub fn report(&self, outcome: Ran) -> QueueEvent {
        QueueEvent::Ran(QueueRan {
            operation: self.operation.clone(),
            outcome: Report::Ran(outcome),
        })
    }

    pub(crate) fn opening_model_picker(&self) -> QueueEvent {
        QueueEvent::Ran(QueueRan {
            operation: self.operation.clone(),
            outcome: Report::OpeningModelPicker,
        })
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

/// A picker or modal receipt can settle its operation even when the writer has detached.
pub(super) fn is_queue_settlement(intent: &ScreenThreadIntent) -> bool {
    use screens::chat::ThreadIntent as Chat;
    matches!(
        intent,
        ScreenThreadIntent::Chat(Chat::QueueEvent(
            QueueEvent::Ran(QueueRan {
                outcome: Report::OpeningModelPicker,
                ..
            }) | QueueEvent::PickerOpened(_)
                | QueueEvent::PickerClosed(_)
                | QueueEvent::ModelApplied(_)
                | QueueEvent::ModalOpening(_)
                | QueueEvent::ModalOwnerLeft(_)
                | QueueEvent::ModalOpened(_)
                | QueueEvent::ModalFinished(_)
                | QueueEvent::ModalApplying(_)
                | QueueEvent::ModalAcknowledged(_)
        ))
    )
}

/// The thread queue's intents. The caller receives every other intent untouched (ADR-012).
pub(super) fn apply(
    model: &mut ThreadModel,
    intent: ScreenThreadIntent,
    fx: &mut ThreadFx<'_>,
) -> Result<(), ScreenThreadIntent> {
    use screens::chat::ThreadIntent as Chat;
    let ScreenThreadIntent::Chat(intent) = intent else {
        return Err(intent);
    };
    // A misalignment stop rejects what starts a turn or waits for one (`rejects_misalignment_
    // policy_op`); a plan's implementation is a submit, which the thread refuses itself.
    if model.safety.blocks_input()
        && matches!(
            intent,
            Chat::Compact
                | Chat::Review(_)
                | Chat::Shell(_)
                | Chat::Queue(_)
                | Chat::ApprovePlan(_)
        )
    {
        return Ok(());
    }
    match intent {
        // Tab: the line waits behind the running turn.
        Chat::Queue(message) => enqueue(model, message, fx),
        // The screen ran the queued command it was handed: the rule of the next item reads what
        // it opened (`submit_queued_slash_prompt`).
        Chat::QueueEvent(event) => match event {
            QueueEvent::ModalOpening(ModalOpening { operation, address }) => {
                let kind = operation.kind();
                let operation = operation.operation();
                if model.id() == operation.thread() {
                    model
                        .queue_modals
                        .entry(operation.seq)
                        .or_insert(ModalPhase::Opening(address, kind));
                    if model
                        .due
                        .as_ref()
                        .is_some_and(|due| &due.operation == operation && due.issued())
                    {
                        model.due = None;
                    }
                }
            }
            QueueEvent::ModalOpened(operation) => {
                if model.id() == operation.thread()
                    && let Some(phase) = model.queue_modals.get_mut(&operation.operation().seq)
                    && let ModalPhase::Opening(address, kind) = *phase
                    && kind == operation.kind()
                {
                    *phase = ModalPhase::Open(address, kind);
                }
            }
            QueueEvent::ModalOwnerLeft(_) => {
                for phase in model.queue_modals.values_mut() {
                    match phase {
                        ModalPhase::Applying { owner_left, .. } => *owner_left = true,
                        _ => *phase = ModalPhase::OwnerLeft,
                    }
                }
            }
            QueueEvent::ModalFinished(receipt) => {
                let seq = receipt.operation().seq;
                if model.id() == receipt.thread()
                    && model.queue_modals.get(&seq).is_some_and(|phase| {
                        matches!(phase, ModalPhase::Opening(_, kind) | ModalPhase::Open(_, kind) if *kind == receipt.kind())
                            || matches!(phase, ModalPhase::OwnerLeft)
                    })
                {
                    model.queue_modals.remove(&seq);
                    send_next(model, fx);
                }
            }
            QueueEvent::ModalApplying(ModalApplying(operation)) => {
                if model.id() == operation.thread()
                    && let Some(phase) = model.queue_modals.get_mut(&operation.operation().seq)
                    && let ModalPhase::Open(address, kind) = *phase
                    && kind == operation.kind()
                {
                    *phase = ModalPhase::Applying {
                        address,
                        kind,
                        owner_left: false,
                    };
                }
            }
            QueueEvent::ModalAcknowledged(ModalAcknowledged(operation)) => {
                let seq = operation.operation().seq;
                if model.id() == operation.thread()
                    && matches!(model.queue_modals.get(&seq), Some(ModalPhase::Applying { kind, .. }) if *kind == operation.kind())
                {
                    model.queue_modals.remove(&seq);
                    send_next(model, fx);
                }
            }
            QueueEvent::Ran(report) => {
                let Some(due) = model
                    .due
                    .as_mut()
                    .filter(|due| due.issued() && due.operation == report.operation)
                else {
                    return Ok(());
                };
                let stop = match report.outcome {
                    Report::Ran(Ran::Submitted) => true,
                    Report::Ran(Ran::Dispatched(ran)) => {
                        ran.drain(model.busy()) == QueueDrain::Stop
                    }
                    Report::OpeningModelPicker => {
                        due.phase = Phase::AwaitingModelPicker;
                        return Ok(());
                    }
                };
                model.due = None;
                if !stop {
                    send_next(model, fx);
                }
            }
            QueueEvent::PickerOpened(operation) => {
                if let Some(due) = model.due.as_mut().filter(|due| {
                    due.operation == operation && due.phase == Phase::AwaitingModelPicker
                }) {
                    due.phase = Phase::ModelPickerOpen;
                }
            }
            QueueEvent::PickerClosed(operation) => {
                if model
                    .due
                    .as_ref()
                    .is_some_and(|due| holds_picker(due, &operation))
                {
                    model.due = None;
                    if !model.busy() {
                        send_next(model, fx);
                    }
                }
            }
            QueueEvent::PickerAccepted(operation) => {
                if let Some(due) = model
                    .due
                    .as_mut()
                    .filter(|due| holds_picker(due, &operation))
                {
                    due.phase = Phase::ApplyingModel;
                }
            }
            QueueEvent::ModelApplied(operation) => {
                if model.due.as_ref().is_some_and(|due| {
                    due.operation == operation && due.phase == Phase::ApplyingModel
                }) {
                    model.due = None;
                    if !model.busy() {
                        send_next(model, fx);
                    }
                }
            }
        },
        // Reconsider admission after a generic event. A retained operation still holds the queue.
        Chat::QueueResume => {
            if !model.busy() {
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
    let busy = model.busy();
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
    if !model.has_writer()
        || model.stream_lost
        || model.reconciling_submissions
        || super::cloud_admission_pending(model, fx)
        || matches!(model.queue.first(), Some(Queued::Unconfirmed { .. }))
        || model.slots.chat.mcp.is_starting()
        || model.safety.blocks_input()
        || model.slots.chat.provider_error.is_some()
        || model.recovered
        || model.due.is_some()
        || !model.queue_modals.is_empty()
        || model.features.rename.holds_queue()
        || model.slots.chat.is_compacting()
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
        if super::reject_image_input(model, fx, &merged) {
            super::recover(model, merged);
        } else if let Err(merged) = fx.start_turn(merged) {
            model.rejected_steers.push(merged);
        }
        return;
    }
    while !model.queue.is_empty() {
        match model.queue.remove(0) {
            entry @ Queued::Unconfirmed { .. } => {
                model.queue.insert(0, entry);
                return;
            }
            // a thread whose model is empty takes no turn: the message goes back to the composer
            // and the rest of the queue stays (`submit_user_message`)
            Queued::Message(message) | Queued::Literal(message, _)
                if super::startup::model_unavailable(model.slots.chat.model.as_deref()) =>
            {
                apply_appended(
                    model,
                    None,
                    Item::Note(Note::from(crate::commands::Notice::ThreadModelUnavailable)),
                );
                give_back(model, vec![message], false);
                return;
            }
            Queued::Message(message) | Queued::Literal(message, _)
                if super::reject_image_input(model, fx, &message) =>
            {
                super::recover(model, message);
                return;
            }
            Queued::Message(message) => {
                if let Err(message) = fx.start_turn(message) {
                    model.queue.insert(0, Queued::Message(message));
                }
                return;
            }
            Queued::Literal(message, label) => {
                if let Err(message) = fx.start_turn(message) {
                    model.queue.insert(0, Queued::Literal(message, label));
                }
                return;
            }
            // `submit_queued_slash_prompt`: the screen dispatches it; the queue goes on when it
            // reports (`ThreadIntent::QueueRan`).
            Queued::Command(line) => {
                model.due = Some(DueCommand::handed(
                    model.id().clone(),
                    fx.mint().seq(),
                    line,
                ));
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
