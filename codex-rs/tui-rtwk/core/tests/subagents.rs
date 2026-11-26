//! The picker reads its active conversation through the owning backend.
#![allow(clippy::expect_used)]

use codex_app_server_protocol::{
    ServerNotification, Thread, ThreadListResponse, ThreadNameUpdatedNotification,
    ThreadUnarchivedNotification,
};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Ingress, Intent, apply_app};
use codex_tui_core::features::{FeatureNote, subagents};
use codex_tui_core::ids::{BackendId, ConnEpoch, ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::agents::{self, Page};
use codex_tui_core::screens::select::agent_picker::{Facts, PickerKey};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenIntent, ScreenReply};
use codex_tui_core::threads::{Item, NoteKey};
use serde_json::json;

const LOCAL: BackendId = BackendId(0);
const CLOUD: BackendId = BackendId(1);
const ROOT_A: &str = "11111111-1111-4111-8111-111111111111";
const ROOT_B: &str = "22222222-2222-4222-8222-222222222222";

#[test]
fn subagents_on_the_blank_chat_reports_no_agents_without_dispatching() {
    let mut app = CodexApp::new();
    let effects = run(
        &mut app,
        AppEv::Intent(Intent::Feature(subagents::Intent::NoAgents(None).into())),
    );
    assert!(effects.commands().is_empty());
    assert!(matches!(
        app.screens().chat.notes.as_slice(),
        [note] if note.key == NoteKey::Feature(FeatureNote::Subagents(subagents::Note::NoAgents))
    ));
}

fn run(app: &mut CodexApp, event: AppEv) -> AppFx {
    let mut effects = AppFx::default();
    apply_app(app, event, &mut effects);
    effects
}

fn start(app: &mut CodexApp, event: AppEv, owner: BackendId, root: &str) {
    let effects = run(app, event);
    let ticket = effects
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread {
                ticket, backend, ..
            } if *backend == owner => Some(*ticket),
            _ => None,
        })
        .expect("a start on the requested backend");
    let epoch = app.epoch_of(owner);
    app.ingest(Ingress::of(
        owner,
        epoch,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(root))),
    ))
    .expect("the start reply was accepted");
}

fn two_backends() -> CodexApp {
    let mut app = CodexApp::new();
    app.attach();
    start(&mut app, AppEv::Connected, LOCAL, ROOT_A);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::CloudBackend(CLOUD))),
    );
    app.attach_backend(CLOUD);
    app
}

fn listed(app: &mut CodexApp, owner: BackendId, threads: Vec<Thread>) {
    run(
        app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::ListedBy(
            owner,
            Page {
                threads,
                unread: Vec::new(),
                next_cursor: None,
                append: true,
                pins: None,
            },
        ))),
    );
}

fn thread(id: &str, parent: &str) -> Thread {
    serde_json::from_value(json!({
        "id": id, "sessionId": parent, "parentThreadId": parent,
        "name": id, "agentNickname": id, "agentRole": "worker",
        "preview": "", "ephemeral": false, "historyMode": "legacy",
        "modelProvider": "openai", "createdAt": 1_700_000_000, "updatedAt": 1_700_000_000,
        "status": {"type": "idle"}, "cwd": "/tmp/project",
        "cliVersion": "0.0.0", "source": "cli", "canAcceptDirectInput": false, "turns": []
    }))
    .expect("thread")
}

fn open(app: &mut CodexApp, source: &str, picker: PickerKey) -> AppFx {
    let event = picker_open(app, source, picker);
    run(app, event)
}

fn picker_open(app: &CodexApp, source: &str, picker: PickerKey) -> AppEv {
    AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
        agents::Intent::ReadDescendants {
            active: ThreadId::new(source),
            picker,
            root_visit: app.screens().agents.descendant_root_visit,
        },
    )))
}

fn close(app: &mut CodexApp, picker: PickerKey) {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
            agents::Intent::StopReadingDescendants(picker),
        ))),
    );
}

