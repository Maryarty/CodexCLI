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

/// The cell of the first row holding `needle`, clicked.
fn tap(h: &mut Harness, needle: &str) {
    let screen = h.t.screen();
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?}:\n{screen}"));
    let x = line.find(needle).map_or(0, |at| at as u16);
    h.t.click(x, y as u16);
    h.t.pump_for(ms(200));
}

fn shows(h: &Harness, what: &str) -> bool {
    h.t.screen().contains(what)
}

/// Family B (MCP elicitation form), the field keys of the oracle's
/// `handle_key_event`: ctrl+p / ctrl+n, PageUp / PageDown and, on a select
/// field, Left / h and Right / l move between fields.
#[test]
fn form_field_keys_move_between_fields() {
    let mut h = open(100, 20, "mcp_two_booleans");
    assert!(shows(&h, "Field 1/2"), "{}", h.t.screen());
    for (key, field) in [
        ("ctrl+n", "Field 2/2"),
        ("ctrl+p", "Field 1/2"),
        ("pagedown", "Field 2/2"),
        ("pageup", "Field 1/2"),
        ("right", "Field 2/2"),
        ("left", "Field 1/2"),
        ("l", "Field 2/2"),
        ("h", "Field 1/2"),
    ] {
        h.press(key);
        assert!(shows(&h, field), "after {key}:\n{}", h.t.screen());
    }
}

/// A digit picks the row, commits it and moves to the next field; the
/// last field's digit submits the form (the dialog closes).
#[test]
fn form_digits_commit_and_the_last_one_submits() {
    let mut h = open(100, 20, "mcp_two_booleans");
    h.press("1");
    assert!(shows(&h, "Field 2/2"), "{}", h.t.screen());
    assert!(
        shows(&h, "Choose values"),
        "the form stays: {}",
        h.t.screen()
    );
    h.press("2");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "Choose values"), "{}", h.t.screen());
}

/// Submitting with a required field unanswered names the error and jumps to
/// that field (`Answer required fields before submitting.`).
#[test]
fn form_submit_with_an_unanswered_field_jumps_back_to_it() {
    let mut h = open(100, 20, "mcp_two_booleans");
    h.press("ctrl+n");
    h.press("enter");
    assert!(
        shows(&h, "Answer required fields before submitting."),
        "{}",
        h.t.screen()
    );
    assert!(shows(&h, "Field 1/2"), "{}", h.t.screen());
    assert!(shows(&h, "Choose values"), "{}", h.t.screen());
}

/// A text field: Enter with nothing typed is the same error, typed text
/// commits and (the only field) submits; Esc cancels.
#[test]
fn form_text_field_enter_submits_the_typed_answer() {
    let mut h = open(100, 20, "mcp_text_chords");
    h.press("enter");
    assert!(
        shows(&h, "Answer required fields before submitting."),
        "{}",
        h.t.screen()
    );
    h.t.type_text("Ada");
    h.t.pump_for(ms(100));
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "Enter a name"), "{}", h.t.screen());
}

/// Ctrl+C on a text field clears the typed text first; a second one cancels.
#[test]
fn form_ctrl_c_clears_the_text_then_cancels() {
    let mut h = open(100, 20, "mcp_text_chords");
    h.t.type_text("Ada");
    h.t.pump_for(ms(100));
    assert!(shows(&h, "Ada"), "{}", h.t.screen());
    h.press("ctrl+c");
    assert!(!shows(&h, "Ada"), "{}", h.t.screen());
    assert!(shows(&h, "Enter a name"), "{}", h.t.screen());
    h.press("ctrl+c");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "Enter a name"), "{}", h.t.screen());
}

#[test]
fn form_escape_cancels() {
    let mut h = open(100, 20, "mcp_two_booleans");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "Choose values"), "{}", h.t.screen());
}

/// ADR-011 (beyond the oracle): a tap on an option row moves the cursor
/// there, a second tap commits it and moves to the next field.
#[test]
fn form_a_tap_moves_the_cursor_and_a_second_tap_commits() {
    let mut h = open(100, 20, "mcp_two_booleans");
    tap(&mut h, "2. False");
    assert!(shows(&h, "› 2. False"), "{}", h.t.screen());
    assert!(shows(&h, "Field 1/2"), "the first tap only moves");
    tap(&mut h, "2. False");
    assert!(shows(&h, "Field 2/2"), "{}", h.t.screen());
    // the cursor of the last field is on its first row: a tap there submits
    tap(&mut h, "1. True");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "Choose values"), "{}", h.t.screen());
}

/// The approval actions of a message-only form answer by a digit or two taps.
#[test]
fn message_only_form_actions_answer_by_digit_and_by_tap() {
    let mut h = open(100, 16, "mcp_message_only");
    assert!(shows(&h, "1. Allow"), "{}", h.t.screen());
    h.press("2");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "1. Allow"), "{}", h.t.screen());
    let mut h = open(100, 16, "mcp_message_only");
    tap(&mut h, "3. Cancel");
    assert!(shows(&h, "› 3. Cancel"), "{}", h.t.screen());
    tap(&mut h, "3. Cancel");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "1. Allow"), "{}", h.t.screen());
}

