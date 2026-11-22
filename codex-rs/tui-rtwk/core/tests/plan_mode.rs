//! Plan collaboration mode on the plain model: the user's switches, the plan the turn
//! proposes, the question asked when the turn ends and its answer. No terminal, no backend
//! (ADR-003: the model is testable without the UI); the mode is a controller of the chat's
//! thread part and only `threads::apply` changes it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::rc::Rc;

use codex_app_server_protocol::{
    ItemCompletedNotification, PlanDeltaNotification, ServerNotification, ThreadItem,
    ThreadItemPayload,
};
use codex_protocol::config_types::ModeKind;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::fixtures::builders::{turn_completed, turn_failed, turn_started};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::chat::plan_mode::IMPLEMENT_MESSAGE;
use codex_tui_core::screens::{ScreenCommand, ScreenThreadIntent, chat};
use codex_tui_core::threads::{Item, NoteKey, ThreadModel, UserMessage};

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn notification(app: &mut CodexApp, ev: BackendEv) -> AppFx {
    run(app, AppEv::Backend(ev))
}

/// The app with its thread started, as after `Connected` and the reply.
fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket = fx
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

fn mode(app: &CodexApp) -> ModeKind {
    thread(app).screen_slots().chat.mode()
}

fn prompt_open(app: &CodexApp) -> bool {
    thread(app).screen_slots().chat.plan.prompt().is_some()
}

fn chat_intent(app: &mut CodexApp, intent: chat::ThreadIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: ScreenThreadIntent::Chat(intent),
        }),
    )
}

/// The modes the server was asked to move to in `fx`.
fn mode_requests(fx: &AppFx) -> Vec<ModeKind> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Mode(mode)),
                ..
            } => Some(mode.mode),
            _ => None,
        })
        .collect()
}

/// The texts of every `StartTurn` in `fx`.
fn started(fx: &AppFx) -> Vec<String> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::StartTurn { message, .. } => Some(message.text.clone()),
            _ => None,
        })
        .collect()
}

fn item_completed(text: &str) -> BackendEv {
    BackendEv::Notification(Box::new(ServerNotification::ItemCompleted(
        ItemCompletedNotification {
            item: ThreadItem::untimed(ThreadItemPayload::Plan {
                id: "plan-1".into(),
                text: text.into(),
            }),
            thread_id: T.into(),
            turn_id: "turn-1".into(),
            content_token_count: None,
            completed_at_ms: 0,
        },
    )))
}

fn plan_delta(delta: &str) -> BackendEv {
    BackendEv::Notification(Box::new(ServerNotification::PlanDelta(
        PlanDeltaNotification {
            thread_id: T.into(),
            turn_id: "turn-1".into(),
            item_id: "plan-1".into(),
            delta: delta.into(),
        },
    )))
}

/// A Plan-mode turn that proposed a plan and is still running.
fn planning() -> CodexApp {
    let mut app = ready();
    chat_intent(&mut app, chat::ThreadIntent::SetMode(ModeKind::Plan));
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, plan_delta("- Step 1\n"));
    notification(&mut app, item_completed("- Step 1\n"));
    app
}

fn notes(app: &CodexApp) -> Vec<NoteKey> {
    thread(app)
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => Some(note.key.clone()),
            _ => None,
        })
        .collect()
}

/// The user's switch changes the thread's mode at once and tells the server.
#[test]
fn a_switch_changes_the_mode_at_once_and_tells_the_server() {
    let mut app = ready();
    assert_eq!(mode(&app), ModeKind::Default);
    let fx = chat_intent(&mut app, chat::ThreadIntent::SetMode(ModeKind::Plan));
    assert_eq!(mode(&app), ModeKind::Plan);
    assert_eq!(mode_requests(&fx), vec![ModeKind::Plan]);
}

