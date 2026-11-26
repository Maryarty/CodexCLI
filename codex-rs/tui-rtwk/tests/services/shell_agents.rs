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
use codex_tui_core::app::worktree::{BindFailure, RetainedReason};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::rpc::{RpcError, ServerError};
use codex_tui_core::screens::agents::{DirKeys, Ev, Grouping, Intent as AgentsIntent, Notice};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use codex_tui_rtwk::services::agents::search::{Gate, Latest, SEARCH_DEBOUNCE};
use codex_tui_rtwk::services::agents::workspace::{
    Boxed, Checkout, Facts, Workspace, account, backend_ev, bind_worktree, create_worktree,
    directories, save_grouping,
};
use codex_tui_rtwk::services::backend::fake::FakeWorkspace;
use codex_tui_rtwk::services::backend::rpc::{
    Ids, USAGE_TIMED_OUT, USAGE_TIMEOUT, answer, answer_with,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "../oracle_snap/mod.rs"]
mod oracle_snap;

#[allow(clippy::duplicate_mod)] // one helper, a module in each binary with a live app
#[path = "../common/wait.rs"]
mod wait;

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

/// A read of the pinned section (the first page of the list also makes it).
fn is_pin_read(request: &ClientRequest) -> bool {
    !serde_json::to_value(request).unwrap()["params"]["sectionId"].is_null()
}

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    fx
}

fn agents(app: &CodexApp) -> &codex_tui_core::screens::agents::Model {
    &app.screens().agents
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
        let rows = if is_pin_read(&request) {
            page(&[])
        } else if kinds(&request) == json!([]) {
            page(&[("a1", 100, "/p"), ("a2", 60, "/p")])
        } else {
            page(&[("b1", 80, "/p")])
        };
        async move { Ok(rows) }
    }));
    // The two sources of the list, then the pins of both.
    assert_eq!(
        *asked.lock().unwrap(),
        [
            json!([]),
            json!(["exec", "appServer"]),
            json!([]),
            json!(["exec", "appServer"])
        ]
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
        let rows = if is_pin_read(&request) {
            page(&[])
        } else {
            page(&[("a1", 100, "/p")])
        };
        async move { Ok(rows) }
    }));
    // The list asks one source; the pins ask both, as the oracle's do.
    assert_eq!(
        *asked.lock().unwrap(),
        [json!([]), json!([]), json!(["exec", "appServer"])]
    );
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
                Err("down".into())
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
            AgentsIntent::NeedUsage(Some("a1".into())),
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
    let pending = std::future::pending::<Result<Value, ServerError>>;
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
fn started(request: &ClientRequest) -> Result<Value, ServerError> {
    let wire = serde_json::to_value(request).unwrap();
    if wire["method"] != "thread/start" {
        // The unsubscribe after a failed bind.
        return Ok(json!({}));
    }
    let cwd = wire["params"]["cwd"].as_str().unwrap_or("/none").to_owned();
    Ok(json!({"thread": thread_json("new-1", 500, &cwd)}))
}

fn make_worktree(
    workspace: &Arc<FakeWorkspace>,
    cwd: Option<&str>,
    send: impl Fn(ClientRequest) -> std::future::Ready<Result<Value, ServerError>>,
) -> CodexApp {
    let mut app = CodexApp::new();
    run(&mut app, backend_ev(LOCAL));
    if let Some(cwd) = cwd {
        let mut thread: codex_app_server_protocol::Thread =
            serde_json::from_value(thread_json("selected", 500, cwd)).unwrap();
        thread.turns.clear();
        run(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(Ev::Listed(
                codex_tui_core::screens::agents::Page {
                    threads: vec![thread],
                    unread: vec![],
                    next_cursor: None,
                    append: false,
                    pins: None,
                },
            ))),
        );
    }
    let mut effects = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
            AgentsIntent::NewWorktree(cwd.map(|_| "selected".to_owned())),
        ))),
    );
    assert_eq!(agents(&app).notice, Some(Notice::CreatingWorktree));
    let ids = Ids::default();
    while let Some(command) = effects.into_commands().into_iter().next() {
        let event = runtime().block_on(async {
            match command {
                Command::Effect {
                    ask: codex_tui_core::app::effects::Ask::CreateWorktree { ticket, cwd, .. },
                    ..
                } => create_worktree(workspace.clone(), ticket, cwd).await,
                Command::Effect {
                    ask:
                        codex_tui_core::app::effects::Ask::BindWorktree {
                            ticket,
                            checkout,
                            thread,
                            cwd,
                            ..
                        },
                    ..
                } => bind_worktree(workspace.clone(), ticket, checkout, thread, cwd).await,
                command => {
                    let (ticket, reply) = answer(command, LOCAL, ids.clone(), None, &send)
                        .await
                        .unwrap();
                    AppEv::Reply(ticket, reply)
                }
            }
        });
        effects = run(&mut app, event);
    }
    app
}

#[test]
fn a_new_worktree_starts_a_task_in_the_checkout_and_opens_it() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Ok(checkout()));
    let app = make_worktree(&workspace, Some("/src/repo"), |request| {
        std::future::ready(started(&request))
    });
    assert_eq!(workspace.created(), [Some("/src/repo".to_owned())]);
    assert_eq!(
        workspace.bound(),
        [(checkout(), "new-1".to_owned(), "/wt/abc/sub".to_owned())]
    );
    assert_eq!(
        app.session()
            .active
            .as_ref()
            .map(codex_tui_core::ids::ThreadId::as_str),
        Some("new-1")
    );
    assert_eq!(agents(&app).notice, None);
}

#[test]
fn a_worktree_that_cannot_be_made_shows_the_reason() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Err("The source project is not trusted.".into()));
    let app = make_worktree(&workspace, None, |_| {
        std::future::ready(Err("never".into()))
    });
    assert_eq!(
        agents(&app).notice.clone(),
        Some(Notice::Error("The source project is not trusted.".into()))
    );
}

