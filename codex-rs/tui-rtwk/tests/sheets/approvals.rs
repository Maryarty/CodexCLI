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

/// The screen's rows without their trailing blanks.
fn rows_of(h: &Harness) -> Vec<String> {
    h.t.screen()
        .lines()
        .map(|line| line.trim_end().to_owned())
        .collect()
}

/// A header that does not fit is cut to the room the options and the hint leave, a dim
/// `[… N lines] ⌃a view all` in place of its last row. The frames are the oracle's
/// (`clipping_tests.rs`, bound=true; its binding is ctrl+g, ours the default ctrl+a); the
/// blank rows give way before the header is cut.
#[test]
fn a_header_taller_than_the_pane_is_cut_with_its_line_count() {
    let h = open(80, 14, "exec_clipped");
    assert_eq!(
        rows_of(&h),
        [
            "",
            "  Would you like to run the following command?",
            "",
            "  Environment: local",
            "",
            "  Reason: A reason requiring review.A reason requiring review.A reason",
            "  requiring review.A reason requiring review.A reason requiring review.A",
            "  reason requiring review.A reason requiring review.A reason requiring",
            "  review.A reason requiring review.A reason requiring review.A reason",
            "  requiring review.A reason requiring review.A reason requiring review.A",
            "  [\u{2026} 34 lines] \u{2303}a view all",
            "\u{203a} 1. Yes, proceed (y)",
            "  2. No, and tell Codex what to do differently (esc)",
            "  Press enter to confirm or esc to cancel",
        ]
    );
}

#[test]
fn a_cut_header_gives_up_its_rows_to_the_options_and_the_hint_last() {
    let frames: [(u16, u16, &[&str]); 3] = [
        (
            40,
            10,
            &[
                "",
                "  Would you like to run the following",
                "  command?",
                "",
                "  [\u{2026} 43 lines] \u{2303}a view all",
                "\u{203a} 1. Yes, proceed (y)",
                "  2. No, and tell Codex what to do",
                "     differently (esc)",
                "  Press enter to confirm or esc to",
                "  cancel",
            ],
        ),
        (
            40,
            8,
            &[
                "",
                "  Would you like to run the following",
                "  [\u{2026} 43 lines] \u{2303}a view all",
                "\u{203a} 1. Yes, proceed (y)",
                "  2. No, and tell Codex what to do",
                "     differently (esc)",
                "  Press enter to confirm or esc to",
                "  cancel",
            ],
        ),
        (
            40,
            7,
            &[
                "",
                "  [\u{2026} 43 lines] \u{2303}a view all",
                "\u{203a} 1. Yes, proceed (y)",
                "  2. No, and tell Codex what to do",
                "     differently (esc)",
                "  Press enter to confirm or esc to",
                "  cancel",
            ],
        ),
    ];
    for (width, height, want) in frames {
        let h = open(width, height, "exec_clipped");
        assert_eq!(rows_of(&h), want, "{width}x{height}");
    }
}

/// A header that fits shows no elision and keeps its blank rows.
#[test]
fn a_header_that_fits_is_shown_whole() {
    let h = open(100, 24, "exec_command");
    assert!(!shows(&h, "[\u{2026}"), "{}", h.t.screen());
    assert!(!shows(&h, "view all"), "{}", h.t.screen());
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

/// Backspace on the options clears the selection: no row is marked until a key sets one (the
/// oracle's `backspace_in_options_clears_selection`: `selected_idx` is `None`); a Down selects the
/// first row again.
#[test]
fn user_input_backspace_on_the_options_unmarks_every_row() {
    let mut h = open(120, 16, "user_input");
    h.press("down");
    assert!(shows(&h, "\u{203a} 2. Option 2"), "{}", h.t.screen());
    h.press("backspace");
    assert!(!shows(&h, "\u{203a} "), "{}", h.t.screen());
    h.press("down");
    assert!(shows(&h, "\u{203a} 1. Option 1"), "{}", h.t.screen());
}

/// A draft belongs to its question (the oracle's per-question `ComposerDraft`): it stays when the
/// dialog goes to another question and comes back.
#[test]
fn user_input_a_draft_stays_with_its_question() {
    let mut h = open(120, 16, "user_input_options_then_freeform");
    h.press("ctrl+n");
    assert!(shows(&h, "Question 2/2"), "{}", h.t.screen());
    h.t.type_text("kept draft");
    h.t.pump_for(ms(100));
    h.press("ctrl+p");
    assert!(shows(&h, "Question 1/2"), "{}", h.t.screen());
    assert!(!shows(&h, "kept draft"), "{}", h.t.screen());
    h.press("ctrl+n");
    assert!(shows(&h, "kept draft"), "{}", h.t.screen());
}

/// A request that is not blocking hides its countdown for a minute, shows it in red for the next
/// minute and then answers itself, empty (the oracle's `auto_resolution_*`: grace 60 s, countdown
/// 60 s, `pre_draw_tick`).
#[test]
fn user_input_a_request_that_is_not_blocking_answers_itself_after_two_minutes() {
    let mut h = open(120, 16, "user_input_auto_resolution");
    h.t.pump_for(ms(59_000));
    assert!(!shows(&h, "auto-resolves"), "{}", h.t.screen());
    h.t.pump_for(ms(1_000));
    assert!(shows(&h, "auto-resolves in 1m 00s"), "{}", h.t.screen());
    h.t.pump_for(ms(31_000));
    assert!(shows(&h, "auto-resolves in 29s"), "{}", h.t.screen());
    assert!(shows(&h, "Choose an option."), "{}", h.t.screen());
    h.t.pump_for(ms(30_000));
    assert!(!shows(&h, "Choose an option."), "{}", h.t.screen());
}

/// The countdown is drawn in the error colour (the oracle's `.red()`), the progress it follows is
/// not.
#[test]
fn user_input_the_countdown_is_red() {
    let mut h = open(120, 16, "user_input_auto_resolution");
    h.t.pump_for(ms(60_000));
    let screen = h.t.screen();
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("auto-resolves"))
        .expect("the countdown shows");
    let column = |needle: &str| line[..line.find(needle).expect(needle)].chars().count();
    let countdown = h.t.cell(column("auto-resolves"), y).fg;
    let progress = h.t.cell(column("Question"), y).fg;
    assert_ne!(countdown, progress, "{screen}");
    assert_eq!(countdown, rtwk::style::Color::RED, "{screen}");
}

