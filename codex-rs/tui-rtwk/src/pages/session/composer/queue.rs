//! The queue's side of a submitted line (`actions.rs` runs the line): the message a line is, the
//! Tab that keeps it behind a running turn, and what a command that ran tells the queue.

use rtwk::Cx;

use codex_tui_core::commands::SlashCommand;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::question_reply::Intent as QuestionIntent;
use codex_tui_core::screens::{
    ScreenIntent, ScreenStates, ScreenThreadIntent, chat, popovers, select,
};
use codex_tui_core::threads::queue::Dispatched;
use codex_tui_core::threads::{DueCommand, Queued, Ran, UserMessage};

use crate::app::view::Hs;
use crate::features;
use crate::pages::session::actions::run;
use crate::pages::session::composer::strings;
use crate::pages::session::session_page::{Chat, ChatState};

/// What a line that is a message for the model does while a turn runs:
/// Enter steers it (`apply` decides), Tab queues it behind the turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::pages::session) enum Delivery {
    Submit,
    Queue,
    /// A line of the queue that is due (`dispatch_queued`): it runs as if typed now, but it is
    /// no draft: the composer, its history and its shell mode are the user's, as they are.
    Drain,
}

impl Delivery {
    /// Whether `line` waits behind the running turn: Tab says so, and so does Enter on a paste
    /// that merely starts with `!` (not shell mode), which is never a steer
    /// (`QueuedInputAction::Literal`).
    pub(in crate::pages::session) fn queues(self, line: &str, busy: bool, shell: bool) -> bool {
        self == Delivery::Queue
            || (self == Delivery::Submit && busy && !shell && line.trim_start().starts_with('!'))
    }
}

/// The message `line` is: the draft with its images and mentions when it is the draft, else
/// the line as text (a popup's pick).
pub(in crate::pages::session) fn message(
    line: &str,
    w: &Chat,
    cx: &mut Cx<'_, ChatState>,
    states: &ScreenStates,
) -> UserMessage {
    let table = strings::of(cx);
    let draft = w.ui.draft.submission_with(
        &w.ui.composer,
        |n| table.image(n),
        states.popovers.sources().mentions,
    );
    if draft.text.trim() == line.trim() {
        draft
    } else {
        UserMessage::text(line)
    }
}

/// A message of the user's own: it goes to the thread and answers the questions that wait there
/// (`clear_pending_questions`).
pub(in crate::pages::session) fn send(
    cx: &mut Cx<'_, ChatState>,
    hs: &Hs,
    message: UserMessage,
    thread: Option<ThreadId>,
) {
    cx.emit(&hs.submit, message);
    if let Some(thread) = thread {
        features::verb(cx, QuestionIntent::Superseded { thread });
    }
}

/// Tab behind a running turn keeps the line as it is: a command or a shell line is parsed when
/// its turn comes, never now (`queued_input_action`).
pub(in crate::pages::session) fn keep(
    line: &str,
    w: &Chat,
    cx: &mut Cx<'_, ChatState>,
    hs: &Hs,
    thread: &ThreadId,
    states: &ScreenStates,
) {
    let record = popovers::Intent::Record(line.to_string());
    cx.emit(&hs.screen, ScreenIntent::from(record));
    // a literal paste keeps its placeholder in the queue's row, the expansion is the turn's text
    let queued = Queued::of(message(line, w, cx, states), w.ui.draft.shell())
        .shown_as(&w.ui.composer.text());
    let intent = ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(queued));
    cx.emit(&hs.thread_screen, (thread.clone(), intent));
}

/// A supplied line can come from the queue or an immediately recalled preset.
#[derive(Clone, Copy)]
pub(in crate::pages::session) enum DeferredLine<'a> {
    Due(&'a DueCommand),
    Preset(&'a UserMessage),
}

impl<'a> DeferredLine<'a> {
    pub(in crate::pages::session) fn message(self) -> &'a UserMessage {
        match self {
            Self::Due(due) => due.line(),
            Self::Preset(message) => message,
        }
    }
}

pub(in crate::pages::session) fn scope_select(
    mut intent: select::Intent,
    deferred: Option<DeferredLine<'_>>,
    ran: &mut Option<Ran>,
) -> select::Intent {
    if let (Some(DeferredLine::Due(due)), select::Intent::OpenModels(request)) =
        (deferred, &mut intent)
    {
        request.operation = Some(due.operation().clone());
        // The select reducer acknowledges this exact request. A second report could release it.
        *ran = None;
    }
    intent
}

/// The queued command that is due runs through the dispatch a typed line does
/// (`submit_queued_slash_prompt`); what it did is the thread's to hear of (`QueueRan`).
pub(in crate::pages::session) fn run_due(
    due: &DueCommand,
    w: &Chat,
    cx: &mut Cx<'_, ChatState>,
    hs: &Hs,
) -> Option<Ran> {
    let mut ran = Some(Ran::Dispatched(none_opened()));
    let line = due.line();
    run(
        &line.text,
        Some(DeferredLine::Due(due)),
        w,
        cx,
        hs,
        Delivery::Drain,
        &mut ran,
    );
    ran
}

/// A command that opened nothing and has no rule of its own: the queue goes on.
fn none_opened() -> Dispatched {
    Dispatched {
        command: None,
        modal: false,
        can_change_directory: false,
        managed_worktree: false,
    }
}

/// What a command that opens a sheet or a page leaves the queue with: it waits until the page is
/// in front again (`!no_modal_or_popup_active`).
pub(in crate::pages::session) fn opened_by(command: SlashCommand) -> Ran {
    Ran::Dispatched(Dispatched {
        command: Some(command),
        modal: true,
        ..none_opened()
    })
}

/// What a command of the session leaves the queue with.
pub(in crate::pages::session) fn session_ran(command: SlashCommand) -> Ran {
    Ran::Dispatched(Dispatched {
        command: Some(command),
        ..none_opened()
    })
}
