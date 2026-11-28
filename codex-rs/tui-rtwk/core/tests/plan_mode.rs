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
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::fixtures::builders::{turn_completed, turn_failed, turn_started};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::chat::plan_mode::IMPLEMENT_MESSAGE;
use codex_tui_core::screens::select::{self, collaboration};
use codex_tui_core::screens::{
    ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, ScreenThreadIntent, chat,
};
use codex_tui_core::threads::{Item, ModeNote, NoteKey, ThreadModel, UserMessage};

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

/// The server's presets as `collaborationMode/list` answers them (`builtin_collaboration_mode_presets`).
fn presets() -> Vec<collaboration::Mask> {
    collaboration::masks(serde_json::json!({
        "data": [
            {"name": "Plan", "mode": "plan", "model": null, "reasoning_effort": "medium"},
            {"name": "Default", "mode": "default", "model": null, "reasoning_effort": null},
        ]
    }))
    .expect("the presets")
}

/// The ticket of the `collaborationMode/list` in `fx`.
fn catalog_ticket(fx: &AppFx) -> Option<codex_tui_core::ids::Ticket> {
    fx.commands().iter().find_map(|c| match c {
        Command::Screen {
            ticket,
            command: ScreenCommand::Select(select::Command::Collaboration),
            ..
        } => Some(*ticket),
        _ => None,
    })
}

fn answer_catalog(
    app: &mut CodexApp,
    ticket: codex_tui_core::ids::Ticket,
    masks: Vec<collaboration::Mask>,
) {
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Select(select::Reply::Collaboration(masks))),
        ),
    );
}

/// The app with its thread started, as after `Connected` and the reply, and the server's
/// presets read.
fn ready_with(masks: Vec<collaboration::Mask>) -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let catalog = catalog_ticket(&fx).expect("the catalog is asked at connect");
    answer_catalog(&mut app, catalog, masks);
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

fn ready() -> CodexApp {
    ready_with(presets())
}

/// `/plan` or `/plan <prompt>`, typed in the thread.
fn plan_command(app: &mut CodexApp, prompt: Option<&str>) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Plan,
            intent: SessionIntent::Plan {
                prompt: prompt.map(str::to_owned),
            },
        }))),
    )
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
    plan_command(&mut app, None);
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
    let fx = plan_command(&mut app, None);
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
    assert_eq!(
        mode(&app),
        ModeKind::Default,
        "a running turn keeps its mode"
    );
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
    plan_command(&mut app, None);
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
        chat::ThreadIntent::Queue(UserMessage::text("next").into()),
    );
    notification(&mut app, turn_completed(T, "turn-1"));
    assert!(!prompt_open(&app), "a queued message goes first");
}

/// While the plan streams, Enter queues the text instead of steering the turn.
#[test]
fn a_message_sent_while_the_plan_streams_waits_in_the_queue() {
    let mut app = ready();
    plan_command(&mut app, None);
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, plan_delta("- Step"));
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: Some(ThreadId::new(T)),
            message: UserMessage::text("later"),
        }),
    );
    let queue: Vec<_> = thread(&app)
        .queue()
        .iter()
        .map(|m| m.text.clone())
        .collect();
    assert_eq!(queue, vec!["later".to_string()]);
    assert!(started(&fx).is_empty());
    assert!(
        !fx.commands().iter().any(|c| matches!(
            c,
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Steer { .. }),
                ..
            }
        )),
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

#[test]
fn a_plan_answer_for_another_thread_cannot_apply_to_the_visible_plan() {
    use codex_tui_core::screens::chat::plan_mode::Answer;
    use codex_tui_core::screens::select::Intent as SelectIntent;
    use codex_tui_core::screens::ScreenIntent;

    let mut app = planning();
    notification(&mut app, turn_completed(T, "turn-1"));
    for answer in [Answer::Implement, Answer::ImplementFresh] {
        let fx = run(
            &mut app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Select(SelectIntent::PlanAnswer {
                thread: ThreadId::new("previous-thread"),
                answer,
            }))),
        );
        assert!(started(&fx).is_empty());
        assert!(
            fx.commands()
                .iter()
                .all(|command| !matches!(command, Command::StartThread { .. }))
        );
        assert_eq!(mode(&app), ModeKind::Plan);
    }
}

