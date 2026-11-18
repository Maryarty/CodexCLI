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
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::agents::{DirKeys, Ev, Intent as AgentsIntent, Notice};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use codex_tui_rtwk::shell::fake::FakeWorkspace;
use codex_tui_rtwk::shell::rpc::{Ids, USAGE_TIMED_OUT, USAGE_TIMEOUT, answer, answer_with};
use codex_tui_rtwk::shell::search::{Gate, Latest, SEARCH_DEBOUNCE};
use codex_tui_rtwk::shell::workspace::{
    Boxed, Checkout, Facts, Workspace, account, backend_ev, directories, worktree,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

#[allow(clippy::duplicate_mod)] // one helper, a module in each slice file
#[path = "oracle_snap/mod.rs"]
mod oracle_snap;

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
    assert_eq!(wire["method"], "thread/start");
    let cwd = wire["params"]["cwd"].as_str().unwrap_or("/none").to_owned();
    Ok(json!({"thread": thread_json("new-1", 500, &cwd)}))
}

fn make_worktree(
    workspace: &Arc<FakeWorkspace>,
    cwd: Option<&str>,
    send: impl Fn(ClientRequest) -> std::future::Ready<Result<Value, ServerError>>,
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
        app.session
            .active
            .as_ref()
            .map(codex_tui_core::ids::ThreadId::as_str),
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
                AgentsIntent::SetSearch(line.clone()),
            ))),
        );
        sent.extend(screen_commands(fx));
    }
    sent
}

fn searches(app: &mut CodexApp, text: &str) -> Vec<Command> {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
            AgentsIntent::BeginSearch,
        ))),
    );
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
            assert!((&mut waiting).now_or_never().is_none(), "asked before the pause");
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
    use codex_tui_rtwk::l10n::Lang;
    use codex_tui_rtwk::run::{app_view, localized, root};
    use codex_tui_rtwk::shell::Backend;
    use codex_tui_rtwk::shell::fake::{FakeHandle, FakeServer};
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
        /// shell's tasks, until a few rounds in a row change nothing.
        pub fn settle(&mut self) {
            for _ in 0..10 {
                self.t.pump_for(ms(20));
                std::thread::yield_now();
            }
        }

        pub fn wait_for(&mut self, what: &str, ready: impl Fn(&str) -> bool) {
            for _ in 0..400 {
                self.t.pump_for(ms(20));
                if ready(&self.t.screen()) {
                    return;
                }
                std::thread::sleep(ms(5));
            }
            panic!("never saw {what}; the screen:\n{}", self.t.screen());
        }
    }
}

fn server(request: &Value) -> Option<Result<Value, ServerError>> {
    match request["method"].as_str()? {
        "thread/list" if !request["params"]["sectionId"].is_null() => Some(Ok(page(&[]))),
        "thread/list" => Some(Ok(if request["params"]["sourceKinds"] == json!([]) {
            page(&[("Interactive task", 100, "/w/one")])
        } else {
            page(&[("Exec task", 90, "/w/two")])
        })),
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
    h.t.press("e");
    h.t.press("x");
    h.wait_for("the typed text", |s| s.contains("Search \u{203a} ex"));
    assert_eq!(scans(), before, "nothing is asked before the pause");
    // The pause ends: only the search that was left asks, once per source.
    gate.release();
    h.wait_for("the search", |_| scans() > before);
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
    h.t.press("e");
    h.wait_for("the search line", |s| s.contains("Search \u{203a} e"));
    h.t.press("ctrl+c");
    h.wait_for("the line closed", |s| !s.contains("Search \u{203a}"));
    assert!(h.t.is_running(), "the first Ctrl+C only closed the line");
    h.t.press("ctrl+c");
    assert!(!h.t.is_running(), "the second one quits");
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
        h.t.pump_for(Duration::from_millis(50));
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
        asked.lock().unwrap().push(serde_json::to_value(&request).unwrap());
        std::future::ready(Ok(account_body("business")))
    }));
    run(&mut app, ev.expect("an event"));
    let asked = asked.into_inner().unwrap();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0]["method"], "account/read");
    assert_eq!(asked[0]["params"]["refreshToken"], false);
    assert!(agents(&app).backend_auth);
    assert_eq!(agents(&app).plan, Some(serde_json::from_value(json!("business")).unwrap()));
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
    let ev = make_worktree(&workspace, None, |request| {
        let wire = serde_json::to_value(&request).unwrap();
        let answer = match wire["method"].as_str() {
            Some("thread/start") => started(&request),
            _ => Ok(json!({})),
        };
        sent.lock().unwrap().push(wire);
        std::future::ready(answer)
    });
    assert!(matches!(notice_of(ev), Some(Notice::Error(_))));
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
        sent.lock().unwrap().push(serde_json::to_value(&request).unwrap());
        std::future::ready(started(&request))
    });
    assert_eq!(sent.into_inner().unwrap().len(), 1);
}
