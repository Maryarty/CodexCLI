// test root: pages
//! The status row's reasoning header (oracle `chatwidget/streaming.rs`
//! `restore_reasoning_status_header`): it follows the model's summary, and sits after what the
//! oracle lets own the row first: a wait for a background terminal, a compaction.

use crate::kit::{Owner, Screen};

const WAITING: &str = include_str!(
    "../oracle/fixtures/chat_approvals_1/unified_exec_wait_status_renders_command_in_single_details_row_snapshot.jsonl"
);
const COMPACTING: &str = include_str!("../oracle/fixtures/chat_live_2/compaction_running.jsonl");

/// A reasoning item of `turn-1` that said `summary`.
fn reasoning(summary: &str) -> String {
    serde_json::json!({"step": "notify", "message": {"method": "item/completed", "params": {
        "threadId": "thread-1", "turnId": "turn-1", "completedAtMs": 0,
        "item": {"type": "reasoning", "id": summary, "summary": [summary], "content": []}}}})
    .to_string()
}

#[test]
fn the_row_reads_the_reasoning_header_instead_of_working() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 12)
        .script(WAITING)
        .open();
    // the wait owns the row; the reasoning waits behind it
    s.feed(&reasoning("**Checking tests**"));
    s.assert_contains("• Waiting for background terminal");
    s.assert_absent("Checking tests");
    // the next thing the turn does ends the wait, and the model's words take the row
    s.feed(
        r#"{"step":"notify","message":{"method":"item/agentMessage/delta","params":{"threadId":"thread-1","turnId":"turn-1","itemId":"msg-1","delta":"Done waiting\n"}}}"#,
    );
    s.assert_contains("Checking tests (");
    s.assert_absent("Working");
}

#[test]
fn a_compaction_keeps_its_word_over_the_reasoning_header() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 12)
        .script(COMPACTING)
        .open();
    s.feed(&reasoning("**Checking tests**"));
    s.assert_contains("• Compacting");
    s.assert_absent("Checking tests");
}
