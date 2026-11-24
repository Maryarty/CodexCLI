//! The side-conversation seed (`Open::side()`): the live chat inside the conversation `/side`
//! forks off the main one (oracle `app/side.rs`), as a user gets there: the command typed, the
//! server answering `thread/fork`. The footer then says `Side from main thread · ctrl+/ to
//! switch · ctrl+c to close`.

use super::Screen;
use crate::common::Harness;

/// The id of the forked thread the seed's server answers with.
pub const SIDE_THREAD: &str = "side-1";

/// The server's answer to the fork `/side` asks for.
fn forked() -> serde_json::Value {
    serde_json::json!({
        "thread": {
            "id": SIDE_THREAD, "environments": null, "extra": null, "sessionId": SIDE_THREAD,
            "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
            "preview": SIDE_THREAD, "ephemeral": true, "section": null, "sectionEnteredAt": null,
            "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
            "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
            "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
            "status": {"type": "idle"}, "path": null, "cwd": "/work", "cliVersion": "0.0.0",
            "originator": null, "source": "cli", "canAcceptDirectInput": true,
            "threadSource": null, "agentNickname": null, "agentRole": null, "gitInfo": null,
            "name": SIDE_THREAD, "daybreakEnabled": null, "turns": []
        },
        "model": "gpt-5", "modelProvider": "openai", "serviceTier": null, "cwd": "/work",
        "instructionSources": [], "approvalPolicy": "on-request", "approvalsReviewer": "user",
        "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
    })
}

/// `/side` on the open chat, the fork answered: waits for the side conversation's footer.
pub(super) fn enter(h: &mut Harness) {
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
    h.wait_for("the side conversation", |s| s.contains("to switch"));
}

impl Screen {
    /// The side conversation was closed on the server (`thread/unsubscribe` of its thread).
    pub fn side_closed(&self) -> bool {
        self.h.fake.requests().iter().any(|request| {
            request["method"] == "thread/unsubscribe"
                && request["params"]["threadId"] == SIDE_THREAD
        })
    }
}
