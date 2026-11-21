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
    AgentOutcome, AgentRef, CollabEvent, DelegatedMessage, GeneratedImage, HookEntryKind, HookRun,
    HookStatus, Item, Notice, PatchChange, PatchKind, PlanUpdate, StepStatus, SubAgentKind,
    ThreadEv, ToolStatus, WebAction, WebActivity, project, project_unrouted,
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
            Item::Agent { text } => text.clone(),
            Item::Error { message } => format!("error: {message}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["Before the failure", "error: Quota exceeded."]);
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
