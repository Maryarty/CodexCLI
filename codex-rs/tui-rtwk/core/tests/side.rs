#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Side conversations (oracle `app/side.rs`): `/side` forks the thread in use into an ephemeral
//! thread that starts empty on the screen, links it to its parent, injects the policy and the
//! boundary, sends the `/btw` prompt once they are in, shows a label while it is shown, refuses a
//! second one, reuses the parent's own, and drops the side conversation the user returned from.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::side::{
    Footer, Intent as SideIntent, Note as SideNote, Reply as SideReply,
};
use codex_tui_core::features::{FeatureCommand, FeatureIntent, FeatureNote, FeatureReply};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, ScreenReply, agents};
use codex_tui_core::threads::{Item as Entry, Note, NoteKey, ThreadAsk};
use pretty_assertions::assert_eq;

const MAIN: &str = "thread-1";
const SIDE: &str = "side-1";

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
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(MAIN))),
    );
    app
}

fn feature(app: &mut CodexApp, intent: SideIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::Side(intent))),
    )
}

fn start(app: &mut CodexApp, source: &str, prompt: Option<&str>) -> AppFx {
    feature(
        app,
        SideIntent::Start {
            source: Some(ThreadId::new(source)),
            prompt: prompt.map(str::to_owned),
        },
    )
}

fn forks(fx: &AppFx) -> Vec<(Ticket, ThreadAsk)> {
    fx.commands()
        .iter()
        .filter_map(|c| match c {
            Command::Screen {
                ticket,
                command:
                    ScreenCommand::Agents(agents::Command::Thread(ask @ ThreadAsk::ForkAt { .. })),
                ..
            } => Some((*ticket, ask.clone())),
            _ => None,
        })
        .collect()
}

fn forked_thread(id: &str) -> codex_app_server_protocol::Thread {
    serde_json::from_value(serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": MAIN, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "", "ephemeral": true, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": null,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

/// `/side` in the main thread, answered with the fork `SIDE`.
fn opened(prompt: Option<&str>) -> (CodexApp, AppFx) {
    let mut app = ready();
    let fx = start(&mut app, MAIN, prompt);
    let [(ticket, _)] = forks(&fx).try_into().expect("one fork");
    let fx = run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Forked(Box::new(
                forked_thread(SIDE),
            )))),
        ),
    );
    (app, fx)
}

fn boundary_ticket(fx: &AppFx) -> Option<Ticket> {
    fx.commands().iter().find_map(|c| match c {
        Command::Feature {
            ticket,
            thread,
            command: FeatureCommand::Side(_),
            ..
        } if thread.as_str() == SIDE => Some(*ticket),
        _ => None,
    })
}

