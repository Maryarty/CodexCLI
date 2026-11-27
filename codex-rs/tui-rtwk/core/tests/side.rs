#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Side conversations (oracle `app/side.rs`): `/side` forks the thread in use into an ephemeral
//! thread that starts empty on the screen, links it to its parent, injects the policy and the
//! boundary, sends the `/btw` prompt once they are in, shows a label while it is shown, refuses a
//! second one, reuses the parent's own, and drops the side conversation the user returned from.

use codex_app_server_protocol::{ClientRequest, RequestId, Thread, ThreadHistoryMode};
use codex_tui_core::app::facts::TuiSettings;
use codex_tui_core::app::{
    AppEv, AppFx, CodexApp, Command, Ingress, Intent, LocalSideStorage, apply_app,
};
use codex_tui_core::commands::{SessionIntent, SlashCommand};
use codex_tui_core::features::side::{
    Footer, Intent as SideIntent, Note as SideNote, Reply as SideReply, Storage,
};
use codex_tui_core::features::{FeatureCommand, FeatureIntent, FeatureNote, FeatureReply};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::protocol::client_request;
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, ScreenReply, agents, chat};
use codex_tui_core::threads::{Item as Entry, Note, NoteKey, ThreadAsk, ThreadReply};
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

fn slash(app: &mut CodexApp, command: SlashCommand, args: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Session {
            thread: Some(ThreadId::new(MAIN)),
            command,
            intent: SessionIntent::Feature {
                args: args.to_owned(),
            },
        }))),
    )
}

fn enable_workspace(app: &mut CodexApp, local: Option<LocalSideStorage>) {
    run(
        app,
        AppEv::Tui(TuiSettings {
            workspace_enabled: true,
            transcript_owned: true,
            ..TuiSettings::default()
        }),
    );
    if let Some(local) = local {
        app.ingest(Ingress::local_side_storage(app.epoch(), app.epoch(), local))
            .expect("current local transport");
    }
}

fn thread_ask(fx: &AppFx, expected: impl Fn(&ThreadAsk) -> bool) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::Thread(ask)),
                ..
            } if expected(ask) => Some(*ticket),
            _ => None,
        })
        .expect("expected thread ask")
}

fn reply_fork(app: &mut CodexApp, ticket: Ticket, thread: Thread) -> AppFx {
    run(
        app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Forked(Box::new(thread)))),
        ),
    )
}

fn wire(fx: &AppFx, ticket: Ticket) -> ClientRequest {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket: command_ticket,
                ..
            }
            | Command::Feature {
                ticket: command_ticket,
                ..
            } if *command_ticket == ticket => client_request(command, RequestId::Integer(1), None),
            _ => None,
        })
        .expect("request wire")
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
    opened_with_fork(prompt, forked_thread(SIDE))
}

fn opened_with_fork(
    prompt: Option<&str>,
    fork: codex_app_server_protocol::Thread,
) -> (CodexApp, AppFx) {
    let mut app = ready();
    let fx = start(&mut app, MAIN, prompt);
    let [(ticket, _)] = forks(&fx).try_into().expect("one fork");
    let fx = run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Forked(Box::new(fork)))),
        ),
    );
    (app, fx)
}

fn boundary_ticket(fx: &AppFx) -> Option<Ticket> {
    boundary_ticket_for(fx, SIDE)
}

fn boundary_ticket_for(fx: &AppFx, child: &str) -> Option<Ticket> {
    fx.commands().iter().find_map(|c| match c {
        Command::Feature {
            ticket,
            thread,
            command: FeatureCommand::Side(_),
            ..
        } if thread.as_str() == child => Some(*ticket),
        _ => None,
    })
}

