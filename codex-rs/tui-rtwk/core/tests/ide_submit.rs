//! An IDE read belongs to the accepted submission and its original transport.

#![allow(clippy::expect_used)]

use codex_app_server_protocol::{
    ClientRequest, ItemStartedNotification, RequestId, ServerNotification, ThreadItem,
    ThreadItemPayload, UserInput,
};
use codex_tui_core::app::effects::{Ask, Done, IdeFailure, IdePrompt, IdeReading};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Ingress, Intent, apply_app};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::fixtures::builders::{turn_completed, turn_started};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::{BackendId, ThreadId};
use codex_tui_core::protocol::client_request;
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, ScreenThreadIntent, chat};
use codex_tui_core::threads::{Item, UserMessage};

const T: &str = "thread-1";
const FIRST_SELECTION: &str = "value selected in editor.rs";
const NEW_SELECTION: &str = "new selection in editor.rs";

fn ready() -> CodexApp {
    build(concat!(
        r#"{"step":"seed","seed":"session","thread":"thread-1"}"#,
        "\n",
        r#"{"step":"screen","dir":"chat","ev":{"status":{"cwd":"/workspace"}}}"#,
    ))
    .expect("plays")
    .app
}

fn run(app: &mut CodexApp, event: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, event, &mut fx);
    app.assert_consistent();
    fx
}

fn ide(app: &mut CodexApp, args: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(T)),
            command: SlashCommand::Ide,
            intent: SessionIntent::Feature { args: args.into() },
        }))),
    )
}

fn enable(app: &mut CodexApp) {
    let fx = ide(app, "on");
    let request = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Effect {
                request,
                ask: Ask::IdeContext { .. },
                ..
            } => Some(*request),
            _ => None,
        })
        .expect("the opt-in status probe");
    run(
        app,
        AppEv::Effect {
            request,
            done: Done::IdeContext(Ok(IdeReading { has_context: true })),
        },
    );
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

fn fresh_read(app: &CodexApp, fx: &AppFx) -> u64 {
    assert!(submissions(fx).is_empty(), "{:?}", fx.commands());
    let reads: Vec<_> = fx
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Effect {
                request,
                ask: Ask::IdePrompt { thread, cwd },
                ..
            } => {
                assert_eq!(thread.as_str(), T);
                assert_eq!(cwd, "/workspace");
                assert!(app.dispatch_route(command, None).is_some());
                Some(*request)
            }
            _ => None,
        })
        .collect();
    let [request] = reads.as_slice() else {
        panic!("expected one owned fresh read, got {:?}", fx.commands());
    };
    *request
}

fn result(request: u64, text: &str) -> AppEv {
    AppEv::Effect {
        request,
        done: Done::IdePrompt(Ok(Some(IdePrompt::new(text.into())))),
    }
}

fn submissions(fx: &AppFx) -> Vec<&Command> {
    fx.commands()
        .iter()
        .filter(|command| {
            matches!(
                command,
                Command::StartTurn { .. }
                    | Command::Screen {
                        command: ScreenCommand::Chat(chat::Command::Steer { .. }),
                        ..
                    }
            )
        })
        .collect()
}

fn start(fx: &AppFx, original: &str) -> (Vec<UserInput>, String) {
    let sends = submissions(fx);
    let [command @ Command::StartTurn { message, .. }] = sends.as_slice() else {
        panic!("expected one start, got {:?}", fx.commands());
    };
    assert_eq!(message, &UserMessage::text(original));
    let Some(ClientRequest::TurnStart { params, .. }) =
        client_request(command, RequestId::Integer(1), None)
    else {
        panic!("expected turn/start on wire");
    };
    (
        params.input,
        params.client_user_message_id.expect("same user admission"),
    )
}

fn text(input: &[UserInput]) -> &str {
    let [UserInput::Text { text, .. }] = input else {
        panic!("expected one text item, got {input:?}");
    };
    text
}

#[test]
fn only_an_opted_in_start_reads_context_and_the_live_echo_displays_original_text() {
    let mut no_consent = ready();
    let fx = submit(&mut no_consent, "plain");
    assert_eq!(text(&start(&fx, "plain").0), "plain");
    assert!(!fx.commands().iter().any(|command| matches!(
        command,
        Command::Effect {
            ask: Ask::IdePrompt { .. },
            ..
        }
    )));

    let mut app = ready();
    enable(&mut app);
    let original = "user input\n## My request for Codex:\ntyped literal header";
    let fx = submit(&mut app, original);
    let request = fresh_read(&app, &fx);
    let foreign = BackendId(4);
    let old_connector = app.epoch();
    assert!(
        app.ingest(Ingress::of(
            foreign,
            old_connector,
            result(request, "secret from another backend")
        ))
        .is_none()
    );
    assert!(
        app.ingest(Ingress::new(
            old_connector,
            AppEv::Effect {
                request,
                done: Done::IdeContext(Ok(IdeReading { has_context: false })),
            }
        ))
        .is_none()
    );
    let ready = run(&mut app, result(request, FIRST_SELECTION));
    let (wire, id) = start(&ready, original);
    assert!(text(&wire).contains(FIRST_SELECTION));
    assert!(text(&wire).ends_with(original));
    assert!(
        app.ingest(Ingress::new(old_connector, result(request, NEW_SELECTION)))
            .is_none()
    );
    run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
    run(
        &mut app,
        AppEv::Backend(BackendEv::Notification(Box::new(
            ServerNotification::ItemStarted(ItemStartedNotification {
                item: ThreadItem::untimed(ThreadItemPayload::UserMessage {
                    id: "echo".into(),
                    client_id: Some(id),
                    content: wire,
                }),
                thread_id: T.into(),
                turn_id: "turn-1".into(),
                started_at_ms: 0,
            }),
        ))),
    );
    assert!(
        app.active_thread()
            .transcript()
            .iter()
            .any(|(_, item)| matches!(item, Item::User(prompt) if prompt.text == original))
    );
}

