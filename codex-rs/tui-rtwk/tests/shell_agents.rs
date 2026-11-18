//! The shell side of S21, the Agent command center against a backend: the
//! effects the shell runs for core's commands and the events it sends back,
//! proven on the scripted backend (`shell::fake`). Oracle: the same facts the
//! oracle reads in `agents_overview_discovery.rs` (two sources, merge, cloud),
//! `agents_overview_usage.rs` (65 s), `agents_overview_view.rs` (directory and
//! repository keys), `agents_overview_new.rs` (worktree). The pure half is in
//! `core/tests/agents_live.rs`; here the event reaches the core model.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::sync::Arc;
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::screens::agents::{DirKeys, Ev, Intent as AgentsIntent, Notice};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use codex_tui_rtwk::shell::fake::FakeWorkspace;
use codex_tui_rtwk::shell::rpc::{Ids, USAGE_TIMED_OUT, USAGE_TIMEOUT, answer, answer_with};
use codex_tui_rtwk::shell::workspace::{
    Checkout, Facts, Workspace, backend_ev, directories, worktree,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const TIMESTAMP: i64 = 1_700_000_000;
const LOCAL: Facts = Facts {
    cloud: false,
    worktrees: true,
};

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_time()
            .build()
            .expect("runtime")
    })
}

fn thread_json(id: &str, recency: i64, cwd: &str) -> Value {
    json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": TIMESTAMP,
        "updatedAt": recency, "recencyAt": recency, "status": {"type": "idle"}, "path": null,
        "cwd": cwd, "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": id,
        "daybreakEnabled": null, "turns": []
    })
}

fn page(rows: &[(&str, i64, &str)]) -> Value {
    let data: Vec<Value> = rows
        .iter()
        .map(|(id, at, cwd)| thread_json(id, *at, cwd))
        .collect();
    json!({"data": data, "nextCursor": null, "backwardsCursor": null})
}

fn kinds(request: &ClientRequest) -> Value {
    serde_json::to_value(request).unwrap()["params"]["sourceKinds"].clone()
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn agents(app: &CodexApp) -> &codex_tui_core::screens::agents::Model {
    &app.screens.agents
}

/// The `Command::Screen` effects of `fx`, which only the core can make.
fn screen_commands(fx: AppFx) -> Vec<Command> {
    fx.into_commands()
        .into_iter()
        .filter(|command| matches!(command, Command::Screen { .. }))
        .collect()
}

fn open(app: &mut CodexApp) -> Command {
    let fx = run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(AgentsIntent::Open))),
    );
    screen_commands(fx)
        .into_iter()
        .next()
        .expect("opening lists the tasks")
}

/// A reply of the shell goes back into the core the way the ingress does.
fn feed(
    app: &mut CodexApp,
    (ticket, reply): (codex_tui_core::ids::Ticket, codex_tui_core::rpc::Reply),
) {
    run(app, AppEv::Reply(ticket, reply));
}

#[test]
fn backend_facts_reach_the_center() {
    let mut app = CodexApp::new();
    run(
        &mut app,
        backend_ev(Facts {
            cloud: true,
            worktrees: false,
        }),
    );
    assert!(agents(&app).cloud && !agents(&app).worktrees);
    run(&mut app, backend_ev(LOCAL));
    assert!(!agents(&app).cloud && agents(&app).worktrees);
}

#[test]
fn a_list_page_merges_both_sources_into_the_center() {
    let mut app = CodexApp::new();
    let command = open(&mut app);
    let asked = std::sync::Mutex::new(Vec::new());
    let reply = runtime().block_on(answer(command, LOCAL, Ids::default(), None, |request| {
        asked.lock().unwrap().push(kinds(&request));
        let rows = if kinds(&request) == json!([]) {
            page(&[("a1", 100, "/p"), ("a2", 60, "/p")])
        } else {
            page(&[("b1", 80, "/p")])
        };
        async move { Ok(rows) }
    }));
    assert_eq!(
        *asked.lock().unwrap(),
        [json!([]), json!(["exec", "appServer"])]
    );
    feed(&mut app, reply.expect("an RPC"));
    let ids: Vec<&str> = agents(&app)
        .threads
        .iter()
        .map(|thread| thread.id.as_str())
        .collect();
    assert_eq!(ids, ["a1", "b1", "a2"]);
}