/// Oracle: `agents_overview_retained_worktree_error`.
#[test]
fn a_task_that_fails_to_start_names_the_retained_checkout() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Ok(checkout()));
    let app = make_worktree(&workspace, None, |_| {
        std::future::ready(Err("offline".into()))
    });
    assert_eq!(
        agents(&app).notice.clone(),
        Some(Notice::WorktreeRetained {
            root: checkout().root,
            reason: RetainedReason::Start(RpcError::Remote("offline".into()))
        })
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
    let methods = std::cell::RefCell::new(Vec::new());
    let app = make_worktree(&workspace, None, |request| {
        let wire = serde_json::to_value(&request).unwrap();
        methods
            .borrow_mut()
            .push(wire["method"].as_str().unwrap().to_owned());
        std::future::ready(started(&request))
    });
    assert_eq!(methods.into_inner(), ["thread/start", "thread/unsubscribe"]);
    assert_eq!(
        agents(&app).notice,
        Some(Notice::WorktreeRetained {
            root: checkout().root,
            reason: RetainedReason::Bind(BindFailure::Workspace(
                "The server did not apply the worktree directory.".into()
            )),
        }),
    );
    assert_eq!(app.session().active, None);
}

#[test]
fn the_fake_workspace_is_a_workspace() {
    fn takes(_: &dyn Workspace) {}
    takes(&FakeWorkspace::default());
}

// The history search's debounce (oracle: `search_agents_overview`, the
// 180 ms `sleep` before the first page, the task dropped by the next search).

/// The commands the center sends when the search line gets `text`, one
/// keystroke at a time.
fn typed(app: &mut CodexApp, text: &str) -> Vec<Command> {
    let mut sent = Vec::new();
    let mut line = String::new();
    for ch in text.chars() {
        line.push(ch);
        let fx = run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
                AgentsIntent::SetSearch {
                    text: line.clone(),
                    lens: codex_tui_core::screens::agents::Lens {
                        searching: true,
                        ..Default::default()
                    },
                },
            ))),
        );
        sent.extend(screen_commands(fx));
    }
    sent
}

fn searches(app: &mut CodexApp, text: &str) -> Vec<Command> {
    typed(app, text)
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Screen {
                    command: codex_tui_core::screens::ScreenCommand::Agents(
                        codex_tui_core::screens::agents::Command::Search { .. }
                    ),
                    ..
                }
            )
        })
        .collect()
}

