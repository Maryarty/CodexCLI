//! The transcript cell kinds beside messages and tools, projected once from
//! the protocol (R4). Expected values follow the oracle's history cells:
//! codex-rs/tui/src/history_cell/{messages,patches,search}.rs and
//! diff_render.rs.

#![allow(clippy::expect_used)]

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, apply_app};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::threads::{
    Item, Notice, PatchChange, PatchKind, PlanUpdate, StepStatus, ThreadEv, ToolStatus, WebAction,
    WebActivity, project,
};
use serde_json::{Value, json};

fn item(method: &str, item: Value) -> Option<Item> {
    let notification: ServerNotification = serde_json::from_value(json!({
        "method": method,
        "params": {"item": item, "threadId": "thread-1", "turnId": "turn-1",
            "completedAtMs": 0, "startedAtMs": 0}
    }))
    .expect("item notification");
    match project(&notification)?.1 {
        ThreadEv::ItemUpsert { item, .. } | ThreadEv::ItemStarted { item, .. } => Some(item),
        _ => None,
    }
}

fn completed(payload: Value) -> Item {
    item("item/completed", payload).expect("a transcript entry")
}

fn web(method: &str, query: &str, action: Value) -> WebActivity {
    match item(
        method,
        json!({"type": "webSearch", "id": "w", "query": query, "action": action}),
    ) {
        Some(Item::Web(web)) => web,
        other => panic!("not a web entry: {other:?}"),
    }
}

#[test]
fn file_change_maps_every_change_with_its_counts() {
    let Item::Patch(patch) = completed(json!({
        "type": "fileChange", "id": "patch-1", "status": "completed",
        "changes": [
            {"path": "new.txt", "kind": {"type": "add"}, "diff": "one\ntwo\nthree\n"},
            {"path": "existing.txt", "kind": {"type": "update", "move_path": "moved.txt"},
             "diff": "--- a\n+++ b\n@@ -1,3 +1,3 @@\n context\n-before\n+after\n tail\n"},
            {"path": "gone.txt", "kind": {"type": "delete"}, "diff": "x\ny\n"}
        ]
    })) else {
        panic!("not a patch")
    };
    assert_eq!(patch.status, ToolStatus::Completed);
    let kinds: Vec<_> = patch.changes.iter().map(|c| c.kind.clone()).collect();
    assert_eq!(
        kinds,
        [
            PatchKind::Update {
                move_path: Some("moved.txt".into())
            },
            PatchKind::Delete,
            PatchKind::Add,
        ],
        "by path, as diff_render.rs collect_rows sorts them"
    );
    let counts: Vec<_> = patch.changes.iter().map(PatchChange::counts).collect();
    assert_eq!(counts, [(1, 1), (0, 2), (3, 0)]);
}

#[test]
fn failed_file_change_keeps_its_status() {
    let Item::Patch(patch) = completed(json!({
        "type": "fileChange", "id": "patch-2", "changes": [], "status": "failed"
    })) else {
        panic!("not a patch")
    };
    assert_eq!(patch.status, ToolStatus::Failed);
}

fn reasoning(summary: Value) -> String {
    match completed(json!({"type": "reasoning", "id": "r", "content": [], "summary": summary})) {
        Item::Reasoning { summary } => summary,
        other => panic!("not a reasoning entry: {other:?}"),
    }
}

#[test]
fn reasoning_summary_drops_empty_html_comment_parts_and_the_header() {
    // oracle messages.rs split_reasoning_summary_parts: the header that opens
    // the content and ends its line is the status header, not the body.
    assert_eq!(
        reasoning(json!([
            "**Header**\n\n<!-- -->",
            "  ",
            "**Plan**\n\nWe should test."
        ])),
        "\n\nWe should test."
    );
}

