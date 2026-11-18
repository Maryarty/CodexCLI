//! The core half of the shell side of S21: discovery over two sources,
//! the directory keys the shell is asked for, and the worktree's task that
//! opens in the chat. Oracle: `agents_overview_discovery.rs` (`SourcePage`,
//! `next_batch`), `agents_overview_view.rs` (`directory_group_key`,
//! `AgentsOverviewProjectGroup`), `agents_overview_new.rs`. The expected
//! values are the oracle's: the merge is by `recency_at` then id, a source
//! keeps what the page did not show, cloud asks the interactive source only.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;
use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, apply_app};
use codex_tui_core::rpc::RpcError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{
    Command as AgentsCommand, DirKeys, Ev, Notice, Page, Reply, fanout,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const TIMESTAMP: i64 = 1_700_000_000;

fn thread(id: &str, recency: i64, cwd: &str) -> Thread {
    serde_json::from_value(json!({
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
    }))
    .expect("a thread")
}

/// A `thread/list` result of the given rows (`(id, recency)`) and cursor.
fn page(rows: &[(&str, i64)], next: Option<&str>) -> Result<Value, String> {
    let data: Vec<Value> = rows
        .iter()
        .map(|(id, recency)| serde_json::to_value(thread(id, *recency, "/tmp/p")).unwrap())
        .collect();
    Ok(json!({"data": data, "nextCursor": next, "backwardsCursor": null}))
}

fn ids(reply: Reply) -> (Vec<String>, Option<String>) {
    match reply {
        Reply::Listed(page) => (
            page.threads.into_iter().map(|thread| thread.id).collect(),
            page.next_cursor,
        ),
        other => panic!("not a page: {other:?}"),
    }
}

fn request_json(request: &ClientRequest) -> Value {
    serde_json::to_value(request).unwrap()
}

fn next_id() -> impl FnMut() -> RequestId {
    let mut n = 0;
    move || {
        n += 1;
        RequestId::Integer(n)
    }
}

fn list(cursor: Option<&str>) -> AgentsCommand {
    AgentsCommand::List {
        cursor: cursor.map(str::to_owned),
    }
}

/// `next_batch` asks the interactive source (no kinds) and the Exec and
/// AppServer one.
#[test]
fn the_list_asks_both_sources() {
    let fanout = fanout(&list(None), false).expect("a list fans out");
    let requests = fanout.requests(next_id());
    assert_eq!(requests.len(), 2);
    let first = request_json(&requests[0]);
    let second = request_json(&requests[1]);
    assert_eq!(first["method"], "thread/list");
    assert_eq!(first["params"]["sourceKinds"], json!([]));
    assert_eq!(
        second["params"]["sourceKinds"],
        json!(["exec", "appServer"])
    );
    for request in [&first, &second] {
        assert_eq!(request["params"]["limit"], 10);
        assert_eq!(request["params"]["sortKey"], "recency_at");
        assert_eq!(request["params"]["archived"], false);
    }
}

/// Cloud does not support the Exec / AppServer filter: it asks one source.
#[test]
fn cloud_asks_the_interactive_source_only() {
    let fanout = fanout(&list(None), true).expect("a list fans out");
    let requests = fanout.requests(next_id());
    assert_eq!(requests.len(), 1);
    assert_eq!(
        request_json(&requests[0])["params"]["sourceKinds"],
        json!([])
    );
}

/// Commands that are not pages are not fanned out.
#[test]
fn only_pages_fan_out() {
    assert!(fanout(&AgentsCommand::Usage { thread: "t".into() }, false).is_none());
    assert!(fanout(&AgentsCommand::Archive { thread: "t".into() }, false).is_none());
    assert!(
        fanout(
            &AgentsCommand::Search {
                cursor: None,
                first: true
            },
            false
        )
        .is_some()
    );
}

/// The merge pops the newer head of the two sources, the greater id on a tie.
#[test]
fn the_rows_merge_by_recency_then_id() {
    let fanout = fanout(&list(None), false).unwrap();
    let reply = fanout
        .reply(vec![
            page(&[("a1", 100), ("a2", 80), ("a3", 50)], None),
            page(&[("b1", 90), ("b2", 80), ("b3", 40)], None),
        ])
        .expect("merges");
    let (rows, next) = ids(reply);
    assert_eq!(rows, ["a1", "b1", "b2", "a2", "a3", "b3"]);
    assert_eq!(next, None, "both sources ended");
}

/// One list page shows `PAGE_SIZE` rows; what a source fetched beyond stays
/// with it (oracle: the source's retained rows) and the next page shows it.
#[test]
fn a_page_shows_ten_rows_and_the_sources_keep_the_rest() {
    let interactive: Vec<(String, i64)> = (0..10).map(|n| (format!("a{n}"), 200 - 2 * n)).collect();
    let other: Vec<(String, i64)> = (0..10).map(|n| (format!("b{n}"), 199 - 2 * n)).collect();
    fn as_refs(rows: &[(String, i64)]) -> Vec<(&str, i64)> {
        rows.iter().map(|(id, at)| (id.as_str(), *at)).collect()
    }
    let first = fanout(&list(None), false).unwrap();
    let (rows, next) = ids(first
        .reply(vec![
            page(&as_refs(&interactive), Some("ca")),
            page(&as_refs(&other), Some("cb")),
        ])
        .unwrap());
    assert_eq!(
        rows,
        ["a0", "b0", "a1", "b1", "a2", "b2", "a3", "b3", "a4", "b4"]
    );
    let next = next.expect("more rows remain");
    // The next page asks both sources again at the cursors that hold the
    // rows not shown yet, and shows those first.
    let second = fanout(&list(Some(&next)), false).unwrap();
    let requests = second.requests(next_id());
    assert_eq!(requests.len(), 2);
    assert_eq!(request_json(&requests[0])["params"]["cursor"], Value::Null);
    assert_eq!(request_json(&requests[1])["params"]["cursor"], Value::Null);
    let (rows, next) = ids(second
        .reply(vec![
            page(&as_refs(&interactive), Some("ca")),
            page(&as_refs(&other), Some("cb")),
        ])
        .unwrap());
    assert_eq!(
        rows,
        ["a5", "b5", "a6", "b6", "a7", "b7", "a8", "b8", "a9", "b9"]
    );
    // Everything fetched was shown: the sources continue from their cursors.
    let third = fanout(&list(next.as_deref()), false).unwrap();
    let requests = third.requests(next_id());
    assert_eq!(request_json(&requests[0])["params"]["cursor"], "ca");
    assert_eq!(request_json(&requests[1])["params"]["cursor"], "cb");
}

/// A source that ended is not asked again.
#[test]
fn an_ended_source_is_not_asked_again() {
    let first = fanout(&list(None), false).unwrap();
    let (_, next) = ids(first
        .reply(vec![
            page(&[("a1", 5)], None),
            page(&[("b1", 4)], Some("cb")),
        ])
        .unwrap());
    let second = fanout(&list(next.as_deref()), false).unwrap();
    let requests = second.requests(next_id());
    assert_eq!(requests.len(), 1);
    assert_eq!(
        request_json(&requests[0])["params"]["sourceKinds"],
        json!(["exec", "appServer"])
    );
    assert_eq!(request_json(&requests[0])["params"]["cursor"], "cb");
}

/// A failing source fails the page (the retry asks both again).
#[test]
fn a_failed_source_fails_the_page() {
    let fanout = fanout(&list(None), false).unwrap();
    let result = fanout.reply(vec![page(&[("a1", 5)], None), Err("boom".to_owned())]);
    assert_eq!(result, Err(RpcError::Remote("boom".to_owned())));
}

/// A search scans pages of 100 rows of each source, all merged.
#[test]
fn a_search_scans_hundred_row_pages_of_both_sources() {
    let command = AgentsCommand::Search {
        cursor: None,
        first: true,
    };
    let fanout = fanout(&command, false).unwrap();
    let requests = fanout.requests(next_id());
    assert_eq!(requests.len(), 2);
    assert_eq!(request_json(&requests[0])["params"]["limit"], 100);
    assert_eq!(request_json(&requests[1])["params"]["limit"], 100);
    let rows: Vec<(String, i64)> = (0..12).map(|n| (format!("a{n}"), 500 - n)).collect();
    let refs: Vec<(&str, i64)> = rows.iter().map(|(id, at)| (id.as_str(), *at)).collect();
    let (merged, next) = ids(fanout
        .reply(vec![page(&refs, Some("ca")), page(&[("b1", 495)], None)])
        .unwrap());
    assert_eq!(
        merged.len(),
        13,
        "a search page is not cut at the page size"
    );
    assert!(next.is_some());
}

fn keys(project: &str, heading: &str, directory: &str) -> DirKeys {
    DirKeys {
        project: (project.to_owned(), String::new()),
        heading: heading.to_owned(),
        directory: directory.to_owned(),
    }
}

fn run(app: &mut CodexApp, ev: Ev) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, AppEv::Screen(ScreenEv::Agents(ev)), &mut fx);
    fx
}