fn job(label: &'static str) -> Boxed<Option<&'static str>> {
    Box::pin(async move { Some(label) })
}

/// No wall clock: the pause is a gate the test opens (`Gate`), so the
/// tests hold under any machine load.
#[test]
fn the_search_waits_before_it_asks_the_server() {
    use futures::FutureExt as _;
    assert_eq!(SEARCH_DEBOUNCE, Duration::from_millis(180));
    let mut app = CodexApp::new();
    open(&mut app);
    let command = searches(&mut app, "a").remove(0);
    let gate = Gate::new();
    let mut latest = Latest::paused_by(&gate);
    let mut waiting = latest.wrap(&command, job("a"));
    runtime().block_on(async {
        // Polled a few times, it is still held back.
        for _ in 0..5 {
            assert!(
                (&mut waiting).now_or_never().is_none(),
                "asked before the pause"
            );
            tokio::task::yield_now().await;
        }
        gate.release();
        assert_eq!(waiting.await, Some("a"));
    });
}

#[test]
fn a_newer_search_drops_the_pending_one() {
    let mut app = CodexApp::new();
    open(&mut app);
    let both = searches(&mut app, "ab");
    assert_eq!(both.len(), 2, "one search per keystroke: {both:?}");
    let gate = Gate::new();
    let mut latest = Latest::paused_by(&gate);
    let first = latest.wrap(&both[0], job("a"));
    let second = latest.wrap(&both[1], job("ab"));
    runtime().block_on(async {
        let both = tokio::spawn(async move { tokio::join!(first, second) });
        gate.release();
        let (first, second) = both.await.expect("joined");
        assert_eq!(first, None, "the older search never asks");
        assert_eq!(second, Some("ab"));
    });
}

#[test]
fn other_commands_neither_wait_nor_drop_a_search() {
    use futures::FutureExt as _;
    let mut app = CodexApp::new();
    let list = open(&mut app);
    let search = searches(&mut app, "a").remove(0);
    let gate = Gate::new();
    let mut latest = Latest::paused_by(&gate);
    let mut pending = latest.wrap(&search, job("search"));
    let mut at_once = latest.wrap(&list, job("list"));
    runtime().block_on(async {
        assert_eq!(
            (&mut at_once).now_or_never(),
            Some(Some("list")),
            "a list is not held back"
        );
        assert!(
            (&mut pending).now_or_never().is_none(),
            "the search is still waiting, not dropped"
        );
        gate.release();
        assert_eq!(pending.await, Some("search"));
    });
}

// The app on the scripted backend: the center opened at start
// (`codex-rtwk agents`), the backend answering as the app-server does.

mod live {
    use super::*;
    use codex_tui_rtwk::app::UiApp;
    use codex_tui_rtwk::app::run::{app_view, localized, root};
    use codex_tui_rtwk::l10n::Lang;
    use codex_tui_rtwk::services::backend::fake::{FakeHandle, FakeServer};
    use codex_tui_rtwk::services::shell::Backend;
    use rtwk_test::{Options, Tester};

    pub struct Live {
        pub t: Tester,
        pub fake: FakeHandle,
        _runtime: tokio::runtime::EnterGuard<'static>,
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    pub fn start(
        script: impl Fn(&Value) -> Option<Result<Value, ServerError>> + Send + Sync + 'static,
    ) -> Live {
        start_sized(100, 30, script)
    }

    pub fn start_sized(
        width: u16,
        height: u16,
        script: impl Fn(&Value) -> Option<Result<Value, ServerError>> + Send + Sync + 'static,
    ) -> Live {
        start_with(width, height, false, script, |_| {})
    }

    /// On `pump_app`, which gives the app the frame loop's handle: only
    /// there does a quit end the loop.
    pub fn start_quittable(
        script: impl Fn(&Value) -> Option<Result<Value, ServerError>> + Send + Sync + 'static,
    ) -> Live {
        start_with(100, 30, true, script, |_| {})
    }

    /// The scripted backend with the history search held at the gate: its
    /// first page goes out when the test opens it.
    pub fn start_held(
        script: impl Fn(&Value) -> Option<Result<Value, ServerError>> + Send + Sync + 'static,
    ) -> (Live, Gate) {
        let gate = Gate::new();
        let held = gate.clone();
        let live = start_with(100, 30, false, script, move |fake| {
            fake.hold_searches(&held);
        });
        (live, gate)
    }

    fn start_with(
        width: u16,
        height: u16,
        quittable: bool,
        script: impl Fn(&Value) -> Option<Result<Value, ServerError>> + Send + Sync + 'static,
        prepare: impl FnOnce(&FakeHandle),
    ) -> Live {
        let guard = rtwk_tokio::enter(runtime());
        let (server, fake) = FakeServer::new();
        fake.respond(script);
        prepare(&fake);
        let app = UiApp::new(Backend::Fake(server)).with_start_agents(true);
        let mut t = Tester::with(Options::new().size(width, height));
        if quittable {
            t.pump_app(app_view(app));
        } else {
            t.pump_widget(localized(root(app_view(app)), Lang::En));
        }
        t.pump_for(ms(200));
        Live {
            t,
            fake,
            _runtime: guard,
        }
    }

    impl Live {
        /// Lets everything that is ready run: the frame loop and the
        /// shell's tasks, a few rounds. For "nothing more happens" only; what
        /// a test expects to happen it waits for (`wait_until`).
        pub fn settle(&mut self) {
            for _ in 0..10 {
                self.t.pump_for(ms(20));
                std::thread::yield_now();
            }
        }

        /// Frames until `ready` holds (`wait::wait_until`): the answer of the
        /// shell's task is applied at some flush, whenever the machine gets
        /// to it.
        pub fn wait_until(&mut self, what: &str, ready: impl FnMut(&mut Live) -> bool) {
            super::wait::wait_until(self, what, ready);
        }

        /// Waits for the app to end.
        pub fn wait_until_ended(&mut self) {
            self.wait_until("the app to end", |h| !h.t.is_running());
        }

        /// Waits until the backend has been asked `method`.
        pub fn wait_for_request(&mut self, method: &str) {
            self.wait_until(&format!("a request {method}"), |h| {
                h.fake.requests().iter().any(|r| r["method"] == method)
            });
        }

        pub fn wait_for(&mut self, what: &str, ready: impl Fn(&str) -> bool) {
            self.wait_until(what, |h| ready(&h.t.screen()));
        }
    }

    impl super::wait::Pumped for Live {
        fn tester(&mut self) -> &mut Tester {
            &mut self.t
        }

        fn pause(&mut self) {
            std::thread::sleep(ms(5));
        }
    }
}

fn server(request: &Value) -> Option<Result<Value, ServerError>> {
    match request["method"].as_str()? {
        "thread/list" if !request["params"]["sectionId"].is_null() => Some(Ok(page(&[]))),
        // The center asks [] (everything) and ["exec","appServer"]; the resume
        // picker asks ["cli","vscode"] and gets the interactive sessions.
        "thread/list" => Some(Ok(
            if request["params"]["sourceKinds"] == json!([])
                || request["params"]["sourceKinds"] == json!(["cli", "vscode"])
            {
                page(&[("Interactive task", 100, "/w/one")])
            } else {
                page(&[("Exec task", 90, "/w/two")])
            },
        )),
        "thread/start" => {
            let cwd = request["params"]["cwd"].as_str().unwrap_or("/none");
            Some(Ok(json!({"thread": thread_json("wt-1", 500, cwd)})))
        }
        _ => None,
    }
}

/// The center lists the interactive and the Exec / AppServer sources.
#[test]
fn the_live_center_lists_both_sources() {
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| {
        s.contains("Interactive task") && s.contains("Exec task")
    });
    let kinds: Vec<Value> = h
        .fake
        .requests()
        .iter()
        .filter(|request| request["method"] == "thread/list")
        .map(|request| request["params"]["sourceKinds"].clone())
        .collect();
    assert!(
        kinds.contains(&json!([])) && kinds.contains(&json!(["exec", "appServer"])),
        "{kinds:?}"
    );
}

fn lists(h: &live::Live) -> usize {
    h.fake
        .requests()
        .iter()
        .filter(|request| request["method"] == "thread/list")
        .count()
}

/// Second review, item 44: `o` opens the picker on the live backend: it lists
/// the sessions again and shows them (oracle: `OpenResumePicker`).
#[test]
fn the_picker_of_the_center_lists_the_sessions() {
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    let before = lists(&h);
    h.t.press("o");
    h.wait_for("the picker with the sessions", |s| {
        s.contains("Resume a previous session") && s.contains("Interactive task")
    });
    // The listing is asked of the shell: it shows when the backend has it.
    h.wait_until("the sessions listed again", |h| lists(h) > before);
    assert!(lists(&h) > before, "the picker lists the sessions again");
}

/// Second review, item 69 (oracle
/// `cancelling_resume_picker_preserves_command_center_state`): the picker is
/// a route over the center; Esc pops back to the center as it was.
#[test]
fn cancelling_resume_picker_preserves_command_center_state() {
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    let ungrouped = h.t.screen();
    h.t.press("g");
    h.wait_for("the grouping", |s| s != ungrouped);
    h.settle();
    let before = h.t.screen();
    h.t.press("o");
    h.wait_for("the picker", |s| s.contains("Resume a previous session"));
    h.t.press("escape");
    h.wait_for("the center again", |s| {
        !s.contains("Resume a previous session")
    });
    h.settle();
    assert_eq!(h.t.screen(), before);
}

/// A pick resumes that task and closes the picker.
#[test]
fn a_pick_in_the_picker_resumes_the_task() {
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("o");
    h.wait_for("the sessions", |s| {
        s.contains("Resume a previous session") && s.contains("Interactive task")
    });
    h.t.press("enter");
    h.wait_for_request("thread/resume");
    h.settle();
    let resumes: Vec<Value> = h
        .fake
        .requests()
        .into_iter()
        .filter(|request| request["method"] == "thread/resume")
        .collect();
    assert_eq!(resumes.len(), 1, "{resumes:?}");
    assert!(
        !h.t.screen().contains("Resume a previous session"),
        "{}",
        h.t.screen()
    );
}

/// A hidden task (`h`) comes back through the picker, and picking it resumes it.
#[test]
fn a_hidden_task_comes_back_through_the_picker() {
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("h");
    h.wait_for("one task hidden", |s| {
        s.contains("Interactive task") != s.contains("Exec task")
    });
    h.settle();
    let screen = h.t.screen();
    let hidden = if screen.contains("Interactive task") {
        "Exec task"
    } else {
        "Interactive task"
    };
    assert!(!screen.contains(hidden), "{screen}");
    h.t.press("o");
    h.wait_for("the hidden task", |s| {
        s.contains("Resume a previous session") && s.contains(hidden)
    });
    h.t.press("enter");
    h.wait_for_request("thread/resume");
}

/// Worktrees on: the shell reads the keys of the listed cwds, and the rows
/// of two directories of one repository share a group.
#[test]
fn the_shell_reads_the_directory_keys_of_the_listed_tasks() {
    let mut h = live::start(server);
    h.fake
        .workspace()
        .set_keys("/w/one", keys("/repo/.git", "/repo", "/real/one"));
    h.fake
        .workspace()
        .set_keys("/w/two", keys("/repo/.git", "/repo", "/real/two"));
    h.fake.announce(LOCAL);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    for _ in 0..50 {
        h.t.pump_for(Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut reads = h.fake.workspace().key_reads();
    reads.sort();
    assert_eq!(reads, ["/w/one", "/w/two"]);
    let screen = h.t.screen();
    assert_eq!(
        screen.matches("/repo").count(),
        1,
        "one group heading:\n{screen}"
    );
}

/// Typing into the search line asks the server once, after the pause: the
/// keystrokes of one word are one search (oracle: 180 ms and the pending
/// search dropped), while the model's text is the typed text at once.
#[test]
fn typing_a_word_asks_the_server_once() {
    let (mut h, gate) = live::start_held(server);
    h.wait_for("the tasks", |s| s.contains("Interactive task"));
    let fake = h.fake.clone();
    let scans = move || {
        fake.requests()
            .iter()
            .filter(|request| {
                request["method"] == "thread/list" && request["params"]["limit"] == json!(100)
            })
            .count()
    };
    let before = scans();
    h.t.press("/");
    // The search line has the focus once it is on the screen; each key is
    // typed after the one before it shows.
    h.wait_for("the search line", |s| s.contains("Search \u{203a}"));
    h.t.press("e");
    h.wait_for("the first letter", |s| s.contains("Search \u{203a} e"));
    h.t.press("x");
    h.wait_for("the typed text", |s| s.contains("Search \u{203a} ex"));
    assert_eq!(scans(), before, "nothing is asked before the pause");
    // The fake's loop has taken every search the keys made, and each dropped
    // the one before it, when the barrier passes: opening the gate earlier
    // would let a search the loop has not yet dropped through.
    let mut taken = h.fake.barrier();
    h.wait_until("the fake's loop to take the searches", |_| taken.passed());
    // The pause ends: only the search that was left asks, once per source.
    // The earlier keystrokes' searches were dropped when the next one came
    // (they never reach the gate's far side), so the two requests of the
    // last one, one per source, are all there ever is.
    gate.release();
    h.wait_for("both sources asked", |_| scans() >= before + 2);
    h.settle();
    assert_eq!(scans() - before, 2, "{:?}", h.fake.requests());
}

/// `agents_overview_view.rs` `on_ctrl_c`: with a line open Ctrl+C closes it
/// (the center stays); otherwise it is the app's, and the app quits
/// (pty scenario `agents`: `ctrl-c` then exit 0).
#[test]
fn ctrl_c_closes_a_line_first_then_quits() {
    let mut h = live::start_quittable(server);
    h.wait_for("the tasks", |s| s.contains("Interactive task"));
    h.t.press("/");
    h.wait_for("the search line", |s| s.contains("Search \u{203a}"));
    h.t.press("e");
    h.wait_for("the letter", |s| s.contains("Search \u{203a} e"));
    h.t.press("ctrl+c");
    h.wait_for("the line closed", |s| !s.contains("Search \u{203a}"));
    assert!(h.t.is_running(), "the first Ctrl+C only closed the line");
    h.t.press("ctrl+c");
    assert!(!h.t.is_running(), "the second one quits");
}

/// The server did not start (the daemon has no package to start, say): the
/// oracle's `run_main_inner` returns the error and the process ends with it;
/// the command center is never left on "Loading tasks…" (pty scenario
/// `agents` without the daemon: the error and exit 1).
#[test]
fn a_connection_that_does_not_start_ends_the_app() {
    let mut h = live::start_quittable(server);
    h.wait_for("the tasks", |s| s.contains("Interactive task"));
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        codex_tui_core::app::AppEv::StartFailed("no daemon package".into()),
    );
    h.wait_until_ended();
}

/// An empty filter keeps the keys: Tab moves to a filter without tasks and
/// Shift+Tab comes back (pty scenario `agents`, checkpoint `filter-all`).
#[test]
fn shift_tab_leaves_a_filter_without_tasks() {
    let mut h = live::start(server);
    h.wait_for("the tasks", |s| s.contains("Interactive task"));
    h.t.press("tab");
    h.wait_for("an empty filter", |s| s.contains("No matching tasks"));
    h.t.press("shift+tab");
    h.wait_for("the tasks again", |s| s.contains("Interactive task"));
}

/// A server that lists pins (even none) offers `p Pin/unpin` in the help, and
/// a pinned task outside the recent page is a row (oracle: `refresh` reads
/// `list_pinned_threads`; pty scenario `agents`, checkpoint `help`).
#[test]
fn the_pins_of_the_server_reach_the_center_and_its_help() {
    let mut h = live::start(|request| {
        if request["method"] == "thread/list"
            && !request["params"]["sectionId"].is_null()
            && request["params"]["sourceKinds"] == json!([])
        {
            return Some(Ok(page(&[("Pinned old task", 1, "/w/one")])));
        }
        server(request)
    });
    h.wait_for("the pinned task", |s| s.contains("Pinned old task"));
    h.t.press("?");
    h.wait_for("the help", |s| s.contains("Pin/unpin"));
}

/// `w`: a worktree is made of the highlighted task's directory, a task is
/// started in it and opens in the chat.
#[test]
fn w_creates_a_worktree_and_opens_a_task_in_it() {
    let mut h = live::start(server);
    h.fake.workspace().set_create(Ok(checkout()));
    h.fake.announce(LOCAL);
    h.wait_for("the task", |s| s.contains("Interactive task"));
    h.t.press("w");
    let workspace = h.fake.workspace().clone();
    h.wait_for("the worktree", |_| !workspace.created().is_empty());
    assert_eq!(h.fake.workspace().created(), [Some("/w/one".to_owned())]);
    h.wait_for("the center to close", |s| !s.contains("Interactive task"));
    let started: Vec<Value> = h
        .fake
        .requests()
        .into_iter()
        .filter(|request| request["method"] == "thread/start")
        .collect();
    assert_eq!(started[0]["params"]["cwd"], "/wt/abc/sub");
    assert_eq!(h.fake.workspace().bound().len(), 1);
}

/// `lifecycle_failure_dialogs` (agents_overview_actions_tests.rs:845, :1007),
/// snapshots archive_task_failure and delete_task_failure: the server refuses
/// the removal as the oracle's ephemeral task does; the dialog names the
/// verb, the error, the note about stopped work and one row `Back to agents`.
/// (Here, on the live shell: a debug row has no backend to refuse.)
#[test]
fn lifecycle_failure_dialogs() {
    for (key, verb, snapshot) in [
        ("a", "archive", "actions__archive_task_failure"),
        ("backspace", "delete", "actions__delete_task_failure"),
    ] {
        let mut h = live::start_sized(80, 24, move |request| {
            let method = request["method"].as_str()?;
            if method == format!("thread/{verb}") {
                return Some(Err(format!(
                    "failed to {verb} session: thread/{verb} failed: no rollout found for thread id 00000000-0000-0000-0000-000000000001 (code -32600)"
                )
                .into()));
            }
            server(request)
        });
        h.wait_for("the task", |s| s.contains("Interactive task"));
        h.t.press(key);
        // The confirmation is up before its option is chosen: a key that
        // arrives earlier would go to the list.
        let title = if key == "a" {
            "Archive \u{201c}"
        } else {
            "Permanently delete \u{201c}"
        };
        h.wait_for("the confirmation", |s| s.contains(title));
        h.t.press("2");
        if key == "backspace" {
            h.t.press("enter");
        }
        h.wait_for("the failure", |s| s.contains("Back to agents"));
        let path = format!("app/snapshots/codex_tui__app__agents_overview__tests__{snapshot}.snap");
        let expected = oracle_snap::rows(&path);
        oracle_snap::assert_same(snapshot, &oracle_snap::screen(&h.t), &expected);
    }
}

// ---- the account (oracle: `SessionAccount::from`, read at bootstrap), so the
// usage the highlighted task shows is fetched

fn account_body(kind: &str) -> Value {
    match kind {
        "business" => json!({
            "account": {"type": "chatgpt", "email": "a@b.c", "planType": "business"},
            "requiresOpenaiAuth": true
        }),
        _ => json!({"account": null, "requiresOpenaiAuth": true}),
    }
}

#[test]
fn the_account_read_reaches_the_center() {
    let mut app = CodexApp::new();
    let asked = std::sync::Mutex::new(Vec::new());
    let ev = runtime().block_on(account(Ids::default(), |request| {
        asked
            .lock()
            .unwrap()
            .push(serde_json::to_value(&request).unwrap());
        std::future::ready(Ok(account_body("business")))
    }));
    run(&mut app, ev.expect("an event"));
    let asked = asked.into_inner().unwrap();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0]["method"], "account/read");
    assert!(
        asked[0]["params"].get("refreshToken").is_none(),
        "false is not serialized"
    );
    assert!(agents(&app).backend_auth);
    assert_eq!(
        agents(&app).plan,
        Some(serde_json::from_value(json!("business")).unwrap())
    );
}

