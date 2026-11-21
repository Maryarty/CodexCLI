//! The transcript cell kinds beside messages and tools, projected once from
//! the protocol (R4). Expected values follow the oracle's history cells:
//! codex-rs/tui/src/history_cell/{messages,patches,search}.rs and
//! diff_render.rs.

#![allow(clippy::expect_used)]

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::threads::{
    Item, Notice, PatchChange, PatchKind, ThreadEv, ToolStatus, WebAction, WebActivity, project,
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