fn listed(rows: Vec<Thread>) -> Ev {
    Ev::Listed(Page {
        threads: rows,
        unread: Vec::new(),
        next_cursor: None,
        append: false,
    })
}

fn directories(fx: &AppFx) -> Vec<Vec<String>> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Directories { cwds, .. } => Some(cwds.clone()),
            _ => None,
        })
        .collect()
}

/// The keys are read for the cwds of listed tasks, each once, and only with
/// worktrees on (oracle: `directory_group_key`, `for_thread`).
#[test]
fn the_shell_is_asked_the_directory_keys_of_listed_cwds_once() {
    let mut app = CodexApp::new();
    // Worktrees off: nothing to read.
    let fx = run(&mut app, listed(vec![thread("t1", 9, "/w/one")]));
    assert!(directories(&fx).is_empty());
    // Switching worktrees on is itself an event the center reacts to: the
    // task already listed is asked for.
    let fx = run(
        &mut app,
        Ev::Backend {
            cloud: false,
            worktrees: true,
        },
    );
    assert_eq!(directories(&fx), [vec!["/w/one".to_owned()]]);
    let fx = run(
        &mut app,
        listed(vec![
            thread("t1", 9, "/w/one"),
            thread("t2", 8, "/w/two"),
            thread("t3", 7, "/w/two"),
        ]),
    );
    assert_eq!(directories(&fx), [vec!["/w/two".to_owned()]]);
    // The same rows again: nothing new.
    let fx = run(&mut app, listed(vec![thread("t1", 9, "/w/one")]));
    assert!(directories(&fx).is_empty());
    // A new cwd is asked on its own.
    let fx = run(&mut app, listed(vec![thread("t4", 6, "/w/three")]));
    assert_eq!(directories(&fx), [vec!["/w/three".to_owned()]]);
}