fn ask(app: &CodexApp, fx: &AppFx, owner: BackendId, root: &str, archived: bool) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command:
                    ScreenCommand::Agents(agents::Command::Descendants {
                        owner: origin,
                        list: codex_tui_core::threads::ThreadAsk::List(query),
                    }),
                ..
            } => {
                assert_eq!(*origin, owner);
                assert_eq!(
                    app.dispatch_route(command, None)
                        .map(|(backend, _)| backend),
                    Some(owner)
                );
                assert_eq!(query.ancestor_thread_id.as_deref(), Some(root));
                assert_eq!(query.archived, Some(archived));
                Some(*ticket)
            }
            _ => None,
        })
        .expect("root-scoped list")
}

fn deliver(
    app: &mut CodexApp,
    owner: BackendId,
    epoch: ConnEpoch,
    ticket: Ticket,
    data: Vec<Thread>,
) -> Option<AppFx> {
    deliver_page(app, owner, epoch, ticket, data, None)
}

fn deliver_page(
    app: &mut CodexApp,
    owner: BackendId,
    epoch: ConnEpoch,
    ticket: Ticket,
    data: Vec<Thread>,
    next_cursor: Option<String>,
) -> Option<AppFx> {
    app.ingest(Ingress::of(
        owner,
        epoch,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Descendants(Box::new(
                ThreadListResponse {
                    data,
                    read_states: None,
                    next_cursor,
                    backwards_cursor: None,
                },
            )))),
        ),
    ))
}

fn notify(app: &mut CodexApp, owner: BackendId, event: ServerNotification) {
    let epoch = app.epoch_of(owner);
    let _ = app.ingest(Ingress::of(
        owner,
        epoch,
        AppEv::Backend(BackendEv::Notification(Box::new(event))),
    ));
}

#[test]
fn subagents_reads_the_owner_and_ignores_foreign_and_old_epoch_replies() {
    let mut app = two_backends();
    listed(&mut app, CLOUD, vec![thread("collision", ROOT_B)]);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Displayed(CLOUD))),
    );
    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_A, picker.clone());
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    let local_epoch = app.epoch_of(LOCAL);
    let cloud_epoch = app.epoch_of(CLOUD);
    assert!(
        deliver(
            &mut app,
            CLOUD,
            cloud_epoch,
            live,
            vec![thread("foreign", ROOT_A)]
        )
        .is_none()
    );
    assert!(
        deliver(
            &mut app,
            LOCAL,
            ConnEpoch(0),
            live,
            vec![thread("stale", ROOT_A)]
        )
        .is_none()
    );
    let fx = deliver(
        &mut app,
        LOCAL,
        local_epoch,
        live,
        vec![
            thread("owned-child", ROOT_A),
            thread("collision", ROOT_A),
            thread("unrelated", ROOT_B),
        ],
    )
    .expect("live reply");
    let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
    deliver(&mut app, LOCAL, local_epoch, archived, Vec::new()).expect("archive reply");
    let model = &app.screens().agents;
    assert!(model.descendants_read(&picker));
    assert_eq!(model.owners.get("owned-child"), Some(&LOCAL));
    assert_eq!(model.owners.get("collision"), Some(&CLOUD));
    for id in ["foreign", "stale", "unrelated"] {
        assert_eq!(model.owners.get(id), None);
    }
    let rows = Facts::of(app.screens()).rows(Some(&ThreadId::new(ROOT_A)), &[], |_| false);
    assert_eq!(
        rows.iter()
            .map(|row| row.thread.as_str())
            .collect::<Vec<_>>(),
        [ROOT_A, "owned-child"]
    );
}

