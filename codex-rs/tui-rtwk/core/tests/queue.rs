#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The queue key's lines and their drain at the turn's end (oracle
//! `chatwidget/input_flow.rs` `maybe_send_next_queued_input`, `slash_dispatch.rs`
//! `submit_queued_slash_prompt`, `input_submission.rs` `submit_queued_shell_prompt`): a message
//! starts the next turn, a command is handed to the screen and the queue waits for what it
//! reports, a shell line runs, a bare `!` says how to use it.

use codex_app_server_protocol::{
    ClientRequest, ErrorNotification, ItemCompletedNotification, RequestId, ServerNotification,
    ThreadItem, ThreadItemPayload, TurnError, TurnStatus, UserInput,
};
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::{
    AppEv, AppFx, CodexApp, Command, Ingress, Intent, apply_app, apply_app_from,
};
use codex_tui_core::commands::{
    DispatchCx, Outcome, QueueDrain, SlashCommand, Target, dispatch_queued,
};
use codex_tui_core::features::{FeatureIntent, settings};
use codex_tui_core::fixtures::builders::{
    history_turn, turn_completed, turn_started, user_item_with_receipt,
};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::rpc::{CloudSubmission, Reply};
use codex_tui_core::screens::{
    ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, ScreenThreadIntent, agents, chat, select,
};
use codex_tui_core::threads::ThreadAsk;
use codex_tui_core::threads::queue::{Dispatched, QueueEvent, QueueOperation};
use codex_tui_core::threads::{Item, Notice, Queued, Ran, ThreadModel, UserMessage};
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

fn report_due(app: &CodexApp, outcome: Ran) -> chat::ThreadIntent {
    let thread = thread(app);
    let due = thread.due_command().expect("queued command is due");
    chat::ThreadIntent::QueueEvent(due.report(outcome))
}

fn ran(app: &mut CodexApp, outcome: Ran) -> AppFx {
    let report = report_due(app, outcome);
    intent(app, report)
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
}

#[test]
fn a_command_that_lets_the_queue_go_on_sends_the_next_message() {
    let mut app = running();
    tab(&mut app, "/status", false);
    tab(&mut app, "hello after status", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    assert_eq!(queued(&app), ["hello after status"], "waits for the screen");
    let fx = ran(&mut app, dispatched(Some(SlashCommand::Status), false));
    assert_eq!(started_texts(&fx), ["hello after status"]);
    assert!(thread(&app).due_command().is_none());
}

#[test]
fn a_reopened_thread_rejects_the_previous_threads_queue_report() {
    let mut app = ready();
    tab(&mut app, "/status", false);
    let stale = report_due(&app, dispatched(Some(SlashCommand::Status), false));
    let old = issued_operation(&app);
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(
            select::Intent::Confirmed(select::Confirm::Archive { running: false }),
        ))),
    );
    let archive = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::Archive { .. }),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("archive was sent");
    run(
        &mut app,
        AppEv::Reply(
            archive,
            Reply::thread(codex_tui_core::threads::ThreadReply::Done),
        ),
    );
    let id = ThreadId::new(T);
    assert!(app.thread(&id).is_none());
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(agents::Page {
            threads: vec![listed(T)],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    let fx = run(&mut app, AppEv::Intent(Intent::OpenThread(id)));
    let resume = resume_ticket(&fx).expect("same id reopened");
    run(
        &mut app,
        AppEv::Reply(
            resume,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(
                listed(T),
            )))),
        ),
    );
    tab(&mut app, "/status", false);
    let current = issued_operation(&app);
    assert_ne!(current, old);
    tab(&mut app, "after the new status", false);
    assert!(started_texts(&intent(&mut app, stale)).is_empty());
    assert_eq!(issued_operation(&app), current);
    let fx = ran(&mut app, dispatched(Some(SlashCommand::Status), false));
    assert_eq!(started_texts(&fx), ["after the new status"]);
}

#[test]
fn a_duplicate_acknowledgment_cannot_consume_the_next_command() {
    let mut app = running();
    tab(&mut app, "/status", false);
    tab(&mut app, "/status", false);
    tab(&mut app, "after both commands", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let first_ack = report_due(&app, dispatched(Some(SlashCommand::Status), false));
    assert!(started_texts(&intent(&mut app, first_ack.clone())).is_empty());
    let model = thread(&app);
    let second = model.due_command().expect("second queued command").clone();
    let repeated = intent(&mut app, first_ack);
    assert!(
        started_texts(&repeated).is_empty(),
        "the second command is still due"
    );
    assert_eq!(thread(&app).due_command(), Some(&second));
    let second_ack = chat::ThreadIntent::QueueEvent(
        second.report(dispatched(Some(SlashCommand::Status), false)),
    );
    assert_eq!(
        started_texts(&intent(&mut app, second_ack)),
        ["after both commands"]
    );
}

#[test]
fn a_menu_command_stops_the_queue_until_the_screen_resumes_it() {
    let mut app = running();
    tab(&mut app, "/model", false);
    tab(&mut app, "hello after selection", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let operation = issued_operation(&app);
    let fx = open_queued_picker(&mut app, operation.clone());
    assert!(started_texts(&fx).is_empty(), "the picker opened first");
    assert_eq!(queued(&app), ["hello after selection"]);
    // A generic nudge can reconsider admission but the picker still owns this operation.
    let fx = intent(&mut app, chat::ThreadIntent::QueueResume);
    assert!(started_texts(&fx).is_empty());
    let fx = intent(
        &mut app,
        chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(operation)),
    );
    assert_eq!(started_texts(&fx), ["hello after selection"]);
}

fn issued_operation(app: &CodexApp) -> QueueOperation {
    thread(app)
        .due_command()
        .expect("queued command is due")
        .operation()
        .clone()
}

fn open_queued_picker(app: &mut CodexApp, operation: QueueOperation) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(
            select::Intent::OpenModels(Box::new(select::ModelOpen {
                operation: Some(operation),
                current: select::Current::new("gpt-5.6"),
            })),
        ))),
    )
}

