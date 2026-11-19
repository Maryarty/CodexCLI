//! The command center over more than one backend (the local server and the
//! cloud): rows keep their owner, pins are kept per backend, a refresh of one
//! backend never reorders the other's. Ported from the oracle
//! (`app/agents_overview_tests.rs`: `refreshing_one_backends_pins_preserves_cross_backend_rank`
//! at 181, `overview_keeps_rows_and_routes_actions_to_the_owner` at 223,
//! codex-tui 0.170.0). The oracle's refresh result carries the backend that
//! was asked; here the page does (`Ev::ListedBy`).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::Thread;
use codex_tui_core::app::AppFx;
use codex_tui_core::ids::BackendId;
use codex_tui_core::screens::agents::{Ev, Model, Page, Pins, apply};
use serde_json::json;

fn id(n: u128) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

/// The oracle's `overview_thread`: an idle root named `name`.
fn thread(id: &str, name: &str) -> Thread {
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn page(threads: Vec<Thread>, pins: Option<Vec<String>>) -> Page {
    Page {
        threads,
        unread: Vec::new(),
        next_cursor: None,
        append: false,
        pins: Some(Pins {
            ids: pins,
            threads: Vec::new(),
        }),
    }
}

fn listed(model: &mut Model, owner: BackendId, page: Page) {
    apply(model, Ev::ListedBy(owner, page), &mut AppFx::default());
}

/// `refreshing_one_backends_pins_preserves_cross_backend_rank`: the pins of
/// the local backend and the cloud keep one order after each refreshes in turn.
#[test]
fn refreshing_one_backends_pins_preserves_cross_backend_rank() {
    let mut model = Model::default();
    let (local, cloud) = (BackendId(1), BackendId(2));
    model.displayed = local;
    let (local_pin, cloud_pin) = (id(1), id(2));
    let mut previous = None;
    for (owner, pin) in [
        (local, &local_pin),
        (cloud, &cloud_pin),
        (local, &local_pin),
    ] {
        listed(&mut model, owner, page(Vec::new(), Some(vec![pin.clone()])));
        let ranks = model.pinned.clone().expect("pins");
        if owner == cloud {
            assert_eq!(ranks.len(), 2);
            previous = Some(ranks);
        } else if let Some(expected) = &previous {
            assert_eq!(&ranks, expected);
        }
    }
}

/// `overview_keeps_rows_and_routes_actions_to_the_owner`, the rows' part: a
/// thread belongs to the first backend that listed it; the displayed
/// backend's pins stay `None` when it has none, the other's are kept; a row of
/// an id another backend owns is dropped.
#[test]
fn overview_keeps_rows_and_owners_per_backend() {
    let mut model = Model::default();
    let (local, remote) = (BackendId(1), BackendId(2));
    model.displayed = local;
    let (local_id, remote_id, colliding_id) = (id(1), id(2), id(3));
    listed(
        &mut model,
        local,
        page(
            vec![
                thread(&local_id, "Local task"),
                thread(&colliding_id, "Name from the displayed backend"),
            ],
            None,
        ),
    );
    assert_eq!(model.owners.get(&remote_id), None);
    assert_eq!(model.pins_by_backend.get(&local), Some(&None));
    listed(
        &mut model,
        remote,
        page(
            vec![
                thread(&remote_id, "Remote task"),
                thread(&colliding_id, "Colliding task"),
            ],
            Some(Vec::new()),
        ),
    );
    assert_eq!(model.pins_by_backend.get(&remote), Some(&Some(Vec::new())));
    assert_eq!(model.owners.get(&remote_id), Some(&remote));
    assert_eq!(model.owners.get(&colliding_id), Some(&local));
    // The oracle's rows are a map; the order is not asserted.
    let mut names: Vec<_> = model
        .threads
        .iter()
        .map(|held| held.name.clone().unwrap_or_default())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "Local task",
            "Name from the displayed backend",
            "Remote task"
        ]
    );
}

fn paged(threads: Vec<Thread>, next: Option<&str>) -> Page {
    Page {
        threads,
        unread: Vec::new(),
        next_cursor: next.map(str::to_owned),
        append: false,
        pins: None,
    }
}

/// `AgentsOverviewState::{initialized, discovery}` are per backend: another
/// backend's first page sets its own cursor and leaves the displayed list's
/// alone, and displaying it later continues from there.
#[test]
fn discovery_is_kept_per_backend() {
    let mut model = Model::default();
    let (local, cloud) = (BackendId(1), BackendId(2));
    model.displayed = local;
    listed(
        &mut model,
        local,
        paged(vec![thread(&id(1), "Local")], Some("L")),
    );
    listed(
        &mut model,
        cloud,
        paged(vec![thread(&id(2), "Cloud")], Some("C")),
    );
    assert_eq!(model.paging.cursor.as_deref(), Some("L"));
    assert!(model.paging.has_more);
    assert!(model.paging_of(cloud).initialized);
    assert_eq!(model.paging_of(cloud).cursor.as_deref(), Some("C"));
    assert!(model.has_more(cloud));
    // A backend nothing was read from has more to read.
    assert!(model.has_more(BackendId(9)));
    assert!(!model.paging_of(BackendId(9)).initialized);

    apply(&mut model, Ev::Displayed(cloud), &mut AppFx::default());
    assert_eq!(model.displayed, cloud);
    assert_eq!(model.paging.cursor.as_deref(), Some("C"));
    assert!(model.paging.initialized);
    assert_eq!(model.paging_of(local).cursor.as_deref(), Some("L"));
    // Back again: nothing was lost.
    apply(&mut model, Ev::Displayed(local), &mut AppFx::default());
    assert_eq!(model.paging.cursor.as_deref(), Some("L"));
    assert_eq!(model.paging_of(cloud).cursor.as_deref(), Some("C"));
}