#[test]
fn subagents_rejects_delayed_mounts_and_replies_after_the_root_changes() {
    let mut app = two_backends();
    let fx = open(&mut app, ROOT_A, PickerKey::default());
    let old = ask(&app, &fx, LOCAL, ROOT_A, false);
    let local_epoch = app.epoch_of(LOCAL);
    start(&mut app, AppEv::Intent(Intent::CycleBackend), CLOUD, ROOT_B);
    assert_eq!(
        app.session().active.as_ref().map(ThreadId::as_str),
        Some(ROOT_B)
    );
    assert!(
        open(&mut app, ROOT_A, PickerKey::default())
            .commands()
            .is_empty()
    );
    if let Some(fx) = deliver(
        &mut app,
        LOCAL,
        local_epoch,
        old,
        vec![thread("late-A", ROOT_A)],
    ) {
        assert!(fx.commands().is_empty());
    }
    assert_eq!(app.screens().agents.owners.get("late-A"), None);
    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_B, picker.clone());
    let live = ask(&app, &fx, CLOUD, ROOT_B, false);
    let epoch = app.epoch_of(CLOUD);
    let fx =
        deliver(&mut app, CLOUD, epoch, live, vec![thread("new-B", ROOT_B)]).expect("live reply");
    let archived = ask(&app, &fx, CLOUD, ROOT_B, true);
    deliver(&mut app, CLOUD, epoch, archived, Vec::new()).expect("archive reply");
    assert!(app.screens().agents.descendants_read(&picker));
    assert_eq!(app.screens().agents.owners.get("new-B"), Some(&CLOUD));
}

#[test]
fn subagents_abandons_the_read_even_if_the_same_root_becomes_active_again() {
    let mut app = two_backends();
    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_A, picker);
    let old = ask(&app, &fx, LOCAL, ROOT_A, false);
    let epoch = app.epoch_of(LOCAL);
    start(&mut app, AppEv::Intent(Intent::CycleBackend), CLOUD, ROOT_B);
    run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(ROOT_A))),
    );
    assert_eq!(
        app.session().active.as_ref().map(ThreadId::as_str),
        Some(ROOT_A)
    );
    if let Some(fx) = deliver(
        &mut app,
        LOCAL,
        epoch,
        old,
        vec![thread("late-after-return", ROOT_A)],
    ) {
        assert!(fx.commands().is_empty());
    }
    assert_eq!(app.screens().agents.owners.get("late-after-return"), None);
}

#[test]
fn subagents_ignores_a_first_mount_queued_before_opening_two_held_roots() {
    let mut app = two_backends();
    start(&mut app, AppEv::Intent(Intent::CycleBackend), CLOUD, ROOT_B);
    run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(ROOT_A))),
    );
    let delayed = picker_open(&app, ROOT_A, PickerKey::default());
    let switches = app.session().switches;
    for root in [ROOT_B, ROOT_A] {
        run(
            &mut app,
            AppEv::Intent(Intent::OpenThread(ThreadId::new(root))),
        );
        assert_eq!(
            app.session().active.as_ref().map(ThreadId::as_str),
            Some(root)
        );
    }
    assert_eq!(app.session().switches, switches);
    assert!(run(&mut app, delayed).commands().is_empty());
    let effects = open(&mut app, ROOT_A, PickerKey::default());
    ask(&app, &effects, LOCAL, ROOT_A, false);
}

#[test]
fn subagents_close_and_reopen_retires_the_earlier_generation() {
    let mut app = two_backends();
    let prior = PickerKey::default();
    let fx = open(&mut app, ROOT_A, prior.clone());
    let old = ask(&app, &fx, LOCAL, ROOT_A, false);
    close(&mut app, prior.clone());
    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_A, picker.clone());
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    close(&mut app, prior);
    let epoch = app.epoch_of(LOCAL);
    if let Some(fx) = deliver(&mut app, LOCAL, epoch, old, vec![thread("retired", ROOT_A)]) {
        assert!(fx.commands().is_empty());
    }
    let fx = deliver(
        &mut app,
        LOCAL,
        epoch,
        live,
        vec![thread("current", ROOT_A)],
    )
    .expect("live reply");
    let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
    deliver(&mut app, LOCAL, epoch, archived, Vec::new()).expect("archive reply");
    assert!(app.screens().agents.descendants_read(&picker));
    assert_eq!(app.screens().agents.owners.get("retired"), None);
    assert_eq!(app.screens().agents.owners.get("current"), Some(&LOCAL));
}