#[test]
fn only_the_matching_picker_close_releases_the_queue_in_each_event_order() {
    #[derive(Clone, Copy)]
    enum Event {
        Lag,
        Recover,
        Nudge,
    }
    use Event::{Lag, Nudge, Recover};
    for events in [
        [Lag, Recover, Nudge],
        [Lag, Nudge, Recover],
        [Recover, Lag, Nudge],
        [Recover, Nudge, Lag],
        [Nudge, Lag, Recover],
        [Nudge, Recover, Lag],
    ] {
        let mut app = running();
        tab(&mut app, "/model", false);
        tab(&mut app, "/model", false);
        tab(&mut app, "after both pickers", false);
        notify(&mut app, turn_completed(T, "turn-1"));
        let first = issued_operation(&app);
        let stale_ack = report_due(&app, dispatched(Some(SlashCommand::Model), false));
        assert!(started_texts(&open_queued_picker(&mut app, first.clone())).is_empty());
        assert!(
            thread(&app).due_command().is_none(),
            "owned picker is not issued again"
        );
        assert!(
            started_texts(&intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerOpened(first.clone()))
            ))
            .is_empty()
        );

        for event in events {
            let fx = match event {
                Lag => notify(&mut app, BackendEv::Lagged(1)),
                Recover => {
                    assert!(started_texts(&notify(&mut app, lost(T))).is_empty());
                    notify(&mut app, restored(T))
                }
                Nudge => intent(&mut app, chat::ThreadIntent::QueueResume),
            };
            assert!(
                started_texts(&fx).is_empty(),
                "the first picker still owns admission"
            );
        }
        assert!(started_texts(&intent(&mut app, stale_ack.clone())).is_empty());
        assert!(
            started_texts(&intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(first.clone()))
            ))
            .is_empty()
        );
        let second = issued_operation(&app);
        assert_ne!(first, second);
        assert!(started_texts(&open_queued_picker(&mut app, second.clone())).is_empty());
        assert!(
            started_texts(&intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(first))
            ))
            .is_empty()
        );
        assert!(started_texts(&intent(&mut app, stale_ack)).is_empty());
        assert_eq!(
            started_texts(&intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(second.clone()))
            )),
            ["after both pickers"]
        );
        assert!(
            started_texts(&intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerOpened(second.clone()))
            ))
            .is_empty()
        );
        assert!(
            started_texts(&intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(second))
            ))
            .is_empty()
        );
    }
}

#[test]
fn a_declined_queued_picker_does_not_leave_a_hold_or_a_show() {
    let mut app = running();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Select(select::Ev::Provider(
            select::Provider::Saving,
        ))),
    );
    tab(&mut app, "/model", false);
    tab(&mut app, "after refusal", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let operation = issued_operation(&app);
    let fx = open_queued_picker(&mut app, operation.clone());
    assert_eq!(started_texts(&fx), ["after refusal"]);
    assert!(app.screens().select.shows.pending().next().is_none());
    assert!(started_texts(&open_queued_picker(&mut app, operation)).is_empty());
}

#[test]
fn a_picker_closed_while_detached_releases_the_hold_when_the_writer_returns() {
    for opened in [false, true] {
        let mut app = running();
        tab(&mut app, "/model", false);
        tab(&mut app, "after reconnect", false);
        notify(&mut app, turn_completed(T, "turn-1"));
        let operation = issued_operation(&app);
        if opened {
            open_queued_picker(&mut app, operation.clone());
            intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerOpened(operation.clone())),
            );
        }
        run(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
        );
        let fx = if opened {
            intent(
                &mut app,
                chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(operation)),
            )
        } else {
            let declined = open_queued_picker(&mut app, operation);
            assert!(app.screens().select.shows.pending().next().is_none());
            declined
        };
        assert!(thread(&app).due_command().is_none());
        assert!(started_texts(&fx).is_empty());
        let history = reattach(&mut app);
        assert_eq!(
            started_texts(&reply_history(&mut app, history, None)),
            ["after reconnect"]
        );
    }
}

#[test]
fn a_picker_close_is_addressed_to_its_original_thread() {
    const B: &str = "thread-2";
    let mut app = running();
    tab(&mut app, "/model", false);
    tab(&mut app, "for original thread", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let original = issued_operation(&app);
    let stale_ack = report_due(&app, dispatched(Some(SlashCommand::Model), false));
    open_queued_picker(&mut app, original.clone());
    intent(
        &mut app,
        chat::ThreadIntent::QueueEvent(QueueEvent::PickerOpened(original.clone())),
    );

    open_listed_thread(&mut app, B);
    let on_second = |app: &mut CodexApp, intent: chat::ThreadIntent| {
        run(
            app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: ThreadId::new(B),
                intent: ScreenThreadIntent::Chat(intent),
            }),
        )
    };
    notify(&mut app, turn_started(B, "second-turn"));
    for text in ["/model", "for second thread"] {
        on_second(
            &mut app,
            chat::ThreadIntent::Queue(Queued::of(UserMessage::text(text), false)),
        );
    }
    notify(&mut app, turn_completed(B, "second-turn"));
    let second = issued_operation(&app);
    assert!(started_texts(&on_second(&mut app, stale_ack)).is_empty());
    assert_eq!(issued_operation(&app), second);
    open_queued_picker(&mut app, second.clone());
    assert!(
        started_texts(&on_second(
            &mut app,
            chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(original.clone()))
        ))
        .is_empty()
    );
    let fx = intent(
        &mut app,
        chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(original)),
    );
    assert_eq!(started_texts(&fx), ["for original thread"]);
    assert!(
        fx.commands()
            .iter()
            .any(|c| matches!(c, Command::StartTurn { thread, .. } if thread.as_str() == T))
    );
    let fx = on_second(
        &mut app,
        chat::ThreadIntent::QueueEvent(QueueEvent::PickerClosed(second)),
    );
    assert_eq!(started_texts(&fx), ["for second thread"]);
    assert!(
        fx.commands()
            .iter()
            .any(|c| matches!(c, Command::StartTurn { thread, .. } if thread.as_str() == B))
    );
}

