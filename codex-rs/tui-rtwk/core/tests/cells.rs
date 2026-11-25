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
    AgentOutcome, AgentRef, BlockBody, CollabEvent, DelegatedMessage, DynamicOutput,
    GeneratedImage, HookEntryKind, HookRun, HookStatus, Item, Notice, PatchChange, PatchKind,
    PlanUpdate, StepStatus, SubAgentKind, ThreadEv, ToolKind, ToolStatus, WebAction, WebActivity,
    project, project_unrouted,
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
        Item::Reasoning { summary, .. } => summary,
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

fn reasoning_with_raw(summary: Value, content: Value) -> (String, Option<String>) {
    match completed(json!({"type": "reasoning", "id": "r", "content": content, "summary": summary}))
    {
        Item::Reasoning { summary, with_raw } => (summary, with_raw),
        other => panic!("not a reasoning entry: {other:?}"),
    }
}

#[test]
fn reasoning_without_content_has_no_raw_form() {
    // oracle chatwidget/streaming.rs replace_reasoning_parts: `show_raw_agent_reasoning` adds the
    // `content` parts; an item without any has nothing to add, and the view shows the summary.
    let (summary, with_raw) = reasoning_with_raw(json!(["**Plan**\n\nSteps."]), json!([]));
    assert_eq!(with_raw, None);
    let item = Item::Reasoning { summary, with_raw };
    assert_eq!(item.reasoning_text(true), "\n\nSteps.");
    assert_eq!(item.reasoning_text(false), "\n\nSteps.");
}

#[test]
fn reasoning_raw_content_follows_the_summary_parts_and_is_split_with_them() {
    // The live and snapshot form joins the raw parts to the summary parts, then splits: the
    // leading bold line of the whole is the status header, a heading of the raw part stays.
    let (summary, with_raw) = reasoning_with_raw(
        json!(["**Plan**\n\nSteps."]),
        json!(["**Raw heading**\n\nthinking", "more"]),
    );
    assert_eq!(summary, "\n\nSteps.");
    assert_eq!(
        with_raw.as_deref(),
        Some("\n\nSteps.\n\n**Raw heading**\n\nthinking\n\nmore")
    );
    let item = Item::Reasoning { summary, with_raw };
    assert_eq!(item.reasoning_text(false), "\n\nSteps.");
    assert_eq!(
        item.reasoning_text(true),
        "\n\nSteps.\n\n**Raw heading**\n\nthinking\n\nmore"
    );
    // No summary part: the raw parts lead and their first bold line is the status header.
    let (_, only_raw) = reasoning_with_raw(json!([]), json!(["**Raw heading**\n\nthinking"]));
    assert_eq!(only_raw.as_deref(), Some("\n\nthinking"));
}

#[test]
fn a_loaded_turns_raw_reasoning_keeps_its_own_heading() {
    // oracle thread_transcript.rs thread_items_to_transcript_cells: the split summary, then the
    // content joined by blank lines as it is, so its heading is not taken for a status header.
    let mut app = ready();
    let turn: codex_app_server_protocol::Turn = serde_json::from_value(json!({
        "id": "turn-old", "itemsView": "full", "status": "completed",
        "items": [{"type": "reasoning", "id": "r1", "summary": [],
            "content": ["**Raw heading**\n\nthinking", "more"]}]
    }))
    .expect("turn");
    codex_tui_core::app::apply_thread(
        &mut app,
        &ThreadId::new("thread-1"),
        codex_tui_core::threads::history_page(&[turn], None),
        &mut AppFx::default(),
    );
    let items: Vec<_> = app
        .active_thread()
        .transcript()
        .iter()
        .filter(|(_, item)| matches!(item, Item::Reasoning { .. }))
        .map(|(_, item)| {
            (
                item.reasoning_text(false).to_owned(),
                item.reasoning_text(true).to_owned(),
            )
        })
        .collect();
    assert_eq!(
        items,
        [(
            String::new(),
            "**Raw heading**\n\nthinking\n\nmore".to_owned()
        )]
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
        completed(json!({"type": "enteredReviewMode", "id": "e", "review": "current changes"})),
        Item::Notice(Notice::EnteredReview("current changes".into()))
    );
}

#[test]
fn a_completed_context_compaction_is_a_notice_made_on_completion() {
    // the status header owns the start; the cell comes with the completion (`compaction.rs`)
    let mut app = ready();
    send(
        &mut app,
        "item/completed",
        json!({"type": "contextCompaction", "id": "c"}),
    );
    let notices: Vec<Item> = app
        .active_thread()
        .transcript()
        .iter()
        .filter(|(_, item)| matches!(item, Item::Notice(_)))
        .map(|(_, item)| item.clone())
        .collect();
    assert_eq!(
        notices,
        [Item::Notice(Notice::ContextCompacted { took: None })]
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

fn hook_notification(turn: Option<&str>, status: &str, entries: Value) -> ServerNotification {
    serde_json::from_value(json!({
        "method": "hook/completed",
        "params": {"threadId": "thread-1", "turnId": turn, "run": {
            "id": "hook-1", "eventName": "sessionStart", "handlerType": "command",
            "executionMode": "sync", "scope": "turn", "sourcePath": "/workspace/.codex/hooks.json",
            "source": "user", "displayOrder": 0, "status": status, "statusMessage": null,
            "startedAt": 0, "completedAt": 1, "durationMs": 1, "entries": entries}}
    }))
    .expect("hook notification")
}

fn hook_rows(app: &CodexApp) -> Vec<std::sync::Arc<HookRun>> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Hook(run) => Some(run.clone()),
            _ => None,
        })
        .collect()
}

