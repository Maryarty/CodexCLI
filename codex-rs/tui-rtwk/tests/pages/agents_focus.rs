// test root: pages
//! `codex-rtwk agents`: opening a task from the command center leaves the chat with the focus
//! on its composer, so that the keys typed next land in the draft (oracle: the composer is the
//! chat's only input, it has the keys from the first frame after the session is opened).

use crate::common::{self, Harness};
use codex_tui_core::app::AppEv;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::{Ev as StartupEv, Facts};

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

/// The command center on a server that lists one task and resumes it.
fn center() -> Harness {
    let now: u64 = 1_777_399_200;
    // as the binary starts it: the home waits for the host's facts
    let mut h = common::start_app(|app| app.with_start_agents(true).with_awaited_facts(true));
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
    // the host's answers: the facts, no device code, no launch prompts
    h.inject([
        AppEv::Screen(ScreenEv::Onboarding(StartupEv::Facts {
            request: 1,
            facts: Facts::default(),
        })),
        AppEv::Screen(ScreenEv::Onboarding(StartupEv::GatewayDevice {
            request: 2,
            code: None,
        })),
    ]);
    h.wait_until("the launch ask", |h| {
        h.fake
            .commands()
            .iter()
            .any(|c| format!("{c:?}").contains("Launch { request: 3 }"))
    });
    h.inject([AppEv::Screen(ScreenEv::Onboarding(StartupEv::Launched {
        request: 3,
        prompts: Vec::new(),
    }))]);
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