fn notes(app: &CodexApp, thread: &str) -> Vec<SideNote> {
    app.thread(&ThreadId::new(thread))
        .expect("the thread")
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Entry::Note(Note {
                key: NoteKey::Feature(FeatureNote::Side(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn side_forks_an_ephemeral_thread_with_the_side_policy() {
    let mut app = ready();
    let fx = start(&mut app, MAIN, None);
    let [(_, ask)] = forks(&fx).try_into().expect("one fork");
    let ThreadAsk::ForkAt {
        thread, temporary, ..
    } = &ask
    else {
        unreachable!()
    };
    assert_eq!(thread.as_str(), MAIN);
    assert!(
        temporary
            .as_deref()
            .is_some_and(|text| text.contains("side conversation"))
    );
    // nothing is shown before the fork answers
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
}

#[test]
fn the_fork_is_shown_linked_and_labelled() {
    let (app, fx) = opened(None);
    assert_eq!(app.session().active, Some(ThreadId::new(SIDE)));
    let state = &app.features().side;
    assert!(state.is_side(&ThreadId::new(SIDE)));
    assert_eq!(
        state
            .link(&ThreadId::new(SIDE))
            .map(|link| link.parent.clone()),
        Some(ThreadId::new(MAIN))
    );
    assert!(app.screens().chat.flags.side_conversation_active);
    let Some(Footer::Side {
        parent_is_main,
        status,
        ..
    }) = app.screens().chat.facts.side.clone()
    else {
        panic!("the footer says it is a side conversation");
    };
    assert_eq!((parent_is_main, status), (true, None));
    // the policy and the boundary go in as items
    assert!(boundary_ticket(&fx).is_some(), "{:?}", fx.commands());
}

#[test]
fn the_forked_history_is_not_read_for_the_screen() {
    let (_, fx) = opened(None);
    let reads = fx.commands().iter().filter(|c| {
        matches!(c, Command::Screen { command: ScreenCommand::Agents(agents::Command::Thread(
            ThreadAsk::History { thread, .. } | ThreadAsk::Turns { thread })), .. } if thread == SIDE)
    });
    assert_eq!(reads.count(), 0);
}

#[test]
fn a_btw_prompt_goes_once_the_boundary_is_in() {
    let (mut app, fx) = opened(Some("what is this file?"));
    let ticket = boundary_ticket(&fx).expect("the boundary ask");
    let started = |fx: &AppFx| {
        fx.commands()
            .iter()
            .filter_map(|c| match c {
                Command::StartTurn {
                    message, thread, ..
                } => Some((thread.as_str().to_owned(), message.text.clone())),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(started(&fx), vec![], "not before the boundary");
    let fx = run(
        &mut app,
        AppEv::Reply(ticket, Reply::feature(FeatureReply::Side(SideReply))),
    );
    assert_eq!(
        started(&fx),
        vec![(SIDE.to_owned(), "what is this file?".to_owned())]
    );
}

#[test]
fn a_boundary_that_failed_keeps_the_prompt_back_and_says_so() {
    let (mut app, fx) = opened(Some("hello"));
    let ticket = boundary_ticket(&fx).expect("the boundary ask");
    let fx = run(&mut app, AppEv::Reply(ticket, Reply::failed("denied")));
    assert!(
        fx.commands()
            .iter()
            .all(|c| !matches!(c, Command::StartTurn { .. }))
    );
    assert_eq!(
        notes(&app, SIDE),
        vec![SideNote::PrepareFailed {
            thread: ThreadId::new(SIDE),
            error: "denied".to_owned()
        }]
    );
}

#[test]
fn a_second_side_is_refused_in_a_side_conversation() {
    let (mut app, _) = opened(None);
    let fx = start(&mut app, SIDE, Some("again"));
    assert!(forks(&fx).is_empty());
    assert_eq!(notes(&app, SIDE), vec![SideNote::AlreadyOpen]);
}

#[test]
fn side_in_the_parent_shows_its_own_side_conversation_again() {
    let (mut app, _) = opened(None);
    run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(MAIN))),
    );
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
    assert!(!app.screens().chat.flags.side_conversation_active);
    assert!(matches!(
        app.screens().chat.facts.side,
        Some(Footer::Parent { .. })
    ));
    let fx = start(&mut app, MAIN, None);
    assert!(forks(&fx).is_empty(), "no second fork");
    assert_eq!(app.session().active, Some(ThreadId::new(SIDE)));
}

#[test]
fn returning_drops_the_side_conversation() {
    let (mut app, _) = opened(None);
    // while it is shown nothing is dropped
    let fx = feature(
        &mut app,
        SideIntent::Discard {
            child: ThreadId::new(SIDE),
        },
    );
    assert!(fx.commands().is_empty());
    assert!(app.features().side.is_side(&ThreadId::new(SIDE)));
    run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(MAIN))),
    );
    let fx = feature(
        &mut app,
        SideIntent::Discard {
            child: ThreadId::new(SIDE),
        },
    );
    let unsubscribed = fx.commands().iter().any(|c| {
        matches!(c, Command::Screen { command: ScreenCommand::Agents(agents::Command::Thread(
            ThreadAsk::Unsubscribe { thread })), .. } if thread == SIDE)
    });
    assert!(unsubscribed, "{:?}", fx.commands());
    assert!(!app.features().side.is_side(&ThreadId::new(SIDE)));
    assert!(app.thread(&ThreadId::new(SIDE)).is_none());
    assert_eq!(app.screens().chat.facts.side, None);
}

#[test]
fn a_fork_that_failed_before_the_first_message_says_to_send_one() {
    let mut app = ready();
    let fx = start(&mut app, MAIN, None);
    let [(ticket, _)] = forks(&fx).try_into().expect("one fork");
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::failed("thread/fork failed: no rollout found for thread id thread-1"),
        ),
    );
    assert_eq!(notes(&app, MAIN), vec![SideNote::NoStartedConversation]);
}