#[test]
fn cloud_lists_the_interactive_source_only() {
    let mut app = CodexApp::new();
    let command = open(&mut app);
    let asked = std::sync::Mutex::new(Vec::new());
    let facts = Facts {
        cloud: true,
        worktrees: false,
    };
    let reply = runtime().block_on(answer(command, facts, Ids::default(), None, |request| {
        asked.lock().unwrap().push(kinds(&request));
        async move { Ok(page(&[("a1", 100, "/p")])) }
    }));
    assert_eq!(*asked.lock().unwrap(), [json!([])]);
    feed(&mut app, reply.unwrap());
    assert_eq!(agents(&app).threads.len(), 1);
}

#[test]
fn a_failing_source_leaves_the_list_failed_inline() {
    let mut app = CodexApp::new();
    let command = open(&mut app);
    let reply = runtime().block_on(answer(command, LOCAL, Ids::default(), None, |request| {
        let ok = kinds(&request) == json!([]);
        async move {
            if ok {
                Ok(page(&[("a1", 100, "/p")]))
            } else {
                Err("down".to_owned())
            }
        }
    }));
    feed(&mut app, reply.unwrap());
    assert_eq!(agents(&app).notice, Some(Notice::LoadFailed));
    assert!(agents(&app).paging.failed);
}

/// The Business plan shows usage; the highlighted task's fetch is the RPC.
fn usage_command(app: &mut CodexApp) -> Command {
    run(
        app,
        AppEv::Screen(ScreenEv::Agents(Ev::Account {
            plan: Some(serde_json::from_value(json!("business")).unwrap()),
            backend_auth: true,
        })),
    );
    let list = open(app);
    let page_reply = runtime().block_on(answer(list, LOCAL, Ids::default(), None, |_| async {
        Ok(page(&[("a1", 100, "/p")]))
    }));
    feed(app, page_reply.unwrap());
    let fx = run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
            AgentsIntent::NeedUsage,
        ))),
    );
    screen_commands(fx)
        .into_iter()
        .next()
        .expect("a usage fetch")
}

/// The oracle gives up on the usage fetch after 65 seconds
/// (`THREAD_USAGE_FETCH_TIMEOUT`), and the failure settles the cache cadence.
#[test]
fn the_usage_fetch_gives_up_as_the_oracle_does() {
    assert_eq!(USAGE_TIMEOUT, Duration::from_secs(65));
    let mut app = CodexApp::new();
    let command = usage_command(&mut app);
    let pending = std::future::pending::<Result<Value, String>>;
    let reply = runtime().block_on(answer_with(
        command,
        LOCAL,
        Ids::default(),
        None,
        Duration::from_millis(30),
        |_| pending(),
    ));
    let (ticket, reply) = reply.expect("the timeout is an answer");
    assert_eq!(reply, codex_tui_core::rpc::Reply::failed(USAGE_TIMED_OUT));
    feed(&mut app, (ticket, reply));
    assert!(
        agents(&app).pending_usage.is_none(),
        "the failure ends the fetch"
    );
}

/// Only the usage fetch is cut short; a list that answers late still does.
#[test]
fn only_the_usage_fetch_has_the_timeout() {
    let mut app = CodexApp::new();
    let command = open(&mut app);
    let reply = runtime().block_on(answer_with(
        command,
        LOCAL,
        Ids::default(),
        None,
        Duration::from_millis(10),
        |request| {
            let rows = if kinds(&request) == json!([]) {
                page(&[("a1", 1, "/p")])
            } else {
                page(&[])
            };
            async move {
                tokio::time::sleep(Duration::from_millis(60)).await;
                Ok(rows)
            }
        },
    ));
    feed(&mut app, reply.unwrap());
    assert_eq!(agents(&app).threads.len(), 1);
}

fn keys(project: &str, heading: &str, directory: &str) -> DirKeys {
    DirKeys {
        project: (project.to_owned(), String::new()),
        heading: heading.to_owned(),
        directory: directory.to_owned(),
    }
}

