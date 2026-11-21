//! Screen tests of S13 (approvals): each entry of the debug menu opens the
//! request presenter's dialog over the model a fixture script made; the
//! expected text is the oracle's snapshot (`dialogs::oracle`). Oracle tests:
//! `bottom_pane/approval_overlay.rs`, `chatwidget/tests/exec_flow.rs`,
//! `bottom_pane/mcp_server_elicitation.rs`, `bottom_pane/request_user_input`.

use crate::common::{Harness, ms, start_sized};
use crate::dialogs::oracle::{assert_rows, assert_sequence};

fn open(width: u16, height: u16, entry: &str) -> Harness {
    let mut h = start_sized(true, width, height);
    h.open_debug_row("approvals", Some(entry));
    h.t.pump_for(ms(800));
    h
}

#[test]
fn approval_modal_exec() {
    let h = open(100, 24, "exec_command");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__approval_modal_exec.snap",
    );
}

#[test]
fn approval_overlay_patch_destination_unavailable() {
    let h = open(120, 24, "patch_edits");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__approval_overlay__tests__approval_overlay_patch_destination_unavailable.snap",
    );
}

#[test]
fn permissions_prompt_snapshot() {
    let h = open(120, 24, "permissions");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__approval_overlay__tests__approval_overlay_permissions_prompt.snap",
    );
}

#[test]
fn approval_form_tool_approval_snapshot() {
    let h = open(120, 16, "mcp_elicitation");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/snapshots/codex_tui__bottom_pane__mcp_server_elicitation__tests__mcp_server_elicitation_approval_form_without_schema.snap",
    );
}

#[test]
fn request_user_input_options_snapshot() {
    let h = open(120, 16, "user_input");
    assert_rows(
        &h.t.screen(),
        "bottom_pane/request_user_input/snapshots/codex_tui__bottom_pane__request_user_input__tests__request_user_input_options.snap",
    );
}

/// The skeleton's dismiss dialog stays the fallback for a request whose
/// presentation the model does not hold (a live request today).
#[test]
fn a_request_without_a_prompt_falls_back_to_the_dismiss_dialog() {
    let mut h = start_sized(true, 80, 24);
    h.open_debug_row("approvals", Some("exec_approval"));
    h.t.pump_for(ms(800));
    assert!(
        h.t.screen().contains("1 needs an answer."),
        "{}",
        h.t.screen()
    );
}

#[test]
fn shortcut_triggers_selection() {
    // The `y` shortcut answers "Yes, proceed": the dialog closes.
    let mut h = open(100, 24, "exec_command");
    assert!(
        h.t.screen().contains("Yes, proceed (y)"),
        "{}",
        h.t.screen()
    );
    h.press("y");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Yes, proceed"), "{}", h.t.screen());
}

#[test]
fn configured_list_cancel_aborts_exec_approval() {
    let mut h = open(100, 24, "exec_command");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Yes, proceed"), "{}", h.t.screen());
}

#[test]
fn enter_confirms_the_highlighted_option() {
    let mut h = open(100, 24, "exec_command");
    h.press("down");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Yes, proceed"), "{}", h.t.screen());
}

#[test]
fn deny_shortcut_submits_denied_exec_decision() {
    // `n`/`d` are not bound for an exec prompt that offers no decline; the
    // dialog stays (oracle: `deny` is bound only where the option exists).
    let mut h = open(100, 24, "exec_command");
    h.press("d");
    h.t.pump_for(ms(200));
    assert!(h.t.screen().contains("Yes, proceed"), "{}", h.t.screen());
}

/// `network_exec_prompt.snap`, row by row (the oracle's row 1 holds a bare
/// quote, which the snapshot reader of the oracle shard cannot read).
#[test]
fn network_exec_prompt_rows() {
    let h = open(100, 13, "exec_network");
    let want: Vec<String> = [
        "Do you want to approve network access to \"example.com\"?",
        "Reason: network request blocked",
        "› 1. Yes, just this once (y)",
        "2. Yes, and allow this host for this conversation (a)",
        "3. Yes, and allow this host in the future (p)",
        "4. No, and tell Codex what to do differently (esc)",
        "Press enter to confirm or esc to cancel",
    ]
    .map(String::from)
    .to_vec();
    assert_sequence(&h.t.screen(), "network_exec_prompt", &want);
    assert!(!h.t.screen().contains("$ curl"), "{}", h.t.screen());
}

/// `approval_header_wrapped_url.snap`: the URL wraps at 32 columns, a reason
/// line of its own, the command after it.
#[test]
fn a_url_reason_wraps_under_its_label() {
    let h = open(36, 16, "exec_url_reason");
    let want: Vec<String> = [
        "Reason:",
        "https://github.com/openai/codex/",
        "pull/12345?diff=split",
        "$ echo hi",
    ]
    .map(String::from)
    .to_vec();
    assert_sequence(&h.t.screen(), "approval_header_wrapped_url", &want);
}

/// The oracle's `approval_footer_hint`: a request of another thread adds
/// ` or o to open thread`; one of the shown thread has no such key.
#[test]
fn the_open_thread_key_belongs_to_a_request_of_another_thread() {
    let h = open(100, 16, "exec_cross_thread");
    assert!(
        h.t.screen().contains("Thread: Robie [explorer]"),
        "{}",
        h.t.screen()
    );
    assert!(
        h.t.screen().contains("or o to open thread"),
        "{}",
        h.t.screen()
    );
    let mut own = open(100, 16, "exec_request");
    assert!(
        !own.t.screen().contains("to open thread"),
        "{}",
        own.t.screen()
    );
    own.press("o");
    own.t.pump_for(ms(200));
    assert!(
        own.t.screen().contains("Yes, proceed"),
        "{}",
        own.t.screen()
    );
}

/// ADR-011 (beyond the oracle): a click on a row moves the cursor there, a
/// click on the row the cursor is on answers it and the dialog closes.
#[test]
fn a_click_moves_the_cursor_and_a_second_click_answers() {
    let mut h = open(100, 16, "exec_request");
    let screen = h.t.screen();
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("No, and tell Codex"))
        .expect("the cancel row is listed");
    let x = u16::try_from(line.find("No, and").unwrap()).unwrap();
    let y = u16::try_from(y).unwrap();
    h.t.click(x, y);
    h.t.pump_for(ms(200));
    assert!(
        h.t.screen().contains("› 2. No, and tell"),
        "{}",
        h.t.screen()
    );
    assert!(
        h.t.screen().contains("Yes, proceed"),
        "the first click only moves"
    );
    h.t.click(x, y);
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Yes, proceed"), "{}", h.t.screen());
}

/// A digit answers its row (the oracle's `1`..`9` of a list).
#[test]
fn a_digit_answers_its_row() {
    let mut h = open(100, 16, "exec_request");
    h.press("2");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Yes, proceed"), "{}", h.t.screen());
}