#[test]
fn reasoning_summary_keeps_a_header_that_does_not_end_its_line() {
    assert_eq!(
        reasoning(json!(["**Important** conclusion"])),
        "**Important** conclusion"
    );
    assert_eq!(
        reasoning(json!(["no closing **header"])),
        "no closing **header"
    );
    // A header with no body: the header is the whole summary.
    assert_eq!(
        reasoning(json!(["**Confirming backend JSONL source**"])),
        "**Confirming backend JSONL source**"
    );
}

#[test]
fn web_search_labels_follow_the_action() {
    // oracle search.rs web_search_action_detail
    let open = web(
        "item/completed",
        "",
        json!({"type": "openPage", "url": "https://example.com/docs"}),
    );
    assert_eq!(open.action, WebAction::OpenPage);
    assert_eq!(open.detail, "https://example.com/docs");
    assert!(open.completed);

    let find = web(
        "item/completed",
        "",
        json!({"type": "findInPage", "url": "https://example.com", "pattern": "needle"}),
    );
    assert_eq!(find.action, WebAction::FindInPage { pattern: true });
    assert_eq!(find.detail, "'needle' in https://example.com");

    let queries = web(
        "item/completed",
        "",
        json!({"type": "search", "query": null, "queries": ["a", "b"]}),
    );
    assert_eq!(queries.detail, "a, b");

    let started = web("item/started", "", Value::Null);
    assert_eq!(started.action, WebAction::Search { known: false });
    assert!(!started.completed);
    assert_eq!(started.detail, "");

    let fallback = web("item/completed", "rust ratatui", json!({"type": "other"}));
    assert_eq!(fallback.detail, "rust ratatui");
}

#[test]
fn plan_image_view_and_notices_are_entries() {
    assert_eq!(
        completed(json!({"type": "plan", "id": "p", "text": "1. Do it"})),
        Item::Plan {
            text: "1. Do it".into()
        }
    );
    assert_eq!(
        completed(json!({"type": "imageView", "id": "i", "path": "/tmp/shot.png"})),
        Item::ImageView {
            path: "/tmp/shot.png".into(),
            name: "shot.png".into()
        }
    );
    assert_eq!(
        completed(json!({"type": "contextCompaction", "id": "c"})),
        Item::Notice(Notice::ContextCompacted)
    );
    assert_eq!(
        completed(json!({"type": "enteredReviewMode", "id": "e", "review": "current changes"})),
        Item::Notice(Notice::EnteredReview("current changes".into()))
    );
}

/// The app with its thread `thread-1` started, as after `Connected` and the reply.
fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let mut fx = AppFx::default();
    apply_app(&mut app, AppEv::Connected, &mut fx);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread command");
    apply_app(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
        &mut AppFx::default(),
    );
    app
}

fn send(app: &mut CodexApp, method: &str, item: Value) {
    let notification: ServerNotification = serde_json::from_value(json!({
        "method": method,
        "params": {"item": item, "threadId": "thread-1", "turnId": "turn-1",
            "completedAtMs": 0, "startedAtMs": 0}
    }))
    .expect("item notification");
    apply_app(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        &mut AppFx::default(),
    );
}

fn web_rows(app: &CodexApp) -> Vec<WebActivity> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Web(web) => Some(web.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_completed_web_search_follows_the_row_its_start_left() {
    // oracle tool_lifecycle.rs: item/completed runs `on_web_search_begin` (which commits the
    // cell item/started left active: `• Browsing the web`) and then `on_web_search_end`
    // (the completed cell, committed after it).
    let search =
        |action: Value| json!({"type": "webSearch", "id": "w", "query": "rust", "action": action});
    let mut app = ready();
    send(
        &mut app,
        "item/started",
        json!({"type": "webSearch", "id": "w", "query": ""}),
    );
    assert_eq!(web_rows(&app).len(), 1);
    send(
        &mut app,
        "item/completed",
        search(json!({"type": "search", "query": "rust"})),
    );
    let rows = web_rows(&app);
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(!rows[0].completed);
    assert_eq!(rows[0].action, WebAction::Search { known: false });
    assert_eq!(rows[0].detail, "");
    assert!(rows[1].completed);
    assert_eq!(rows[1].detail, "rust");
    // The same completion again changes the completed row, not the count.
    send(
        &mut app,
        "item/completed",
        search(json!({"type": "search", "query": "rust again"})),
    );
    let rows = web_rows(&app);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].detail, "rust again");
}