/// On a text field ctrl+n and ctrl+p change the field (the text area would move its cursor).
#[test]
fn form_ctrl_n_and_ctrl_p_change_field_on_a_text_field() {
    let mut h = open(100, 20, "mcp_two_strings");
    h.t.type_text("Ada");
    h.t.pump_for(ms(100));
    h.press("ctrl+n");
    assert!(shows(&h, "Field 2/2"), "{}", h.t.screen());
    h.press("ctrl+p");
    assert!(shows(&h, "Field 1/2"), "{}", h.t.screen());
    assert!(
        shows(&h, "Ada"),
        "the first answer is kept: {}",
        h.t.screen()
    );
    h.press("pagedown");
    assert!(shows(&h, "Field 2/2"), "{}", h.t.screen());
}

/// An answer over the input limit flashes the length error under the field
/// and does not commit; switching fields clears the flash.
#[test]
fn form_text_over_the_limit_flashes_the_length_error_until_the_field_changes() {
    let limit = codex_tui_core::screens::composer::MAX_INPUT_CHARS;
    let mut h = open(100, 20, "mcp_two_strings");
    h.t.paste(&"x".repeat(limit + 1));
    h.t.pump_for(ms(300));
    h.press("enter");
    assert!(
        shows(
            &h,
            &format!("Message exceeds the maximum length of {limit}")
        ),
        "{}",
        h.t.screen()
    );
    assert!(shows(&h, "Field 1/2"), "{}", h.t.screen());
    h.press("ctrl+n");
    assert!(shows(&h, "Field 2/2"), "{}", h.t.screen());
    assert!(!shows(&h, "Message exceeds"), "{}", h.t.screen());
}

/// Family C (`request_user_input`): Tab opens the notes of an option question, Backspace on
/// empty notes closes them again (the oracle's `backspace_on_empty_notes_closes_notes_ui`).
#[test]
fn user_input_backspace_on_empty_notes_closes_them() {
    let mut h = open(120, 16, "user_input");
    assert!(shows(&h, "tab to add notes"), "{}", h.t.screen());
    h.press("tab");
    assert!(shows(&h, "tab or esc to clear notes"), "{}", h.t.screen());
    h.press("backspace");
    assert!(shows(&h, "tab to add notes"), "{}", h.t.screen());
    assert!(!shows(&h, "clear notes"), "{}", h.t.screen());
}

/// A text field the user comes to from a list field takes the keys: what is typed shows.
#[test]
fn form_a_text_field_reached_from_a_list_takes_the_typed_text() {
    let mut h = open(100, 20, "mcp_select");
    // the properties come alphabetically (dry, note, region, tier: a `BTreeMap` of the protocol)
    h.press("ctrl+n");
    assert!(shows(&h, "Field 2/4"), "{}", h.t.screen());
    h.t.type_text("hello");
    h.t.pump_for(ms(100));
    assert!(shows(&h, "hello"), "{}", h.t.screen());
    h.press("ctrl+n");
    assert!(shows(&h, "Field 3/4"), "{}", h.t.screen());
    // back on the text field from a list field: it takes the keys again
    h.press("ctrl+p");
    h.t.type_text("!");
    h.t.pump_for(ms(100));
    assert!(shows(&h, "hello!"), "{}", h.t.screen());
}

/// A freeform answer takes Shift+Enter as a new line and stays on the question (the oracle's
/// `freeform_shift_enter_inserts_newline_without_advancing`; the pty kit has no bytes for the key,
/// the Tester takes the kitty sequence).
#[test]
fn user_input_freeform_shift_enter_inserts_a_newline_without_advancing() {
    let mut h = open(120, 16, "user_input_options_then_freeform");
    h.press("ctrl+n");
    assert!(shows(&h, "Question 2/2"), "{}", h.t.screen());
    h.t.type_text("Draft");
    h.t.pump_for(ms(100));
    h.t.input(b"\x1b[13;2u");
    h.t.pump_for(ms(100));
    h.t.type_text("more");
    h.t.pump_for(ms(100));
    let screen = h.t.screen();
    assert!(shows(&h, "Question 2/2"), "{screen}");
    let rows: Vec<&str> = screen.lines().collect();
    let draft = rows.iter().position(|r| r.contains("Draft")).expect("Draft");
    let more = rows.iter().position(|r| r.contains("more")).expect("more");
    assert!(!rows[draft].contains("more") && more > draft, "{screen}");
}

/// Enter on the last question with a question unanswered asks first (the oracle's confirmation:
/// a title, the count, `Proceed` and `Go back`, the press hint at the foot), Go back returns to the
/// unanswered question and Proceed answers.
#[test]
fn user_input_unanswered_confirmation_rows() {
    let mut h = open(80, 12, "user_input_freeform_then_options");
    h.t.type_text("freeform notes");
    h.t.pump_for(ms(100));
    h.press("ctrl+n");
    assert!(shows(&h, "Question 2/2 (2 unanswered)"), "{}", h.t.screen());
    h.press("enter");
    let screen = h.t.screen();
    for row in [
        "Submit with unanswered questions?",
        "1 unanswered question",
        "› 1. Proceed  Submit with 1 unanswered question",
        "2. Go back  Return to the first unanswered question",
        "Press enter to confirm or esc to go back",
    ] {
        assert!(screen.lines().any(|l| l.trim() == row), "{row}\n{screen}");
    }
    // Go back lands on the first question that is not answered
    h.press("esc");
    assert!(shows(&h, "Question 1/2"), "{}", h.t.screen());
    h.press("ctrl+n");
    h.press("enter");
    assert!(shows(&h, "Submit with unanswered questions?"), "{}", h.t.screen());
    h.press("enter");
    assert!(!shows(&h, "Submit with unanswered questions?"), "{}", h.t.screen());
}
