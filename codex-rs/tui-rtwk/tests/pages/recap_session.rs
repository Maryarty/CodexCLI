// test root: pages
//! `/recap` in the session page (oracle `app/recap.rs`, `chatwidget/recap.rs`, `history_cell`
//! `ThreadRecapLoadingCell` and `ThreadRecapHistoryCell`): the loading row while the recap is
//! generated, the recap cell (`↳ Recap: ` hanging over the summary, `Next: ` over the action) and
//! the refusal of a cloud conversation. The flow's own steps are `core/tests/recap.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

use crate::kit::{Owner, Screen};

/// The conversation, and what the temporary thread's three requests are answered with: the
/// servers to disable, the thread (read-only, as the request asked) and the turn.
const SCRIPT: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"notify","message":{"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"startedAtMs":0,"item":{"type":"userMessage","id":"u0","clientId":null,"content":[{"type":"text","text":"fix the bug","text_elements":[]}]}}}}
{"step":"notify","message":{"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"startedAtMs":0,"item":{"type":"agentMessage","id":"a0","text":"done","phase":null}}}}
{"step":"respond","method":"config/read","result":{"config":{},"origins":{},"layers":null}}
{"step":"respond","method":"thread/start","result":{"thread":{"id":"temp-1","environments":null,"extra":null,"sessionId":"temp-1","forkedFromId":null,"attachedToThreadId":null,"parentThreadId":null,"preview":"","ephemeral":true,"section":null,"sectionEnteredAt":null,"projectId":null,"historyMode":"legacy","modelProvider":"openai","model":null,"reasoningEffort":null,"createdAt":0,"updatedAt":0,"recencyAt":0,"status":{"type":"idle"},"path":null,"cwd":"/work","cliVersion":"0.0.0","originator":null,"source":"cli","canAcceptDirectInput":true,"threadSource":null,"agentNickname":null,"agentRole":null,"gitInfo":null,"name":null,"daybreakEnabled":null,"turns":[]},"model":"gpt-5","modelProvider":"openai","serviceTier":null,"cwd":"/work","instructionSources":[],"approvalPolicy":"never","approvalsReviewer":"user","sandbox":{"type":"readOnly"},"reasoningEffort":null}}
{"step":"respond","method":"turn/start","result":{"turn":{"id":"turn-r","items":[],"itemsView":"summary","status":"inProgress","error":null}}}"#;

fn recap(width: u16, height: u16) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .script(SCRIPT)
        .open();
    s.type_text("/recap");
    s.keys("enter");
    s
}

/// The temporary thread's answer: the recap as the model says it, and its turn's end.
fn model_says(s: &mut Screen, text: &str) {
    let text = serde_json::to_string(text).unwrap();
    s.feed(&format!(
        r#"{{"step":"notify","message":{{"method":"item/completed","params":{{"threadId":"temp-1","turnId":"turn-r","completedAtMs":0,"startedAtMs":0,"item":{{"type":"agentMessage","id":"m1","text":{text},"phase":null}}}}}}}}
{{"step":"notify","message":{{"method":"turn/completed","params":{{"threadId":"temp-1","turn":{{"id":"turn-r","items":[],"itemsView":"summary","status":"completed","error":null}}}}}}}}"#
    ));
}

#[test]
fn the_loading_row_stands_while_the_recap_is_generated() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 10)
        .script(SCRIPT)
        .open();
    let hold = s.hold_request("config/read");
    s.type_text("/recap");
    s.keys("enter");
    s.assert_has_row("• Generating conversation recap…");
    s.release(&hold);
    // the run goes on to its turn: the row stays until the answer is in
    s.assert_has_row("• Generating conversation recap…");
    model_says(
        &mut s,
        r#"{"summary":"Fixed the bug.","next_action":"Ship it."}"#,
    );
    s.assert_absent("Generating conversation recap");
}

#[test]
fn the_recap_is_a_cell_with_a_hanging_heading_and_its_next_action() {
    let mut s = recap(60, 12);
    model_says(
        &mut s,
        r#"{"summary":"Fixed the parser bug and added a test for the empty input case.","next_action":"Run the full suite."}"#,
    );
    s.assert_in_order(&[
        "  ↳ Recap: Fixed the parser bug and added a test for the",
        "           empty input case.",
        "           Next: Run the full suite.",
    ]);
}

#[test]
fn a_recap_without_a_next_action_has_the_summary_only() {
    let mut s = recap(60, 12);
    model_says(&mut s, r#"{"summary":"Fixed the bug.","next_action":null}"#);
    s.assert_has_row("  ↳ Recap: Fixed the bug.");
    s.assert_absent("Next:");
}

#[test]
fn a_terminal_too_narrow_for_the_hanging_indent_puts_the_heading_on_its_own_row() {
    let mut s = recap(13, 14);
    model_says(&mut s, r#"{"summary":"Fixed it.","next_action":null}"#);
    s.assert_in_order(&["↳ Recap:", "Fixed it."]);
}

#[test]
fn an_unreadable_answer_is_the_failure_line() {
    let mut s = recap(80, 10);
    model_says(&mut s, "not json");
    s.assert_contains("Could not generate a recap. Please try again.");
    s.assert_absent("Generating conversation recap");
}