fn hook_applied(app: &mut CodexApp, turn: Option<&str>, status: &str, entries: Value) {
    apply_app(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(hook_notification(
            turn, status, entries,
        )))),
        &mut AppFx::default(),
    );
}

#[test]
fn a_completed_hook_keeps_the_entries_the_screen_shows_and_never_its_context() {
    // oracle hook_cell.rs: model-facing context never appears in the TUI.
    let mut app = ready();
    hook_applied(
        &mut app,
        Some("turn-1"),
        "stopped",
        json!([{"kind": "stop", "text": "first\nsecond"},
            {"kind": "context", "text": "Private"}, {"kind": "warning", "text": "Heads up"}]),
    );
    let rows = hook_rows(&app);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, HookStatus::Stopped);
    let entries: Vec<_> = rows[0]
        .entries
        .iter()
        .map(|entry| (entry.kind, entry.text.as_str()))
        .collect();
    assert_eq!(
        entries,
        [
            (HookEntryKind::Stop, "first\nsecond"),
            (HookEntryKind::Warning, "Heads up")
        ]
    );
}

#[test]
fn a_quiet_hook_leaves_no_entry() {
    // oracle `hook_run_is_quiet_success`: completed with context only (or nothing) is silent;
    // a started or running hook is the activity line, not a row.
    let mut app = ready();
    hook_applied(
        &mut app,
        Some("turn-1"),
        "completed",
        json!([{"kind": "context", "text": "x"}]),
    );
    hook_applied(&mut app, Some("turn-1"), "completed", json!([]));
    hook_applied(
        &mut app,
        Some("turn-1"),
        "running",
        json!([{"kind": "warning", "text": "w"}]),
    );
    assert!(hook_rows(&app).is_empty());
    // A failed hook with nothing to say still says it failed.
    hook_applied(&mut app, Some("turn-1"), "failed", json!([]));
    let rows = hook_rows(&app);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, HookStatus::Failed);
    assert!(rows[0].entries.is_empty());
}

