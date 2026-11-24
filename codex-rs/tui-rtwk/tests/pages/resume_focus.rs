// test root: pages
//! `codex-rtwk resume`: a pick in the picker leaves the chat with the focus on its composer, so
//! that the keys typed next land in the draft (oracle: the composer is the chat's only input,
//! it has the keys from the first frame after `SessionSelection::Resume`).

use crate::common::{self, Harness};
use codex_tui_core::app::{AppEv, Now};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::onboarding::{Ev as StartupEv, Facts};
use codex_tui_rtwk::services::shell::FixedClock;

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
        "name": null, "daybreakEnabled": null, "turns": []
    })
}

/// The picker on a server that lists one session and resumes it.
fn picker() -> Harness {
    let now: u64 = 1_777_399_200;
    // as the binary starts it: the home waits for the host's facts
    // the shell tells the app this time at its start: a clock the test injects could lose to it
    let fixed = FixedClock::at(Now::at_secs(i64::try_from(now).unwrap_or(0)));
    let mut h = common::start_app(|app| {
        app.with_start_resume(true)
            .with_awaited_facts(true)
            .with_clock(fixed)
    });
    h.fake
        .respond(move |request| match request["method"].as_str()? {
            "thread/list" => Some(Ok(serde_json::json!({
                "data": [thread_json(ID, "Selected session", now)],
                "nextCursor": null,
            }))),
            "thread/resume" => Some(Ok(serde_json::json!({
                "thread": thread_json(ID, "Selected session", now),
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
    // The first listing went out before the fake could answer: a sort change lists again.
    h.press("tab");
    h.press("tab");
    h.press("right");
    h.settle();
    h.t.pump_for(common::ms(400));
    h
}

/// What the picker's end leaves: the picker is gone and what is typed next is in the composer.
fn typed_into_the_composer(h: &mut Harness) {
    h.settle();
    h.t.pump_for(common::ms(400));
    let screen = h.t.screen();
    assert!(
        !screen.contains("Selected session"),
        "the picker is gone: {screen}"
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
fn a_pick_in_the_picker_leaves_the_composer_focused() {
    let mut h = picker();
    let screen = h.t.screen();
    assert!(screen.contains("Selected session"), "{screen}");
    h.press("enter");
    typed_into_the_composer(&mut h);
}

#[test]
fn a_double_tap_on_a_row_leaves_the_composer_focused() {
    let mut h = picker();
    let screen = h.t.screen();
    let y = screen
        .lines()
        .position(|row| row.contains("Selected session"))
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
fn leaving_the_picker_with_escape_leaves_the_composer_focused() {
    let mut h = picker();
    h.press("esc");
    typed_into_the_composer(&mut h);
}