fn open_row(model: &mut Model, owner: BackendId) -> Vec<codex_tui_core::screens::agents::Out> {
    let _ = owner;
    codex_tui_core::screens::agents::update(
        model,
        codex_tui_core::screens::agents::Intent::Accept(
            codex_tui_core::screens::agents::Sel::Task(id(2)),
        ),
    )
}

/// Opening a row another backend owns resumes it there (oracle:
/// `select_agents_overview_thread` sends `ResumeBackendSession(owner)`); the
/// answer makes that backend the displayed one.
#[test]
fn opening_a_foreign_row_resumes_it_on_its_owner() {
    use codex_tui_core::screens::agents::{Command, Ctx, Reply, apply_answer, backend_of};
    let mut model = Model::default();
    let (local, cloud) = (BackendId(1), BackendId(2));
    model.displayed = local;
    listed(
        &mut model,
        local,
        paged(vec![thread(&id(1), "Local")], None),
    );
    listed(
        &mut model,
        cloud,
        paged(vec![thread(&id(2), "Cloud")], Some("C")),
    );
    let outs = open_row(&mut model, cloud);
    assert_eq!(outs.len(), 1);
    assert!(matches!(outs[0].command, Command::Resume { .. }));
    assert_eq!(backend_of(&model, &outs[0].command), cloud);
    // The displayed backend's own rows and the lists go to it.
    assert_eq!(
        backend_of(
            &model,
            &Command::Rename {
                thread: id(1),
                name: "x".into()
            }
        ),
        local
    );
    assert_eq!(backend_of(&model, &Command::List { cursor: None }), local);
    let answer = Reply::Resumed(Box::new(thread(&id(2), "Cloud")));
    let _ = apply_answer(&mut model, Ctx::Resume { thread: id(2) }, Ok(answer));
    assert_eq!(model.displayed, cloud);
    assert_eq!(model.paging.cursor.as_deref(), Some("C"));
    assert_eq!(model.current.as_deref(), Some(id(2).as_str()));
}

/// A cloud task archives alone, whichever backend is displayed (oracle:
/// the thread params mode is the owner's).
#[test]
fn a_cloud_owned_row_archives_only_the_task_from_a_local_session() {
    use codex_tui_core::screens::agents::{Action, Ctx, Reply, apply_answer};
    let mut model = Model::default();
    let (local, cloud) = (BackendId(1), BackendId(2));
    model.displayed = local;
    apply(&mut model, Ev::CloudBackend(cloud), &mut AppFx::default());
    let mut child = thread(&id(3), "Child");
    child.parent_thread_id = Some(id(2));
    listed(
        &mut model,
        cloud,
        paged(vec![thread(&id(2), "Cloud parent"), child], None),
    );
    let ctx = Ctx::Action {
        thread: id(2),
        action: Some(Action::Archive),
    };
    let _ = apply_answer(&mut model, ctx, Ok(Reply::Done));
    let left: Vec<_> = model.threads.iter().map(|held| held.id.clone()).collect();
    assert_eq!(left, vec![id(3)]);
}

/// `view.pin_action_supported`: judged by the displayed backend's pins alone
/// (oracle: `pins.get(&displayed).map(is_some).unwrap_or(true)`).
#[test]
fn pin_is_supported_by_the_displayed_backends_pins() {
    use codex_tui_core::screens::agents::{Intent, update};
    let mut model = Model::default();
    let (local, cloud) = (BackendId(1), BackendId(2));
    model.displayed = local;
    listed(&mut model, local, page(vec![thread(&id(1), "Local")], None));
    listed(
        &mut model,
        cloud,
        page(vec![thread(&id(2), "Cloud")], Some(Vec::new())),
    );
    // The local backend has no pins even though the cloud has.
    assert!(update(&mut model, Intent::TogglePin(id(1))).is_empty());
    assert!(!model.pin_action_supported());
    apply(&mut model, Ev::Displayed(cloud), &mut AppFx::default());
    assert!(model.pin_action_supported());
}

/// What the shell routes by: the owner of a task for a command about it, the
/// displayed backend for the lists (`CodexApp::backend_of`).
#[test]
fn the_app_routes_a_resume_to_the_backend_that_owns_the_task() {
    use codex_tui_core::app::{AppEv, Command, Intent, apply_app};
    use codex_tui_core::fixtures::script::build;
    use codex_tui_core::screens::agents::{Intent as AgentsIntent, Sel};
    use codex_tui_core::screens::{ScreenEv, ScreenIntent};
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    let mut app = build(&format!("{script}\n")).expect("plays").app;
    let cloud = BackendId(1);
    let send = |app: &mut _, ev: AppEv| {
        let mut fx = AppFx::default();
        apply_app(app, ev, &mut fx);
        fx
    };
    send(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::ListedBy(
            cloud,
            paged(vec![thread(&id(2), "Cloud")], None),
        ))),
    );
    let fx = send(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(AgentsIntent::Accept(
            Sel::Task(id(2)),
        )))),
    );
    let [command] = fx.commands() else {
        panic!("one RPC: {:?}", fx.commands());
    };
    assert!(matches!(command, Command::Screen { .. }));
    assert_eq!(app.backend_of(command), cloud);
    // The lists go to the displayed backend.
    let fx = send(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(AgentsIntent::Refresh))),
    );
    assert_eq!(app.backend_of(&fx.commands()[0]), BackendId::default());
}