fn open_listed_thread(app: &mut CodexApp, name: &str) {
    run(
        app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(agents::Page {
            threads: vec![listed(name)],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    let fx = run(app, AppEv::Intent(Intent::OpenThread(ThreadId::new(name))));
    let ticket = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command:
                    ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Resume { thread })),
                ..
            } if thread == name => Some(*ticket),
            _ => None,
        })
        .expect("the second thread is resumed");
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(
                listed(name),
            )))),
        ),
    );
    assert_eq!(thread(app).id().as_str(), name);
}

fn drain_to_status_line(app: &mut CodexApp, receiver: &str, turn: &str) -> QueueOperation {
    assert!(started_texts(&notify(app, turn_completed(receiver, turn))).is_empty());
    let due = app
        .thread(&ThreadId::new(receiver))
        .and_then(|model| model.due_command())
        .cloned()
        .expect("queued status editor");
    let operation = due.operation().clone();
    run(
        app,
        AppEv::Intent(Intent::Feature(
            settings::Intent::OpenQueuedStatusLine(operation.clone()).into(),
        )),
    );
    assert_eq!(
        app.wishes()
            .pending()
            .last()
            .and_then(|wish| wish.queue_operation.as_ref()),
        Some(&operation),
    );
    assert!(
        app.thread(&ThreadId::new(receiver))
            .unwrap()
            .due_command()
            .is_none()
    );
    operation
}

fn modal_receipt(app: &mut CodexApp, receiver: &str, event: QueueEvent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(receiver),
            intent: ScreenThreadIntent::Chat(chat::ThreadIntent::QueueEvent(event)),
        }),
    )
}

fn saved_status(operation: QueueOperation) -> QueueEvent {
    QueueEvent::ModalFinished {
        operation,
        accepted: Some(Box::new(FeatureIntent::from(settings::Intent::Request(
            settings::Edit::SetupSaved {
                kind: settings::SetupKind::StatusLine,
                items: vec!["model".to_owned()],
                use_theme_colors: false,
            },
        )))),
    }
}

fn config_writes(fx: &AppFx) -> usize {
    fx.commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                Command::Effect {
                    ask: Ask::ConfigWrite { .. },
                    ..
                }
            )
        })
        .count()
}

#[test]
fn an_unopened_modal_releases_its_queue_without_applying_a_selection() {
    let mut app = running();
    tab(&mut app, "/statusline", false);
    tab(&mut app, "after the failed opening", false);
    let operation = drain_to_status_line(&mut app, T, "turn-1");
    assert!(started_texts(&intent(&mut app, chat::ThreadIntent::QueueResume)).is_empty());

    let closed = modal_receipt(&mut app, T, saved_status(operation.clone()));
    assert_eq!(config_writes(&closed), 0);
    assert_eq!(started_texts(&closed), ["after the failed opening"]);
    assert!(
        modal_receipt(&mut app, T, saved_status(operation))
            .commands()
            .is_empty()
    );
}

#[test]
fn a_modal_result_only_settles_the_queue_of_its_original_thread() {
    const B: &str = "thread-2";
    let mut app = running();
    tab(&mut app, "/statusline", false);
    tab(&mut app, "for the original thread", false);
    let original = drain_to_status_line(&mut app, T, "turn-1");
    modal_receipt(&mut app, T, QueueEvent::ModalOpened(original.clone()));
    assert!(started_texts(&notify(&mut app, lost(T))).is_empty());
    assert!(started_texts(&notify(&mut app, restored(T))).is_empty());

    open_listed_thread(&mut app, B);
    notify(&mut app, turn_started(B, "second-turn"));
    for text in ["/statusline", "for the second thread"] {
        run(
            &mut app,
            AppEv::Intent(Intent::ThreadScreen {
                thread: ThreadId::new(B),
                intent: ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(Queued::of(
                    UserMessage::text(text),
                    false,
                ))),
            }),
        );
    }
    let second = drain_to_status_line(&mut app, B, "second-turn");
    modal_receipt(&mut app, B, QueueEvent::ModalOpened(second.clone()));

    let wrong_owner = modal_receipt(&mut app, B, saved_status(original.clone()));
    assert!(wrong_owner.commands().is_empty());
    let closed = modal_receipt(&mut app, T, saved_status(original.clone()));
    assert_eq!(config_writes(&closed), 0);
    assert_eq!(started_texts(&closed), ["for the original thread"]);
    assert!(closed.commands().iter().any(|command| {
        matches!(command, Command::StartTurn {thread, ..} if thread.as_str() == T)
    }));
    assert!(
        modal_receipt(&mut app, T, saved_status(original))
            .commands()
            .is_empty()
    );

    let accepted = modal_receipt(&mut app, B, saved_status(second.clone()));
    assert_eq!(config_writes(&accepted), 1);
    assert_eq!(started_texts(&accepted), ["for the second thread"]);
    assert!(accepted.commands().iter().any(|command| {
        matches!(command, Command::StartTurn {thread, ..} if thread.as_str() == B)
    }));
    assert!(
        modal_receipt(&mut app, B, saved_status(second))
            .commands()
            .is_empty()
    );
}

