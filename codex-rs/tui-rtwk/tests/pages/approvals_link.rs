// test root: pages
//! The app link dialog a server's request opens in the live chat (oracle
//! `bottom_pane/app_link_view.rs`): a tool suggestion to install an app, another server's URL.
//! What the user does with it by mouse and keys, and what a late event of a request that is gone
//! does to the screen.

use codex_tui_core::app::Command;
use codex_tui_core::screens::{ScreenCommand, select};
use serde_json::{Value, json};

use crate::kit::{Owner, Screen};

const INSTALL_URL: &str = "https://example.test/google-calendar";

/// A `request` step of the script: a server request of `method`.
fn request(id: u64, params: &Value) -> String {
    json!({ "step": "request", "message": {
        "method": "mcpServer/elicitation/request", "id": id, "params": params
    } })
    .to_string()
}

fn install(id: u64) -> String {
    request(
        id,
        &json!({
            "threadId": "thread-1", "turnId": "turn-1", "serverName": "codex_apps",
            "environmentId": null, "mode": "form", "message": "Install Google Calendar",
            "requestedSchema": { "type": "object", "properties": {} },
            "_meta": {
                "codex_approval_kind": "tool_suggestion", "tool_type": "connector",
                "suggest_type": "install", "suggest_reason": "Plan and reference events",
                "tool_id": "connector_google_calendar", "tool_name": "Google Calendar",
                "install_url": INSTALL_URL
            }
        }),
    )
}

fn url(id: u64, url: &str) -> String {
    request(
        id,
        &json!({
            "threadId": "thread-1", "turnId": "turn-1", "serverName": "payments",
            "mode": "url", "_meta": null, "message": "Review the payment details to continue.",
            "url": url, "elicitationId": "payment-123"
        }),
    )
}

fn chat(width: u16, jsonl: &str) -> Screen {
    let mut s = Screen::build(Owner::ChatLive).size(width, 24).live().open();
    s.feed(jsonl);
    s
}

/// The answers the app sent for the requests, as their `action`.
fn actions(s: &Screen) -> Vec<String> {
    s.resolved_requests()
        .iter()
        .map(|result| result["action"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn directory_reads(s: &Screen) -> Vec<Command> {
    s.commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Apps { .. }),
                    ..
                }
            )
        })
        .collect()
}

#[test]
fn a_tap_on_the_primary_button_opens_the_page_and_a_tap_on_the_next_one_finishes() {
    let mut s = chat(80, &install(21));
    s.assert_contains("Install on ChatGPT");
    s.click_text("Install on ChatGPT");
    assert_eq!(s.opened_urls(), [INSTALL_URL]);
    s.assert_contains("I already Installed it");
    assert!(actions(&s).is_empty());
    s.click_text("I already Installed it");
    assert_eq!(actions(&s), ["accept"]);
    s.assert_absent("I already Installed it");
    s.assert_absent("Google Calendar");
}

#[test]
fn a_tap_on_the_back_row_declines_the_request() {
    let mut s = chat(80, &install(21));
    // the first tap moves the cursor to the row, the second one answers (ADR-011)
    s.click_text("2. Back");
    s.click_text("2. Back");
    assert_eq!(actions(&s), ["decline"]);
    assert!(s.opened_urls().is_empty());
    s.assert_absent("Install on ChatGPT");
}

#[test]
fn a_double_enter_on_the_confirmation_resolves_the_request_once() {
    let mut s = chat(80, &install(21));
    s.keys("enter");
    assert_eq!(s.opened_urls(), [INSTALL_URL]);
    s.keys("enter enter");
    assert_eq!(actions(&s), ["accept"]);
    // more Enters reach the composer, not the answered request
    s.keys("enter enter");
    assert_eq!(actions(&s), ["accept"]);
    assert_eq!(s.opened_urls(), [INSTALL_URL]);
    assert_eq!(directory_reads(&s).len(), 1);
}

#[test]
fn a_late_reply_for_a_request_that_is_gone_does_not_reopen_the_dialog() {
    let mut s = chat(80, &install(21));
    s.keys("enter enter");
    assert_eq!(actions(&s), ["accept"]);
    s.assert_absent("Install on ChatGPT");
    // the directory the finish asked for arrives after the request left the queue
    let reads = directory_reads(&s);
    assert_eq!(reads.len(), 1, "{reads:?}");
    s.answer_request(&reads[0], json!({ "data": [], "nextCursor": null }));
    s.assert_absent("Install on ChatGPT");
    s.assert_absent("Google Calendar");
    // and so does the server's own word that it resolved the request
    s.feed(
        r#"{"step":"notify","message":{"method":"serverRequest/resolved","params":{"threadId":"thread-1","requestId":21}}}"#,
    );
    s.assert_absent("Install on ChatGPT");
    assert_eq!(actions(&s), ["accept"]);
}

#[test]
fn a_request_withdrawn_by_the_server_closes_its_dialog_and_answers_nothing() {
    let mut s = chat(80, &install(21));
    s.assert_contains("Install on ChatGPT");
    s.feed(
        r#"{"step":"notify","message":{"method":"serverRequest/resolved","params":{"threadId":"thread-1","requestId":21}}}"#,
    );
    s.assert_absent("Install on ChatGPT");
    assert!(actions(&s).is_empty());
}

#[test]
fn a_narrow_terminal_keeps_the_whole_url_visible_on_the_confirmation() {
    let link = "https://payments.example/checkout/123?session=abcdef0123456789abcdef&return=done";
    let mut s = chat(40, &url(7, link));
    s.keys("enter");
    assert_eq!(s.opened_urls(), [link]);
    // the URL wraps over the rows of the dialog; every character of it is on the screen
    let joined: String = s
        .rows()
        .iter()
        .flat_map(|row| row.chars())
        .filter(|c| !c.is_whitespace())
        .collect();
    assert!(joined.contains(link), "the URL is cut:\n{}", s.text());
}