#[test]
fn side_before_the_session_has_started_says_so() {
    let mut app = CodexApp::new();
    let fx = feature(
        &mut app,
        SideIntent::Start {
            source: None,
            prompt: None,
        },
    );
    assert!(forks(&fx).is_empty());
    assert!(!app.features().side.is_side(&ThreadId::new(MAIN)));
}

/// The parent's status is the slice's own reading of the stream (`Feature::project` and
/// `project_request`, delivered to `on_event`): a notification of the parent and a request that
/// waits for the user both reach the side conversation's footer, and a request is only observed.
#[test]
fn the_parents_status_follows_the_stream_through_the_slices_events() {
    use codex_app_server_protocol::ServerRequest;
    use codex_tui_core::app::BackendEv;
    use codex_tui_core::features::side::ParentStatus;
    let status = |app: &CodexApp| match app.screens().chat.facts.side.clone() {
        Some(Footer::Side { status, .. }) => status,
        other => panic!("a side footer: {other:?}"),
    };
    let (mut app, _) = opened(None);
    assert_eq!(status(&app), None);
    run(
        &mut app,
        AppEv::Backend(codex_tui_core::fixtures::builders::turn_completed(
            MAIN, "turn-1",
        )),
    );
    assert_eq!(status(&app), Some(ParentStatus::Finished));
    let request: ServerRequest = serde_json::from_value(serde_json::json!({
        "method": "item/commandExecution/requestApproval",
        "id": 7,
        "params": {
            "threadId": MAIN, "turnId": "turn-2", "itemId": "call-7", "startedAtMs": 0,
            "approvalId": "call-7", "environmentId": null, "command": "echo hi", "cwd": "/tmp"
        }
    }))
    .expect("a request");
    run(
        &mut app,
        AppEv::Backend(BackendEv::Request(Box::new(request))),
    );
    assert_eq!(status(&app), Some(ParentStatus::NeedsApproval));
    // another thread's notification is not the parent's
    run(
        &mut app,
        AppEv::Backend(codex_tui_core::fixtures::builders::turn_started(
            "elsewhere",
            "turn-9",
        )),
    );
    assert_eq!(status(&app), Some(ParentStatus::NeedsApproval));
}

/// A fork asked for while another conversation is being made is refused to the slice
/// (`Refusal::Busy`): the prompt goes back to the composer it was typed in, and no second fork
/// is sent.
#[test]
fn a_second_fork_while_one_is_being_made_gives_the_prompt_back() {
    let mut app = ready();
    let first = start(&mut app, MAIN, None);
    assert_eq!(forks(&first).len(), 1);
    let second = start(&mut app, MAIN, Some("quick question"));
    assert!(forks(&second).is_empty(), "{:?}", second.commands());
    let restore = app
        .thread(&ThreadId::new(MAIN))
        .expect("the thread")
        .restore()
        .cloned()
        .expect("the prompt went back");
    assert_eq!(
        restore
            .messages()
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>(),
        ["quick question"]
    );
}