#[test]
fn returning_to_a_thread_does_not_revive_its_old_modal_choice() {
    let mut app = running();
    tab(&mut app, "/statusline", false);
    tab(&mut app, "after returning", false);
    let operation = drain_to_status_line(&mut app, T, "turn-1");
    modal_receipt(&mut app, T, QueueEvent::ModalOpened(operation.clone()));

    open_listed_thread(&mut app, "thread-2");
    run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
    );
    assert_eq!(app.session().active, Some(ThreadId::new(T)));
    assert!(
        modal_receipt(&mut app, T, QueueEvent::ModalOpened(operation.clone()))
            .commands()
            .is_empty()
    );

    let closed = modal_receipt(&mut app, T, saved_status(operation.clone()));
    assert_eq!(config_writes(&closed), 0);
    assert_eq!(started_texts(&closed), ["after returning"]);
    assert!(
        modal_receipt(&mut app, T, saved_status(operation))
            .commands()
            .is_empty()
    );
}

#[test]
fn a_modal_result_cannot_apply_its_choice_after_transport_replacement() {
    let mut app = running();
    tab(&mut app, "/statusline", false);
    tab(&mut app, "after reattach", false);
    let operation = drain_to_status_line(&mut app, T, "turn-1");
    modal_receipt(&mut app, T, QueueEvent::ModalOpened(operation.clone()));
    let epoch = app.epoch();
    app.ingest(Ingress::transport_replaced(
        BackendId::default(),
        epoch,
        epoch.next(),
    ))
    .expect("the replacement transport");
    assert_eq!(app.session().active, Some(ThreadId::new(T)));

    let closed = modal_receipt(&mut app, T, saved_status(operation.clone()));
    assert_eq!(config_writes(&closed), 0);
    assert!(started_texts(&closed).is_empty());
    let history = reattach(&mut app);
    assert_eq!(
        started_texts(&reply_history(&mut app, history, None)),
        ["after reattach"]
    );
    assert!(
        modal_receipt(&mut app, T, saved_status(operation))
            .commands()
            .is_empty()
    );
}

#[test]
fn a_continue_command_that_opened_a_popup_stops_all_the_same() {
    let mut app = running();
    tab(&mut app, "/rename", false);
    tab(&mut app, "hello after rename", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let fx = ran(&mut app, dispatched(Some(SlashCommand::Rename), true));
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
    let fx = ran(&mut app, dispatched(None, false));
    assert_eq!(started_texts(&fx), ["next"]);
}

#[test]
fn a_line_that_went_to_the_model_stops_the_queue() {
    let mut app = running();
    tab(&mut app, "/usr/bin/env", false);
    tab(&mut app, "next", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let fx = ran(&mut app, Ran::Submitted);
    assert!(started_texts(&fx).is_empty());
    assert_eq!(queued(&app), ["next"]);
}

#[test]
fn a_report_without_a_due_command_is_nothing() {
    let mut app = running();
    tab(&mut app, "/status", false);
    notify(&mut app, turn_completed(T, "turn-1"));
    let report = report_due(&app, dispatched(Some(SlashCommand::Status), false));
    intent(&mut app, report.clone());
    notify(&mut app, turn_started(T, "turn-2"));
    tab(&mut app, "next", false);
    let fx = intent(&mut app, report);
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
    assert!(matches!(thread(&app).queue()[0], Queued::Literal(..)));
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

/// The ticket of the compaction a `/compact` asked for.
fn compact_ticket(fx: &AppFx) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|c| match c {
            Command::Screen {
                ticket,
                command: ScreenCommand::Chat(chat::Command::Compact),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("the compaction was asked for")
}

/// A compaction asked for holds the thread as a running turn does, from the dispatch of
/// `/compact` (`is_task_running`): a message typed before its turn begins waits behind it.
#[test]
fn a_message_typed_after_compact_was_asked_for_waits_for_its_turn() {
    let mut app = ready();
    let ticket = compact_ticket(&intent(&mut app, chat::ThreadIntent::Compact));
    assert!(thread(&app).busy());
    // the server took the request, and no turn runs yet
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::screen(ScreenReply::Chat(chat::Reply::Ack))),
    );
    assert!(thread(&app).busy());
    let fx = submit(&mut app, "after compact");
    assert!(started_texts(&fx).is_empty(), "queued, not sent");
    assert_eq!(queued(&app), ["after compact"]);
    // its turn begins, and the end of it sends the message
    notify(&mut app, turn_started(T, "turn-2"));
    assert!(thread(&app).busy());
    let fx = notify(&mut app, turn_completed(T, "turn-2"));
    assert_eq!(started_texts(&fx), ["after compact"]);
}

/// A compaction nothing accepted does not hold the thread: the refusal and the lost connection let
/// it go.
#[test]
fn a_refused_compaction_lets_the_thread_go() {
    let mut app = ready();
    let ticket = compact_ticket(&intent(&mut app, chat::ThreadIntent::Compact));
    assert!(thread(&app).busy());
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::failed("compact failed")),
    );
    assert!(!thread(&app).busy());
    let fx = submit(&mut app, "after the refusal");
    assert_eq!(started_texts(&fx), ["after the refusal"]);
}

#[test]
fn a_lost_connection_lets_a_compaction_that_never_began_go() {
    let mut app = ready();
    intent(&mut app, chat::ThreadIntent::Compact);
    assert!(thread(&app).busy());
    run(&mut app, AppEv::Disconnected);
    assert!(!thread(&app).busy());
}

#[test]
fn a_command_waits_while_a_compaction_is_asked_for_and_drains_when_it_ends() {
    let mut app = ready();
    intent(&mut app, chat::ThreadIntent::Compact);
    // Tab on a command while the compaction is asked for: queued, not sent
    let fx = tab(&mut app, "/status", false);
    assert!(fx.commands().is_empty());
    assert_eq!(queued(&app), ["/status"]);
    notify(&mut app, turn_started(T, "turn-2"));
    notify(&mut app, turn_completed(T, "turn-2"));
    assert!(queued(&app).is_empty());
    assert!(thread(&app).due_command().is_some(), "handed to the screen");
}