#[test]
fn subagents_rejects_a_parent_id_owned_by_another_backend() {
    let mut app = two_backends();
    let mut root_b = thread(ROOT_B, ROOT_B);
    root_b.parent_thread_id = None;
    listed(&mut app, CLOUD, vec![root_b]);
    listed(
        &mut app,
        LOCAL,
        vec![thread(ROOT_A, ROOT_B), thread("foreign-child", ROOT_B)],
    );
    assert_eq!(
        app.screens()
            .agents
            .descendants_root(&ThreadId::new(ROOT_A)),
        None
    );
    assert!(
        open(&mut app, ROOT_A, PickerKey::default())
            .commands()
            .is_empty()
    );
    let facts = Facts::of(app.screens());
    assert!(
        facts
            .rows(Some(&ThreadId::new(ROOT_A)), &[], |_| false)
            .is_empty()
    );
    let rows = facts.rows(Some(&ThreadId::new(ROOT_B)), &[], |_| false);
    assert_eq!(
        rows.iter()
            .map(|row| row.thread.as_str())
            .collect::<Vec<_>>(),
        [ROOT_B]
    );
}

#[test]
fn subagents_accepts_a_page_after_switching_to_a_child_of_the_same_root() {
    let mut app = two_backends();
    start(&mut app, AppEv::Intent(Intent::CycleBackend), CLOUD, ROOT_B);
    let child = thread("already-listed-child", ROOT_B);
    listed(&mut app, CLOUD, vec![child.clone()]);
    let fx = open(&mut app, ROOT_B, PickerKey::default());
    let live = ask(&app, &fx, CLOUD, ROOT_B, false);
    let effects = run(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(&child.id))),
    );
    let resume = effects
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::Thread(_)),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("resume the known child");
    let epoch = app.epoch_of(CLOUD);
    app.ingest(Ingress::of(
        CLOUD,
        epoch,
        AppEv::Reply(
            resume,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(child)))),
        ),
    ))
    .expect("resume reply");
    assert_eq!(
        app.session().active.as_ref().map(ThreadId::as_str),
        Some("already-listed-child")
    );
    let fx = deliver(
        &mut app,
        CLOUD,
        epoch,
        live,
        vec![thread("other-child", ROOT_B)],
    )
    .expect("live reply");
    let archived = ask(&app, &fx, CLOUD, ROOT_B, true);
    deliver(&mut app, CLOUD, epoch, archived, Vec::new()).expect("archived reply");
    assert_eq!(app.screens().agents.owners.get("other-child"), Some(&CLOUD));
}