/// A read that fails tells the center nothing: usage stays off.
#[test]
fn a_failed_account_read_sends_nothing() {
    let ev = runtime().block_on(account(Ids::default(), |_| {
        std::future::ready(Err::<Value, ServerError>("offline".into()))
    }));
    assert!(ev.is_none());
}

/// Live: the shell asks for the account as the connection comes up, and
/// the highlighted task's usage is then fetched (it was never before).
#[test]
fn the_live_center_fetches_the_usage_of_the_highlighted_task() {
    let mut h = live::start(|request| match request["method"].as_str()? {
        "account/read" => Some(Ok(account_body("business"))),
        "account/usage/read" => Some(Err("no usage in this test".into())),
        _ => server(request),
    });
    h.wait_for("the tasks", |s| s.contains("Interactive task"));
    let fake = h.fake.clone();
    h.wait_for("the usage request", |_| {
        fake.requests()
            .iter()
            .any(|request| request["method"] == "account/usage/read")
    });
    let requests = h.fake.requests();
    assert!(requests.iter().any(|r| r["method"] == "account/read"));
    let usage = requests
        .iter()
        .find(|r| r["method"] == "account/usage/read")
        .expect("asked");
    assert_eq!(usage["params"]["threadId"], "Interactive task");
}

/// `agents_overview_new.rs`: a started task that cannot be bound to its
/// checkout is unsubscribed (`thread_unsubscribe`) before the error shows.
#[test]
fn a_task_that_cannot_be_bound_is_unsubscribed() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Ok(checkout()));
    workspace.set_bind(Err(
        "The server did not apply the worktree directory.".into()
    ));
    let sent = std::sync::Mutex::new(Vec::new());
    let app = make_worktree(&workspace, None, |request| {
        let wire = serde_json::to_value(&request).unwrap();
        let answer = match wire["method"].as_str() {
            Some("thread/start") => started(&request),
            _ => Ok(json!({})),
        };
        sent.lock().unwrap().push(wire);
        std::future::ready(answer)
    });
    assert!(matches!(
        agents(&app).notice,
        Some(Notice::WorktreeRetained {
            reason: RetainedReason::Bind(_),
            ..
        })
    ));
    let sent = sent.into_inner().unwrap();
    let methods: Vec<&str> = sent.iter().filter_map(|r| r["method"].as_str()).collect();
    assert_eq!(methods, ["thread/start", "thread/unsubscribe"]);
    assert_eq!(sent[1]["params"]["threadId"], "new-1");
}

