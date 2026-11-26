//! State tests of the command center's task details (feature-parity review
//! `agents-feature-parity.md`, items 1, 3 and 6), ported from the oracle
//! (`agents_overview_tests.rs`: `agents_overview_reasoning_uses_existing_events_
//! and_expires_with_attachment`, the `ThreadReverted` block of the refresh
//! test; `agents_overview_threads.rs`: the refresh's reads). Expected values
//! are the oracle's.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::ReasoningSummaryTextDeltaNotification;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadRevertedNotification;
use codex_app_server_protocol::ThreadStatus;
use codex_app_server_protocol::ThreadStatusChangedNotification;
use codex_tui_core::rpc::RpcError;
use codex_tui_core::screens::agents::{
    Command, Ctx, DetailLine, Lens, Model, Out, Page, Reply, apply_answer, decode,
    observe as observe_screen,
};
use codex_tui_core::threads::{ThreadAsk, ThreadPreviews};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const TIMESTAMP: i64 = 1_700_000_000;

fn id(n: u128) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

/// The oracle's `overview_thread` (`agents_overview_tests.rs:474`): a root
/// named `name`, or a child of `parent` (the source says so), in `/tmp/project`.
fn thread(id: &str, parent: Option<&str>, name: &str, status: Value) -> Thread {
    let source = parent.map_or(json!("cli"), |parent| {
        json!({"subAgent": {"thread_spawn": {"parent_thread_id": parent, "depth": 1,
            "agent_path": null, "agent_nickname": null, "agent_role": null}}})
    });
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": parent.unwrap_or(id),
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": TIMESTAMP,
        "updatedAt": TIMESTAMP, "recencyAt": TIMESTAMP, "status": status, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": source,
        "canAcceptDirectInput": parent.is_none(), "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn idle() -> Value {
    json!({"type": "idle"})
}
fn working() -> Value {
    json!({"type": "active", "activeFlags": []})
}

fn listed(m: &mut Model, threads: &mut ThreadPreviews, rows: Vec<Thread>) -> Vec<Out> {
    apply_answer(
        m,
        threads,
        Ctx::List { append: false },
        Ok(Reply::Listed(Page {
            threads: rows,
            unread: Vec::new(),
            next_cursor: None,
            append: false,
            pins: None,
        })),
    )
}

fn reads(outs: &[Out]) -> Vec<String> {
    outs.iter()
        .filter_map(|out| match &out.command {
            Command::Thread(ThreadAsk::Read { thread }) => Some(thread.clone()),
            _ => None,
        })
        .collect()
}

fn details_of(m: &Model, threads: &ThreadPreviews) -> codex_tui_core::screens::agents::Details {
    m.center_at(
        &codex_tui_core::screens::agents::CenterThreads::new(
            None,
            threads.clone(),
            Default::default(),
        ),
        None,
        &Lens::default(),
    )
    .rows[0]
        .details
        .clone()
}

fn observe(
    model: &mut Model,
    threads: &mut ThreadPreviews,
    notification: &ServerNotification,
) -> Vec<Out> {
    threads.observe(notification, true);
    observe_screen(model, notification)
}

fn delta(thread: &str, item: &str, text: &str) -> ServerNotification {
    ServerNotification::ReasoningSummaryTextDelta(ReasoningSummaryTextDeltaNotification {
        thread_id: thread.to_owned(),
        turn_id: "turn".into(),
        item_id: item.into(),
        summary_index: 0,
        delta: text.to_owned(),
    })
}

fn turn_reply(user: &str, agent: &str) -> Reply {
    let result = json!({"data": [{
        "id": "turn-1", "rootTurnId": null, "itemsView": "full", "status": "completed",
        "error": null, "startedAt": null, "completedAt": null, "durationMs": null,
        "items": [
            {"type": "userMessage", "id": "u", "clientId": null,
             "content": [{"type": "text", "text": user, "text_elements": []}]},
            {"type": "agentMessage", "id": "a", "text": "first", "phase": null},
            {"type": "agentMessage", "id": "b", "text": agent, "phase": null},
        ]
    }], "nextCursor": null});
    decode(
        &Command::Thread(ThreadAsk::Turns { thread: "t".into() }),
        result,
    )
    .expect("a reply")
}