/// The keys the shell sends group the rows (the oracle's
/// `worktrees_overview_groups_by_project_or_directory`, here through
/// the event).
#[test]
fn directory_keys_from_the_shell_reach_the_model() {
    let mut app = CodexApp::new();
    run(
        &mut app,
        Ev::Backend {
            cloud: false,
            worktrees: true,
        },
    );
    run(
        &mut app,
        Ev::Directories(vec![(
            "/w/one".into(),
            keys("/repo/.git", "/repo", "/real/one"),
        )]),
    );
    let model = &app.screens.agents;
    assert_eq!(model.dirs["/w/one"].heading, "/repo");
    assert!(model.worktrees);
}

/// A task the shell started (a worktree's) opens in the chat and ends the
/// busy notice (oracle: `start_agents_overview_session`).
#[test]
fn a_started_task_opens_in_the_chat_and_ends_the_notice() {
    let mut app = CodexApp::new();
    run(&mut app, Ev::Notice(Some(Notice::CreatingWorktree)));
    run(
        &mut app,
        Ev::Started(Box::new(thread("new-1", 10, "/wt/checkout"))),
    );
    assert_eq!(
        app.session
            .active
            .as_ref()
            .map(codex_tui_core::ids::ThreadId::as_str),
        Some("new-1")
    );
    let model = &app.screens.agents;
    assert_eq!(model.current.as_deref(), Some("new-1"));
    assert_eq!(model.notice, None);
    assert_eq!(model.resumes, 1);
}

/// A failure the shell reports shows in the center, replacing the busy notice.
#[test]
fn a_failed_worktree_replaces_the_busy_notice_with_the_error() {
    let mut app = CodexApp::new();
    run(&mut app, Ev::Notice(Some(Notice::CreatingWorktree)));
    run(
        &mut app,
        Ev::Notice(Some(Notice::Error(
            "The source project is not trusted.".into(),
        ))),
    );
    assert_eq!(
        app.screens.agents.notice,
        Some(Notice::Error("The source project is not trusted.".into()))
    );
}

/// `codex agents` opens the overview instead of starting a thread (oracle:
/// `cli.rs` `agents_overview`, "open the agents overview instead of starting
/// a thread"): connecting starts none, so no `Untitled task` row exists.
#[test]
fn an_agents_start_does_not_start_a_thread() {
    use codex_tui_core::app::Session;
    let mut app = CodexApp::new();
    app.start_thread_on_connect = false;
    let mut fx = AppFx::default();
    apply_app(&mut app, AppEv::Connected, &mut fx);
    assert!(
        !fx.into_commands()
            .iter()
            .any(|command| matches!(command, Command::StartThread { .. })),
        "no thread is started"
    );
    assert_eq!(app.session.status, Session::Ready);
    assert_eq!(app.session.active, None);

    let mut chat = CodexApp::new();
    let mut fx = AppFx::default();
    apply_app(&mut chat, AppEv::Connected, &mut fx);
    assert!(
        fx.into_commands()
            .iter()
            .any(|command| matches!(command, Command::StartThread { .. })),
        "the chat starts its first thread"
    );
}
