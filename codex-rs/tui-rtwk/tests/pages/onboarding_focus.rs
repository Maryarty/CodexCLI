// test root: pages
//! The folder-trust prompt cancelled at launch opens the command center in place of the folder
//! (oracle `lib.rs`: `consent.should_exit` -> `SessionSelection::AgentsOverview`). Opening a task
//! from it leaves the chat with the focus on its composer, so that the keys typed next land in the
//! draft (the same shape as `agents_focus` and `resume_focus`: the chat takes the focus as it
//! mounts, after the center has popped).

use crate::common::{self, Harness};
use codex_tui_core::app::AppEv;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::{Ev, Facts, TrustAsk, TrustCancel, Welcome};

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

fn trust_facts() -> Facts {
    Facts {
        welcome: Some(Welcome { animations: false }),
        trust: Some(TrustAsk {
            cwd: "/workspace/project".into(),
            target: "/workspace/project".into(),
            cancel: TrustCancel::AgentsOverview,
            ..TrustAsk::default()
        }),
        ..Facts::default()
    }
}

/// A server that lists one task and resumes it, the app started as the binary starts it (the home
/// waits for the host's facts) with `make` the command line's page, and the folder-trust prompt
/// on screen.
fn trust_prompt(
    make: impl FnOnce(codex_tui_rtwk::app::UiApp) -> codex_tui_rtwk::app::UiApp,
) -> Harness {
    let now: u64 = 1_777_399_200;
    let mut h = common::start_app(|app| make(app.with_awaited_facts(true)));
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
    h.inject([AppEv::Screen(ScreenEv::Onboarding(Ev::Facts {
        request: 1,
        facts: trust_facts(),
    }))]);
    h.wait_until("the trust prompt", |h| h.t.screen().contains("trust"));
    h.settle();
    h
}

/// The prompt cancelled: the page is left for the center at once (the oracle opens the overview
/// before its launch prompts are asked).
fn cancelled_into_the_center(h: &mut Harness) {
    h.press("esc");
    h.wait_until("the task", |h| h.t.screen().contains("Selected task"));
    h.settle();
    h.t.pump_for(common::ms(400));
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
fn a_task_opened_from_the_center_the_cancelled_trust_prompt_opened_has_the_composer_focused() {
    let mut h = trust_prompt(|app| app);
    cancelled_into_the_center(&mut h);
    h.press("enter");
    typed_into_the_composer(&mut h);
}

#[test]
fn the_same_with_agents_on_the_command_line() {
    let mut h = trust_prompt(|app| app.with_start_agents(true));
    cancelled_into_the_center(&mut h);
    h.press("enter");
    typed_into_the_composer(&mut h);
}

#[test]
fn the_same_with_resume_on_the_command_line() {
    // the center replaces the resume picker: no picker is left to mount once it pops
    let mut h = trust_prompt(|app| app.with_start_resume(true));
    cancelled_into_the_center(&mut h);
    h.press("enter");
    typed_into_the_composer(&mut h);
}