#[test]
fn off_releases_the_original_once_and_retires_the_private_read() {
    let mut app = ready();
    enable(&mut app);
    let pending = submit(&mut app, "kept original");
    let request = fresh_read(&app, &pending);
    let fx = ide(&mut app, "off");
    assert_eq!(text(&start(&fx, "kept original").0), "kept original");
    let epoch = app.epoch();
    assert!(
        app.ingest(Ingress::new(epoch, result(request, FIRST_SELECTION)))
            .is_none()
    );
    let read = pending
        .commands()
        .iter()
        .find(|command| {
            matches!(
                command,
                Command::Effect {
                    ask: Ask::IdePrompt { .. },
                    ..
                }
            )
        })
        .expect("the retired request");
    assert!(app.dispatch_route(read, None).is_none());
}

#[test]
fn the_deadline_releases_the_original_without_waiting_for_the_host() {
    let mut app = ready();
    enable(&mut app);
    let fx = submit(&mut app, "timed out original");
    let request = fresh_read(&app, &fx);
    let wake = fx.wake().expect("active host read schedules a wake").at();
    assert_eq!(wake.millis(), app.now().millis() + 5_000);
    let before = app.now().plus_millis(4_999);
    let before = run(&mut app, AppEv::Clock(before));
    assert!(submissions(&before).is_empty());
    let fx = run(&mut app, AppEv::Clock(wake));
    assert_eq!(
        text(&start(&fx, "timed out original").0),
        "timed out original"
    );
    let epoch = app.epoch();
    assert!(
        app.ingest(Ingress::new(epoch, result(request, FIRST_SELECTION)))
            .is_none()
    );
}

#[test]
fn pending_steers_keep_send_order_and_only_attach_their_own_results() {
    let mut app = ready();
    enable(&mut app);
    run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
    let first = submit(&mut app, "first");
    let first = fresh_read(&app, &first);
    let second = submit(&mut app, "second");
    let second = fresh_read(&app, &second);
    let fx = run(&mut app, result(second, NEW_SELECTION));
    assert!(submissions(&fx).is_empty());
    let fx = run(&mut app, result(first, FIRST_SELECTION));
    let sends = submissions(&fx);
    assert_eq!(sends.len(), 2);
    for (command, (original, selection)) in sends
        .into_iter()
        .zip([("first", FIRST_SELECTION), ("second", NEW_SELECTION)])
    {
        let Command::Screen {
            command: ScreenCommand::Chat(chat::Command::Steer { message, .. }),
            ..
        } = command
        else {
            panic!("expected a steer, got {command:?}");
        };
        assert_eq!(message, &UserMessage::text(original));
        let Some(ClientRequest::TurnSteer { params, .. }) =
            client_request(command, RequestId::Integer(1), None)
        else {
            panic!("expected turn/steer on wire");
        };
        assert_eq!(params.expected_turn_id, "turn-1");
        assert!(text(&params.input).contains(selection));
        assert!(text(&params.input).ends_with(original));
    }
}

#[test]
fn queued_input_reads_at_drain_and_a_failed_read_preserves_its_original_message() {
    let mut app = ready();
    enable(&mut app);
    run(&mut app, AppEv::Backend(turn_started(T, "turn-1")));
    let fx = run(
        &mut app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(T),
            intent: ScreenThreadIntent::Chat(chat::ThreadIntent::Queue(
                UserMessage::text("queued until next turn").into(),
            )),
        }),
    );
    assert!(!fx.commands().iter().any(|command| matches!(
        command,
        Command::Effect {
            ask: Ask::IdePrompt { .. },
            ..
        }
    )));
    assert!(submissions(&fx).is_empty());
    let fx = run(&mut app, AppEv::Backend(turn_completed(T, "turn-1")));
    let request = fresh_read(&app, &fx);
    let fx = run(
        &mut app,
        AppEv::Effect {
            request,
            done: Done::IdePrompt(Err(IdeFailure {
                hint: "extension failed".into(),
                skip_hint: "read failed".into(),
            })),
        },
    );
    assert_eq!(
        text(&start(&fx, "queued until next turn").0),
        "queued until next turn"
    );
}

#[test]
fn physical_replacement_returns_the_unsent_original_without_dispatching_it() {
    let mut app = ready();
    enable(&mut app);
    let fx = submit(&mut app, "requires explicit retry");
    let request = fresh_read(&app, &fx);
    let backend = BackendId::default();
    let epoch = app.epoch();
    let fx = app
        .ingest(Ingress::transport_replaced(backend, epoch, epoch.next()))
        .expect("the current connector installed the next transport");
    assert!(submissions(&fx).is_empty());
    assert_eq!(
        app.active_thread()
            .restore()
            .expect("the unsent message")
            .messages(),
        [UserMessage::text("requires explicit retry")],
    );
    assert!(app.active_thread().queue().is_empty());
    assert!(
        app.ingest(Ingress::transport_event(
            backend,
            epoch,
            epoch,
            result(request, FIRST_SELECTION),
        ))
        .is_none()
    );
}