#[test]
fn closing_the_current_workspace_tab_returns_to_the_most_recent_sibling() {
    let mut app = ready();
    enable_workspace(&mut app, None);
    let parent = ThreadId::new(MAIN);
    let oldest = ThreadId::new("oldest-side");
    let middle = ThreadId::new("middle-side");
    let newest = ThreadId::new("newest-side");
    for child in [&oldest, &middle, &newest] {
        run(&mut app, AppEv::Intent(Intent::OpenThread(parent.clone())));
        let request = slash(&mut app, SlashCommand::Side, "");
        let fork = thread_ask(&request, |ask| matches!(ask, ThreadAsk::ForkAt { .. }));
        let preparing = reply_fork(&mut app, fork, forked_thread(child.as_str()));
        let boundary = boundary_ticket_for(&preparing, child.as_str()).unwrap();
        run(
            &mut app,
            AppEv::Reply(boundary, Reply::feature(FeatureReply::Side(SideReply))),
        );
        assert_eq!(app.session().active.as_ref(), Some(child));
    }
    for child in [&oldest, &newest] {
        feature(
            &mut app,
            SideIntent::Select {
                parent: parent.clone(),
                child: child.clone(),
            },
        );
    }
    let stale = feature(
        &mut app,
        SideIntent::CloseTab {
            parent: ThreadId::new("other-parent"),
            child: newest.clone(),
        },
    );
    assert!(stale.commands().is_empty());
    assert_eq!(app.session().active.as_ref(), Some(&newest));

    let closed = feature(
        &mut app,
        SideIntent::CloseTab {
            parent: parent.clone(),
            child: newest.clone(),
        },
    );
    thread_ask(
        &closed,
        |ask| matches!(ask, ThreadAsk::Unsubscribe { thread } if thread == newest.as_str()),
    );
    assert_eq!(app.session().active.as_ref(), Some(&oldest));
    assert!(app.thread(&newest).is_none());
    assert!(app.thread(&middle).is_some());
    assert_eq!(
        app.features().side.children_of(&parent),
        &[oldest.clone(), middle.clone()]
    );

    feature(&mut app, SideIntent::Return { child: oldest });
    assert_eq!(app.session().active.as_ref(), Some(&parent));
    assert_eq!(app.features().side.children_of(&parent), &[middle]);
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
fn an_interrupted_turn_of_the_side_conversation_leaves_no_notice() {
    use codex_tui_core::fixtures::builders::turn_started;
    use codex_tui_core::threads::Notice;

    let (mut app, _) = opened(None);
    // the parent keeps the notice, the side conversation does not
    for (thread, notices) in [(MAIN, 1), (SIDE, 0)] {
        run(&mut app, AppEv::Backend(turn_started(thread, "turn-1")));
        run(&mut app, AppEv::Backend(interrupted(thread, "turn-1")));
        let held = app.thread(&ThreadId::new(thread)).expect("held");
        let count = held
            .transcript()
            .iter()
            .filter(|(_, item)| matches!(item, Entry::Notice(Notice::Interrupted)))
            .count();
        assert_eq!(count, notices, "{thread}");
    }
}

/// `turn/completed` of an interrupted turn.
fn interrupted(thread: &str, turn: &str) -> codex_tui_core::app::BackendEv {
    use codex_app_server_protocol::{
        ServerNotification, Turn, TurnCompletedNotification, TurnItemsView, TurnStatus,
    };
    codex_tui_core::app::BackendEv::Notification(Box::new(ServerNotification::TurnCompleted(
        TurnCompletedNotification {
            thread_id: thread.to_string(),
            turn: Turn {
                id: turn.to_string(),
                root_turn_id: None,
                items: Vec::new(),
                items_view: TurnItemsView::NotLoaded,
                status: TurnStatus::Interrupted,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    )))
}

#[test]
fn the_forked_history_is_not_read_for_the_screen() {
    let mut fork = forked_thread(SIDE);
    fork.turns = vec![
        serde_json::from_value(serde_json::json!({
            "id": "parent-interrupted", "itemsView": "full", "status": "interrupted",
            "items": [{"type": "userMessage", "id": "parent-message",
                "content": [{"type": "text", "text": "Parent prompt", "text_elements": []}]}]
        }))
        .expect("inherited interrupted turn"),
    ];
    let (app, fx) = opened_with_fork(None, fork);
    let reads = fx.commands().iter().filter(|c| {
        matches!(c, Command::Screen { command: ScreenCommand::Agents(agents::Command::Thread(
            ThreadAsk::History { thread, .. } | ThreadAsk::Turns { thread })), .. } if thread == SIDE)
    });
    assert_eq!(reads.count(), 0);
    let side = app.thread(&ThreadId::new(SIDE)).expect("held side thread");
    assert!(
        side.transcript()
            .iter()
            .all(|(key, _)| key.turn.as_str() != "parent-interrupted")
    );
    assert!(side.replayed_submissions().entries().is_empty());
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
fn workspace_side_is_fresh_and_btw_uses_the_child_selected_for_its_parent() {
    let mut app = ready();
    let parent = ThreadId::new(MAIN);
    let first = ThreadId::new("side-z");
    let second = ThreadId::new("side-a");
    enable_workspace(&mut app, None);
    for child in [&first, &second] {
        let fx = slash(&mut app, SlashCommand::Side, "");
        let [(ticket, _)] = forks(&fx).try_into().expect("one distinct fork for /side");
        let fx = run(
            &mut app,
            AppEv::Reply(
                ticket,
                Reply::screen(ScreenReply::Agents(agents::Reply::Forked(Box::new(
                    forked_thread(child.as_str()),
                )))),
            ),
        );
        let prepared = fx
            .commands()
            .iter()
            .find_map(|command| match command {
                Command::Feature {
                    ticket,
                    thread,
                    command: FeatureCommand::Side(_),
                    ..
                } if thread == child => Some(*ticket),
                _ => None,
            })
            .expect("boundary for this child");
        run(
            &mut app,
            AppEv::Reply(prepared, Reply::feature(FeatureReply::Side(SideReply))),
        );
        assert_eq!(app.session().active.as_ref(), Some(child));
        run(&mut app, AppEv::Intent(Intent::OpenThread(parent.clone())));
    }
    assert_eq!(
        app.features().side.children_of(&parent),
        &[first.clone(), second.clone()]
    );
    assert_eq!(app.features().side.side_of(&parent), Some(&second));

    feature(
        &mut app,
        SideIntent::Select {
            parent: ThreadId::new("wrong-parent"),
            child: first.clone(),
        },
    );
    assert_eq!(app.session().active.as_ref(), Some(&parent));
    feature(
        &mut app,
        SideIntent::Select {
            parent: parent.clone(),
            child: first.clone(),
        },
    );
    assert_eq!(app.session().active.as_ref(), Some(&first));
    run(&mut app, AppEv::Intent(Intent::OpenThread(parent.clone())));
    let fx = slash(&mut app, SlashCommand::Btw, "selected child?");
    assert!(forks(&fx).is_empty());
    assert!(fx.commands().iter().any(|command| matches!(command, Command::StartTurn { thread, message, .. } if thread == &first && message.text == "selected child?")));
    feature(
        &mut app,
        SideIntent::Return {
            child: first.clone(),
        },
    );
    assert_eq!(app.session().active.as_ref(), Some(&parent));
    assert_eq!(
        app.features().side.children_of(&parent),
        std::slice::from_ref(&second)
    );
    assert_eq!(app.features().side.side_of(&parent), Some(&second));
    assert!(app.thread(&first).is_none());
    assert!(app.thread(&second).is_some());
}

#[test]
fn navigating_away_and_back_parks_the_late_opening_prompt_for_the_exact_child() {
    let (mut app, legacy_boundary) = opened(None);
    run(
        &mut app,
        AppEv::Reply(
            boundary_ticket(&legacy_boundary).unwrap(),
            Reply::feature(FeatureReply::Side(SideReply)),
        ),
    );
    let parent = ThreadId::new(MAIN);
    let away = ThreadId::new("other-parent");
    let child = ThreadId::new("new-side");
    let mut other = forked_thread(away.as_str());
    other.ephemeral = false;
    other.forked_from_id = None;
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(agents::Page {
            threads: vec![other.clone()],
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    let opened = run(&mut app, AppEv::Intent(Intent::OpenThread(away.clone())));
    let resume = thread_ask(
        &opened,
        |ask| matches!(ask, ThreadAsk::Resume { thread } if thread == away.as_str()),
    );
    run(
        &mut app,
        AppEv::Reply(
            resume,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(other)))),
        ),
    );
    assert_eq!(app.session().active.as_ref(), Some(&away));
    run(&mut app, AppEv::Intent(Intent::OpenThread(parent.clone())));
    enable_workspace(&mut app, None);
    let request = slash(&mut app, SlashCommand::Side, "park this question");
    let fork = thread_ask(&request, |ask| matches!(ask, ThreadAsk::ForkAt { .. }));
    run(&mut app, AppEv::Intent(Intent::OpenThread(away.clone())));
    assert_eq!(app.session().active.as_ref(), Some(&away));
    let ignored = feature(
        &mut app,
        SideIntent::Return {
            child: ThreadId::new(SIDE),
        },
    );
    assert!(ignored.commands().is_empty());
    assert!(app.features().side.link(&ThreadId::new(SIDE)).is_some());
    run(&mut app, AppEv::Intent(Intent::OpenThread(parent.clone())));
    let preparing = reply_fork(&mut app, fork, forked_thread(child.as_str()));
    assert_eq!(app.session().active.as_ref(), Some(&parent));
    assert!(app.features().side.link(&child).is_none());
    let boundary = preparing
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Feature { ticket, thread, .. } if thread == &child => Some(*ticket),
            _ => None,
        })
        .expect("boundary requested for fork");
    let explicit_early = run(&mut app, AppEv::Intent(Intent::OpenThread(child.clone())));
    assert_eq!(app.session().active.as_ref(), Some(&parent));
    assert!(
        !explicit_early
            .commands()
            .iter()
            .any(|command| matches!(command, Command::StartTurn { .. }))
    );
    let prepared = run(
        &mut app,
        AppEv::Reply(boundary, Reply::feature(FeatureReply::Side(SideReply))),
    );
    assert_eq!(app.session().active.as_ref(), Some(&parent));
    assert!(
        !prepared
            .commands()
            .iter()
            .any(|command| matches!(command, Command::StartTurn { .. }))
    );
    let opened = feature(
        &mut app,
        SideIntent::Select {
            parent,
            child: child.clone(),
        },
    );
    assert_eq!(app.session().active.as_ref(), Some(&child));
    assert!(opened.commands().iter().any(|command| matches!(command,
        Command::StartTurn { thread, message, .. } if thread == &child && message.text == "park this question")));
    let reopened = run(&mut app, AppEv::Intent(Intent::OpenThread(child)));
    assert!(
        !reopened
            .commands()
            .iter()
            .any(|command| matches!(command, Command::StartTurn { .. }))
    );
}

#[test]
fn attached_fork_requires_its_original_backend_and_an_attested_parent() {
    let mut app = ready();
    let (_, ticket) = attached_fork_ask(&mut app);
    let incorrect = Reply::screen(ScreenReply::Agents(agents::Reply::Forked(Box::new(
        attached_child(),
    ))));
    let epoch = app.epoch();
    assert!(
        app.ingest(Ingress::transport_reply(
            BackendId(9),
            epoch,
            epoch,
            ticket,
            incorrect
        ))
        .is_none()
    );
    let mut child = attached_child();
    child.attached_to_thread_id = Some("unrelated-parent".to_owned());
    let rejected = reply_fork(&mut app, ticket, child);
    assert!(boundary_ticket(&rejected).is_none());
    assert!(app.thread(&ThreadId::new(SIDE)).is_none());
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
}

fn attached_fork_ask(app: &mut CodexApp) -> (AppFx, Ticket) {
    enable_workspace(app, Some(LocalSideStorage::Embedded));
    let start = slash(app, SlashCommand::Side, "first side turn");
    let parent_read = thread_ask(
        &start,
        |ask| matches!(ask, ThreadAsk::Read { thread } if thread == MAIN),
    );
    let mut parent = forked_thread(MAIN);
    parent.history_mode = ThreadHistoryMode::Paginated;
    parent.ephemeral = false;
    parent.forked_from_id = None;
    let fork = run(
        app,
        AppEv::Reply(
            parent_read,
            Reply::thread(ThreadReply::Read {
                thread: Box::new(parent),
                unread: false,
            }),
        ),
    );
    let ticket = thread_ask(
        &fork,
        |ask| matches!(ask, ThreadAsk::ForkAttachedSide { thread, .. } if thread.as_str() == MAIN),
    );
    (fork, ticket)
}

fn attached_child() -> Thread {
    let mut attached = forked_thread(SIDE);
    attached.ephemeral = false;
    attached.history_mode = ThreadHistoryMode::Paginated;
    attached.attached_to_thread_id = Some(MAIN.to_owned());
    attached.path = serde_json::from_value(serde_json::json!("/tmp/side-1.jsonl")).unwrap();
    attached
}

#[test]
fn attached_storage_waits_for_the_durable_boundary_and_close_only_unsubscribes() {
    let mut app = ready();
    let (fork, ticket) = attached_fork_ask(&mut app);
    let ClientRequest::ThreadFork { params, .. } = wire(&fork, ticket) else {
        panic!("expected fork")
    };
    assert!(
        params.attach_to_source
            && params.exclude_turns
            && !params.ephemeral
            && !params.defer_goal_continuation
    );
    assert_eq!(params.thread_id, MAIN);
    assert_eq!(
        params.thread_source,
        Some(codex_app_server_protocol::ThreadSource::Feature(
            "tui_side_chat".to_owned()
        ))
    );
    assert!(
        !params
            .developer_instructions
            .unwrap()
            .contains("cannot retain a worktree")
    );
    let preparing = reply_fork(&mut app, ticket, attached_child());
    let child = ThreadId::new(SIDE);
    assert!(app.features().side.link(&child).is_none());
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
    assert!(
        preparing
            .commands()
            .iter()
            .all(|command| !matches!(command, Command::StartTurn { .. }))
    );
    let boundary = boundary_ticket(&preparing).expect("attached boundary");
    assert!(preparing.commands().iter().any(|command| matches!(command,
        Command::Feature { preparation_checkpoint: Some(checkpoint), thread, .. }
            if thread == &child && checkpoint.parent.as_str() == MAIN
                && checkpoint.rollout.as_deref() == Some(std::path::Path::new("/tmp/side-1.jsonl")))));
    let impatient = slash(&mut app, SlashCommand::Side, "preserve my next draft");
    assert!(impatient.commands().is_empty());
    assert!(
        app.active_thread()
            .restore()
            .unwrap()
            .messages()
            .iter()
            .any(|message| message.text == "preserve my next draft")
    );
    let ClientRequest::ThreadInjectItems { params, .. } = wire(&preparing, boundary) else {
        panic!("expected preparation")
    };
    assert_eq!(params.thread_id, SIDE);
    assert_eq!(params.items.len(), 2);
    assert!(
        !params.items[0]
            .to_string()
            .contains("cannot retain a worktree")
    );
    let prepared = run(
        &mut app,
        AppEv::Reply(boundary, Reply::feature(FeatureReply::Side(SideReply))),
    );
    assert_eq!(
        app.features().side.link(&child).unwrap().storage,
        Storage::Attached
    );
    assert_eq!(app.session().active.as_ref(), Some(&child));
    assert!(prepared.commands().iter().any(|command| matches!(command,
        Command::StartTurn { thread, message, .. } if thread == &child && message.text == "first side turn")));
    let closed = feature(&mut app, SideIntent::Return { child });
    let _ = thread_ask(
        &closed,
        |ask| matches!(ask, ThreadAsk::Unsubscribe { thread } if thread == SIDE),
    );
    assert!(closed.commands().iter().all(|command| !matches!(
        command,
        Command::Screen {
            command: ScreenCommand::Agents(agents::Command::Thread(
                ThreadAsk::Delete { .. } | ThreadAsk::Archive { .. }
            )),
            ..
        }
    )));
}

#[test]
fn a_retired_fork_cannot_be_opened_or_deleted_by_the_replacement_handle() {
    let mut app = ready();
    let (_, fork) = attached_fork_ask(&mut app);
    let preparing = reply_fork(&mut app, fork, attached_child());
    let boundary = boundary_ticket(&preparing).unwrap();
    let prior = app.epoch();
    let next = prior.next();
    let restarted = app
        .ingest(Ingress::transport_replaced(
            BackendId::default(),
            prior,
            next,
        ))
        .unwrap();
    assert!(restarted.commands().iter().all(|command| !matches!(
        command,
        Command::Screen {
            command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Delete { .. })),
            ..
        }
    )));
    assert!(
        app.ingest(Ingress::transport_reply(
            BackendId::default(),
            prior,
            prior,
            boundary,
            Reply::feature(FeatureReply::Side(SideReply)),
        ))
        .is_none()
    );
    let child = ThreadId::new(SIDE);
    let show = run(&mut app, AppEv::Intent(Intent::OpenThread(child.clone())));
    assert!(
        show.commands()
            .iter()
            .all(|command| !matches!(command, Command::Screen {
        command: ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Resume { thread })), ..
    } if thread == SIDE))
    );
    let submit = run(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: Some(child.clone()),
            message: codex_tui_core::threads::UserMessage::text("do not submit".to_owned()),
        }),
    );
    assert!(
        submit
            .commands()
            .iter()
            .all(|command| !matches!(command, Command::StartTurn { .. }))
    );
    assert!(app.features().side.link(&child).is_none());
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
}