/// A key stops the countdown for good: nothing shows, nothing is answered.
#[test]
fn user_input_a_key_stops_the_countdown() {
    let mut h = open(120, 16, "user_input_auto_resolution");
    h.t.pump_for(ms(30_000));
    h.press("down");
    h.t.pump_for(ms(100_000));
    assert!(!shows(&h, "auto-resolves"), "{}", h.t.screen());
    assert!(shows(&h, "Choose an option."), "{}", h.t.screen());
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
    let draft = rows
        .iter()
        .position(|r| r.contains("Draft"))
        .expect("Draft");
    let more = rows.iter().position(|r| r.contains("more")).expect("more");
    assert!(!rows[draft].contains("more") && more > draft, "{screen}");
}

/// A secret question draws what is typed as `*` (the oracle's `render_with_mask`) and answers
/// with the real text: `hunter2` never reaches the screen.
#[test]
fn user_input_a_secret_answer_is_masked() {
    let mut h = open(120, 16, "user_input_secret");
    h.t.type_text("hunter2");
    h.t.pump_for(ms(100));
    let screen = h.t.screen();
    assert!(shows(&h, "*******"), "{screen}");
    assert!(!shows(&h, "hunter2"), "{screen}");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "Share details."), "{}", h.t.screen());
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
    assert!(
        shows(&h, "Submit with unanswered questions?"),
        "{}",
        h.t.screen()
    );
    h.press("enter");
    assert!(
        !shows(&h, "Submit with unanswered questions?"),
        "{}",
        h.t.screen()
    );
}

/// ADR-011: a tap on an option of a question moves the cursor there, a second tap on the cursor's
/// row commits it and goes on (the last question answers and the dialog closes).
#[test]
fn user_input_a_tap_moves_the_cursor_and_a_second_tap_answers() {
    let mut h = open(120, 16, "user_input_options_then_freeform");
    tap(&mut h, "2. Option 2");
    assert!(shows(&h, "› 2. Option 2"), "{}", h.t.screen());
    assert!(shows(&h, "Question 1/2"), "the first tap only moves");
    tap(&mut h, "2. Option 2");
    assert!(shows(&h, "Question 2/2 (1 unanswered)"), "{}", h.t.screen());

    let mut h = open(120, 16, "user_input");
    tap(&mut h, "3. Option 3");
    assert!(shows(&h, "› 3. Option 3"), "{}", h.t.screen());
    tap(&mut h, "3. Option 3");
    h.t.pump_for(ms(800));
    assert!(!shows(&h, "Choose an option."), "{}", h.t.screen());
}

/// The confirmation of the unanswered questions is a list too: a tap on `Go back` moves the cursor,
/// a second tap goes back; a tap on the cursor's `Proceed` answers.
#[test]
fn user_input_confirmation_a_tap_moves_the_cursor_and_a_second_tap_accepts() {
    let reach = |h: &mut Harness| {
        h.t.type_text("freeform notes");
        h.t.pump_for(ms(100));
        h.press("ctrl+n");
        h.press("enter");
        assert!(
            shows(h, "Submit with unanswered questions?"),
            "{}",
            h.t.screen()
        );
    };
    let mut h = open(80, 12, "user_input_freeform_then_options");
    reach(&mut h);
    tap(&mut h, "2. Go back");
    assert!(shows(&h, "› 2. Go back"), "{}", h.t.screen());
    assert!(
        shows(&h, "Submit with unanswered questions?"),
        "the first tap only moves"
    );
    tap(&mut h, "2. Go back");
    assert!(shows(&h, "Question 1/2"), "{}", h.t.screen());

    let mut h = open(80, 12, "user_input_freeform_then_options");
    reach(&mut h);
    tap(&mut h, "1. Proceed");
    h.t.pump_for(ms(800));
    assert!(
        !shows(&h, "Submit with unanswered questions?"),
        "{}",
        h.t.screen()
    );
    assert!(!shows(&h, "Question"), "{}", h.t.screen());
}

/// The banner of the threads that wait (`pending_thread_approvals.rs`
/// `render_single_thread_snapshot`, `render_multiple_threads_snapshot`): the thread labels, three
/// at most and then `...`, over the `/subagents` line; the text wraps under itself.
/// The user-verification request (`bottom_pane/user_verification.rs`): the prompt rows, the
/// waiting view, and the keys of each state.
mod user_verification {
    use super::*;

    const SNAPS: &str = "bottom_pane/snapshots/codex_tui__bottom_pane__user_verification__tests__";

    #[test]
    fn the_prompt_rows_are_the_oracles() {
        let h = open(80, 13, "user_verification");
        assert_rows(
            &h.t.screen(),
            &format!("{SNAPS}user_verification_prompt.snap"),
        );
    }

    #[test]
    fn the_waiting_view_has_no_list_and_no_inset() {
        let h = open(80, 9, "user_verification_waiting");
        assert_rows(
            &h.t.screen(),
            &format!("{SNAPS}user_verification_waiting.snap"),
        );
    }

    #[test]
    fn the_cancel_key_digit_escape_and_a_second_tap_cancel_the_prompt() {
        for keys in [&["c"][..], &["2"], &["esc"], &["down", "enter"]] {
            let mut h = open(80, 13, "user_verification");
            for key in keys {
                h.press(key);
            }
            h.t.pump_for(ms(800));
            assert!(
                !shows(&h, "Verify and approve"),
                "{keys:?}\n{}",
                h.t.screen()
            );
        }
        let mut h = open(80, 13, "user_verification");
        tap(&mut h, "Cancel this request");
        assert!(shows(&h, "› 2. Cancel this request"), "{}", h.t.screen());
        tap(&mut h, "Cancel this request");
        h.t.pump_for(ms(800));
        assert!(!shows(&h, "Verify and approve"), "{}", h.t.screen());
    }