// The lost stream of a thread (a Cloud backend's `ThreadStreamDisconnected` and
// `ThreadStreamReconnected`, oracle `app_server_events.rs`): the host forwards them as
// `BackendEv::StreamLost` and `StreamRestored`.

const LOST: &str = "Cloud stream ended. Reopen the conversation to reconnect.";

fn disconnected(thread: &str) -> Box<ErrorNotification> {
    Box::new(ErrorNotification {
        error: TurnError {
            message: LOST.to_owned(),
            codex_error_info: None,
            additional_details: None,
            misalignment: None,
        },
        will_retry: false,
        thread_id: thread.to_owned(),
        turn_id: String::new(),
    })
}

fn lost(thread: &str) -> BackendEv {
    BackendEv::StreamLost {
        thread: ThreadId::new(thread),
        error: disconnected(thread),
    }
}

fn restored(thread: &str) -> BackendEv {
    BackendEv::StreamRestored(ThreadId::new(thread))
}

/// The error rows of the thread, oldest first.
fn errors(app: &CodexApp) -> Vec<String> {
    thread(app)
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Error { message } => Some(message.clone()),
            _ => None,
        })
        .collect()
}

fn reconnects(fx: &AppFx) -> Vec<ThreadId> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask: codex_tui_core::app::effects::Ask::ReconnectIdleThread { thread, .. },
                ..
            } => Some(thread.clone()),
            _ => None,
        })
        .collect()
}

/// The stream is lost while the connection stands: what the user sends waits in the queue, steers
/// included, until the stream is back, and then goes out as the next turn.
#[test]
fn a_lost_stream_queues_a_submit_until_it_is_back() {
    let mut app = ready();
    notify(&mut app, lost(T));
    assert!(thread(&app).view().stream_lost());
    let fx = submit(&mut app, "hello");
    assert_eq!(started_texts(&fx), Vec::<String>::new());
    assert_eq!(queued(&app), ["hello"]);
    let fx = notify(&mut app, restored(T));
    assert!(!thread(&app).view().stream_lost());
    assert_eq!(started_texts(&fx), ["hello"]);
    assert_eq!(queued(&app), Vec::<String>::new());
}

/// An idle displayed thread of a Cloud backend asks the backend to attach it again and shows no
/// error line (`reconnect_idle_thread` accepted: the handler returns before the error).
#[test]
fn an_idle_lost_stream_asks_the_host_to_reconnect_and_shows_no_line() {
    let mut app = ready();
    let fx = notify(&mut app, lost(T));
    assert_eq!(reconnects(&fx), [ThreadId::new(T)]);
    assert!(thread(&app).view().stream_lost());
    assert_eq!(errors(&app), Vec::<String>::new());
}

/// While a user turn runs nothing is reconnected: the error is the line of the thread, which ends
/// the running turn (`on_error` finalizes it), and the queue waits for the stream.
#[test]
fn a_lost_stream_during_a_turn_shows_the_error_and_holds_the_queue() {
    let mut app = running();
    tab(&mut app, "later", false);
    let fx = notify(&mut app, lost(T));
    assert_eq!(reconnects(&fx), Vec::<ThreadId>::new());
    assert!(thread(&app).view().stream_lost());
    assert_eq!(errors(&app), [LOST]);
    assert!(!thread(&app).is_running(), "the error ended the turn");
    assert_eq!(
        started_texts(&fx),
        Vec::<String>::new(),
        "the queue does not send while the stream is lost"
    );
    let fx = notify(&mut app, restored(T));
    assert_eq!(started_texts(&fx), ["later"]);
}

/// A refused reconnect is the host's: it sends the error back as the notification the line is
/// made of, and the thread stays lost.
#[test]
fn a_refused_reconnect_comes_back_as_the_error_notification() {
    let mut app = ready();
    notify(&mut app, lost(T));
    notify(
        &mut app,
        BackendEv::Notification(Box::new(ServerNotification::Error(*disconnected(T)))),
    );
    assert_eq!(errors(&app), [LOST]);
    assert!(thread(&app).view().stream_lost());
}

/// A thread the user is not looking at is not marked: the error is its line (the oracle marks the
/// displayed thread only), and its restore changes nothing.
#[test]
fn a_lost_stream_of_a_thread_not_displayed_is_a_line_only() {
    let mut app = ready();
    let fx = notify(&mut app, lost("thread-2"));
    assert_eq!(reconnects(&fx), Vec::<ThreadId>::new());
    assert!(!thread(&app).view().stream_lost());
    let fx = notify(&mut app, restored("thread-2"));
    assert!(started_texts(&fx).is_empty());
}

/// A thread another backend owns is not this backend's to speak of.
#[test]
fn a_lost_stream_of_another_backend_is_not_heard() {
    let mut app = ready();
    let mut fx = AppFx::default();
    apply_app_from(&mut app, BackendId(1), AppEv::Backend(lost(T)), &mut fx);
    assert!(!thread(&app).view().stream_lost());
    assert_eq!(reconnects(&fx), Vec::<ThreadId>::new());
    assert_eq!(errors(&app), Vec::<String>::new());
}

/// The restore of a stream nobody lost is nothing (oracle: `thread_stream_disconnected` is set);
/// the restore of a lost one leaves the history it missed unread.
#[test]
fn a_restored_stream_marks_the_history_it_missed() {
    let mut app = ready();
    notify(&mut app, restored(T));
    assert!(!thread(&app).history_may_be_incomplete());
    notify(&mut app, lost(T));
    notify(&mut app, restored(T));
    assert!(thread(&app).history_may_be_incomplete());
    assert!(!thread(&app).view().stream_lost());
}