#[test]
fn subagents_archived_row_cannot_be_restored_by_an_older_center_page_or_picked() {
    let mut app = two_backends();
    start(&mut app, AppEv::Intent(Intent::CycleBackend), CLOUD, ROOT_B);
    let old_row = thread("archived-child", ROOT_B);
    let retained = thread("retained-child", ROOT_B);
    listed(&mut app, CLOUD, vec![old_row.clone(), retained.clone()]);
    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_B, picker.clone());
    let live = ask(&app, &fx, CLOUD, ROOT_B, false);
    let epoch = app.epoch_of(CLOUD);
    let fx = deliver(&mut app, CLOUD, epoch, live, vec![old_row.clone()]).expect("live page");
    let archived = ask(&app, &fx, CLOUD, ROOT_B, true);
    deliver(&mut app, CLOUD, epoch, archived, vec![old_row.clone()]).expect("archived page");
    listed(&mut app, CLOUD, vec![old_row.clone()]);
    let facts = Facts::of(app.screens());
    let rows = facts.rows(Some(&ThreadId::new(ROOT_B)), &[], |_| false);
    assert_eq!(
        rows.iter()
            .map(|row| row.thread.as_str())
            .collect::<Vec<_>>(),
        [ROOT_B, "retained-child"]
    );
    let lost = AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
        agents::Intent::ChooseDescendant {
            picker: picker.clone(),
            thread: ThreadId::new("archived-child"),
        },
    )));
    assert!(run(&mut app, lost).commands().is_empty());
    let current = AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
        agents::Intent::ChooseDescendant {
            picker,
            thread: ThreadId::new(&retained.id),
        },
    )));
    let fx = run(&mut app, current);
    let resume = fx.commands().iter().find(|command| matches!(command, Command::Screen {
        command: ScreenCommand::Agents(agents::Command::Thread(codex_tui_core::threads::ThreadAsk::Resume { thread })),
        ..
    } if thread == &retained.id)).expect("resumes the eligible child through OpenThread");
    assert_eq!(
        app.dispatch_route(resume, None).map(|(owner, _)| owner),
        Some(CLOUD)
    );

    let fx = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(agents::Intent::Open))),
    );
    let list = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(agents::Command::List { .. }),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("a fresh Command Center list");
    app.ingest(Ingress::of(
        CLOUD,
        epoch,
        AppEv::Reply(
            list,
            Reply::screen(ScreenReply::Agents(agents::Reply::Listed(Page {
                threads: vec![old_row],
                unread: Vec::new(),
                next_cursor: None,
                append: false,
                pins: None,
            }))),
        ),
    ))
    .expect("center response");
    assert!(
        app.screens()
            .agents
            .threads
            .iter()
            .any(|thread| thread.id == "archived-child")
    );
}

#[test]
fn subagents_consistent_owned_read_recovers_offline_unarchive_but_preserves_delete() {
    use codex_app_server_protocol::ThreadDeletedNotification;

    let mut app = two_backends();
    let rows = vec![
        thread("offline-unarchive", ROOT_A),
        thread("deleted-after-archive", ROOT_A),
    ];
    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_A, picker.clone());
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    let epoch = app.epoch_of(LOCAL);
    let fx = deliver(&mut app, LOCAL, epoch, live, Vec::new()).expect("live pass");
    let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
    deliver(&mut app, LOCAL, epoch, archived, rows.clone()).expect("archive pass");
    close(&mut app, picker);

    let fresh_epoch = app.attach_backend(LOCAL);
    assert_ne!(fresh_epoch, epoch);
    notify(
        &mut app,
        LOCAL,
        ServerNotification::ThreadDeleted(ThreadDeletedNotification {
            thread_id: "deleted-after-archive".to_owned(),
        }),
    );

    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_A, picker.clone());
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    let fx = deliver(&mut app, LOCAL, fresh_epoch, live, rows.clone()).expect("fresh live data");
    let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
    app.ingest(Ingress::of(
        LOCAL,
        fresh_epoch,
        AppEv::Reply(archived, Reply::failed("archive pass unavailable")),
    ))
    .expect("archive failure");
    assert!(
        app.screens()
            .agents
            .threads
            .iter()
            .all(|held| held.id != "offline-unarchive")
    );
    close(&mut app, picker);

    let picker = PickerKey::default();
    let fx = open(&mut app, ROOT_A, picker.clone());
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    let mut fx =
        deliver(&mut app, LOCAL, fresh_epoch, live, rows.clone()).expect("fresh live data");
    for page in 1..=100 {
        let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
        fx = deliver_page(
            &mut app,
            LOCAL,
            fresh_epoch,
            archived,
            Vec::new(),
            Some(page.to_string()),
        )
        .expect("incomplete archive pass");
    }
    assert!(fx.commands().is_empty());
    assert!(
        app.screens()
            .agents
            .descendant_archives
            .contains("offline-unarchive")
    );
    close(&mut app, picker);

    let fx = open(&mut app, ROOT_A, PickerKey::default());
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    let fx = deliver(&mut app, LOCAL, fresh_epoch, live, rows).expect("fresh live data");
    let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
    deliver(&mut app, LOCAL, fresh_epoch, archived, Vec::new()).expect("fresh empty archive");
    let facts = Facts::of(app.screens());
    let rows = facts.rows(Some(&ThreadId::new(ROOT_A)), &[], |_| false);
    assert_eq!(
        rows.iter()
            .map(|row| row.thread.as_str())
            .collect::<Vec<_>>(),
        [ROOT_A, "offline-unarchive"]
    );
}