#[test]
fn a_hook_without_a_turn_follows_the_last_row() {
    let mut app = ready();
    send(
        &mut app,
        "item/completed",
        json!({"type": "agentMessage", "id": "a1", "text": "Hello."}),
    );
    hook_applied(
        &mut app,
        None,
        "blocked",
        json!([{"kind": "feedback", "text": "no"}]),
    );
    let kinds: Vec<&str> = app
        .active_thread()
        .transcript()
        .iter()
        .map(|(key, item)| match item {
            Item::Agent { .. } => "agent",
            Item::Hook(_) => {
                assert_eq!(key.turn.as_str(), "turn-1");
                "hook"
            }
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["agent", "hook"]);
}

// ---- multi-agent events (oracle multi_agents.rs) ----

const ROBIE: &str = "00000000-0000-0000-0000-000000000002";
const BOB: &str = "00000000-0000-0000-0000-000000000003";

fn collab(method: &str, call: Value) -> Option<(String, std::sync::Arc<CollabEvent>)> {
    let mut base = json!({"type": "collabAgentToolCall", "id": "call-1",
        "senderThreadId": "00000000-0000-0000-0000-000000000001",
        "receiverThreadIds": [], "prompt": null, "model": null,
        "reasoningEffort": null, "agentsStates": {}});
    base.as_object_mut()
        .expect("object")
        .extend(call.as_object().expect("object").clone());
    let notification: ServerNotification = serde_json::from_value(json!({
        "method": method,
        "params": {"item": base, "threadId": "thread-1", "turnId": "turn-1",
            "completedAtMs": 0, "startedAtMs": 0}
    }))
    .expect("collab notification");
    match project(&notification)?.1 {
        ThreadEv::ItemStarted {
            key,
            item: Item::Collab(event),
        }
        | ThreadEv::ItemUpsert {
            key,
            item: Item::Collab(event),
        } => Some((key.item.to_string(), event)),
        other => panic!("not a collab entry: {other:?}"),
    }
}

fn agent(id: &str) -> AgentRef {
    AgentRef {
        id: id.to_owned(),
        nickname: None,
        role: None,
    }
}

#[test]
fn a_spawn_is_a_row_when_it_ends_and_carries_the_request_and_prompt() {
    let spawn = |method, status| {
        collab(
            method,
            json!({"tool": "spawnAgent", "status": status,
            "receiverThreadIds": [ROBIE], "prompt": "  Compute 11!  ",
            "model": "gpt-5", "reasoningEffort": "high"}),
        )
    };
    assert_eq!(spawn("item/started", "inProgress"), None);
    let (key, event) = spawn("item/completed", "completed").expect("a row");
    assert_eq!(key, "call-1");
    assert_eq!(
        *event,
        CollabEvent::Spawned {
            agent: agent(ROBIE),
            request: Some(codex_tui_core::threads::SpawnRequest {
                model: "gpt-5".into(),
                effort: "high".into()
            }),
            prompt: Some("Compute 11!".into()),
        }
    );
}

#[test]
fn a_spawn_without_a_receiver_failed() {
    let (_, event) = collab(
        "item/completed",
        json!({"tool": "spawnAgent", "status": "failed"}),
    )
    .expect("a row");
    assert_eq!(*event, CollabEvent::SpawnFailed { prompt: None });
}

#[test]
fn a_long_prompt_is_cut_to_the_oracle_preview() {
    let prompt = "x".repeat(300);
    let (_, event) = collab(
        "item/completed",
        json!({"tool": "sendInput", "status": "completed",
        "receiverThreadIds": [ROBIE], "prompt": prompt}),
    )
    .expect("a row");
    let CollabEvent::SentInput {
        prompt: Some(prompt),
        ..
    } = &*event
    else {
        panic!("{event:?}")
    };
    assert_eq!(prompt.chars().count(), 160);
    assert!(prompt.ends_with("..."));
}

#[test]
fn a_wait_is_two_rows_the_start_stays_when_the_end_arrives() {
    let (begin_key, begin) = collab(
        "item/started",
        json!({"tool": "wait", "status": "inProgress",
        "receiverThreadIds": [ROBIE, BOB]}),
    )
    .expect("the start");
    let (end_key, end) = collab(
        "item/completed",
        json!({"tool": "wait", "status": "completed",
        "receiverThreadIds": [ROBIE, BOB],
        "agentsStates": {ROBIE: {"status": "completed", "message": "  39916800\n"},
            BOB: {"status": "errored", "message": "tool timeout"}}}),
    )
    .expect("the end");
    assert_ne!(begin_key, end_key);
    assert_eq!(
        *begin,
        CollabEvent::Waiting {
            agents: vec![agent(ROBIE), agent(BOB)]
        }
    );
    assert_eq!(
        *end,
        CollabEvent::FinishedWaiting {
            agents: vec![
                (
                    agent(ROBIE),
                    AgentOutcome::Completed(Some("39916800".into()))
                ),
                (
                    agent(BOB),
                    AgentOutcome::Errored(Some("tool timeout".into()))
                ),
            ]
        }
    );
}

#[test]
fn a_wait_that_nobody_finished_lists_no_agent() {
    let (_, end) = collab(
        "item/completed",
        json!({"tool": "wait", "status": "completed"}),
    )
    .expect("the end");
    assert_eq!(*end, CollabEvent::FinishedWaiting { agents: vec![] });
}

#[test]
fn a_resume_is_two_rows_and_a_failed_one_has_no_state() {
    let (_, begin) = collab(
        "item/started",
        json!({"tool": "resumeAgent", "status": "inProgress",
        "receiverThreadIds": [ROBIE]}),
    )
    .expect("the start");
    assert_eq!(
        *begin,
        CollabEvent::Resuming {
            agent: agent(ROBIE)
        }
    );
    let (_, end) = collab(
        "item/completed",
        json!({"tool": "resumeAgent", "status": "failed",
        "receiverThreadIds": [ROBIE]}),
    )
    .expect("the end");
    assert_eq!(
        *end,
        CollabEvent::Resumed {
            agent: agent(ROBIE),
            outcome: None
        }
    );
}

#[test]
fn the_v2_tools_leave_no_row_of_their_own() {
    for tool in [
        "sendMessage",
        "followupTask",
        "interruptAgent",
        "listAgents",
    ] {
        assert_eq!(
            collab(
                "item/completed",
                json!({"tool": tool, "status": "completed",
                "receiverThreadIds": [ROBIE]})
            ),
            None,
            "{tool}"
        );
    }
}

#[test]
fn a_sub_agent_activity_is_a_row_with_its_path() {
    let notification: ServerNotification = serde_json::from_value(json!({
        "method": "item/completed",
        "params": {"item": {"type": "subAgentActivity", "id": "activity-1", "kind": "started",
            "agentThreadId": ROBIE, "agentPath": "/root/helper", "model": null,
            "reasoningEffort": null},
            "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0, "startedAtMs": 0}
    }))
    .expect("activity");
    let Some((
        _,
        ThreadEv::ItemUpsert {
            item: Item::Collab(event),
            ..
        },
    )) = project(&notification)
    else {
        panic!("no collab entry")
    };
    assert_eq!(
        *event,
        CollabEvent::Activity {
            kind: SubAgentKind::Started,
            path: "/root/helper".into()
        }
    );
}

// ---- delegated messages, generated images and the items the oracle shows nothing for ----

fn completed_item(payload: Value) -> Option<Item> {
    item("item/completed", payload)
}

fn delegated_output(name: &str, namespace: Option<&str>, output: &str) -> Value {
    json!({"type": "functionCallOutput", "id": "out-1", "name": name,
        "namespace": namespace, "output": output})
}

const DELEGATION: &str = "<codex_delegation>\n  <source_thread_id>other</source_thread_id>\n  <input>- One &amp; &lt;two&gt;</input>\n</codex_delegation>";

#[test]
fn a_delegation_the_task_tools_wrote_is_a_message_with_its_source() {
    for (name, namespace) in [
        ("create_thread", "codex_tui"),
        ("send_message_to_thread", "codex_app"),
    ] {
        let Some(Item::Delegated(message)) =
            completed_item(delegated_output(name, Some(namespace), DELEGATION))
        else {
            panic!("{name} in {namespace} is not delegated");
        };
        assert_eq!(
            *message,
            DelegatedMessage {
                source: "other".into(),
                markdown: "- One & <two>".into()
            }
        );
    }
}

#[test]
fn other_tool_output_is_not_a_delegation() {
    for item in [
        delegated_output("create_thread", None, DELEGATION),
        delegated_output("create_thread", Some("other_namespace"), DELEGATION),
        delegated_output("list_threads", Some("codex_tui"), DELEGATION),
        delegated_output("create_thread", Some("codex_tui"), "plain text"),
    ] {
        assert_eq!(completed_item(item.clone()), None, "{item}");
    }
}

fn image(status: &str, revised: Value, saved: Value, started: bool) -> Option<Item> {
    let payload = json!({"type": "imageGeneration", "id": "call-image-generation",
        "status": status, "revisedPrompt": revised, "savedPath": saved, "result": "aGk="});
    item(
        if started {
            "item/started"
        } else {
            "item/completed"
        },
        payload,
    )
}

#[test]
fn a_finished_image_generation_shows_its_prompt_and_where_it_was_saved_as_a_url() {
    let Some(Item::Generated(done)) = image(
        "completed",
        json!("A tiny blue square"),
        json!("/tmp/my images/generated #1.png"),
        false,
    ) else {
        panic!("no generated image");
    };
    assert_eq!(
        *done,
        GeneratedImage {
            failed: false,
            detail: "A tiny blue square".into(),
            saved_to: Some("file:///tmp/my%20images/generated%20%231.png".into()),
        }
    );
}

#[test]
fn a_failed_generation_without_a_prompt_names_the_call() {
    let Some(Item::Generated(failed)) = image("failed", Value::Null, Value::Null, false) else {
        panic!("no generated image");
    };
    assert_eq!(
        *failed,
        GeneratedImage {
            failed: true,
            detail: "call-image-generation".into(),
            saved_to: None
        }
    );
}

#[test]
fn a_generation_still_running_leaves_no_row() {
    assert_eq!(image("inProgress", Value::Null, Value::Null, true), None);
}

#[test]
fn sleep_and_hook_prompt_items_leave_no_row() {
    // oracle replay.rs handle_thread_item: `Sleep(_) => {}`, `HookPrompt { .. } => {}`.
    assert_eq!(
        completed_item(json!({"type": "sleep", "id": "sleep-1", "durationMs": 1500})),
        None
    );
    assert_eq!(
        completed_item(json!({"type": "hookPrompt", "id": "hook-1", "fragments": []})),
        None
    );
}

fn notified(app: &mut CodexApp, notification: Value) {
    let notification: ServerNotification =
        serde_json::from_value(notification).expect("notification");
    apply_app(
        app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        &mut AppFx::default(),
    );
}

fn turn_started(app: &mut CodexApp) {
    notified(
        app,
        json!({"method": "turn/started", "params": {"threadId": "thread-1", "turn": {
            "id": "turn-1", "items": [], "itemsView": "full", "status": "inProgress",
            "error": null}}}),
    );
}

fn error_notice(app: &mut CodexApp, message: &str, will_retry: bool) {
    notified(
        app,
        json!({"method": "error", "params": {"threadId": "thread-1", "turnId": "turn-1",
            "willRetry": will_retry, "error": {"message": message}}}),
    );
}

fn turn_ended(app: &mut CodexApp, status: &str, error: Option<&str>) {
    notified(
        app,
        json!({"method": "turn/completed", "params": {"threadId": "thread-1", "turn": {
            "id": "turn-1", "items": [], "itemsView": "full", "status": status,
            "error": error.map(|message| json!({"message": message}))}}}),
    );
}

fn error_rows(app: &CodexApp) -> Vec<String> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Error { message } => Some(message.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn an_error_notification_leaves_its_message_in_the_transcript() {
    // oracle chatwidget/turn_runtime.rs on_error: `add_to_history(new_error_event(message))`.
    let mut app = ready();
    turn_started(&mut app);
    error_notice(&mut app, "Quota exceeded.", false);
    assert_eq!(error_rows(&app), ["Quota exceeded."]);
    assert_eq!(app.active_thread().last_error(), None);
}

#[test]
fn an_error_that_will_be_retried_leaves_no_row() {
    // oracle protocol.rs: a retried error is the status header's, not a cell.
    let mut app = ready();
    turn_started(&mut app);
    error_notice(&mut app, "Reconnecting... 1/5", true);
    assert_eq!(error_rows(&app), Vec::<String>::new());
}

#[test]
fn the_failed_turn_that_repeats_the_error_adds_no_second_row() {
    // oracle protocol.rs: `last_non_retry_error == Some((turn, message))` ends the turn quietly.
    let mut app = ready();
    turn_started(&mut app);
    error_notice(&mut app, "Quota exceeded.", false);
    turn_ended(&mut app, "failed", Some("Quota exceeded."));
    assert_eq!(error_rows(&app), ["Quota exceeded."]);
}

#[test]
fn a_failed_turn_with_another_message_adds_it() {
    let mut app = ready();
    turn_started(&mut app);
    error_notice(&mut app, "Quota exceeded.", false);
    turn_ended(&mut app, "failed", Some("The turn failed."));
    assert_eq!(error_rows(&app), ["Quota exceeded.", "The turn failed."]);
}

#[test]
fn a_failed_turn_the_server_explains_only_at_its_end_shows_that_message() {
    let mut app = ready();
    turn_started(&mut app);
    turn_ended(&mut app, "failed", Some("Quota exceeded."));
    assert_eq!(error_rows(&app), ["Quota exceeded."]);
}

#[test]
fn a_failed_turn_without_a_message_shows_nothing() {
    // oracle protocol.rs: no error on the turn, `finalize_turn` only.
    let mut app = ready();
    turn_started(&mut app);
    turn_ended(&mut app, "failed", None);
    assert_eq!(error_rows(&app), Vec::<String>::new());
    assert_eq!(app.active_thread().last_error(), None);
}

#[test]
fn a_whole_turn_snapshot_keeps_the_rows_the_server_has_no_item_for() {
    // The turn/completed payload of a turn lists the server's items only: the error row, a hook
    // run and a plan report are ours and stay.
    let mut app = ready();
    turn_started(&mut app);
    error_notice(&mut app, "Quota exceeded.", false);
    hook_applied(
        &mut app,
        Some("turn-1"),
        "stopped",
        json!([{"kind": "stop", "text": "Halt"}]),
    );
    turn_ended(&mut app, "failed", Some("Quota exceeded."));
    assert_eq!(error_rows(&app), ["Quota exceeded."]);
    assert_eq!(hook_rows(&app).len(), 1);
}

#[test]
fn a_warning_notification_is_a_detailed_transcript_row() {
    // oracle history_cell/notices.rs new_warning_event: `visible_in_transcript: false`, its
    // `transcript_lines` is the `⚠ ` message.
    let mut app = ready();
    turn_started(&mut app);
    notified(
        &mut app,
        json!({"method": "warning", "params": {"threadId": "thread-1", "message": "Careful."}}),
    );
    let rows: Vec<_> = app
        .active_thread()
        .transcript()
        .iter()
        .filter(|(_, item)| matches!(item, Item::Warning { message } if message == "Careful."))
        .map(|(_, item)| item.is_detailed_only())
        .collect();
    assert_eq!(rows, [true]);
}

#[test]
fn a_loaded_failed_turn_shows_its_error_after_its_items() {
    // oracle thread_transcript.rs: `turn.error` is a cell after the turn's items.
    let mut app = ready();
    let turn: codex_app_server_protocol::Turn = serde_json::from_value(json!({
        "id": "turn-old", "itemsView": "full", "status": "failed",
        "error": {"message": "Quota exceeded."},
        "items": [{"type": "agentMessage", "id": "agent-1", "text": "Before the failure",
            "phase": "final_answer", "memoryCitation": null, "delivery": null}]
    }))
    .expect("turn");
    codex_tui_core::app::apply_thread(
        &mut app,
        &ThreadId::new("thread-1"),
        codex_tui_core::threads::history_page(&[turn], None),
        &mut AppFx::default(),
    );
    let kinds: Vec<_> = app
        .active_thread()
        .transcript()
        .iter()
        .map(|(_, item)| match item {
            Item::Agent { text, .. } => text.clone(),
            Item::Error { message } => format!("error: {message}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["Before the failure", "error: Quota exceeded."]);
}

#[test]
fn a_loaded_interrupted_turn_ends_with_the_interruption_notice() {
    // oracle chatwidget/replay.rs replay_thread_turns: an interrupted turn is completed as
    // `on_interrupted_server_turn` does, so the notice a live interrupt gets closes its rows; a
    // hidden nested review turn counts as completed and gets none.
    let mut app = ready();
    let message = |id: &str, text: &str| {
        json!({"type": "userMessage", "id": id, "clientId": null,
            "content": [{"type": "text", "text": text, "text_elements": []}]})
    };
    let turn = |id: &str,
                status: &str,
                completed: Value,
                items: Value|
     -> codex_app_server_protocol::Turn {
        serde_json::from_value(json!({
            "id": id, "itemsView": "full", "status": status, "completedAt": completed, "items": items
        }))
        .expect("turn")
    };
    let review = |id: &str| {
        turn(
            id,
            "completed",
            Value::Null,
            json!([
                {"type": "enteredReviewMode", "id": format!("{id}-start"), "review": "changes"},
                {"type": "exitedReviewMode", "id": format!("{id}-end"), "review": "done"}
            ]),
        )
    };
    // the server lists the newest turn first
    let turns = [
        turn(
            "turn-nested",
            "interrupted",
            Value::Null,
            json!([message("nested-1", "prompt"), message("nested-2", "prompt")]),
        ),
        review("turn-review"),
        turn(
            "turn-stopped",
            "interrupted",
            json!(1),
            json!([message("typed", "hello")]),
        ),
    ];
    codex_tui_core::app::apply_thread(
        &mut app,
        &ThreadId::new("thread-1"),
        codex_tui_core::threads::history_page(&turns, None),
        &mut AppFx::default(),
    );
    let rows: Vec<_> = app
        .active_thread()
        .transcript()
        .iter()
        .map(|(_, item)| match item {
            Item::User(prompt) => format!("user: {}", prompt.text),
            Item::Notice(Notice::Interrupted) => "interrupted".to_owned(),
            Item::Notice(Notice::EnteredReview(_)) => "review started".to_owned(),
            Item::Notice(Notice::ExitedReview(_)) => "review finished".to_owned(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        rows,
        [
            "user: hello",
            "interrupted",
            "review started",
            "review finished"
        ]
    );
}

fn unrouted(notification: Value) -> Option<ThreadEv> {
    let notification: ServerNotification =
        serde_json::from_value(notification).expect("notification");
    project_unrouted(&notification)
}

#[test]
fn a_deprecation_notice_is_a_detailed_row_of_the_thread_the_app_picks() {
    // oracle chatwidget/protocol_requests.rs on_deprecation_notice: `new_deprecation_notice`, whose
    // `display_lines` is empty and whose `transcript_lines` is the `⚠ ` summary and the details.
    let ev = unrouted(json!({"method": "deprecationNotice", "params": {
        "summary": "Feature flag `foo`", "details": "Use flag `bar` instead."}}))
    .expect("an event for the active thread");
    let mut app = ready();
    codex_tui_core::app::apply_thread(
        &mut app,
        &ThreadId::new("thread-1"),
        ev,
        &mut AppFx::default(),
    );
    let rows: Vec<_> = app
        .active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Deprecation { summary, details } => {
                Some((summary.clone(), details.clone(), item.is_detailed_only()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        rows,
        [(
            "Feature flag `foo`".to_string(),
            Some("Use flag `bar` instead.".to_string()),
            true
        )]
    );
}

#[test]
fn a_deprecation_notice_without_details_has_none() {
    let ev = unrouted(json!({"method": "deprecationNotice", "params": {
        "summary": "Old flag", "details": null}}))
    .expect("an event");
    assert!(matches!(
        ev,
        ThreadEv::Appended {
            turn: None,
            item: Item::Deprecation { details: None, .. }
        }
    ));
}

#[test]
fn a_notification_with_a_thread_is_the_threads_not_the_actives() {
    assert!(
        unrouted(json!({"method": "warning", "params": {
            "threadId": "thread-2", "message": "Careful."}}))
        .is_none()
    );
    // Without one, the app's active thread shows it (oracle: the chat widget's warning).
    assert!(matches!(
        unrouted(json!({"method": "warning", "params": {"message": "Careful."}})),
        Some(ThreadEv::Appended {
            item: Item::Warning { .. },
            ..
        })
    ));
}

// ---- host fact: how a web link's destination is written (oracle markdown_render/web_links.rs
// `WebLinkDisplay`). The oracle's default is the destination after the label, for a terminal
// not known to show hyperlinks (and for any run that is not a terminal); the host says
// otherwise once it has detected one.

mod link_display {
    use codex_tui_core::fixtures::sandbox::Sandbox;
    use codex_tui_core::screens::cells::LinkDisplay;
    use pretty_assertions::assert_eq;

    const LABEL_ONLY: &str =
        r#"{"step":"screen","dir":"cells","ev":{"kind":"link_display","display":"label_only"}}"#;
    const WITH_DESTINATION: &str = r#"{"step":"screen","dir":"cells","ev":{"kind":"link_display","display":"with_destination"}}"#;

    fn display(script: &str) -> LinkDisplay {
        Sandbox::play(script)
            .expect("plays")
            .app()
            .screens()
            .cells
            .link_display
    }

    #[test]
    fn a_web_link_writes_its_destination_until_the_host_says_the_terminal_shows_hyperlinks() {
        assert_eq!(display(""), LinkDisplay::WithDestination);
        assert_eq!(display(LABEL_ONLY), LinkDisplay::LabelOnly);
        assert_eq!(
            display(&format!("{LABEL_ONLY}\n{WITH_DESTINATION}")),
            LinkDisplay::WithDestination
        );
    }
}

// ---- MCP and dynamic calls (oracle history_cell/{mcp,mcp_result,dynamic}.rs):
// the result is projected once into what the rows show ----

fn mcp(
    server: &str,
    status: &str,
    arguments: Value,
    result: Value,
    error: Value,
) -> codex_tui_core::threads::McpCall {
    let Item::Tool(tool) = completed(json!({
        "type": "mcpToolCall", "id": "m", "server": server, "tool": "js", "status": status,
        "arguments": arguments, "result": result, "error": error
    })) else {
        panic!("not a tool")
    };
    match &tool.kind {
        ToolKind::Mcp(call) => call.clone(),
        other => panic!("not an MCP call: {other:?}"),
    }
}

fn blocks(content: Value) -> Value {
    json!({"content": content, "structuredContent": null, "_meta": null, "isError": false})
}

#[test]
fn an_mcp_result_keeps_what_each_block_shows() {
    let call = mcp(
        "docs",
        "completed",
        json!({}),
        blocks(json!([
            {"type": "text", "text": "plain\n\n"},
            {"type": "image", "data": "AAAA", "mimeType": "image/png"},
            {"type": "audio", "data": "AAAA", "mimeType": "audio/wav"},
            {"type": "resource", "resource": {"uri": "file:///a.txt", "text": "body"}},
            {"type": "resource_link", "uri": "https://x.test/a", "name": "a"},
            {"type": "mystery", "x": 1}
        ])),
        Value::Null,
    );
    let Some(Ok(result)) = &call.result else {
        panic!("no result")
    };
    let shown: Vec<BlockBody<'_>> = result
        .content
        .iter()
        .map(|block| block.render_full())
        .collect();
    assert_eq!(
        shown,
        [
            BlockBody::Text("plain".into()),
            BlockBody::Image,
            BlockBody::Audio,
            BlockBody::Resource("file:///a.txt"),
            BlockBody::Link("https://x.test/a"),
            BlockBody::Text(r#"{"type":"mystery","x":1}"#.into()),
        ]
    );
    assert!(result.has_image && result.content[1].is_image && !result.is_error);
    assert_eq!(call.success(), Some(true));
}

#[test]
fn an_mcp_call_reads_as_failed_or_running_as_the_oracle_decides() {
    // The status, not the wire `isError`, sets the flag.
    let failed = mcp("docs", "failed", json!({}), blocks(json!([])), Value::Null);
    assert_eq!(failed.success(), Some(false));
    // An error wins over a result.
    let errored = mcp(
        "docs",
        "failed",
        json!({}),
        blocks(json!([])),
        json!({"message": "interrupted"}),
    );
    assert_eq!(errored.result, Some(Err("interrupted".to_owned())));
    let bare = mcp("docs", "completed", json!({}), Value::Null, Value::Null);
    assert_eq!(
        bare.result,
        Some(Err("MCP tool call completed without a result".to_owned()))
    );
    let running = mcp("docs", "inProgress", json!({}), Value::Null, Value::Null);
    assert_eq!((running.result.is_none(), running.success()), (true, None));
}

#[test]
fn code_mode_titles_outputs_and_computer_activity_follow_the_server() {
    let ok = blocks(json!([
        {"type": "text", "text": "Script completed\nOutput:\nrows"},
        {"type": "text", "text": "{\"exit_code\":0,\"output\":\"from json\"}"},
        {"type": "image", "data": "AAAA", "mimeType": "image/png", "text": "caption"}
    ]));
    let node = mcp(
        "node_repl",
        "completed",
        json!({"title": "  Inspect \n page "}),
        ok.clone(),
        Value::Null,
    );
    assert_eq!(node.title(), "Inspect page");
    assert!(!node.is_computer_activity());
    let Some(Ok(result)) = &node.result else {
        panic!("no result")
    };
    let previews: Vec<_> = result
        .content
        .iter()
        .map(|block| node.preview_text(block))
        .collect();
    assert_eq!(previews[0], BlockBody::Text("rows".into()));
    assert_eq!(previews[1], BlockBody::Text("from json".into()));
    // A non-text block still reads its top-level text in code mode.
    assert_eq!(result.content[2].text(), Some("caption"));
    // Another server never reads a title.
    let other = mcp("docs", "completed", json!({"title": "x"}), ok, Value::Null);
    assert_eq!(other.title(), "docs.js");
    let cua = mcp(
        "cua_repl",
        "inProgress",
        json!({"title": "Open"}),
        Value::Null,
        Value::Null,
    );
    assert!(cua.is_computer_activity());
    assert_eq!(cua.title(), "Open");
}

#[test]
fn a_dynamic_call_keeps_its_name_status_and_media_markers() {
    let Item::Tool(tool) = completed(json!({
        "type": "dynamicToolCall", "id": "d", "namespace": "example", "tool": "inspect",
        "arguments": {"path": "a"}, "status": "completed", "success": false, "durationMs": 7,
        "contentItems": [
            {"type": "inputText", "text": "Found"},
            {"type": "inputImage", "imageUrl": "data:image/png;base64,AAAA"}
        ]
    })) else {
        panic!("not a tool")
    };
    let ToolKind::Dynamic(call) = &tool.kind else {
        panic!("not dynamic")
    };
    assert_eq!(call.name, "example.inspect");
    // `success: false` fails a completed call.
    assert_eq!(call.status, ToolStatus::Failed);
    assert_eq!(
        call.output.as_deref(),
        Some(
            &[
                DynamicOutput::Text("Found".to_owned()),
                DynamicOutput::Image
            ][..]
        )
    );
    assert_eq!(call.duration_ms, Some(7));
}

#[test]
fn a_computer_failure_reads_as_its_first_line_of_words() {
    let failed = |content: Value| {
        mcp(
            "cua_repl",
            "failed",
            json!({"title": " "}),
            blocks(content),
            Value::Null,
        )
    };
    // A `Script error:` block wins over earlier text; its prefix and blank lines go.
    let call = failed(json!([
        {"type": "text", "text": "Page title"},
        {"type": "text", "text": "Script error:\n\n  permission denied \nmore"}
    ]));
    assert_eq!(call.error_preview(), Some("permission denied"));
    // Without one, the first text with words.
    let call = failed(json!([
        {"type": "text", "text": "  \n"},
        {"type": "text", "text": "Browser state changed\nRe-query the browser"}
    ]));
    assert_eq!(call.error_preview(), Some("Browser state changed"));
    // A transport error is the message; a call still running has none.
    let call = mcp(
        "cua_repl",
        "failed",
        json!({}),
        Value::Null,
        json!({"message": "interrupted"}),
    );
    assert_eq!(call.error_preview(), Some("interrupted"));
    assert_eq!(call.action_title(), None);
    let running = mcp(
        "cua_repl",
        "inProgress",
        json!({"title": "Open"}),
        Value::Null,
        Value::Null,
    );
    assert_eq!(
        (running.error_preview(), running.action_title()),
        (None, Some("Open"))
    );
}

// ---- host facts of the separators (oracle history_cell/separators.rs): the viewer's wall clock
// (12/24-hour setting; a fixed zone only where the machine's must not matter) and a turn's
// runtime metrics. The oracle's defaults hold until the host says otherwise: 12-hour time in the
// process's local zone, and no metrics.

mod separator_facts {
    use codex_tui_core::fixtures::sandbox::Sandbox;
    use codex_tui_core::ids::TurnId;
    use codex_tui_core::screens::cells::{ClockFormat, RuntimeMetrics, Totals, ViewerClock};
    use pretty_assertions::assert_eq;

    const TWENTY_FOUR: &str = r#"{"step":"screen","dir":"cells","ev":{"kind":"clock","format":"twenty_four_hour","utc_offset":0}}"#;
    const METRICS: &str = r#"{"step":"screen","dir":"cells","ev":{"kind":"runtime_metrics","turn":"turn-1","metrics":{"tool_calls":{"count":3,"duration_ms":2450},"responses_api_engine_iapi_tbt_ms":1.5}}}"#;
    const NEWER: &str = r#"{"step":"screen","dir":"cells","ev":{"kind":"runtime_metrics","turn":"turn-1","metrics":{"api_calls":{"count":1,"duration_ms":10}}}}"#;

    fn turn() -> TurnId {
        TurnId::new("turn-1")
    }

    fn cells(script: &str) -> codex_tui_core::screens::cells::Model {
        Sandbox::play(script)
            .expect("plays")
            .app()
            .screens()
            .cells
            .clone()
    }

    #[test]
    fn the_clock_is_twelve_hour_local_time_until_the_host_says_otherwise() {
        assert_eq!(
            cells("").clock,
            ViewerClock {
                format: ClockFormat::TwelveHour,
                utc_offset: None
            }
        );
        assert_eq!(
            cells(TWENTY_FOUR).clock,
            ViewerClock {
                format: ClockFormat::TwentyFourHour,
                utc_offset: Some(0)
            }
        );
    }

    #[test]
    fn a_turns_metrics_are_kept_by_turn_and_the_latest_totals_replace_the_earlier() {
        assert!(cells("").runtime_metrics.is_empty());
        let one = cells(METRICS);
        assert_eq!(
            one.runtime_metrics[&turn()],
            RuntimeMetrics {
                tool_calls: Totals {
                    count: 3,
                    duration_ms: 2450
                },
                responses_api_engine_iapi_tbt_ms: 1.5,
                ..RuntimeMetrics::default()
            }
        );
        let later = cells(&format!("{METRICS}\n{NEWER}"));
        assert_eq!(later.runtime_metrics.len(), 1);
        assert_eq!(later.runtime_metrics[&turn()].tool_calls, Totals::default());
        assert_eq!(later.runtime_metrics[&turn()].api_calls.count, 1);
    }
}

/// What the transcript shows of a user's message (`sanitize_user_text`,
/// history_cell/messages.rs): CSI sequences and control characters removed,
/// tabs and newlines kept, the inline elements dropped when a removal moved
/// their byte ranges.
mod user_prompt_shown {
    use super::*;

    fn prompt(text: &str, elements: Value) -> codex_tui_core::threads::UserPrompt {
        let item = item(
            "item/started",
            json!({"type": "userMessage", "id": "u", "clientId": null, "content": [
                {"type": "text", "text": text, "text_elements": elements}]}),
        );
        match item {
            Some(Item::User(prompt)) => prompt,
            other => panic!("a user prompt, got {other:?}"),
        }
    }

    fn bounds(elements: &[std::ops::Range<usize>]) -> Vec<(usize, usize)> {
        elements
            .iter()
            .map(|range| (range.start, range.end))
            .collect()
    }

    #[test]
    fn a_control_sequence_and_a_control_character_are_removed() {
        let raw = "_count_r\u{1b}[13;2:3uows\tx\u{7}y\nz";
        let prompt = prompt(raw, json!([]));
        assert_eq!(prompt.text, raw, "the text the model has is not changed");
        assert_eq!(prompt.shown().text, "_count_rows\txy\nz");
    }

    #[test]
    fn a_lone_escape_goes_and_an_unfinished_sequence_takes_the_rest() {
        assert_eq!(prompt("a\u{1b}b", json!([])).shown().text, "ab");
        assert_eq!(prompt("a\u{1b}[31", json!([])).shown().text, "a");
        assert_eq!(
            prompt("a\u{1b}[31mb\u{1b}[0m", json!([])).shown().text,
            "ab"
        );
    }

    #[test]
    fn a_clean_text_is_borrowed_and_keeps_its_elements() {
        let elements = json!([{"byteRange": {"start": 0, "end": 3}, "placeholder": "abc"}]);
        let prompt = prompt("abc def", elements);
        let shown = prompt.shown();
        assert!(matches!(shown.text, std::borrow::Cow::Borrowed(_)));
        assert_eq!(bounds(&shown.elements), [(0, 3)]);
    }

    #[test]
    fn a_changed_text_drops_the_elements_whose_ranges_it_moved() {
        let elements = json!([{"byteRange": {"start": 4, "end": 7}, "placeholder": "def"}]);
        let prompt = prompt("\u{7}abc def", elements);
        assert_eq!(bounds(&prompt.elements), [(4, 7)]);
        let shown = prompt.shown();
        assert_eq!(shown.text, "abc def");
        assert!(shown.elements.is_empty());
    }
}