fn read_reply(thread: Thread, unread: bool) -> Result<Reply, RpcError> {
    Ok(Reply::Read {
        thread: Box::new(thread),
        unread,
    })
}

/// Item 1 (the person's case): a page asks `thread/read` of each of its recent
/// rows and `thread/loaded/list` once (oracle: `start_agents_overview_refresh`,
/// `agents_overview_threads.rs:380-509`).
#[test]
fn a_first_page_reads_its_rows_and_asks_the_loaded_list() {
    let mut m = Model::default();
    let mut threads = ThreadPreviews::default();
    let rows = vec![
        thread(&id(1), None, "", idle()),
        thread(&id(2), None, "B", idle()),
    ];
    let outs = listed(&mut m, &mut threads, rows.clone());
    assert_eq!(reads(&outs), vec![id(2), id(1)]);
    assert!(outs.iter().any(|out| out.command == Command::Loaded));
    // A refresh asks again, but the loaded list is asked once.
    let outs = listed(&mut m, &mut threads, rows);
    assert!(!outs.iter().any(|out| out.command == Command::Loaded));
}

/// Item 1: the row becomes the thread as read (its preview), the prompt is the
/// first user message of the last turn, the last message the last answer
/// (`agents_overview.rs:269-275`, `agents_overview_threads.rs:421-509`).
#[test]
fn the_read_fills_preview_prompt_and_last_message() {
    let mut m = Model::default();
    let mut threads = ThreadPreviews::default();
    listed(&mut m, &mut threads, vec![thread(&id(1), None, "", idle())]);
    assert_eq!(details_of(&m, &threads).latest_prompt, None);
    let mut read = thread(&id(1), None, "", idle());
    read.preview = "Read preview".into();
    let outs = apply_answer(
        &mut m,
        &mut threads,
        Ctx::Read { thread: id(1) },
        read_reply(read, true),
    );
    assert_eq!(outs.len(), 1);
    assert_eq!(
        outs[0].command,
        Command::Thread(ThreadAsk::Turns { thread: id(1) })
    );
    assert!(m.unread.contains(&id(1)));
    assert_eq!(
        m.center_at(
            &codex_tui_core::screens::agents::CenterThreads::new(
                None,
                threads.clone(),
                Default::default()
            ),
            None,
            &Lens::default()
        )
        .rows[0]
            .preview,
        "Read preview"
    );
    let outs = apply_answer(
        &mut m,
        &mut threads,
        Ctx::Turns { thread: id(1) },
        Ok(turn_reply("Fix the bug", "Done.")),
    );
    assert!(outs.is_empty());
    let details = details_of(&m, &threads);
    assert_eq!(details.latest_prompt.as_deref(), Some("Fix the bug"));
    assert_eq!(
        details.last_message,
        Some(("Done.".into(), "/tmp/project".into()))
    );
}

/// Item 1: a failed read ends the chain (the oracle logs and goes on).
#[test]
fn a_failed_read_ends_the_chain() {
    let mut m = Model::default();
    let mut threads = ThreadPreviews::default();
    listed(
        &mut m,
        &mut threads,
        vec![thread(&id(1), None, "A", idle())],
    );
    assert!(
        apply_answer(
            &mut m,
            &mut threads,
            Ctx::Read { thread: id(1) },
            Err(RpcError::Mismatch)
        )
        .is_empty()
    );
    assert!(m.reading.is_empty());
}

