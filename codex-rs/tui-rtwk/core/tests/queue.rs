#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The queue key's lines and their drain at the turn's end (oracle
//! `chatwidget/input_flow.rs` `maybe_send_next_queued_input`, `slash_dispatch.rs`
//! `submit_queued_slash_prompt`, `input_submission.rs` `submit_queued_shell_prompt`): a message
//! starts the next turn, a command is handed to the screen and the queue waits for what it
//! reports, a shell line runs, a bare `!` says how to use it.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::{
    DispatchCx, Outcome, QueueDrain, SlashCommand, Target, dispatch_queued,
};
use codex_tui_core::fixtures::builders::{turn_completed, turn_started};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, ScreenThreadIntent, chat};
use codex_tui_core::threads::queue::Dispatched;
use codex_tui_core::threads::{Item, Queued, Ran, ThreadModel, UserMessage};
use pretty_assertions::assert_eq;
use std::rc::Rc;

use codex_tui_core::app::BackendEv;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

fn thread(app: &CodexApp) -> Rc<ThreadModel> {
    app.active_thread()
}

fn notify(app: &mut CodexApp, ev: BackendEv) -> AppFx {
    run(app, AppEv::Backend(ev))
}

fn intent(app: &mut CodexApp, intent: chat::ThreadIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: ScreenThreadIntent::Chat(intent),
        }),
    )
}

/// What the queue key makes of `text` typed while a turn runs.
fn tab(app: &mut CodexApp, text: &str, shell: bool) -> AppFx {
    intent(
        app,
        chat::ThreadIntent::Queue(Queued::of(UserMessage::text(text), shell)),
    )
}

fn started_texts(fx: &AppFx) -> Vec<String> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect()
}

fn shells(fx: &AppFx) -> Vec<String> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Shell(command)),
                ..
            } => Some(command.clone()),
            _ => None,
        })
        .collect()
}

fn queued(app: &CodexApp) -> Vec<String> {
    thread(app).queue().iter().map(|m| m.text.clone()).collect()
}

fn running() -> CodexApp {
    let mut app = ready();
    notify(&mut app, turn_started(T, "turn-1"));
    app
}

fn dispatched(command: Option<SlashCommand>, modal: bool) -> Ran {
    Ran::Dispatched(Dispatched {
        command,
        modal,
        can_change_directory: true,
        managed_worktree: false,
    })
}

fn notice_count(app: &CodexApp) -> usize {
    let thread = thread(app);
    let transcript = thread.transcript();
    (0..transcript.len())
        .filter_map(|at| transcript.get(at))
        .filter(|(_, item)| matches!(item, Item::Note(_)))
        .count()
}

#[test]
fn a_command_waits_in_the_queue_until_the_turn_ends_then_goes_to_the_screen() {
    let mut app = running();
    let fx = tab(&mut app, "/status", false);
    assert!(fx.commands().is_empty(), "queued, not sent");
    assert_eq!(queued(&app), ["/status"]);
    assert!(matches!(thread(&app).queue()[0], Queued::Command(_)));
    assert!(thread(&app).due_command().is_none());
    let fx = notify(&mut app, turn_completed(T, "turn-1"));
    assert!(started_texts(&fx).is_empty(), "a command is no turn");
    assert!(queued(&app).is_empty());
    let model = thread(&app);
    let due = model.due_command().expect("handed to the screen");
    assert_eq!(due.line().text, "/status");
    assert_eq!(due.seq(), 1);
}