    /// Once verification runs only the cancel keys answer: Enter, the digits and the verify key do
    /// nothing (the oracle swallows them in `try_handle_shortcut`).
    #[test]
    fn while_waiting_only_escape_and_the_cancel_key_answer() {
        for key in ["enter", "1", "y"] {
            let mut h = open(80, 9, "user_verification_waiting");
            h.press(key);
            h.t.pump_for(ms(300));
            assert!(
                shows(&h, "Waiting for verification"),
                "{key}\n{}",
                h.t.screen()
            );
        }
        for key in ["esc", "c"] {
            let mut h = open(80, 9, "user_verification_waiting");
            h.press(key);
            h.t.pump_for(ms(800));
            assert!(
                !shows(&h, "Waiting for verification"),
                "{key}\n{}",
                h.t.screen()
            );
        }
    }
}

mod banner {
    use codex_tui_rtwk::app::run::localized;
    use codex_tui_rtwk::l10n::Lang;
    use codex_tui_rtwk::sheets::approvals::PendingThreadApprovals;
    use rtwk_test::{Options, Tester};

    fn rows(width: u16, height: u16, threads: &[&str]) -> Vec<String> {
        let mut t = Tester::with(Options::new().size(width, height));
        let banner = PendingThreadApprovals {
            threads: threads.iter().map(ToString::to_string).collect(),
        };
        t.pump_widget(localized(banner, Lang::En));
        t.screen()
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }

    #[test]
    fn one_thread() {
        assert_eq!(
            rows(40, 2, &["Robie [explorer]"]),
            [
                "  ! Approval needed in Robie [explorer]",
                "    /subagents to review requests"
            ]
        );
    }

    #[test]
    fn four_threads_list_three_and_an_ellipsis() {
        assert_eq!(
            rows(
                44,
                5,
                &[
                    "Main [default]",
                    "Robie [explorer]",
                    "Inspector",
                    "Extra agent"
                ]
            ),
            [
                "  ! Approval needed in Main [default]",
                "  ! Approval needed in Robie [explorer]",
                "  ! Approval needed in Inspector",
                "    ...",
                "    /subagents to review requests"
            ]
        );
    }

    #[test]
    fn a_long_label_wraps_under_its_text() {
        let shown = rows(24, 4, &["A very long agent name"]);
        assert_eq!(shown[0], "  ! Approval needed in A");
        assert_eq!(shown[1], "    very long agent name");
    }

    #[test]
    fn no_thread_draws_nothing() {
        assert!(rows(40, 2, &[]).iter().all(String::is_empty));
    }
}

/// The history cells of an answer (oracle `history_cell/approvals.rs`, `approval_overlay.rs`
/// 2384-2470): a mark, the words under a two-column hanging indent, the verbs bold and the
/// snippet dim.
mod decision_cell {
    use codex_tui_core::screens::approvals::decided::{
        Actor, Decided, Grant, Outcome, Reviewed, Subject, Verdict,
    };
    use codex_tui_rtwk::sheets::approvals::DecisionCell;
    use rtwk::style::{Attrs, Color};