#[test]
fn attached_prepare_failure_rolls_back_only_the_attested_child() {
    let mut app = ready();
    let (_, fork) = attached_fork_ask(&mut app);
    let preparing = reply_fork(&mut app, fork, attached_child());
    let boundary = boundary_ticket(&preparing).unwrap();
    let failed = run(
        &mut app,
        AppEv::Reply(boundary, Reply::failed("disk write failed")),
    );
    let child = ThreadId::new(SIDE);
    let rollback = thread_ask(
        &failed,
        |ask| matches!(ask, ThreadAsk::Delete { thread } if thread == &child),
    );
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
    assert!(app.features().side.link(&child).is_none());
    run(
        &mut app,
        AppEv::Reply(rollback, Reply::thread(ThreadReply::Done)),
    );
    assert!(app.thread(&child).is_none());
    assert!(notes(&app, MAIN).contains(&SideNote::PrepareFailed {
        thread: child,
        error: "disk write failed".to_owned()
    }));
}

#[test]
fn stale_transport_capability_cannot_authorize_persistent_forks() {
    let mut app = ready();
    enable_workspace(&mut app, Some(LocalSideStorage::Embedded));
    let connector = app.epoch();
    let current = connector.next();
    app.ingest(Ingress::transport_replaced(
        BackendId::default(),
        connector,
        current,
    ))
    .unwrap();
    assert!(
        app.ingest(Ingress::local_side_storage(
            connector,
            connector,
            LocalSideStorage::Embedded
        ))
        .is_none()
    );
    let spawned = slash(&mut app, SlashCommand::Side, "");
    let _ = thread_ask(&spawned, |ask| {
        matches!(
            ask,
            ThreadAsk::ForkAt {
                temporary: Some(_),
                ..
            }
        )
    });
}

