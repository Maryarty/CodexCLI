//! The command center over two backends on two scripted servers: the first
//! is the local one, the second is the cloud, which lists nothing until it
//! is displayed (oracle: only the displayed session is refreshed) but owns
//! the tasks it listed. A command about a task goes to its owner; the lists
//! go to the displayed backend. Oracle: `agents_overview_tests.rs`
//! `overview_keeps_rows_and_routes_actions_to_the_owner`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::time::Duration;

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::ids::BackendId;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev, Page};
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::app::run::{localized, root};
use codex_tui_rtwk::l10n::Lang;
use codex_tui_rtwk::services::agents::workspace::Facts;
use codex_tui_rtwk::services::backend::fake::FakeServer;
use codex_tui_rtwk::services::shell::Backend;
use rtwk_test::{Options, Tester};
use serde_json::{Value, json};

const CLOUD: BackendId = BackendId(1);

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

fn thread_json(id: &str, name: &str) -> Value {
    json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_100, "recencyAt": 1_700_000_100, "status": {"type": "idle"},
        "path": null, "cwd": "/w/one", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    })
}

fn local(request: &Value) -> Option<Result<Value, ServerError>> {
    match request["method"].as_str()? {
        "thread/list" if !request["params"]["sectionId"].is_null() => {
            Some(Ok(json!({"data": [], "nextCursor": null})))
        }
        "thread/list" => Some(Ok(json!({
            "data": if request["params"]["sourceKinds"] == json!([]) {
                vec![thread_json("local-1", "Local task")]
            } else {
                Vec::new()
            },
            "nextCursor": null,
        }))),
        _ => None,
    }
}

/// The cloud answers the resume with a failure: what matters is who asked.
fn cloud(request: &Value) -> Option<Result<Value, ServerError>> {
    match request["method"].as_str()? {
        "thread/resume" => Some(Err(ServerError::from("cloud says no"))),
        _ => None,
    }
}

#[test]
fn a_command_about_a_task_goes_to_the_backend_that_owns_it() {
    let guard = rtwk_tokio::enter(runtime());
    let (first, local_fake) = FakeServer::new();
    local_fake.respond(local);
    let (second, cloud_fake) = FakeServer::new();
    cloud_fake.respond(cloud);
    cloud_fake.announce(Facts {
        cloud: true,
        worktrees: false,
    });
    let backend = Backend::Several {
        first: Box::new(Backend::Fake(first)),
        others: vec![(CLOUD, Backend::Fake(second))],
    };
    let app = UiApp::new(backend).with_start_agents(true);
    let mut t = Tester::with(Options::new().size(100, 30));
    t.pump_widget(localized(
        root(codex_tui_rtwk::app::run::app_view(app)),
        Lang::En,
    ));
    for _ in 0..30 {
        t.pump_for(Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(t.screen().contains("Local task"), "{}", t.screen());
    let epoch = local_fake.epoch().expect("attached");
    // The cloud lists a task; it belongs to the cloud from then on.
    let page = Page {
        threads: vec![serde_json::from_value(thread_json("cloud-1", "Cloud task")).unwrap()],
        unread: Vec::new(),
        next_cursor: None,
        append: false,
        pins: None,
    };
    local_fake.inject(
        epoch,
        AppEv::Screen(ScreenEv::Agents(Ev::ListedBy(CLOUD, page))),
    );
    for task in ["cloud-1", "local-1"] {
        local_fake.inject(
            epoch,
            AppEv::Intent(Intent::OpenThread(codex_tui_core::ids::ThreadId::new(task))),
        );
    }
    for _ in 0..30 {
        t.pump_for(Duration::from_millis(20));
        std::thread::sleep(Duration::from_millis(5));
    }
    let resumed = |requests: Vec<Value>| -> Vec<String> {
        requests
            .iter()
            .filter(|request| request["method"] == "thread/resume")
            .map(|request| request["params"]["threadId"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(resumed(cloud_fake.requests()), ["cloud-1"]);
    assert_eq!(resumed(local_fake.requests()), ["local-1"]);
    drop(guard);
}