/// A task that is bound is not unsubscribed.
#[test]
fn a_bound_task_stays_subscribed() {
    let workspace = Arc::new(FakeWorkspace::default());
    workspace.set_create(Ok(checkout()));
    let sent = std::sync::Mutex::new(Vec::new());
    let _ = make_worktree(&workspace, None, |request| {
        sent.lock()
            .unwrap()
            .push(serde_json::to_value(&request).unwrap());
        std::future::ready(started(&request))
    });
    assert_eq!(sent.into_inner().unwrap().len(), 1);
}

/// Item 5 (oracle `agents_overview_tests.rs:3255-3320`): `g` cycles the
/// grouping; the config is written for all but the directory grouping, which
/// is view-local, and the saved one is the next start's.
#[test]
fn a_grouping_change_is_saved_to_the_config_but_the_directory_one() {
    let mut app = CodexApp::new();
    let mut saved = Vec::new();
    for expected in [
        (Grouping::Directory, None),
        (Grouping::Status, Some("status")),
        (Grouping::Model, Some("model")),
        (Grouping::Project, Some("project")),
    ] {
        let fx = run(
            &mut app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
                AgentsIntent::CycleGrouping,
            ))),
        );
        assert_eq!(agents(&app).grouping, expected.0);
        let wrote: Vec<String> = fx
            .into_commands()
            .into_iter()
            .filter_map(|command| match command {
                Command::Effect {
                    ask: codex_tui_core::app::effects::Ask::SaveGrouping { grouping, .. },
                    ..
                } => Some(grouping),
                _ => None,
            })
            .collect();
        assert_eq!(
            wrote,
            expected
                .1
                .map(str::to_owned)
                .into_iter()
                .collect::<Vec<_>>()
        );
        saved.extend(wrote);
    }
    assert_eq!(saved, ["status", "model", "project"]);
}