/// The row that clears the context starts a new thread, hands it the plan, and the prompt the
/// server prepared starts its work.
#[test]
fn clearing_the_context_hands_the_plan_to_a_fresh_thread() {
    use codex_tui_core::screens::chat::plan_mode::Answer;
    use codex_tui_core::screens::select::Intent as SelectIntent;
    use codex_tui_core::screens::{ScreenIntent, ScreenReply};

    let mut app = planning();
    notification(&mut app, turn_completed(T, "turn-1"));
    let select = |app: &mut CodexApp, intent| {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
        )
    };
    let fx = select(
        &mut app,
        SelectIntent::PlanAnswer {
            thread: ThreadId::new(T),
            answer: Answer::ImplementFresh,
        },
    );
    let ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a new thread is started");

    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-2"))),
    );
    let (ticket, thread, markdown) = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::Screen {
                ticket,
                thread,
                command: ScreenCommand::Chat(chat::Command::ApprovePlan(markdown)),
                ..
            } => Some((*ticket, thread.clone(), markdown.clone())),
            _ => None,
        })
        .expect("the fresh thread is handed the plan");
    assert_eq!(thread, ThreadId::new("thread-2"));
    assert_eq!(markdown, "- Step 1\n");

    let fx = run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Chat(chat::Reply::Prepared(
                "Implement this".to_string(),
            ))),
        ),
    );
    assert_eq!(started(&fx), vec!["Implement this".to_string()]);
    assert_eq!(
        app.active_thread().screen_slots().chat.mode(),
        ModeKind::Default
    );
}

// ---- the server's catalog (`collaboration_modes.rs`, `apply_plan_slash_command`)

fn mode_notes(app: &CodexApp) -> Vec<ModeNote> {
    notes(app)
        .into_iter()
        .filter_map(|key| match key {
            NoteKey::Mode(note) => Some(note),
            _ => None,
        })
        .collect()
}

fn restored(app: &CodexApp) -> Vec<String> {
    thread(app)
        .restore()
        .map(|restore| {
            restore
                .messages()
                .iter()
                .map(|message| message.text.clone())
                .collect()
        })
        .unwrap_or_default()
}

/// The catalog is asked once, at connect, and the app keeps what the server answered; a
/// failed read leaves it empty (discovery is optional).
#[test]
fn the_catalog_is_read_at_connect_and_a_failed_read_leaves_it_empty() {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    assert!(
        app.collaboration_modes().is_empty(),
        "nothing before the answer"
    );
    let ticket = catalog_ticket(&fx).expect("asked at connect");
    answer_catalog(&mut app, ticket, presets());
    let kinds: Vec<_> = app
        .collaboration_modes()
        .iter()
        .map(|mask| mask.mode)
        .collect();
    assert_eq!(kinds, vec![Some(ModeKind::Plan), Some(ModeKind::Default)]);
    // the list clears the prompts: the server supplies its own
    assert!(
        app.collaboration_modes()
            .iter()
            .all(|mask| mask.developer_instructions == Some(None))
    );

    let mut failed = CodexApp::new();
    let fx = run(&mut failed, AppEv::Connected);
    let ticket = catalog_ticket(&fx).expect("asked at connect");
    run(
        &mut failed,
        AppEv::Reply(ticket, Reply::failed("unknown method")),
    );
    assert!(failed.collaboration_modes().is_empty());
}

/// The choice of a mask: the visible presets only, Default first, Plan by kind, and the cycle
/// goes on in list order and wraps.
#[test]
fn the_masks_are_chosen_from_the_visible_presets_in_list_order() {
    let mut masks = presets();
    assert_eq!(
        collaboration::default_mask(&masks).and_then(|mask| mask.mode),
        Some(ModeKind::Default)
    );
    assert_eq!(
        collaboration::plan_mask(&masks).and_then(|mask| mask.mode),
        Some(ModeKind::Plan)
    );
    let after = |masks: &[collaboration::Mask], current| {
        collaboration::next_mask(masks, current).and_then(|mask| mask.mode)
    };
    assert_eq!(after(&masks, Some(ModeKind::Plan)), Some(ModeKind::Default));
    assert_eq!(after(&masks, Some(ModeKind::Default)), Some(ModeKind::Plan));
    // a mode the list does not hold starts the list over
    masks.truncate(1);
    assert_eq!(after(&masks, Some(ModeKind::Default)), Some(ModeKind::Plan));
    assert_eq!(collaboration::default_mode_mask(&masks), None);
    // with no Default preset the first one starts a thread
    assert_eq!(
        collaboration::default_mask(&masks).and_then(|mask| mask.mode),
        Some(ModeKind::Plan)
    );
    masks.clear();
    assert_eq!(collaboration::next_mask(&masks, None), None);
    assert_eq!(collaboration::plan_mask(&masks), None);
}