#[test]
fn directory_keys_reach_the_center() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_keys("/w/one", keys("/repo/.git", "/repo", "/real/one"));
    workspace.set_keys("/w/two", keys("/repo/.git", "/repo", "/real/two"));
    let mut app = CodexApp::new();
    let ev = runtime().block_on(directories(
        workspace.clone(),
        vec!["/w/one".into(), "/w/two".into(), "/w/gone".into()],
    ));
    run(&mut app, ev);
    let model = agents(&app);
    assert_eq!(model.dirs["/w/one"].heading, "/repo");
    assert_eq!(
        model.dirs["/w/two"].project,
        ("/repo/.git".to_owned(), String::new())
    );
    assert!(
        !model.dirs.contains_key("/w/gone"),
        "unreadable: its own key"
    );
    assert_eq!(workspace.key_reads(), ["/w/one", "/w/two", "/w/gone"]);
}

fn checkout() -> Checkout {
    Checkout {
        root: "/wt/abc".into(),
        cwd: "/wt/abc/sub".into(),
    }
}

/// The server's `thread/start` answers with the task in the checkout.
fn started(request: &ClientRequest) -> Result<Value, String> {
    let wire = serde_json::to_value(request).unwrap();
    assert_eq!(wire["method"], "thread/start");
    let cwd = wire["params"]["cwd"].as_str().unwrap_or("/none").to_owned();
    Ok(json!({"thread": thread_json("new-1", 500, &cwd)}))
}

fn make_worktree(
    workspace: &Arc<FakeWorkspace>,
    cwd: Option<&str>,
    send: impl Fn(ClientRequest) -> std::future::Ready<Result<Value, String>>,
) -> AppEv {
    runtime().block_on(worktree(
        workspace.clone(),
        cwd.map(str::to_owned),
        Ids::default(),
        send,
    ))
}

#[test]
fn a_new_worktree_starts_a_task_in_the_checkout_and_opens_it() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Ok(checkout()));
    let mut app = CodexApp::new();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::Notice(Some(Notice::CreatingWorktree)))),
    );
    let ev = make_worktree(&workspace, Some("/src/repo"), |request| {
        std::future::ready(started(&request))
    });
    run(&mut app, ev);
    assert_eq!(workspace.created(), [Some("/src/repo".to_owned())]);
    assert_eq!(
        workspace.bound(),
        [(checkout(), "new-1".to_owned(), "/wt/abc/sub".to_owned())]
    );
    assert_eq!(
        app.session.active.as_ref().map(|id| id.as_str()),
        Some("new-1")
    );
    assert_eq!(agents(&app).notice, None);
    assert_eq!(agents(&app).current.as_deref(), Some("new-1"));
}

fn notice_of(ev: AppEv) -> Option<Notice> {
    let mut app = CodexApp::new();
    run(&mut app, ev);
    agents(&app).notice.clone()
}

#[test]
fn a_worktree_that_cannot_be_made_shows_the_reason() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Err("The source project is not trusted.".into()));
    let ev = make_worktree(&workspace, None, |_| {
        std::future::ready(Err("never".into()))
    });
    assert_eq!(
        notice_of(ev),
        Some(Notice::Error("The source project is not trusted.".into()))
    );
}

/// Oracle: `agents_overview_retained_worktree_error`.
#[test]
fn a_task_that_fails_to_start_names_the_retained_checkout() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Ok(checkout()));
    let ev = make_worktree(&workspace, None, |_| {
        std::future::ready(Err("offline".into()))
    });
    assert_eq!(
        notice_of(ev),
        Some(Notice::Error("Failed to start session: offline A checkout was retained at /wt/abc; remove it with `git worktree remove <checkout-path>` from the source repository if it is no longer needed.".into()))
    );
    assert!(workspace.bound().is_empty());
}

#[test]
fn a_task_that_cannot_be_bound_names_the_retained_checkout() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Ok(checkout()));
    workspace.set_bind(Err(
        "The server did not apply the worktree directory.".into()
    ));
    let ev = make_worktree(&workspace, None, |request| {
        std::future::ready(started(&request))
    });
    let notice = notice_of(ev);
    let Some(Notice::Error(message)) = notice else {
        panic!("{notice:?}")
    };
    assert!(
        message.starts_with(
            "The server did not apply the worktree directory. A checkout was retained at /wt/abc;"
        ),
        "{message}"
    );
}

#[test]
fn the_fake_workspace_is_a_workspace() {
    fn takes(_: &dyn Workspace) {}
    takes(&FakeWorkspace::default());
}