#[test]
fn an_older_local_daemon_without_the_filter_uses_a_temporary_fork() {
    let mut app = ready();
    enable_workspace(&mut app, Some(LocalSideStorage::Daemon));
    let start = slash(&mut app, SlashCommand::Side, "");
    let probe = thread_ask(
        &start,
        |ask| matches!(ask, ThreadAsk::ProbeAttachedSide { thread } if thread.as_str() == MAIN),
    );
    let ClientRequest::ThreadList { params, .. } = wire(&start, probe) else {
        panic!("expected filter probe")
    };
    assert_eq!(params.attached_to_thread_id.as_deref(), Some(MAIN));
    assert_eq!(params.parent_thread_id.as_deref(), Some(MAIN));
    let fallback = run(
        &mut app,
        AppEv::Reply(
            probe,
            Reply::thread(ThreadReply::AttachedSideSupported(false)),
        ),
    );
    let _ = thread_ask(&fallback, |ask| {
        matches!(
            ask,
            ThreadAsk::ForkAt {
                temporary: Some(_),
                ..
            }
        )
    });
}

#[test]
fn returning_shows_the_parent_and_drops_the_side_conversation() {
    let (mut app, _) = opened(None);
    // the side conversation is shown: the parent is shown first, then it is dropped
    let fx = feature(
        &mut app,
        SideIntent::Return {
            child: ThreadId::new(SIDE),
        },
    );
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
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
fn returning_from_a_conversation_that_is_not_a_side_one_does_nothing() {
    let (mut app, _) = opened(None);
    let fx = feature(
        &mut app,
        SideIntent::Return {
            child: ThreadId::new(MAIN),
        },
    );
    assert!(fx.commands().is_empty());
    assert_eq!(app.session().active, Some(ThreadId::new(SIDE)));
    assert!(app.features().side.is_side(&ThreadId::new(SIDE)));
}

#[test]
fn a_side_conversation_the_parent_hides_is_only_dropped() {
    let (mut app, _) = opened(None);
    run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(MAIN))),
    );
    let fx = feature(
        &mut app,
        SideIntent::Return {
            child: ThreadId::new(SIDE),
        },
    );
    assert!(!fx.commands().is_empty());
    assert_eq!(app.session().active, Some(ThreadId::new(MAIN)));
    assert!(!app.features().side.is_side(&ThreadId::new(SIDE)));
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