#[test]
fn a_command_that_lets_the_queue_go_on_sends_the_next_message() {
    let mut app = running();
    tab(&mut app, "/status", false);
    tab(&mut app, "hello after status", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(queued(&app), ["hello after status"], "waits for the screen");
    let fx = intent(
        &mut app,
        chat::ThreadIntent::QueueRan(dispatched(Some(SlashCommand::Status), false)),
    );
    assert_eq!(started_texts(&fx), ["hello after status"]);
    assert!(thread(&app).due_command().is_none());
}

#[test]
fn a_menu_command_stops_the_queue_until_the_screen_resumes_it() {
    let mut app = running();
    tab(&mut app, "/model", false);
    tab(&mut app, "hello after selection", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let fx = intent(
        &mut app,
        chat::ThreadIntent::QueueRan(dispatched(Some(SlashCommand::Model), true)),
    );
    assert!(started_texts(&fx).is_empty(), "the picker opened first");
    assert_eq!(queued(&app), ["hello after selection"]);
    // the sheet settled
    let fx = intent(&mut app, chat::ThreadIntent::QueueResume);
    assert_eq!(started_texts(&fx), ["hello after selection"]);
}

#[test]
fn a_continue_command_that_opened_a_popup_stops_all_the_same() {
    let mut app = running();
    tab(&mut app, "/rename", false);
    tab(&mut app, "hello after rename", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let fx = intent(
        &mut app,
        chat::ThreadIntent::QueueRan(dispatched(Some(SlashCommand::Rename), true)),
    );
    assert!(
        started_texts(&fx).is_empty(),
        "the Name thread prompt is open"
    );
    let fx = intent(&mut app, chat::ThreadIntent::QueueResume);
    assert_eq!(started_texts(&fx), ["hello after rename"]);
}

#[test]
fn an_unknown_command_lets_the_queue_go_on() {
    let mut app = running();
    tab(&mut app, "/does-not-exist", false);
    tab(&mut app, "next", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let fx = intent(
        &mut app,
        chat::ThreadIntent::QueueRan(dispatched(None, false)),
    );
    assert_eq!(started_texts(&fx), ["next"]);
}

#[test]
fn a_line_that_went_to_the_model_stops_the_queue() {
    let mut app = running();
    tab(&mut app, "/usr/bin/env", false);
    tab(&mut app, "next", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let fx = intent(&mut app, chat::ThreadIntent::QueueRan(Ran::Submitted));
    assert!(started_texts(&fx).is_empty());
    assert_eq!(queued(&app), ["next"]);
}

#[test]
fn a_report_without_a_due_command_is_nothing() {
    let mut app = running();
    tab(&mut app, "next", false);
    let fx = intent(
        &mut app,
        chat::ThreadIntent::QueueRan(dispatched(Some(SlashCommand::Status), false)),
    );
    assert!(fx.commands().is_empty());
    assert_eq!(queued(&app), ["next"]);
}

#[test]
fn a_bare_bang_says_how_to_use_it_and_the_next_item_goes_out() {
    let mut app = running();
    tab(&mut app, "!", true);
    tab(&mut app, "hello after help", false);
    let before = notice_count(&app);
    let fx = notify(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(started_texts(&fx), ["hello after help"]);
    assert_eq!(notice_count(&app), before + 1, "the help line");
}

#[test]
fn a_shell_line_runs_and_the_turn_it_makes_holds_the_rest() {
    let mut app = running();
    tab(&mut app, "!echo hi", true);
    tab(&mut app, "after the shell", false);
    let fx = notify(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(shells(&fx), ["echo hi"]);
    assert!(started_texts(&fx).is_empty());
    assert_eq!(queued(&app), ["after the shell"]);
}

#[test]
fn a_bang_the_user_did_not_type_in_shell_mode_is_a_message() {
    let mut app = running();
    tab(&mut app, " !echo hi", false);
    assert!(matches!(thread(&app).queue()[0], Queued::Literal(_)));
    let fx = notify(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(started_texts(&fx), ["!echo hi"]);
    assert!(shells(&fx).is_empty());
}

#[test]
fn the_edit_queued_key_gives_a_command_back_as_text() {
    let mut app = running();
    tab(&mut app, "/review check regressions", false);
    intent(&mut app, chat::ThreadIntent::EditQueued);
    assert!(queued(&app).is_empty());
    let restore = thread(&app).restore().cloned().expect("given back");
    let onto = restore.onto(UserMessage::text(""));
    assert_eq!(onto.text, "/review check regressions");
}

#[test]
fn a_command_when_idle_goes_to_the_screen_at_once() {
    let mut app = ready();
    tab(&mut app, "/status", false);
    let model = thread(&app);
    assert_eq!(model.due_command().expect("due").line().text, "/status");
}

fn facts() -> DispatchCx {
    DispatchCx {
        thread_started: true,
        has_backend_auth: true,
        ..DispatchCx::default()
    }
}

#[test]
fn a_queued_line_is_dispatched_as_a_live_one_but_arguments_for_a_bare_command_are_a_message() {
    let queued = dispatch_queued("/model", &facts(), &[]);
    assert_eq!(queued.command, Some(SlashCommand::Model));
    assert!(matches!(queued.outcome, Outcome::Open(ref open) if open.target == Target::Select));
    // `/status now` takes no arguments: the whole line is the user's message
    let queued = dispatch_queued("/status now", &facts(), &[]);
    assert_eq!(queued.outcome, Outcome::NotACommand);
    assert_eq!(queued.command, None);
    // a command that takes them is dispatched with them
    let queued = dispatch_queued("/review check regressions", &facts(), &[]);
    assert_eq!(queued.command, Some(SlashCommand::Review));
    assert!(!matches!(queued.outcome, Outcome::NotACommand));
    // a path is no command
    assert_eq!(
        dispatch_queued("/usr/bin/env", &facts(), &[]).outcome,
        Outcome::NotACommand
    );
    // an unknown name is the oracle's error, and the queue goes on
    let queued = dispatch_queued("/does-not-exist", &facts(), &[]);
    assert_eq!(queued.command, None);
    assert_eq!(
        Dispatched {
            command: queued.command,
            modal: false,
            can_change_directory: true,
            managed_worktree: true,
        }
        .drain(false),
        QueueDrain::Continue
    );
}

/// `item/started` of a command the user (`!`) or the model ran, in `turn-1`.
fn command_started(source: &str, id: &str) -> BackendEv {
    let notification = serde_json::from_value(serde_json::json!({
        "method": "item/started",
        "params": {
            "item": {
                "type": "commandExecution",
                "id": id,
                "command": "echo hi",
                "cwd": "/tmp",
                "processId": null,
                "source": source,
                "status": "inProgress",
                "commandActions": [],
                "aggregatedOutput": null,
                "exitCode": null,
                "durationMs": null
            },
            "threadId": T,
            "turnId": "turn-1",
            "startedAtMs": 0
        }
    }))
    .expect("an item/started notification");
    BackendEv::Notification(Box::new(notification))
}

fn submit(app: &mut CodexApp, text: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            message: UserMessage::text(text),
        }),
    )
}

fn steered(fx: &AppFx) -> bool {
    fx.commands().iter().any(|c| {
        matches!(
            c,
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Steer { .. }),
                ..
            }
        )
    })
}

/// `only_user_shell_commands_running`: while only the user's own `!` command runs, Enter on a
/// message waits for the turn (it is queued) where it would steer a model's turn.
#[test]
fn a_message_typed_while_only_the_users_shell_command_runs_waits_for_the_turn() {
    let mut app = running();
    notify(&mut app, command_started("userShell", "shell-1"));
    let fx = submit(&mut app, "and then this");
    assert!(!steered(&fx));
    assert!(started_texts(&fx).is_empty());
    assert_eq!(queued(&app), ["and then this"]);
    let fx = notify(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(started_texts(&fx), ["and then this"]);
}

#[test]
fn a_message_typed_while_the_model_runs_a_command_steers_the_turn() {
    let mut app = running();
    notify(&mut app, command_started("agent", "exec-1"));
    let fx = submit(&mut app, "wait, not that");
    assert!(steered(&fx));
    assert_eq!(queued(&app), Vec::<String>::new());
    // one command of the model among the user's: the turn is the model's
    let mut app = running();
    notify(&mut app, command_started("userShell", "shell-1"));
    notify(&mut app, command_started("agent", "exec-1"));
    let fx = submit(&mut app, "wait, not that");
    assert!(steered(&fx));
}

/// The queue holds while the thread is refused or a settings pick failed (`submissions_paused`,
/// `suppress_queue_autosend`): the turn's end sends nothing until the user sends again.
#[test]
fn a_held_queue_sends_nothing_at_the_turn_end_and_goes_on_when_the_user_sends() {
    let mut app = running();
    tab(&mut app, "waits", false);
    intent(&mut app, chat::ThreadIntent::HoldQueue);
    assert!(thread(&app).queue_held());
    let fx = notify(&mut app, turn_completed(T, "turn-1"));
    assert!(started_texts(&fx).is_empty());
    assert_eq!(queued(&app), ["waits"]);
    // the user sends: the line goes behind nothing, the held one stays for the next end
    let fx = submit(&mut app, "now");
    assert_eq!(started_texts(&fx), ["now"]);
}