/// Shift+Tab goes Default -> Plan -> Default, only while no turn runs.
#[test]
fn the_cycle_key_flips_the_mode_only_while_idle() {
    let mut app = ready();
    chat_intent(&mut app, chat::ThreadIntent::CycleMode);
    assert_eq!(mode(&app), ModeKind::Plan);
    chat_intent(&mut app, chat::ThreadIntent::CycleMode);
    assert_eq!(mode(&app), ModeKind::Default);

    notification(&mut app, turn_started(T, "turn-1"));
    let fx = chat_intent(&mut app, chat::ThreadIntent::CycleMode);
    assert_eq!(mode(&app), ModeKind::Default, "a running turn keeps its mode");
    assert!(mode_requests(&fx).is_empty());
}

/// A turn that completes in Plan mode with a proposed plan raises the question.
#[test]
fn a_plan_turn_that_completes_asks_to_implement() {
    let mut app = planning();
    assert!(!prompt_open(&app), "not before the turn ends");
    notification(&mut app, turn_completed(T, "turn-1"));
    assert!(prompt_open(&app));
}

/// No question without a plan, in another mode, after a failed turn, or while
/// a message waits to be sent.
#[test]
fn no_question_without_a_plan_a_plan_mode_or_a_completed_turn() {
    // no plan item
    let mut app = ready();
    chat_intent(&mut app, chat::ThreadIntent::SetMode(ModeKind::Plan));
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, turn_completed(T, "turn-1"));
    assert!(!prompt_open(&app), "no plan, no question");

    // Default mode
    let mut app = ready();
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, item_completed("- Step 1\n"));
    notification(&mut app, turn_completed(T, "turn-1"));
    assert!(!prompt_open(&app), "Default mode, no question");

    // a failed turn
    let mut app = planning();
    notification(&mut app, turn_failed(T, "turn-1"));
    assert!(!prompt_open(&app), "a failed turn, no question");

    // a message waits behind the turn
    let mut app = planning();
    chat_intent(
        &mut app,
        chat::ThreadIntent::Queue(UserMessage::text("next")),
    );
    notification(&mut app, turn_completed(T, "turn-1"));
    assert!(!prompt_open(&app), "a queued message goes first");
}

/// While the plan streams, Enter queues the text instead of steering the turn.
#[test]
fn a_message_sent_while_the_plan_streams_waits_in_the_queue() {
    let mut app = ready();
    chat_intent(&mut app, chat::ThreadIntent::SetMode(ModeKind::Plan));
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, plan_delta("- Step"));
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            message: UserMessage::text("later"),
        }),
    );
    let queue: Vec<_> = thread(&app).queue().iter().map(|m| m.text.clone()).collect();
    assert_eq!(queue, vec!["later".to_string()]);
    assert!(started(&fx).is_empty());
    assert!(
        !fx.commands()
            .iter()
            .any(|c| matches!(c, Command::Screen { command: ScreenCommand::Chat(chat::Command::Steer { .. }), .. })),
        "no steer"
    );
}

/// Yes: the thread goes on in Default and `Implement the plan.` starts the work.
#[test]
fn yes_switches_to_default_and_sends_the_implement_message() {
    let mut app = planning();
    notification(&mut app, turn_completed(T, "turn-1"));
    let fx = chat_intent(&mut app, chat::ThreadIntent::ImplementPlan);
    assert_eq!(mode(&app), ModeKind::Default);
    assert_eq!(mode_requests(&fx), vec![ModeKind::Default]);
    assert_eq!(started(&fx), vec![IMPLEMENT_MESSAGE.to_string()]);
}

/// A running turn refuses the switch, says so, and sends nothing.
#[test]
fn yes_while_a_turn_runs_is_refused() {
    let mut app = planning();
    let fx = chat_intent(&mut app, chat::ThreadIntent::ImplementPlan);
    assert_eq!(mode(&app), ModeKind::Plan);
    assert!(mode_requests(&fx).is_empty());
    assert!(started(&fx).is_empty());
    assert!(notes(&app).contains(&NoteKey::ModeBusy));
}