#[test]
fn a_completed_web_search_without_a_start_is_one_row() {
    // oracle replay (a stored thread): begin then end of one item leaves the completed cell only.
    let mut app = ready();
    send(
        &mut app,
        "item/completed",
        json!({"type": "webSearch", "id": "w", "query": "rust",
            "action": {"type": "search", "query": "rust"}}),
    );
    assert_eq!(web_rows(&app).len(), 1);
}

fn plan_update(app: &mut CodexApp, turn: &str, explanation: Option<&str>, steps: &[(&str, &str)]) {
    let plan: Vec<Value> = steps
        .iter()
        .map(|(step, status)| json!({"step": step, "status": status}))
        .collect();
    let notification: ServerNotification = serde_json::from_value(json!({
        "method": "turn/plan/updated",
        "params": {"threadId": "thread-1", "turnId": turn, "explanation": explanation,
            "plan": plan}
    }))
    .expect("plan notification");
    apply_app(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        &mut AppFx::default(),
    );
}

fn plan_rows(app: &CodexApp) -> Vec<(String, std::sync::Arc<PlanUpdate>)> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(key, item)| match item {
            Item::PlanUpdate(update) => Some((key.turn.to_string(), update.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn every_plan_update_is_an_entry_after_the_rows_before_it() {
    // oracle `on_plan_update`: each `turn/plan/updated` adds a `PlanUpdateCell`; two updates
    // are two cells, the later one never replaces the earlier.
    let mut app = ready();
    send(
        &mut app,
        "item/completed",
        json!({"type": "agentMessage", "id": "a1", "text": "Starting."}),
    );
    plan_update(
        &mut app,
        "turn-1",
        Some("Three small steps"),
        &[("Inspect", "inProgress"), ("Write", "pending")],
    );
    plan_update(
        &mut app,
        "turn-1",
        None,
        &[("Inspect", "completed"), ("Write", "inProgress")],
    );
    let kinds: Vec<&str> = app
        .active_thread()
        .transcript()
        .iter()
        .map(|(_, item)| match item {
            Item::Agent { .. } => "agent",
            Item::PlanUpdate(_) => "plan",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["agent", "plan", "plan"]);
    let rows = plan_rows(&app);
    assert_eq!(rows[0].1.explanation.as_deref(), Some("Three small steps"));
    assert_eq!(rows[0].1.steps[0].status, StepStatus::InProgress);
    assert_eq!(rows[0].1.steps[1].status, StepStatus::Pending);
    assert_eq!(rows[1].1.explanation, None);
    assert_eq!(rows[1].1.steps[0].status, StepStatus::Completed);
    assert_eq!(rows[1].1.steps[1].text, "Write");
    assert_eq!(rows[1].1.completed(), 1);
}

#[test]
fn a_plan_update_note_is_kept_only_when_it_has_text() {
    // oracle `display_hyperlink_lines`: the note is trimmed and an empty one is no note.
    let mut app = ready();
    plan_update(&mut app, "turn-1", Some("  \n "), &[("Only", "pending")]);
    plan_update(&mut app, "turn-1", Some(" Why \n"), &[]);
    let rows = plan_rows(&app);
    assert_eq!(rows[0].1.note(), None);
    assert_eq!(rows[1].1.note(), Some("Why"));
    assert!(rows[1].1.steps.is_empty());
}

#[test]
fn a_plan_update_of_another_turn_is_an_entry_of_that_turn() {
    let mut app = ready();
    plan_update(&mut app, "turn-1", None, &[("A", "pending")]);
    plan_update(&mut app, "turn-2", None, &[("B", "pending")]);
    let turns: Vec<String> = plan_rows(&app).into_iter().map(|(turn, _)| turn).collect();
    assert_eq!(turns, ["turn-1", "turn-2"]);
}