/// The configured grouping is the one the center opens with.
#[test]
fn the_configured_grouping_is_the_one_the_center_opens_with() {
    let mut app = CodexApp::new();
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(Ev::Grouping(Grouping::Model))),
    );
    assert_eq!(agents(&app).grouping, Grouping::Model);
}

/// The shell writes it; a failed write is the center's error notice, as the
/// oracle words it.
#[test]
fn the_shell_saves_the_grouping_and_reports_a_failure() {
    let workspace = Arc::new(FakeWorkspace::default());
    assert!(
        runtime()
            .block_on(save_grouping(workspace.clone(), "status".to_owned()))
            .is_none()
    );
    assert_eq!(workspace.saved(), ["status"]);
    workspace.fail_save(Some("disk full"));
    let ev = runtime()
        .block_on(save_grouping(workspace, "model".to_owned()))
        .expect("an event");
    assert_eq!(
        format!("{ev:?}"),
        format!(
            "{:?}",
            AppEv::Screen(ScreenEv::Agents(Ev::Notice(Some(Notice::Error(
                "Failed to save Command Center grouping: disk full".to_owned()
            )))))
        )
    );
}

/// What the backend answers a resume with: the thread and the settings the
/// chat starts from.
fn resumed(id: &str) -> Value {
    json!({
        "thread": thread_json(id, 90, "/w/two"),
        "model": "gpt-5", "modelProvider": "openai", "serviceTier": null,
        "cwd": "/w/two", "instructionSources": [],
        "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

/// Enter on a task of the center (open-instant): the session page is pushed
/// at once, before the backend answers the resume, in a loading state; the
/// command center is not on the screen any more.
#[test]
fn enter_shows_the_session_page_before_the_resume_reply() {
    // `server` leaves `thread/resume` unanswered: the reply never comes.
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("enter");
    // The backend leaves the request unanswered throughout this check.
    h.wait_for("loading page", |s| s.contains("Loading task…"));
    let screen = h.t.screen();
    assert!(!screen.contains("Agent command center"), "{screen}");
    // The oracle's frame (`agents_overview_loading.rs`): the bold line at the
    // top left of an otherwise empty screen.
    assert_eq!(
        screen.lines().next().map(str::trim_end),
        Some("Loading task…"),
        "{screen}"
    );
    assert!(
        screen.lines().skip(1).all(|line| line.trim().is_empty()),
        "{screen}"
    );
    assert!(!screen.contains("Ask Codex to do anything"), "{screen}");
}

/// The reply fills the page: the loading state is replaced by the chat of
/// the resumed thread.
#[test]
fn the_resume_reply_fills_the_session_page() {
    let mut h = live::start(|request| match request["method"].as_str()? {
        "thread/resume" => Some(Ok(resumed(request["params"]["threadId"].as_str()?))),
        _ => server(request),
    });
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("enter");
    h.wait_for("the chat", |s| s.contains("Ask Codex to do anything"));
    let screen = h.t.screen();
    assert!(!screen.contains("Loading task…"), "{screen}");
    assert!(!screen.contains("Agent command center"), "{screen}");
}

/// A failed resume goes back to the center as it was, with the error the
/// oracle shows there; the highlight is where it was.
#[test]
fn a_failed_resume_returns_to_the_command_center_with_the_error() {
    let mut h = live::start(|request| match request["method"].as_str()? {
        "thread/resume" => Some(Err(ServerError::from("no such session"))),
        _ => server(request),
    });
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("down");
    h.settle();
    let before = h.t.screen();
    h.t.press("enter");
    h.wait_for("the error", |s| s.contains("no such session"));
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Loading task…"), "{screen}");
    assert!(screen.contains("Agent command center"), "{screen}");
    let highlighted = |s: &str| {
        s.lines()
            .position(|line| line.contains("Exec task") || line.contains("Interactive task"))
    };
    assert_eq!(highlighted(&screen), highlighted(&before));
}

/// open-rest: Esc on the "Loading task…" page goes back to the command center
/// with its highlight where it was.
#[test]
fn esc_on_the_loading_page_returns_to_the_command_center() {
    // `server` leaves `thread/resume` unanswered.
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("down");
    h.settle();
    let before = h.t.screen();
    h.t.press("enter");
    h.wait_for("the loading page", |s| s.contains("Loading task…"));
    h.t.press("esc");
    h.settle();
    let screen = h.t.screen();
    assert!(!screen.contains("Loading task…"), "{screen}");
    assert!(screen.contains("Agent command center"), "{screen}");
    let highlighted = |s: &str| {
        s.lines()
            .position(|line| line.contains("Exec task") || line.contains("Interactive task"))
    };
    assert_eq!(highlighted(&screen), highlighted(&before));
}

/// open-rest: a pick in the resume picker (`o`) opens through the same
/// loading page: pushed at once, before the resume's reply.
#[test]
fn a_pick_in_the_resume_picker_shows_the_loading_page() {
    let mut h = live::start(server);
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("o");
    h.wait_for("the picker", |s| {
        s.contains("Resume a previous session") && s.contains("Interactive task")
    });
    h.t.press("enter");
    h.wait_for("the loading page", |s| s.contains("Loading task…"));
    let screen = h.t.screen();
    assert!(!screen.contains("Resume a previous session"), "{screen}");
}

/// A turn of `thread/turns/list`: a prompt and an answer.
fn turn_json(id: &str, prompt: &str, answer: &str) -> Value {
    json!({
        "id": id, "rootTurnId": null, "itemsView": "full", "status": "completed",
        "error": null, "startedAt": null, "completedAt": null, "durationMs": null,
        "items": [
            {"type": "userMessage", "id": format!("{id}-u"), "clientId": null,
             "content": [{"type": "text", "text": prompt, "text_elements": []}]},
            {"type": "agentMessage", "id": format!("{id}-a"), "text": answer}
        ]
    })
}

/// open-rest: the opened session shows its history: `thread/turns/list`
/// pages the turns newest first, the transcript shows them oldest first.
#[test]
fn the_opened_session_shows_its_history() {
    let mut h = live::start(|request| match request["method"].as_str()? {
        "thread/resume" => Some(Ok(resumed(request["params"]["threadId"].as_str()?))),
        "thread/turns/list" if request["params"]["limit"].as_u64() != Some(1) => Some(Ok(json!({
            "data": [
                turn_json("t2", "second question", "second answer"),
                turn_json("t1", "first question", "first answer")
            ],
            "nextCursor": null
        }))),
        _ => server(request),
    });
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("enter");
    h.wait_for("the history", |s| s.contains("second answer"));
    let screen = h.t.screen();
    let at = |needle: &str| {
        screen
            .find(needle)
            .unwrap_or_else(|| panic!("{needle}: {screen}"))
    };
    assert!(at("first question") < at("first answer"), "{screen}");
    assert!(at("first answer") < at("second question"), "{screen}");
    assert!(at("second question") < at("second answer"), "{screen}");
    let asked: Vec<Value> = h
        .fake
        .requests()
        .iter()
        .filter(|r| r["method"] == "thread/turns/list" && r["params"]["limit"] != 1)
        .map(|r| r["params"].clone())
        .collect();
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(asked[0]["sortDirection"], "desc");
    assert_eq!(asked[0]["itemsView"], "full");
}

/// open-rest (review #56, attach conflict): a thread another process writes
/// does not fail to open: the oracle opens it read-only with its history and
/// says so where the composer would be.
#[test]
fn a_thread_with_another_writer_opens_read_only_with_its_history() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let attempts = Arc::new(AtomicUsize::new(0));
    let replies = attempts.clone();
    let mut h = live::start(move |request| match request["method"].as_str()? {
        "thread/resume" => {
            let thread = request["params"]["threadId"].as_str()?;
            if replies.fetch_add(1, Ordering::SeqCst) == 0 {
                Some(Err(ServerError {
                    code: Some(-32600),
                    data: None,
                    message: format!("thread {thread} already has an active writer"),
                }))
            } else {
                Some(Ok(resumed(thread)))
            }
        }
        "thread/turns/list" if request["params"]["limit"].as_u64() != Some(1) => {
            let mut turn = turn_json("t1", "first question", "first answer");
            turn["durationMs"] = json!(8_500);
            let items = turn["items"].as_array_mut().expect("fixture items");
            items[0]["content"]
                .as_array_mut()
                .expect("prompt inputs")
                .push(json!({"type":"localImage", "path":"fixture.png"}));
            items.insert(1, json!({
                "type": "commandExecution", "id": "command-1", "command": "/bin/zsh -lc 'printf example'",
                "cwd": "/tmp", "processId": null, "source": "agent", "status": "completed",
                "commandActions": [], "aggregatedOutput": "hidden first\nsecond\nthird\ntool preview", "exitCode": 0, "durationMs": 8
            }));
            items.insert(
                2,
                json!({
                    "type": "mcpToolCall", "id": "mcp-1", "server": "catalog", "tool": "lookup",
                    "status": "completed", "arguments": {"query":"fixture"},
                    "result": {"content":[{"type":"text","text":"lookup result"}]}, "durationMs": 10
                }),
            );
            Some(Ok(json!({"data": [turn], "nextCursor": null})))
        }
        _ => server(request),
    });
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    h.t.press("enter");
    h.wait_for("the history", |s| s.contains("Worked for 8s"));
    let screen = h.t.screen();
    for needle in [
        "This conversation is open in another app",
        "Close it there and press R to continue here.",
        "r to retry",
        "[Image #1]",
        "Ran printf example",
        "└ second",
        "+ 1 line",
        "Called catalog.lookup",
        "└ lookup result",
        "+ Show details",
        "first answer",
    ] {
        assert!(screen.contains(needle), "{needle} missing:\n{screen}");
    }
    assert!(!screen.contains("Ask Codex to do anything"), "{screen}");
    assert!(!screen.contains("active writer"), "{screen}");
    assert!(!screen.contains("Agent command center"), "{screen}");
    assert!(!screen.contains("hidden first"), "{screen}");

    let first_prompt = screen
        .lines()
        .position(|line| line.contains("first question"));
    assert!(first_prompt.is_some_and(|row| row < 6), "{screen}");

    let details_row = screen
        .lines()
        .position(|line| line.contains("+ 1 line"))
        .expect("command disclosure");
    h.t.click(9, details_row as u16);
    h.wait_for("expanded command", |s| s.contains("hidden first"));
    let expanded_row =
        h.t.screen()
            .lines()
            .position(|line| line.contains("Hide details"))
            .expect("collapse control");
    h.t.click(9, expanded_row as u16);
    h.wait_for("collapsed command", |s| {
        s.contains("+ 1 line") && !s.contains("hidden first")
    });

    h.t.press("ctrl+t");
    h.wait_for("full transcript while the writer is locked", |s| {
        s.contains("hidden first")
            && s.contains(r#""query":"fixture""#)
            && s.contains("This conversation is open in another app")
    });
    h.t.press("ctrl+t");
    h.wait_for("compact transcript", |s| {
        s.contains("+ 1 line") && !s.contains("hidden first")
    });

    h.t.press("r");
    h.wait_for("the writer attached on retry", |s| {
        s.contains("Ask Codex to do anything")
    });
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

/// open-rest (ADR-003): several sessions stay attached to the process; opening
/// A, going back, opening B, back, and A again resumes each thread once (the
/// server has one writer per thread: a second resume of A would conflict).
#[test]
fn open_a_back_open_b_back_open_a_resumes_each_thread_once() {
    let mut h = live::start(|request| match request["method"].as_str()? {
        "thread/resume" => Some(Ok(resumed(request["params"]["threadId"].as_str()?))),
        _ => server(request),
    });
    h.wait_for("both tasks", |s| s.contains("Exec task"));
    let open_row = |h: &mut live::Live, downs: usize| {
        for _ in 0..downs {
            h.t.press("down");
        }
        h.settle();
        h.t.press("enter");
        h.wait_for("the chat", |s| s.contains("Ask Codex to do anything"));
        h.t.press("left");
        h.wait_for("the center", |s| s.contains("Agent command center"));
    };
    open_row(&mut h, 0);
    open_row(&mut h, 1);
    open_row(&mut h, 0);
    let resumed: Vec<String> = h
        .fake
        .requests()
        .iter()
        .filter(|r| r["method"] == "thread/resume")
        .map(|r| {
            r["params"]["threadId"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(resumed.len(), 2, "{resumed:?}");
    assert_ne!(resumed[0], resumed[1], "{resumed:?}");
}

fn request_redemption() -> Command {
    use codex_tui_core::screens::ScreenCommand;
    use codex_tui_core::screens::usage;

    let mut app = CodexApp::new();
    run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Usage(usage::Intent::Redeem {
            credit_id: Some("credit".to_owned()),
        }))),
    )
    .into_commands()
    .into_iter()
    .find(|command| {
        matches!(
            command,
            Command::Screen {
                command: ScreenCommand::Usage(_),
                ..
            }
        )
    })
    .expect("redemption RPC")
}

#[tokio::test(start_paused = true)]
async fn redemption_uses_the_shared_rpc_and_its_own_timeout() {
    let command = request_redemption();
    let started = tokio::time::Instant::now();
    let (_, reply) = answer(command, LOCAL, Ids::default(), None, |request| {
        let ClientRequest::ConsumeAccountRateLimitResetCredit { params, .. } = request else {
            panic!("expected redemption");
        };
        assert_eq!(params.credit_id.as_deref(), Some("credit"));
        assert!(!params.idempotency_key.is_empty());
        std::future::pending()
    })
    .await
    .expect("timeout is a result under the original ticket");
    assert_eq!(started.elapsed(), Duration::from_secs(30));
    assert_eq!(
        reply,
        codex_tui_core::rpc::Reply::failed("account/rateLimitResetCredit/consume timed out in TUI")
    );
}

#[tokio::test]
async fn an_unsupported_cloud_redemption_settles_without_sending_a_mutation() {
    let command = request_redemption();
    let Command::Screen { ticket, .. } = &command else {
        panic!("the pending redemption");
    };
    let expected = *ticket;
    let result = answer(
        command,
        Facts {
            cloud: true,
            worktrees: false,
        },
        Ids::default(),
        None,
        |_| -> std::future::Ready<Result<Value, ServerError>> {
            panic!("a secondary Cloud backend must not receive this mutation");
        },
    )
    .await;
    assert_eq!(
        result,
        Some((
            expected,
            codex_tui_core::rpc::Reply::failed("Usage resets are unavailable on this backend.",)
        ))
    );
}