/// The connection itself going takes the thread's stream with it.
#[test]
fn a_lost_connection_clears_the_lost_stream() {
    let mut app = ready();
    notify(&mut app, lost(T));
    run(&mut app, AppEv::Disconnected);
    assert!(!thread(&app).view().stream_lost());
}

fn listed(id: &str) -> codex_app_server_protocol::Thread {
    serde_json::from_value(serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

/// The ticket of the resume `fx` asks for.
fn resume_ticket(fx: &AppFx) -> Option<Ticket> {
    fx.commands().iter().find_map(|command| match command {
        Command::Screen {
            ticket,
            command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Resume { thread })),
            ..
        } if thread == T => Some(*ticket),
        _ => None,
    })
}

/// "Reopen the conversation to reconnect" (the Cloud client's words): opening a held thread
/// whose stream was lost reads it from the server again (oracle `ignore_same_thread_resume`
/// lets the resume through), replaces what was held, clears the loss and sends what waited.
#[test]
fn opening_a_thread_whose_stream_was_lost_reads_it_again_and_sends_the_queue() {
    let mut app = ready();
    notify(&mut app, lost(T));
    submit(&mut app, "hello");
    assert_eq!(queued(&app), ["hello"]);
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
    );
    let ticket = resume_ticket(&fx).expect("the lost thread is resumed, not just shown");
    let fx = run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(
                listed(T),
            )))),
        ),
    );
    assert!(!thread(&app).view().stream_lost());
    assert!(!thread(&app).history_may_be_incomplete());
    assert_eq!(started_texts(&fx), ["hello"]);
    assert_eq!(queued(&app), Vec::<String>::new());
}

/// The same for a thread whose stream came back with its gap unread; a thread nothing was lost
/// in is only shown.
#[test]
fn opening_a_thread_with_a_gap_reads_it_again_and_a_sound_one_is_only_shown() {
    let mut app = ready();
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
    );
    assert!(resume_ticket(&fx).is_none());
    notify(&mut app, lost(T));
    notify(&mut app, restored(T));
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
    );
    let ticket = resume_ticket(&fx).expect("the gap is read");
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(
                listed(T),
            )))),
        ),
    );
    assert!(!thread(&app).history_may_be_incomplete());
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
    );
    assert!(resume_ticket(&fx).is_none());
}

fn submission_id(fx: &AppFx) -> (Ticket, String) {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::StartTurn {
                ticket, client_id, ..
            }
            | Command::Screen {
                ticket,
                command: ScreenCommand::Chat(chat::Command::Steer { client_id, .. }),
                ..
            } => {
                let wire =
                    codex_tui_core::protocol::client_request(command, RequestId::Integer(1), None);
                let wire_id = match wire {
                    Some(ClientRequest::TurnStart { params, .. }) => params.client_user_message_id,
                    Some(ClientRequest::TurnSteer { params, .. }) => params.client_user_message_id,
                    _ => panic!("submission should encode as turn/start or turn/steer"),
                };
                assert_eq!(wire_id.as_deref(), Some(client_id.as_str()));
                Some((*ticket, client_id.clone()))
            }
            _ => None,
        })
        .expect("a submitted start or steer")
}

fn history_ticket(fx: &AppFx) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command:
                    ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::History {
                        thread,
                        cursor: None,
                    })),
                ..
            } if thread == T => Some(*ticket),
            _ => None,
        })
        .expect("a fresh history read")
}

fn reply_history(app: &mut CodexApp, ticket: Ticket, receipt: Option<String>) -> AppFx {
    reply_history_with_status(app, ticket, receipt, TurnStatus::Completed)
}

fn reply_history_with_status(
    app: &mut CodexApp,
    ticket: Ticket,
    receipt: Option<String>,
    status: TurnStatus,
) -> AppFx {
    let mut turn = history_turn("turn-1", "first message", "ok");
    turn.status = status;
    if let ThreadItemPayload::UserMessage { client_id, .. } = &mut turn.items[0].payload {
        *client_id = receipt;
    } else {
        panic!("history should begin with the user's message");
    }
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::History {
                turns: vec![turn],
                next: None,
            })),
        ),
    )
}

fn reattach(app: &mut CodexApp) -> Ticket {
    let fx = run(
        app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected)),
    );
    resume_history(app, &fx)
}

fn resume_history(app: &mut CodexApp, fx: &AppFx) -> Ticket {
    let resume = resume_ticket(fx).expect("same thread resumes on reconnect");
    let fx = run(
        app,
        AppEv::Reply(
            resume,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(
                listed(T),
            )))),
        ),
    );
    assert!(started_texts(&fx).is_empty(), "await the latest page");
    history_ticket(&fx)
}

#[test]
fn a_live_receipt_before_transport_loss_leaves_only_the_unsent_follow_up() {
    let mut app = ready();
    let (_, client_id) = submission_id(&submit(&mut app, "first message"));
    submit_or_queue_follow_up(&mut app, true);
    notify(
        &mut app,
        user_item_with_receipt(T, "turn-1", "live", "first message", Some(client_id)),
    );
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
    );
    assert_eq!(queued(&app), ["follow-up"]);
    let history = reattach(&mut app);
    let fx = reply_history(&mut app, history, None);
    assert_eq!(started_texts(&fx), ["follow-up"]);
    assert!(thread(&app).queue().is_empty());
}

fn completed_user_item(
    item: &str,
    client_id: Option<&str>,
    parts: &[&str],
    image: Option<&str>,
) -> BackendEv {
    let content = parts
        .iter()
        .map(|text| UserInput::Text {
            text: (*text).to_owned(),
            text_elements: Vec::new(),
        })
        .chain(image.map(|path| UserInput::LocalImage {
            path: path.into(),
            detail: None,
        }))
        .collect();
    BackendEv::Notification(Box::new(ServerNotification::ItemCompleted(
        ItemCompletedNotification {
            item: ThreadItem::untimed(ThreadItemPayload::UserMessage {
                id: item.to_owned(),
                client_id: client_id.map(str::to_owned),
                content,
            }),
            thread_id: T.to_owned(),
            turn_id: "turn-1".to_owned(),
            content_token_count: None,
            completed_at_ms: 0,
        },
    )))
}

