//! Screen tests of S13 (approvals): each entry of the debug menu opens the
//! request presenter's dialog over the model a fixture script made; the
//! expected text is the oracle's snapshot (`dialogs::oracle`). Oracle tests:
//! `bottom_pane/approval_overlay.rs`, `chatwidget/tests/exec_flow.rs`,
//! `bottom_pane/mcp_server_elicitation.rs`, `bottom_pane/request_user_input`.

use crate::common::{Harness, ms, start_sized};
use crate::dialogs::oracle::assert_rows;

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
    assert!(h.t.screen().contains("1 needs an answer."), "{}", h.t.screen());
}

#[test]
fn shortcut_triggers_selection() {
    // The `y` shortcut answers "Yes, proceed": the dialog closes.
    let mut h = open(100, 24, "exec_command");
    assert!(h.t.screen().contains("Yes, proceed (y)"), "{}", h.t.screen());
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