    fn show(decided: Decided, width: u16) -> rtwk_test::Tester {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(width, 8));
        t.pump_widget(DecisionCell { decided });
        t
    }

    fn rows(t: &rtwk_test::Tester) -> Vec<String> {
        let mut rows: Vec<String> = t
            .screen()
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    fn user(subject: Subject, outcome: Outcome) -> Decided {
        Decided::Request {
            actor: Actor::User,
            subject,
            outcome,
        }
    }

    fn command(text: &str) -> Subject {
        Subject::Command(text.to_string())
    }

    fn network(target: &str) -> Subject {
        Subject::Network {
            target: target.to_string(),
        }
    }

    // approval_overlay.rs:2384 exec_history_cell_wraps_with_two_space_indent
    #[test]
    #[ignore = "pending(G49): TextWrap::Word does not break after a slash; the oracle's rows break `tui/src/render/` | `mod.rs tui/src/render/` | `renderable.rs this time`, ours break at the spaces and cut the path at the width"]
    fn exec_history_cell_wraps_with_two_space_indent() {
        let t = show(
            user(
                command("git add tui/src/render/mod.rs tui/src/render/renderable.rs"),
                Outcome::Approved,
            ),
            28,
        );
        assert_eq!(
            rows(&t),
            [
                "✔ You approved codex to run",
                "  git add tui/src/render/",
                "  mod.rs tui/src/render/",
                "  renderable.rs this time",
            ]
        );
    }

    // the same cell where the breaks are spaces: the rows wait two columns in, under the mark
    #[test]
    fn the_rows_under_the_mark_wait_two_columns_in() {
        let t = show(
            user(command("git add alpha beta gamma delta"), Outcome::Approved),
            20,
        );
        assert_eq!(
            rows(&t),
            [
                "✔ You approved codex",
                "  to run git add",
                "  alpha beta gamma",
                "  delta this time",
            ]
        );
    }

    // approval_overlay.rs:2415 exec_history_cell_does_not_render_blank_action_for_empty_command
    #[test]
    fn an_empty_command_says_this_request() {
        assert_eq!(
            rows(&show(user(command(""), Outcome::Approved), 80)),
            ["✔ You approved this request this time"]
        );
        assert_eq!(
            rows(&show(user(command(""), Outcome::ApprovedForSession), 80)),
            ["✔ You approved this request every time this session"]
        );
        assert_eq!(
            rows(&show(user(command(""), Outcome::Denied), 80)),
            ["✗ You did not approve this request"]
        );
        assert_eq!(
            rows(&show(user(command(""), Outcome::Aborted), 80)),
            ["✗ You canceled this request"]
        );
    }

    // approval_requests.rs:204, :251, :294 (the three network cells), approval_overlay.rs:2438
    #[test]
    fn a_network_cell_names_its_target() {
        let target = "https://example.com:8443";
        let say = |outcome| rows(&show(user(network(target), outcome), 100));
        let rule = |allow| Outcome::NetworkRule {
            host: "example.com".into(),
            allow,
        };
        assert_eq!(
            say(Outcome::Approved),
            ["✔ You approved codex network access to https://example.com:8443 this time"]
        );
        assert_eq!(
            say(Outcome::ApprovedForSession),
            [
                "✔ You approved codex network access to https://example.com:8443 every time this session"
            ]
        );
        assert_eq!(
            say(Outcome::Denied),
            ["✗ You did not approve codex network access to https://example.com:8443"]
        );
        assert_eq!(
            say(Outcome::Aborted),
            ["✗ You canceled the request for codex network access to https://example.com:8443"]
        );
        assert_eq!(
            say(rule(true)),
            ["✔ You persisted Codex network access to https://example.com:8443"]
        );
        assert_eq!(
            say(rule(false)),
            ["✗ You denied codex network access to https://example.com:8443 and saved that rule"]
        );
    }

    // `NetworkPolicyAmendment` for a command that names no target: the rule's host is it
    #[test]
    fn a_rule_for_a_command_names_the_rules_host() {
        let rule = Outcome::NetworkRule {
            host: "example.com".into(),
            allow: true,
        };
        assert_eq!(
            rows(&show(user(command("curl example.com"), rule), 100)),
            ["✔ You persisted Codex network access to example.com"]
        );
    }

    #[test]
    fn a_command_cell_says_what_the_user_chose() {
        let say = |outcome| rows(&show(user(command("echo hi"), outcome), 100));
        assert_eq!(
            say(Outcome::ApprovedForSession),
            ["✔ You approved codex to run echo hi every time this session"]
        );
        assert_eq!(
            say(Outcome::ApprovedWithPrefix("echo".into())),
            ["✔ You approved codex to always run commands that start with echo"]
        );
        assert_eq!(
            say(Outcome::Denied),
            ["✗ You did not approve codex to run echo hi"]
        );
        assert_eq!(
            say(Outcome::Aborted),
            ["✗ You canceled the request to run echo hi"]
        );
    }

    // approval_requests.rs:433 exec_approval_decision_truncates_multiline_and_long_commands
    #[test]
    fn a_multi_line_command_shows_its_first_line_and_dots() {
        let t = show(
            user(command("echo line1\necho line2"), Outcome::Aborted),
            80,
        );
        assert_eq!(
            rows(&t),
            ["✗ You canceled the request to run echo line1 ..."]
        );
    }

    #[test]
    fn a_long_command_is_cut_to_eighty_graphemes() {
        let long = format!("echo {}", "a".repeat(195));
        let t = show(user(command(&long), Outcome::Aborted), 120);
        let kept = format!("echo {}...", "a".repeat(72));
        assert_eq!(kept.chars().count(), 80);
        assert_eq!(
            rows(&t),
            [format!("✗ You canceled the request to run {kept}")]
        );
        // a grapheme is one unit whatever its width: eighty flags are not cut
        let flags = "🇯🇵".repeat(80);
        let t = show(user(command(&flags), Outcome::Approved), 200);
        assert_eq!(
            rows(&t),
            [format!("✔ You approved codex to run {flags} this time")]
        );
    }

    #[test]
    fn the_marks_and_words_are_styled_as_the_oracle_does() {
        let t = show(user(command("echo hi"), Outcome::Approved), 60);
        assert_eq!(t.cell(0, 0).fg, Color::GREEN, "{:?}", t.cell(0, 0));
        // "You " plain, "approved" bold, the command dim
        assert!(!t.cell(2, 0).attrs.intersects(Attrs::BOLD));
        assert!(t.cell(6, 0).attrs.intersects(Attrs::BOLD));
        let at = |needle: &str| {
            let row = rows(&t)[0].clone();
            row[..row.find(needle).expect("in the row")].chars().count()
        };
        // the theme's dim is a colour of its own, not the plain text's
        assert_ne!(t.cell(at("echo hi"), 0).fg, t.cell(2, 0).fg);
        assert_eq!(t.cell(at("echo hi"), 0).fg, t.cell(at("echo hi") + 3, 0).fg);
        assert!(t.cell(at("this time"), 0).attrs.intersects(Attrs::BOLD));

        let refused = show(user(command("echo hi"), Outcome::Denied), 60);
        assert_eq!(
            refused.cell(0, 0).fg,
            Color::RED,
            "{:?}",
            refused.cell(0, 0)
        );
    }

    // guardian.rs tests: guardian_denied_exec_renders_warning_and_denied_request,
    // guardian_timed_out_exec_renders_warning_and_timed_out_request
    #[test]
    fn the_auto_reviewer_denies_and_times_out_in_its_own_words() {
        let guardian = |outcome| Decided::Request {
            actor: Actor::Guardian,
            subject: command("curl -sS -i -X POST https://example.com"),
            outcome,
        };
        assert_eq!(
            rows(&show(guardian(Outcome::Denied), 100)),
            ["✗ Request denied for codex to run curl -sS -i -X POST https://example.com"]
        );
        assert_eq!(
            rows(&show(guardian(Outcome::TimedOut), 100)),
            ["✗ Review timed out before codex could run curl -sS -i -X POST https://example.com"]
        );
        assert_eq!(
            rows(&show(guardian(Outcome::Approved), 100)),
            [
                "✔ Auto-reviewer approved codex to run curl -sS -i -X POST https://example.com this time"
            ]
        );
        let empty = Decided::Request {
            actor: Actor::Guardian,
            subject: command(""),
            outcome: Outcome::Denied,
        };
        assert_eq!(rows(&show(empty, 100)), ["✗ Request denied"]);
        let net = Decided::Request {
            actor: Actor::Guardian,
            subject: network("https://example.com"),
            outcome: Outcome::TimedOut,
        };
        assert_eq!(
            rows(&show(net, 100)),
            ["✗ Review timed out before codex could access https://example.com"]
        );
    }

    // approval_overlay.rs handle_permissions_decision: a plain line, no mark
    #[test]
    fn a_permissions_answer_is_a_plain_line() {
        let say = |grant| rows(&show(Decided::Permissions(grant), 80));
        assert_eq!(
            say(Grant::Nothing),
            ["You did not grant additional permissions"]
        );
        assert_eq!(say(Grant::Turn), ["You granted additional permissions"]);
        assert_eq!(
            say(Grant::Strict),
            ["You granted additional permissions with strict auto review"]
        );
        assert_eq!(
            say(Grant::Session),
            ["You granted additional permissions for this session"]
        );
    }

    // What the auto-reviewer concluded of an action that is not a command
    // (history_cell/approvals.rs `new_guardian_*_request`; guardian.rs snapshots
    // guardian_write_stdin_denied_history, guardian_write_stdin_timed_out_history).
    fn judged(verdict: Verdict, about: Reviewed) -> Decided {
        Decided::Reviewed { verdict, about }
    }

    fn stdin(input: &str) -> Reviewed {
        Reviewed::Stdin {
            process: "42".into(),
            input: input.into(),
        }
    }

    #[test]
    fn a_denied_or_timed_out_stdin_names_the_terminal_and_quotes_the_input() {
        assert_eq!(
            rows(&show(judged(Verdict::Denied, stdin("confirm\n")), 100)),
            [r#"✗ Request denied for codex to send input to terminal 42: "confirm\n""#]
        );
        assert_eq!(
            rows(&show(judged(Verdict::TimedOut, stdin("confirm\n")), 100)),
            [r#"✗ Review timed out before codex could send input to terminal 42: "confirm\n""#]
        );
    }

    #[test]
    fn a_long_stdin_is_cut_at_eighty_graphemes_of_its_quoted_text() {
        let long = "x".repeat(100);
        let quoted = format!("\"{long}\"");
        let cut: String = quoted.chars().take(77).collect::<String>() + "...";
        assert_eq!(cut.chars().count(), 80);
        assert_eq!(
            rows(&show(judged(Verdict::Denied, stdin(&long)), 200)),
            [format!(
                "✗ Request denied for codex to send input to terminal 42: {cut}"
            )]
        );
    }

    // history_cell/approvals.rs: new_guardian_denied_patch_request,
    // new_guardian_timed_out_patch_request
    #[test]
    fn a_patch_names_its_file_or_how_many_files() {
        let one = Reviewed::Patch(vec!["src/main.rs".into()]);
        let many = Reviewed::Patch(vec!["a.rs".into(), "b.rs".into(), "c.rs".into()]);
        let say = |verdict, about| rows(&show(judged(verdict, about), 100));
        assert_eq!(
            say(Verdict::Denied, one.clone()),
            ["✗ Request denied for codex to apply a patch touching src/main.rs"]
        );
        assert_eq!(
            say(Verdict::Denied, many.clone()),
            ["✗ Request denied for codex to apply a patch touching 3 files"]
        );
        assert_eq!(
            say(Verdict::TimedOut, one),
            ["✗ Review timed out before codex could apply a patch touching src/main.rs"]
        );
        assert_eq!(
            say(Verdict::TimedOut, many),
            ["✗ Review timed out before codex could apply a patch touching 3 files"]
        );
    }

    // chatwidget/tool_requests.rs `on_guardian_assessment`: the MCP, network and permissions arms
    #[test]
    fn an_mcp_call_network_access_and_a_permission_request_each_have_their_sentence() {
        let say = |verdict, about| rows(&show(judged(verdict, about), 100));
        let mcp = || Reviewed::Mcp {
            server: "docs".into(),
            tool: "search".into(),
        };
        assert_eq!(
            say(Verdict::Denied, mcp()),
            ["✗ Request denied for codex to call MCP tool docs.search"]
        );
        assert_eq!(
            say(Verdict::TimedOut, mcp()),
            ["✗ Review timed out before codex could call MCP tool docs.search"]
        );
        let net = || Reviewed::Network("https://example.com".into());
        assert_eq!(
            say(Verdict::Denied, net()),
            ["✗ Request denied for codex to access https://example.com"]
        );
        assert_eq!(
            say(Verdict::TimedOut, net()),
            ["✗ Review timed out before codex could access https://example.com"]
        );
        let permissions = |reason: Option<&str>| Reviewed::Permissions(reason.map(String::from));
        assert_eq!(
            say(Verdict::Denied, permissions(Some("write to /etc"))),
            ["✗ Request denied for codex to request permissions: write to /etc"]
        );
        assert_eq!(
            say(Verdict::TimedOut, permissions(Some("write to /etc"))),
            ["✗ Review timed out before codex could request permissions: write to /etc"]
        );
        // a blank reason is no reason
        assert_eq!(
            say(Verdict::Denied, permissions(Some("  \n"))),
            ["✗ Request denied for codex to request permissions"]
        );
        assert_eq!(
            say(Verdict::Denied, permissions(None)),
            ["✗ Request denied for codex to request permissions"]
        );
    }

    #[test]
    fn a_verdict_cell_has_a_red_mark_a_bold_verb_and_a_dim_subject() {
        let t = show(judged(Verdict::Denied, stdin("hi")), 100);
        let row = rows(&t)[0].clone();
        let at = |needle: &str| row[..row.find(needle).expect("in the row")].chars().count();
        assert_eq!(t.cell(0, 0).fg, Color::RED, "{:?}", t.cell(0, 0));
        assert!(!t.cell(2, 0).attrs.intersects(Attrs::BOLD), "Request");
        assert!(t.cell(at("denied"), 0).attrs.intersects(Attrs::BOLD));
        let dim = t.cell(at("codex to"), 0).fg;
        assert_ne!(dim, t.cell(2, 0).fg, "the subject is dim");
        assert_eq!(t.cell(at("terminal"), 0).fg, dim);
    }

    #[test]
    fn a_patch_cell_dims_the_file_or_the_count_and_not_the_words_around_it() {
        let cells = |files: &[&str]| {
            let about = Reviewed::Patch(files.iter().map(ToString::to_string).collect());
            let t = show(judged(Verdict::Denied, about), 100);
            let row = rows(&t)[0].clone();
            let at = |needle: &str| row[..row.find(needle).expect("in the row")].chars().count();
            (t, at("a patch"), row)
        };
        let (t, patch, row) = cells(&["src/main.rs"]);
        let file = patch + "a patch touching ".len();
        assert_ne!(t.cell(file, 0).fg, t.cell(patch, 0).fg, "{row}");
        let (t, patch, row) = cells(&["a.rs", "b.rs"]);
        let count = patch + "a patch touching ".len();
        assert_ne!(t.cell(count, 0).fg, t.cell(patch, 0).fg, "{row}");
        assert_eq!(t.cell(count + 2, 0).fg, t.cell(patch, 0).fg, "{row}");
    }

    #[test]
    fn a_verdict_cell_waits_two_columns_in_under_its_mark() {
        let t = show(judged(Verdict::Denied, stdin("hi")), 30);
        let rows = rows(&t);
        assert!(rows.len() > 1, "{rows:?}");
        assert!(rows[0].starts_with("✗ Request denied"), "{rows:?}");
        for row in &rows[1..] {
            assert!(row.starts_with("  ") && !row.starts_with("   "), "{row:?}");
        }
    }
}

/// A thread's approvals part after the guardian notifications a fixture would send: the model
/// is made through core's `apply`, never handed to a widget.
#[allow(clippy::expect_used)]
mod reviews {
    use codex_app_server_protocol::ServerNotification;
    use codex_tui_core::screens::approvals::ThreadPart;
    use codex_tui_core::screens::approvals::guardian::project;

    fn notification(method: &str, params: &str) -> ServerNotification {
        serde_json::from_str(&format!(r#"{{"method":"{method}","params":{params}}}"#))
            .expect("a notification")
    }

    pub fn command(text: &str) -> String {
        format!(r#"{{"type":"command","source":"shell","command":{text:?},"cwd":"/tmp"}}"#)
    }

    pub const STDIN: &str = r#"{"type":"writeStdin","approvalId":"a","processId":"42","stdin":"confirm\n","cwd":"/tmp"}"#;

    pub fn patch(files: &[&str]) -> String {
        let files = files
            .iter()
            .map(|file| format!("{file:?}"))
            .collect::<Vec<_>>()
            .join(",");
        format!(r#"{{"type":"applyPatch","cwd":"/tmp","files":[{files}]}}"#)
    }

    pub fn network(target: &str) -> String {
        format!(
            r#"{{"type":"networkAccess","id":"n","turnId":"t","target":{target:?},"host":"example.com","protocol":"https","port":443}}"#
        )
    }

    pub fn mcp(server: &str, tool: &str, connector: Option<&str>) -> String {
        let connector = connector.map_or("null".to_string(), |name| format!("{name:?}"));
        format!(
            r#"{{"type":"mcpToolCall","server":{server:?},"toolName":{tool:?},"connectorId":null,"connectorName":{connector},"toolTitle":null}}"#
        )
    }

    pub fn permissions(reason: Option<&str>) -> String {
        let reason = reason.map_or("null".to_string(), |text| format!("{text:?}"));
        format!(r#"{{"type":"requestPermissions","reason":{reason},"permissions":{{}}}}"#)
    }

    /// One `item/autoApprovalReview/...` notification of `thread-1`.
    pub fn review(
        id: &str,
        status: &str,
        action: &str,
        rationale: Option<&str>,
    ) -> ServerNotification {
        let rationale = rationale.map_or("null".to_string(), |r| format!("{r:?}"));
        let done = if status == "inProgress" {
            String::new()
        } else {
            r#","completedAtMs":1,"decisionSource":"agent""#.to_string()
        };
        let method = if status == "inProgress" {
            "item/autoApprovalReview/started"
        } else {
            "item/autoApprovalReview/completed"
        };
        notification(
            method,
            &format!(
                r#"{{"threadId":"thread-1","turnId":"turn-1","startedAtMs":0{done},"reviewId":"{id}","targetItemId":null,"review":{{"status":"{status}","riskLevel":null,"userAuthorization":null,"rationale":{rationale}}},"action":{action}}}"#
            ),
        )
    }

    /// The part after `notifications`, applied the way the app applies them.
    pub fn part(notifications: &[ServerNotification]) -> ThreadPart {
        let mut part = ThreadPart::default();
        for notification in notifications {
            let (_, ev) = project(notification).expect("a guardian notification");
            part.apply(ev);
        }
        part
    }
}

// chatwidget/tool_requests.rs `on_guardian_assessment`, status_state.rs
// `PendingGuardianReviewStatus::status_indicator_state`; guardian.rs snapshots
// guardian_write_stdin_review_status, guardian_parallel_reviews_render_aggregate_status
mod review_status {
    use super::reviews::{self, STDIN, command, part};
    use codex_tui_rtwk::sheets::approvals::strings::EN;
    use codex_tui_rtwk::sheets::approvals::{ReviewStatus, review_status};

    fn running(actions: &[String]) -> Option<ReviewStatus> {
        let notes: Vec<_> = actions
            .iter()
            .enumerate()
            .map(|(i, action)| reviews::review(&format!("g{i}"), "inProgress", action, None))
            .collect();
        review_status(&EN, &part(&notes).reviews)
    }

    #[test]
    fn no_review_under_way_says_nothing() {
        assert_eq!(running(&[]), None);
    }

    #[test]
    fn one_review_is_the_header_and_its_one_line_of_detail() {
        let status = running(&[STDIN.into()]).expect("a status");
        assert_eq!(status.header, "Reviewing approval request");
        assert_eq!(status.details, r#"send input to terminal 42: "confirm\n""#);
        assert_eq!(status.max_lines, 1);
    }

    #[test]
    fn parallel_reviews_are_bullets_in_arrival_order() {
        let status = running(&[
            command("rm -rf '/tmp/guardian target 1'"),
            command("rm -rf '/tmp/guardian target 2'"),
        ])
        .expect("a status");
        assert_eq!(status.header, "Reviewing 2 approval requests");
        assert_eq!(
            status.details,
            "• rm -rf '/tmp/guardian target 1'\n• rm -rf '/tmp/guardian target 2'"
        );
        assert_eq!(status.max_lines, 4);
    }

    #[test]
    fn more_than_three_reviews_list_three_and_count_the_rest() {
        let many: Vec<String> = (1..=5).map(|n| command(&format!("job {n}"))).collect();
        let status = running(&many).expect("a status");
        assert_eq!(status.header, "Reviewing 5 approval requests");
        assert_eq!(status.details, "• job 1\n• job 2\n• job 3\n+2 more");
    }

    #[test]
    fn each_kind_of_action_has_its_summary() {
        let detail = |action: String| running(&[action]).expect("a status").details;
        assert_eq!(
            detail(reviews::patch(&["/tmp/a.rs"])),
            "apply_patch touching /tmp/a.rs"
        );
        assert_eq!(
            detail(reviews::patch(&["/tmp/a.rs", "/tmp/b.rs"])),
            "apply_patch touching 2 files"
        );
        assert_eq!(
            detail(reviews::network("https://example.com")),
            "network access to https://example.com"
        );
        assert_eq!(
            detail(reviews::mcp("docs", "search", None)),
            "MCP search on docs"
        );
        // the connector's name stands in for the server's
        assert_eq!(
            detail(reviews::mcp("docs", "search", Some("Docs app"))),
            "MCP search on Docs app"
        );
    }

    #[test]
    fn a_permission_request_names_its_reason_unless_it_is_blank() {
        let detail = |reason: Option<&str>| {
            running(&[reviews::permissions(reason)])
                .expect("a status")
                .details
        };
        assert_eq!(detail(None), "permission request");
        assert_eq!(detail(Some("  ")), "permission request");
        assert_eq!(
            detail(Some("  write to /etc \n")),
            "permission request: write to /etc"
        );
    }

    #[test]
    fn a_finished_review_leaves_the_status() {
        let mut notes = vec![reviews::review("g0", "inProgress", &command("ls"), None)];
        notes.push(reviews::review("g0", "approved", &command("ls"), None));
        assert_eq!(review_status(&EN, &part(&notes).reviews), None);
    }
}

// chatwidget/permission_popups.rs `open_auto_review_denials_popup`; guardian.rs snapshots
// guardian_write_stdin_denials_popup, auto_review_denials_popup
mod auto_review {
    use super::reviews::{self, STDIN, command, part};
    use crate::common::{Harness, ms, start_sized};
    use crate::dialogs::oracle::assert_rows;
    use codex_tui_core::app::{AppEv, BackendEv};
    use codex_tui_rtwk::sheets::approvals::auto_review_menu;
    use codex_tui_rtwk::sheets::approvals::strings::EN;
    use codex_tui_rtwk::widgets::picker::list::List;

    fn denied(
        id: &str,
        action: &str,
        rationale: Option<&str>,
    ) -> codex_app_server_protocol::ServerNotification {
        reviews::review(id, "denied", action, rationale)
    }

    fn menu(notes: &[codex_app_server_protocol::ServerNotification]) -> List {
        auto_review_menu(&EN, &part(notes).recent_denials)
    }

    #[test]
    fn the_list_is_a_header_row_then_a_row_per_denial_newest_first() {
        let list = menu(&[
            denied("g1", &command("rm one"), Some("Destructive.")),
            denied("g2", &command("rm two"), None),
        ]);
        assert_eq!(list.title.as_deref(), Some("Auto-review Denials"));
        assert_eq!(
            list.subtitle.as_deref(),
            Some("Select a denied action to approve.")
        );
        assert!(list.searchable);
        assert!(!list.numbered, "the rows carry no numbers");
        let rows: Vec<_> = list
            .items
            .iter()
            .map(|item| {
                (
                    item.name.as_str(),
                    item.description.as_deref(),
                    item.id.as_str(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            [
                ("Action", Some("Rationale"), ""),
                (
                    "rm two",
                    Some("Auto-review did not include a rationale"),
                    "g2"
                ),
                ("rm one", Some("Destructive."), "g1"),
            ]
        );
    }

    #[test]
    fn the_header_row_cannot_be_picked_and_every_search_keeps_it() {
        let list = menu(&[denied("g1", &command("rm one"), Some("Destructive."))]);
        assert!(list.items[0].is_disabled());
        assert_eq!(list.items[0].search_value.as_deref(), Some(""));
    }

    #[test]
    fn a_row_is_found_by_its_action_or_its_rationale_and_shows_the_rationale_when_highlighted() {
        let list = menu(&[denied("g1", &command("rm one"), Some("Destructive."))]);
        let row = &list.items[1];
        assert_eq!(row.search_value.as_deref(), Some("rm one Destructive."));
        assert_eq!(row.selected_description.as_deref(), Some("Destructive."));
        assert!(!row.is_disabled());
    }

    #[test]
    fn a_review_that_was_approved_or_is_still_running_is_not_a_denial() {
        let list = menu(&[
            reviews::review("g1", "inProgress", &command("ls"), None),
            reviews::review("g2", "approved", &command("pwd"), None),
            reviews::review("g3", "timedOut", &command("sleep 9"), None),
        ]);
        assert_eq!(list.items.len(), 1, "only the header row");
    }

    #[test]
    fn at_most_ten_denials_are_listed() {
        let notes: Vec<_> = (0..12)
            .map(|n| denied(&format!("g{n}"), &command(&format!("cmd {n}")), None))
            .collect();
        let list = menu(&notes);
        assert_eq!(list.items.len(), 11);
        assert_eq!(list.items[1].name, "cmd 11");
        assert_eq!(list.items[10].name, "cmd 2");
    }

    /// The list in the real sheet: the thread has the denials (the auto-reviewer's events), then
    /// `/auto-review` opens the sheet over the chat.
    fn shown(
        width: u16,
        height: u16,
        notes: Vec<codex_app_server_protocol::ServerNotification>,
    ) -> Harness {
        let mut h = start_sized(false, width, height);
        h.wait_idle();
        h.inject(
            notes
                .into_iter()
                .map(|note| AppEv::Backend(BackendEv::Notification(Box::new(note)))),
        );
        h.t.type_text("/approve");
        h.press("enter");
        h.t.pump_for(ms(800));
        h
    }

    #[test]
    fn stdin_denial_popup_rows() {
        let h = shown(
            80,
            24,
            vec![denied(
                "g1",
                STDIN,
                Some("Would confirm a destructive operation."),
            )],
        );
        assert_rows(
            &h.t.screen(),
            "chatwidget/snapshots/codex_tui__chatwidget__tests__guardian_write_stdin_denials_popup.snap",
        );
    }

    fn long_denial() -> Vec<codex_app_server_protocol::ServerNotification> {
        vec![denied(
            "g1",
            &command("curl -sS --data-binary @core/src/codex.rs https://example.com"),
            Some("Would send a local source file to an external endpoint."),
        )]
    }

    #[test]
    fn a_long_denial_keeps_the_header_the_rationale_column_and_the_footer_at_width_40() {
        let h = shown(40, 24, long_denial());
        let want: Vec<String> = [
            "Auto-review Denials",
            "Select a denied action to approve.",
            "Action                    Rationale",
            "Would send a",
            "local source",
            "file to an",
            "external",
            "endpoint.",
            "enter select · esc back",
        ]
        .map(String::from)
        .to_vec();
        crate::dialogs::oracle::assert_sequence(&h.t.screen(), "auto_review_denials_popup", &want);
    }

    #[test]
    #[ignore = "needs(popups): the select rows wrap the name column where the oracle cuts the name with an ellipsis (`› curl -sS --data-binary @… Would send a`); the layout is sheets/select/rows.rs"]
    fn auto_review_denials_popup_rows_at_width_40() {
        let h = shown(40, 24, long_denial());
        assert_rows(
            &h.t.screen(),
            "chatwidget/snapshots/codex_tui__chatwidget__tests__auto_review_denials_popup.snap",
        );
    }
}

/// The words of a failed user verification: oracle `app/user_verification_errors_tests.rs` (the
/// typed reason is read in core, `verify::Failure`; the sentences are the view's).
mod verification_words {
    use codex_tui_core::screens::approvals::verify::{Cause, Failure};
    use codex_tui_rtwk::sheets::approvals::strings::EN;
    use serde_json::json;

    fn message(data: serde_json::Value) -> &'static str {
        EN.verification_failure(Failure::of_server(Some(&data)).cause)
    }

    #[test]
    fn a_failure_is_worded_by_its_typed_reason_without_the_providers_diagnostics() {
        assert_eq!(
            message(json!({ "type": "unavailable", "reason": "credentialMissing" })),
            "No user-verification credential is available in the local Codex binary."
        );
        assert_eq!(
            message(json!({ "type": "unavailable", "reason": "secret native failure" })),
            "The local Codex binary could not complete user verification."
        );
    }

    #[test]
    fn unavailable_reasons_use_device_and_session_neutral_copy() {
        assert_eq!(
            message(json!({ "type": "unavailable", "reason": "biometricsUnavailable" })),
            "Biometric verification is currently unavailable on this device."
        );
        assert_eq!(
            message(json!({ "type": "unavailable", "reason": "providerUnavailable" })),
            "User verification is unavailable for this request."
        );
    }

    #[test]
    fn the_other_reasons_have_their_sentences() {
        let of = |cause| EN.verification_failure(cause);
        assert_eq!(
            of(Failure::unreachable().cause),
            "Could not complete user verification with the local Codex binary."
        );
        assert_eq!(of(Cause::UserCancelled), "User verification was cancelled.");
        assert_eq!(of(Cause::Interrupted), "User verification was interrupted.");
        assert_eq!(
            of(Cause::AuthenticationFailed),
            "Biometric verification did not succeed. The request was cancelled."
        );
        assert_eq!(
            of(Cause::Timeout),
            "User verification timed out. The request was cancelled."
        );
        assert_eq!(
            of(Cause::InvalidParams),
            "The local Codex binary could not verify this request."
        );
        assert_eq!(
            EN.verify_remote,
            "User verification is unavailable for remote workspaces."
        );
    }
}

/// The lines `/auto-review` leaves in the history (oracle `new_info_event`: a dim bullet, the
/// message, the hint after a space; `new_error_event`: `■ ` and the message, all red).
mod review_line {
    use codex_tui_core::threads::ReviewLine;
    use codex_tui_rtwk::sheets::approvals::ReviewLineCell;
    use rtwk::style::Color;

    fn show(line: ReviewLine) -> rtwk_test::Tester {
        let mut t = rtwk_test::Tester::with(rtwk_test::Options::new().size(160, 3));
        t.pump_widget(ReviewLineCell { line });
        t
    }

    fn first_row(t: &rtwk_test::Tester) -> String {
        t.screen()
            .lines()
            .next()
            .unwrap_or_default()
            .trim_end()
            .to_string()
    }

    #[test]
    fn no_denials_is_an_info_line_with_its_hint() {
        let t = show(ReviewLine::NoDenials);
        assert_eq!(
            first_row(&t),
            "• No recent auto-review denials in this thread. Denials are recorded after auto-review rejects an action."
        );
        // the bullet and the hint are the muted colour, the message is not
        let hint_at = first_row(&t).find("Denials").unwrap_or_default();
        assert_ne!(t.cell(0, 0).fg, t.cell(2, 0).fg, "{:?}", t.cell(0, 0));
        assert_eq!(t.cell(0, 0).fg, t.cell(hint_at, 0).fg);
    }

    #[test]
    fn a_retry_is_an_info_line_too() {
        let t = show(ReviewLine::RetryApproved);
        assert_eq!(
            first_row(&t),
            "• Approval recorded for one retry of the selected auto-review denial. The model will see the approval context; the retry still goes through auto-review."
        );
    }

    #[test]
    fn a_denial_that_is_gone_is_a_red_error_line() {
        let t = show(ReviewLine::DenialGone);
        assert_eq!(
            first_row(&t),
            "■ That auto-review denial is no longer available."
        );
        assert_eq!(t.cell(0, 0).fg, Color::RED, "{:?}", t.cell(0, 0));
        assert_eq!(t.cell(10, 0).fg, Color::RED);
    }

    #[test]
    fn a_failure_to_localize_says_what_the_server_said() {
        let t = show(ReviewLine::Unlocalized("bad path".into()));
        assert!(first_row(&t).starts_with("■ "), "{}", first_row(&t));
        assert!(first_row(&t).contains("bad path"), "{}", first_row(&t));
    }
}