#[test]
fn a_legacy_live_completion_matches_only_the_oldest_steers_wire_content() {
    let mut app = running();
    submit(&mut app, "first message");
    submit(&mut app, "second message");
    let pending = |app: &CodexApp| {
        thread(app)
            .pending_steers()
            .iter()
            .map(|steer| steer.message().text.clone())
            .collect::<Vec<_>>()
    };
    for completion in [
        completed_user_item("other-id", Some("unrelated"), &["first message"], None),
        completed_user_item(
            "other-image-count",
            None,
            &["first message"],
            Some("/tmp/other.png"),
        ),
        completed_user_item("second", None, &["second message"], None),
    ] {
        notify(&mut app, completion);
        assert_eq!(pending(&app), ["first message", "second message"]);
    }
    notify(
        &mut app,
        completed_user_item("first", None, &["first ", "message"], None),
    );
    assert_eq!(pending(&app), ["second message"]);
    notify(
        &mut app,
        completed_user_item("second", None, &["second message"], None),
    );
    assert_eq!(
        pending(&app),
        ["second message"],
        "a duplicate completion cannot retire a different steer"
    );
    notify(
        &mut app,
        completed_user_item("second-new", None, &["second message"], None),
    );
    assert!(pending(&app).is_empty());
}

#[test]
fn a_legacy_completion_seen_during_recovery_cannot_retire_a_later_steer() {
    for before_reattach in [false, true] {
        let mut app = running();
        let (ticket, client_id) = submission_id(&submit(&mut app, "first message"));
        let legacy = || completed_user_item("earlier-completion", None, &["first message"], None);
        unconfirmed_reply(&mut app, ticket);
        if before_reattach {
            notify(&mut app, legacy());
        }
        run(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
        );
        let recovery = reattach(&mut app);
        if !before_reattach {
            notify(&mut app, legacy());
        }
        assert_eq!(queued(&app), ["first message"]);
        reply_history_with_status(&mut app, recovery, Some(client_id), TurnStatus::InProgress);
        assert!(queued(&app).is_empty());
        assert!(thread(&app).is_running());
        assert!(steered(&submit(&mut app, "first message")));
        notify(&mut app, legacy());
        assert_eq!(
            thread(&app).pending_steers().len(),
            1,
            "an already seen completion is stale"
        );
        notify(
            &mut app,
            completed_user_item("new-completion", None, &["first message"], None),
        );
        assert!(thread(&app).pending_steers().is_empty());
    }
}

#[test]
fn an_open_reuses_the_writer_until_recovery_history_fails_or_the_stream_is_lost() {
    for stream_lost in [false, true] {
        let mut app = ready();
        let (_, client_id) = submission_id(&submit(&mut app, "first message"));
        submit_or_queue_follow_up(&mut app, true);
        run(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
        );
        let previous_page = reattach(&mut app);
        let open = || AppEv::Intent(Intent::OpenThread(ThreadId::new(T)));
        assert!(
            run(&mut app, open()).commands().is_empty(),
            "the recovery page is in flight"
        );
        assert_eq!(queued(&app), ["first message", "follow-up"]);

        if stream_lost {
            notify(&mut app, lost(T));
        } else {
            run(
                &mut app,
                AppEv::Reply(previous_page, Reply::failed("history unavailable")),
            );
        }
        let fx = run(&mut app, open());
        let fresh_page = resume_history(&mut app, &fx);
        if stream_lost {
            let fx = reply_history(&mut app, previous_page, Some(client_id.clone()));
            assert!(
                started_texts(&fx).is_empty(),
                "the lost stream's history is stale"
            );
            assert_eq!(queued(&app), ["first message", "follow-up"]);
        }
        let fx = reply_history(&mut app, fresh_page, Some(client_id));
        assert_eq!(started_texts(&fx), ["follow-up"]);
        assert!(thread(&app).queue().is_empty());
    }
}

#[test]
fn same_thread_reattach_reconciles_only_an_exact_start_or_steer_receipt() {
    for pending_start in [false, true] {
        for exact_receipt in [None, Some(false), Some(true)] {
            assert_submission_recovery(pending_start, exact_receipt, false);
        }
    }
}

#[test]
fn a_transport_reply_before_disconnect_preserves_pending_starts_and_steers_once() {
    for pending_start in [false, true] {
        for exact_receipt in [None, Some(false), Some(true)] {
            assert_submission_recovery(pending_start, exact_receipt, true);
        }
    }
}

#[test]
fn a_cloud_submission_without_a_receipt_reopens_for_exact_history_confirmation() {
    for receipt in [
        CloudSubmission::Duplicate,
        CloudSubmission::Unconfirmed("Cloud mutation outcome unknown".to_owned()),
    ] {
        for matching in [false, true] {
            let mut app = ready();
            let (ticket, client_id) = submission_id(&submit(&mut app, "first message"));
            submit_or_queue_follow_up(&mut app, true);
            let fx = run(
                &mut app,
                AppEv::Reply(ticket, Reply::cloud_submission(receipt.clone())),
            );
            assert!(started_texts(&fx).is_empty());
            assert!(matches!(
                &thread(&app).queue()[0],
                Queued::Unconfirmed { client_id: held, .. } if held == &client_id
            ));

            let fx = run(
                &mut app,
                AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
            );
            let history = resume_history(&mut app, &fx);
            let fx = reply_history(&mut app, history, matching.then_some(client_id));
            if matching {
                assert_eq!(started_texts(&fx), ["follow-up"]);
                assert!(thread(&app).queue().is_empty());
            } else {
                assert!(started_texts(&fx).is_empty());
                assert_eq!(queued(&app), ["first message", "follow-up"]);
            }
        }
    }
}

