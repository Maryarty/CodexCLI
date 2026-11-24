//! The return from a side conversation (oracle `app/side.rs` `maybe_return_from_side`): Ctrl+C or
//! Ctrl+D over an empty composer shows the parent and drops the side conversation (its turn is
//! interrupted, the server stops holding it); with a draft in the composer the keys stay the
//! page's.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use crate::common::{Harness, start_driven};

const SIDE: &str = "side-1";

/// The server's answer to the fork `/side` asks for.
fn forked() -> serde_json::Value {
    serde_json::json!({
        "thread": {
            "id": SIDE, "environments": null, "extra": null, "sessionId": SIDE,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": SIDE, "ephemeral": true, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
            "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
            "status": {"type": "idle"}, "path": null, "cwd": "/work", "cliVersion": "0.0.0",
            "originator": null, "source": "cli", "canAcceptDirectInput": true,
            "threadSource": null, "agentNickname": null, "agentRole": null, "gitInfo": null,
            "name": SIDE, "daybreakEnabled": null, "turns": []
        },
        "model": "gpt-5", "modelProvider": "openai", "serviceTier": null, "cwd": "/work",
        "instructionSources": [], "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

/// A chat that has `/side` open: the footer over the side conversation says `to switch`.
fn in_side() -> Harness {
    let mut h = start_driven(false, 100, 12);
    h.connect();
    h.quiesce();
    h.fake.respond(|request| match request["method"].as_str()? {
        "thread/fork" => Some(Ok(forked())),
        "thread/inject_items" | "thread/unsubscribe" | "turn/interrupt" => {
            Some(Ok(serde_json::json!({})))
        }
        _ => None,
    });
    h.t.type_text("/side");
    h.press("enter");
    h.advance(300);
    h.wait_for("side open", |s| s.contains("to switch"));
    h
}

fn closed(h: &Harness) -> bool {
    h.fake.requests().iter().any(|request| {
        request["method"] == "thread/unsubscribe" && request["params"]["threadId"] == SIDE
    })
}

#[test]
fn ctrl_c_and_ctrl_d_over_an_empty_composer_return_from_the_side_conversation() {
    for key in ["ctrl+c", "ctrl+d"] {
        let mut h = in_side();
        assert!(!closed(&h));
        h.press(key);
        h.advance(300);
        // the parent is shown (no side label: it has no side conversation any more), the side
        // conversation is closed on the server, and the process is still running
        h.wait_for(key, |s| !s.contains("to switch"));
        assert!(h.t.is_running(), "{key} quit the chat\n{}", h.t.screen());
        assert!(!h.t.screen().contains("for side"), "{}", h.t.screen());
        assert!(closed(&h), "{key}: the side conversation was kept");
    }
}

#[test]
fn a_draft_keeps_ctrl_c_with_the_page() {
    let mut h = in_side();
    h.t.type_text("a question");
    h.advance(100);
    h.press("ctrl+d");
    h.advance(300);
    assert!(h.t.screen().contains("to switch"), "{}", h.t.screen());
    assert!(!closed(&h));
}