/// The mode the server is told is the base settings with the mask over them: the preset's
/// effort wins, the model stays, the instructions are the server's own.
#[test]
fn the_mask_lies_over_the_base_settings() {
    let mut app = ready();
    notification(
        &mut app,
        BackendEv::Notification(Box::new(
            codex_app_server_protocol::ServerNotification::ThreadSettingsUpdated(
                codex_app_server_protocol::ThreadSettingsUpdatedNotification {
                    thread_id: T.into(),
                    thread_settings: serde_json::from_value(serde_json::json!({
                        "cwd": "/work", "approvalPolicy": "on-request", "approvalsReviewer": "user",
                        "sandboxPolicy": {"type": "dangerFullAccess"}, "activePermissionProfile": null,
                        "model": "gpt-5.2", "modelProvider": "openai", "serviceTier": null,
                        "effort": "low", "summary": null, "personality": null,
                        "collaborationMode": {"mode": "default", "settings": {
                            "model": "gpt-5.2", "reasoning_effort": "low", "developer_instructions": null}}
                    }))
                    .expect("settings"),
                },
            ),
        )),
    );
    let fx = plan_command(&mut app, None);
    let sent: Vec<_> = fx
        .commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                command: ScreenCommand::Chat(chat::Command::Mode(mode)),
                ..
            } => Some(mode.clone()),
            _ => None,
        })
        .collect();
    let [plan] = sent.as_slice() else {
        panic!("one mode request: {:?}", fx.commands());
    };
    assert_eq!(plan.mode, ModeKind::Plan);
    assert_eq!(plan.settings.model, "gpt-5.2");
    assert_eq!(
        plan.settings
            .reasoning_effort
            .as_ref()
            .map(codex_protocol::openai_models::ReasoningEffort::as_str),
        Some("medium")
    );
    assert_eq!(plan.settings.developer_instructions, None);
    // the effort changed with the mode: the line says so
    assert_eq!(
        mode_notes(&app),
        vec![ModeNote::ModelChanged(collaboration::ModelChange {
            model: "gpt-5.2".into(),
            effort: Some("medium".into()),
            mode: ModeKind::Plan,
        })]
    );
    // and back: the Default preset keeps the base's effort, which comes back
    let fx = chat_intent(&mut app, chat::ThreadIntent::CycleMode);
    let [back] = mode_requests(&fx)[..] else {
        panic!("one mode request");
    };
    assert_eq!(back, ModeKind::Default);
    assert_eq!(
        mode_notes(&app).last(),
        Some(&ModeNote::ModelChanged(collaboration::ModelChange {
            model: "gpt-5.2".into(),
            effort: Some("low".into()),
            mode: ModeKind::Default,
        }))
    );
}

/// No Plan preset: `Plan mode unavailable right now.`, nothing is asked of the server, the task
/// keeps its mode, and `/plan <text>` gives the typed line back to the composer.
#[test]
fn an_empty_catalog_keeps_the_mode_and_the_typed_plan_input() {
    let mut app = ready_with(Vec::new());
    let fx = plan_command(&mut app, None);
    assert_eq!(mode(&app), ModeKind::Default);
    assert!(mode_requests(&fx).is_empty());
    assert_eq!(mode_notes(&app), vec![ModeNote::PlanUnavailable]);
    assert!(restored(&app).is_empty());

    let fx = plan_command(&mut app, Some("keep my draft"));
    assert_eq!(mode(&app), ModeKind::Default);
    assert!(mode_requests(&fx).is_empty());
    assert!(started(&fx).is_empty(), "the text is not sent");
    assert_eq!(
        mode_notes(&app),
        vec![ModeNote::PlanUnavailable, ModeNote::PlanUnavailable]
    );
    assert_eq!(restored(&app), vec!["/plan keep my draft".to_string()]);

    // the cycle key has nothing to go to either
    let fx = chat_intent(&mut app, chat::ThreadIntent::CycleMode);
    assert!(mode_requests(&fx).is_empty());
    assert_eq!(mode(&app), ModeKind::Default);
}

/// With the modes off the note says so, with its hint, and the prompt goes back as typed.
#[test]
fn modes_off_say_so_and_keep_the_typed_input() {
    use codex_tui_core::commands::CommandFlags;
    let mut app = ready();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Flags(CommandFlags {
            collaboration_modes_enabled: false,
            ..CommandFlags::default()
        }))),
    );
    let fx = plan_command(&mut app, Some("do it"));
    assert_eq!(mode(&app), ModeKind::Default);
    assert!(mode_requests(&fx).is_empty());
    assert_eq!(mode_notes(&app), vec![ModeNote::Disabled]);
    assert_eq!(restored(&app), vec!["/plan do it".to_string()]);
}

/// `/plan <text>` switches and then sends the text as the first message of the Plan thread.
#[test]
fn a_plan_with_text_switches_then_sends_it() {
    let mut app = ready();
    let fx = plan_command(&mut app, Some("do it"));
    assert_eq!(mode(&app), ModeKind::Plan);
    assert_eq!(mode_requests(&fx), vec![ModeKind::Plan]);
    assert_eq!(started(&fx), vec!["do it".to_string()]);
}

/// The prompt's Yes needs a Default preset: with none the row is disabled and nothing happens.
#[test]
fn yes_without_a_default_preset_does_nothing() {
    let plan_only = vec![presets().remove(0)];
    let mut app = ready_with(plan_only);
    plan_command(&mut app, None);
    notification(&mut app, turn_started(T, "turn-1"));
    notification(&mut app, plan_delta("- Step 1\n"));
    notification(&mut app, item_completed("- Step 1\n"));
    notification(&mut app, turn_completed(T, "turn-1"));
    let fx = chat_intent(&mut app, chat::ThreadIntent::ImplementPlan);
    assert_eq!(mode(&app), ModeKind::Plan);
    assert!(mode_requests(&fx).is_empty());
    assert!(started(&fx).is_empty());
}