#[test]
fn pre_gap_history_cannot_settle_an_early_transport_failure() {
    for pending_start in [true, false] {
        for stale_success in [true, false] {
            let mut app = ready();
            notify(&mut app, lost(T));
            let fx = run(
                &mut app,
                AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
            );
            let stale_history = resume_history(&mut app, &fx);

            if !pending_start {
                notify(&mut app, turn_started(T, "turn-1"));
            }
            let (ticket, client_id) = submission_id(&submit(&mut app, "first message"));
            submit_or_queue_follow_up(&mut app, pending_start);
            unconfirmed_reply(&mut app, ticket);

            let fx = if stale_success {
                reply_history(&mut app, stale_history, None)
            } else {
                run(
                    &mut app,
                    AppEv::Reply(stale_history, Reply::failed("transport closed")),
                )
            };
            assert!(started_texts(&fx).is_empty());
            assert_eq!(queued(&app), ["first message", "follow-up"]);
            assert!(
                !thread(&app).transcript().iter().any(|(_, item)| {
                    matches!(item, Item::Notice(Notice::SubmissionUnconfirmed(_)))
                }),
                "a read sent before the gap cannot settle recovery"
            );

            run(
                &mut app,
                AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
            );
            let history = reattach(&mut app);
            let fx = reply_history(&mut app, history, Some(client_id));
            assert_eq!(started_texts(&fx), ["follow-up"]);
            assert!(thread(&app).queue().is_empty());
        }
    }
}

fn unconfirmed_reply(app: &mut CodexApp, ticket: Ticket) -> AppFx {
    run(
        app,
        AppEv::Reply(ticket, Reply::submission_unconfirmed("transport closed")),
    )
}

#[test]
fn an_out_of_order_steer_failure_preserves_the_order_at_disconnect() {
    let mut app = running();
    let (first, _) = submission_id(&submit(&mut app, "first"));
    let (second, _) = submission_id(&submit(&mut app, "second"));
    tab(&mut app, "follow-up", false);
    unconfirmed_reply(&mut app, second);
    assert_eq!(queued(&app), ["second", "follow-up"]);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
    );
    unconfirmed_reply(&mut app, first);
    unconfirmed_reply(&mut app, second);
    assert_eq!(queued(&app), ["first", "second", "follow-up"]);
}

fn assert_submission_recovery(pending_start: bool, exact_receipt: Option<bool>, error_first: bool) {
    let mut app = if pending_start { ready() } else { running() };
    let (ticket, client_id) = submission_id(&submit(&mut app, "first message"));
    submit_or_queue_follow_up(&mut app, pending_start);
    if error_first {
        let fx = unconfirmed_reply(&mut app, ticket);
        assert!(started_texts(&fx).is_empty(), "delivery is still unknown");
        assert!(thread(&app).restore().is_none(), "keep the draft editable");
        assert_eq!(queued(&app), ["first message", "follow-up"]);
    }
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting)),
    );
    assert_eq!(queued(&app), ["first message", "follow-up"]);
    let history = reattach(&mut app);
    assert!(matches!(
        &thread(&app).queue()[0],
        Queued::Unconfirmed { .. }
    ));
    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::failed("no active turn to steer")),
    );
    assert!(started_texts(&fx).is_empty(), "retire the old request");
    assert_eq!(queued(&app), ["first message", "follow-up"]);
    let receipt = exact_receipt.map(|exact| {
        if exact {
            client_id.clone()
        } else {
            "unrelated".into()
        }
    });
    let fx = reply_history(&mut app, history, receipt);
    let notice = thread(&app).transcript().iter().any(|(_, item)| {
        matches!(
            item, Item::Notice(Notice::SubmissionUnconfirmed(text)) if text == "first message"
        )
    });
    if exact_receipt == Some(true) {
        assert_eq!(started_texts(&fx), ["follow-up"]);
        assert!(thread(&app).queue().is_empty());
        assert!(!notice);
    } else {
        assert!(started_texts(&fx).is_empty());
        assert_eq!(queued(&app), ["first message", "follow-up"]);
        assert!(notice);
        let fx = notify(
            &mut app,
            completed_user_item("legacy-live", None, &["first message"], None),
        );
        assert!(started_texts(&fx).is_empty());
        assert_eq!(queued(&app), ["first message", "follow-up"]);
        let fx = notify(
            &mut app,
            user_item_with_receipt(
                T,
                "turn-1",
                "late-exact-receipt",
                "first message",
                Some(client_id),
            ),
        );
        assert_eq!(
            started_texts(&fx),
            ["follow-up"],
            "live receipt releases the queue"
        );
        assert!(thread(&app).queue().is_empty());
    }
}

fn submit_or_queue_follow_up(app: &mut CodexApp, pending_start: bool) {
    let fx = if pending_start {
        submit(app, "follow-up")
    } else {
        tab(app, "follow-up", false)
    };
    assert!(started_texts(&fx).is_empty());
    assert!(!steered(&fx));
    assert_eq!(queued(app), ["follow-up"]);
}

#[test]
fn a_live_stream_restore_reads_exact_receipts_before_sending_follow_ups() {
    let mut app = running();
    let (_, client_id) = submission_id(&submit(&mut app, "first message"));
    tab(&mut app, "follow-up", false);
    notify(&mut app, lost(T));
    assert_eq!(queued(&app), ["first message", "follow-up"]);
    let fx = notify(&mut app, restored(T));
    assert!(started_texts(&fx).is_empty());
    let fx = reply_history(&mut app, history_ticket(&fx), Some(client_id));
    assert_eq!(started_texts(&fx), ["follow-up"]);
}