#[test]
fn subagents_reports_running_and_stopped_path_backed_children_on_the_source_transcript() {
    use codex_app_server_protocol::{SessionSource, ThreadStatus};
    use codex_protocol::AgentPath;
    use codex_protocol::protocol::SubAgentSource;

    let mut app = two_backends();
    let epoch = app.epoch_of(LOCAL);
    let source = ThreadId::new(ROOT_A);
    let mut child = thread("path-child", ROOT_A);
    child.source = SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
        parent_thread_id: codex_protocol::ThreadId::try_from(ROOT_A).expect("root UUID"),
        depth: 1,
        agent_path: Some(AgentPath::try_from("/root/path_child").expect("agent path")),
        agent_role: None,
        agent_nickname: None,
    });
    for status in [
        ThreadStatus::Active {
            active_flags: vec![],
        },
        ThreadStatus::Idle,
    ] {
        child.status = status;
        let picker = PickerKey::default();
        let fx = open(&mut app, ROOT_A, picker.clone());
        let live = ask(&app, &fx, LOCAL, ROOT_A, false);
        let fx = deliver(&mut app, LOCAL, epoch, live, vec![child.clone()]).expect("live reply");
        let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
        deliver(&mut app, LOCAL, epoch, archived, Vec::new()).expect("archived reply");
        run(
            &mut app,
            AppEv::Intent(Intent::Feature(
                subagents::Intent::Report {
                    source: source.clone(),
                    picker,
                }
                .into(),
            )),
        );
    }
    let notes = app
        .active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Feature(FeatureNote::Subagents(note)) => Some(note.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        notes,
        [
            subagents::Note::Running(vec![subagents::Entry {
                path: "/root/path_child".to_owned(),
                recent: vec![]
            }]),
            subagents::Note::Running(vec![]),
        ]
    );
}

#[test]
fn subagents_retries_an_uncached_unarchive_and_keeps_newer_metadata() {
    let mut app = two_backends();
    let picker = PickerKey::default();
    let epoch = app.epoch_of(LOCAL);
    let fx = open(&mut app, ROOT_A, picker.clone());
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    let fx = deliver(&mut app, LOCAL, epoch, live, Vec::new()).expect("first live pass");
    let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
    notify(
        &mut app,
        LOCAL,
        ServerNotification::ThreadUnarchived(ThreadUnarchivedNotification {
            thread_id: "unarchived".to_owned(),
        }),
    );
    let fx = deliver(&mut app, LOCAL, epoch, archived, Vec::new()).expect("first archived pass");
    let live = ask(&app, &fx, LOCAL, ROOT_A, false);
    notify(
        &mut app,
        LOCAL,
        ServerNotification::ThreadNameUpdated(ThreadNameUpdatedNotification {
            thread_id: "unarchived".to_owned(),
            thread_name: Some("new live name".to_owned()),
        }),
    );
    let fx = deliver(
        &mut app,
        LOCAL,
        epoch,
        live,
        vec![thread("unarchived", ROOT_A)],
    )
    .expect("follow-up live pass");
    let archived = ask(&app, &fx, LOCAL, ROOT_A, true);
    deliver(&mut app, LOCAL, epoch, archived, Vec::new()).expect("follow-up archived pass");
    let model = &app.screens().agents;
    assert!(model.descendants_read(&picker));
    let child = model
        .threads
        .iter()
        .find(|thread| thread.id == "unarchived")
        .expect("fresh row");
    assert_eq!(child.name.as_deref(), Some("new live name"));
    assert_eq!(child.agent_role.as_deref(), Some("worker"));
}
