// test root: pages
//! `codex-rtwk agents`: opening a task from the command center leaves the chat with the focus
//! on its composer, so that the keys typed next land in the draft (oracle: the composer is the
//! chat's only input, it has the keys from the first frame after the session is opened).

use crate::common::{self, Harness};
use codex_tui_core::app::Command;
use codex_tui_core::screens::onboarding::{Facts, TrustAsk, TrustCancel};

const ID: &str = "00000000-0000-0000-0000-000000000001";

fn thread_json(id: &str, preview: &str, now: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": preview, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": now - 7200,
        "updatedAt": now - 3600, "recencyAt": now - 3600, "status": {"type": "idle"},
        "path": null, "cwd": "/tmp/codex", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null,
        "gitInfo": {"sha": null, "branch": "main", "originUrl": null},
        "name": "Selected task", "daybreakEnabled": null, "turns": []
    })
}

/// The server lists one task and resumes it.
fn answer_task(h: &Harness) {
    let now: u64 = 1_777_399_200;
    h.fake
        .respond(move |request| match request["method"].as_str()? {
            "thread/list" => Some(Ok(if request["params"]["sectionId"].is_null() {
                serde_json::json!({
                    "data": [thread_json(ID, "Selected task", now)],
                    "nextCursor": null, "backwardsCursor": null,
                })
            } else {
                serde_json::json!({"data": [], "nextCursor": null, "backwardsCursor": null})
            })),
            "thread/resume" => Some(Ok(serde_json::json!({
                "thread": thread_json(ID, "Selected task", now),
                "model": "gpt-5", "modelProvider": "openai", "serviceTier": null,
                "cwd": "/tmp/codex", "instructionSources": [],
                "approvalPolicy": "on-request", "approvalsReviewer": "user",
                "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
            }))),
            _ => None,
        });
}

/// The command center on a server that lists one task and resumes it.
fn center() -> Harness {
    let mut h = common::start_app(|app| app.with_start_agents(true).with_awaited_facts(true));
    answer_task(&h);
    h.finish_startup_without_prompts();
    h.wait_until("the task", |h| h.t.screen().contains("Selected task"));
    h.settle();
    h.t.pump_for(common::ms(400));
    h
}

/// What opening a task leaves: the center is gone and what is typed next is in the composer.
fn typed_into_the_composer(h: &mut Harness) {
    h.wait_until("the chat", |h| {
        h.t.screen().contains("Ask Codex to do anything")
    });
    h.settle();
    h.t.pump_for(common::ms(400));
    let screen = h.t.screen();
    assert!(
        !screen.contains("Agent command center"),
        "the center is gone: {screen}"
    );
    h.t.type_text("hi");
    h.t.pump_for(common::ms(100));
    let screen = h.t.screen();
    assert!(
        screen.contains("› hi"),
        "the composer shows what was typed: {screen}"
    );
}

#[test]
fn opening_a_task_with_enter_leaves_the_composer_focused() {
    let mut h = center();
    h.press("enter");
    typed_into_the_composer(&mut h);
}

#[test]
fn a_double_tap_on_a_task_leaves_the_composer_focused() {
    let mut h = center();
    let screen = h.t.screen();
    let y = screen
        .lines()
        .position(|row| row.contains("Selected task"))
        .expect("the row");
    let x = screen
        .lines()
        .nth(y)
        .and_then(|row| row.find("Selected"))
        .expect("its text");
    let (x, y) = (u16::try_from(x).unwrap_or(0), u16::try_from(y).unwrap_or(0));
    h.t.click(x, y);
    h.settle();
    h.t.click(x, y);
    typed_into_the_composer(&mut h);
}

#[test]
fn declining_trust_cancels_early_enter_before_opening_another_task() {
    let mut h = common::start_app(|app| app.with_awaited_facts(true).with_provisional_draft(true));
    answer_task(&h);
    h.wait_for("the draft", |s| s.contains("Ask Codex to do anything"));
    h.t.type_text("original workspace draft");
    h.press("enter");
    h.wait_for("early confirmation", |s| s.contains("Waiting for startup"));
    h.answer_startup_facts(Facts {
        trust: Some(TrustAsk {
            cwd: "/workspace/project".into(),
            target: "/workspace/project".into(),
            cancel: TrustCancel::AgentsOverview,
            ..TrustAsk::default()
        }),
        ..Facts::default()
    });
    h.wait_for("the trust prompt", |s| s.contains("trust"));
    h.press("esc");
    h.wait_for("the other task", |s| s.contains("Selected task"));
    h.press("enter");
    h.wait_for("the retained draft", |s| {
        s.contains("› original workspace draft")
    });
    h.settle();
    assert!(!h.t.screen().contains("Waiting for startup"));
    assert!(
        h.fake
            .commands()
            .iter()
            .all(|command| !matches!(command, Command::StartTurn { .. })),
        "the old confirmation must not submit to the task opened from the command center"
    );
}