/// Item 1: a notification that changes a task reads it again; during a read
/// the re-read waits for the end of the chain (oracle: `refresh_thread_ids`).
#[test]
fn a_changed_task_is_read_again_after_the_read_in_flight() {
    let mut m = Model::default();
    let mut threads = ThreadPreviews::default();
    listed(
        &mut m,
        &mut threads,
        vec![thread(&id(1), None, "A", idle())],
    );
    let status = ServerNotification::ThreadStatusChanged(ThreadStatusChangedNotification {
        thread_id: id(1),
        status: serde_json::from_value::<ThreadStatus>(idle()).expect("a status"),
    });
    // The page's read is in flight: nothing new is sent, the change is kept.
    assert!(observe(&mut m, &mut threads, &status).is_empty());
    assert!(m.reread.contains(id(1).as_str()));
    apply_answer(
        &mut m,
        &mut threads,
        Ctx::Read { thread: id(1) },
        read_reply(thread(&id(1), None, "A", idle()), false),
    );
    let outs = apply_answer(
        &mut m,
        &mut threads,
        Ctx::Turns { thread: id(1) },
        Ok(turn_reply("p", "m")),
    );
    assert_eq!(reads(&outs), vec![id(1)]);
    // With no chain running a change reads at once.
    apply_answer(
        &mut m,
        &mut threads,
        Ctx::Read { thread: id(1) },
        Err(RpcError::Mismatch),
    );
    assert_eq!(reads(&observe(&mut m, &mut threads, &status)), vec![id(1)]);
}

/// Item 3 (oracle `agents_overview_reasoning_uses_existing_events_and_expires_
/// with_attachment`): deltas of one summary accumulate until the first bold
/// run closes; an oversized header never grows the buffer past 512 chars; a new
/// summary replaces the old.
#[test]
fn latest_activity_is_the_first_bold_run_across_deltas() {
    let mut m = Model::default();
    let mut threads = ThreadPreviews::default();
    listed(
        &mut m,
        &mut threads,
        vec![thread(&id(1), None, "Check startup", working())],
    );
    for text in ["**Checking", " cold-start regressions**\nFurther reasoning"] {
        observe(&mut m, &mut threads, &delta(&id(1), "reasoning", text));
    }
    assert_eq!(
        details_of(&m, &threads).lines,
        vec![
            DetailLine::Blank,
            DetailLine::LatestActivity("Checking cold-start regressions".into())
        ]
    );
    for text in [format!("**{}", "界".repeat(10_000)), "**".to_owned()] {
        observe(&mut m, &mut threads, &delta(&id(1), "oversized", &text));
    }
    let details = details_of(&m, &threads);
    assert_eq!((details.lines, details.last_message), (Vec::new(), None));
    observe(
        &mut m,
        &mut threads,
        &delta(&id(1), "next", "**Running tests**"),
    );
    assert_eq!(
        details_of(&m, &threads).lines,
        vec![
            DetailLine::Blank,
            DetailLine::LatestActivity("Running tests".into())
        ]
    );
}

/// Item 6 (oracle: the `ThreadReverted` block of the refresh test): the
/// history was rewound; prompt, last message, activity and tokens go.
#[test]
fn a_reverted_thread_resets_prompt_last_message_and_tokens() {
    let mut m = Model::default();
    let mut threads = ThreadPreviews::default();
    listed(
        &mut m,
        &mut threads,
        vec![thread(&id(1), None, "A", idle())],
    );
    apply_answer(
        &mut m,
        &mut threads,
        Ctx::Turns { thread: id(1) },
        Ok(turn_reply("p", "m")),
    );
    m.usage.entry(id(1)).or_default().tokens = Some((1, 2));
    observe(&mut m, &mut threads, &delta(&id(1), "r", "**x**"));
    assert!(threads.get(id(1).as_str()).is_some());
    let outs = observe(
        &mut m,
        &mut threads,
        &ServerNotification::ThreadReverted(ThreadRevertedNotification { thread_id: id(1) }),
    );
    assert!(outs.is_empty());
    assert!(threads.get(id(1).as_str()).is_none());
    assert_eq!(m.usage[&id(1)].tokens, None);
}

/// No turn, no previews.
#[test]
fn a_turnless_thread_has_no_previews() {
    let reply = decode(
        &Command::Thread(ThreadAsk::Turns { thread: "t".into() }),
        json!({"data": [], "nextCursor": null}),
    )
    .expect("a reply");
    assert_eq!(
        reply,
        Reply::Turns {
            prompt: None,
            last_message: None
        }
    );
}
