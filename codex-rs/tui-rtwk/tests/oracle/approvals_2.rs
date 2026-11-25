//! Shard `approvals_2` (screens S13, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The first row of the banner of the threads that wait for an approval.
fn banner_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .position(|row| row.contains("Approval needed in"))
        .unwrap_or_else(|| panic!("no banner on the screen:\n{}", s.text()))
}

// ---- oracle: bottom_pane/pending_thread_approvals.rs (3 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/pending_thread_approvals.rs:109 desired_height_empty kind=ui owner=dialogs-pane
#[test]
fn desired_height_empty() {
    kit::na(
        Owner::DialogsPane,
        "asserts the banner's desired height with no pending thread (0), a layout number that draws no row",
    );
}

// ORACLE bottom_pane/pending_thread_approvals.rs:115 render_single_thread_snapshot kind=snap owner=dialogs-pane
#[test]
fn render_single_thread_snapshot() {
    // The typing delay keeps the dialog from covering the pane: the banner is what shows.
    let mut s = Screen::build(Owner::DialogsPane).size(40, 14).live().open();
    s.type_text("a");
    s.feed(include_str!("fixtures/approvals_2/pending_single.jsonl"));
    let at = banner_row(&s);
    s.assert_row(at, "  ! Approval needed in Robie [explorer]");
    s.assert_row(at + 1, "    /subagents to review requests");
    // the delayed dialog comes up: no timer is left pending
    s.wait(1_000);
}

// ORACLE bottom_pane/pending_thread_approvals.rs:129 render_multiple_threads_snapshot kind=snap owner=dialogs-pane
#[test]
fn render_multiple_threads_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane).size(44, 16).live().open();
    s.type_text("a");
    s.feed(include_str!("fixtures/approvals_2/pending_multiple.jsonl"));
    let at = banner_row(&s);
    s.assert_row(at, "  ! Approval needed in Main [default]");
    s.assert_row(at + 1, "  ! Approval needed in Robie [explorer]");
    s.assert_row(at + 2, "  ! Approval needed in Inspector");
    s.assert_row(at + 4, "    /subagents to review requests");
    s.wait(1_000);
}

// ---- oracle: bottom_pane/questions_tests.rs (1 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/questions_tests.rs:7 questions_flush_buffered_typing_and_record_activity kind=snap owner=dialogs-pane snap=codex_tui__bottom_pane__question_tests__question_collapsed_countdown
#[test]
fn questions_flush_buffered_typing_and_record_activity() {
    kit::pending(
        Owner::DialogsPane,
        "no async-questions pane in the port: a message's questions without options (push_async_questions) have no seed; the collapsed countdown summary is the visible half; flushing what was typed ahead when a question arrives is G8 (rtwk has no input discard)",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(21, 2).live().open();
    s.paste("pasted ");
    s.type_text("answer");
    s.wait(15_000);
    s.assert_snap("question_collapsed_countdown");
}

// ---- oracle: bottom_pane/request_user_input/mod.rs (55 screen-level tests here; 21 state tests left to dialogs-pane)
// ORACLE bottom_pane/request_user_input/mod.rs:1796 interrupt_discards_queued_requests_and_emits_interrupt kind=ui owner=dialogs-pane
#[test]
fn interrupt_discards_queued_requests_and_emits_interrupt() {
    kit::na(
        Owner::DialogsPane,
        "asserts the overlay's done flag and the Interrupt op sent on Esc with queued requests; no drawn text is asserted",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:1881 auto_resolution_hides_timer_during_grace_period kind=ui owner=dialogs-pane
#[test]
fn auto_resolution_hides_timer_during_grace_period() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/auto_options.jsonl"))
        .open();
    s.assert_absent("auto-resolves");
}

// ORACLE bottom_pane/request_user_input/mod.rs:1912 auto_resolution_visible_countdown_snapshot kind=snap owner=dialogs-pane snap=request_user_input_auto_resolution_countdown
#[test]
fn auto_resolution_visible_countdown_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/auto_three.jsonl"))
        .open();
    s.wait(60_000);
    // The oracle renders the overlay alone; here it is the bottom pane of the chat: ten rows
    // from the bottom of the 16-row screen.
    s.assert_snap_region("request_user_input_auto_resolution_countdown", 6, 0);
}

// ORACLE bottom_pane/request_user_input/mod.rs:1938 auto_resolution_visible_countdown_is_red kind=ui owner=dialogs-pane
#[test]
fn auto_resolution_visible_countdown_is_red() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/auto_options.jsonl"))
        .open();
    s.wait(60_000);
    s.assert_text_style("auto-resolves in 1m 00s", "fg=Red");
    s.assert_text_style("Question 1/1", "!fg=Red");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2011 auto_resolution_key_interaction_snoozes_timer kind=ui owner=dialogs-pane
#[test]
fn auto_resolution_key_interaction_snoozes_timer() {
    kit::na(
        Owner::DialogsPane,
        "asserts the snoozed timer state, pre_draw_tick and that no answer is sent after a key; the timer is not drawn in the test",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2038 auto_resolution_paste_interaction_snoozes_timer kind=ui owner=dialogs-pane
#[test]
fn auto_resolution_paste_interaction_snoozes_timer() {
    kit::na(
        Owner::DialogsPane,
        "asserts the snoozed timer state, pre_draw_tick and that no answer is sent after a paste; the timer is not drawn in the test",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2251 enter_commits_default_selection_on_last_option_question kind=ui owner=dialogs-pane
#[test]
fn enter_commits_default_selection_on_last_option_question() {
    kit::na(
        Owner::DialogsPane,
        "asserts the UserInputAnswer event Enter sends for the default selection; no drawn text is asserted",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2270 enter_commits_default_selection_on_non_last_option_question kind=ui owner=dialogs-pane
#[test]
fn enter_commits_default_selection_on_non_last_option_question() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_options.jsonl"))
        .open();
    s.keys("enter");
    s.assert_contains("Question 2/2 (1 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2316 number_keys_select_and_submit_options kind=ui owner=dialogs-pane
#[test]
fn number_keys_select_and_submit_options() {
    kit::na(
        Owner::DialogsPane,
        "asserts the UserInputAnswer event after digit 2 (select and submit); no drawn text is asserted",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2335 vim_keys_move_option_selection kind=ui owner=dialogs-pane
#[test]
fn vim_keys_move_option_selection() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("j");
    s.assert_contains("› 2. Option 2");
    s.keys("k");
    s.assert_contains("› 1. Option 1");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2357 typing_in_options_does_not_open_notes kind=ui owner=dialogs-pane
#[test]
fn typing_in_options_does_not_open_notes() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_options.jsonl"))
        .open();
    s.assert_contains("Question 1/2 (2 unanswered)");
    s.type_text("x");
    s.assert_contains("Question 1/2 (2 unanswered)");
    s.assert_contains(
        "tab to add notes | enter to submit answer | ←/→ to navigate questions | esc to interrupt",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2383 h_l_move_between_questions_in_options kind=ui owner=dialogs-pane
#[test]
fn h_l_move_between_questions_in_options() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_options.jsonl"))
        .open();
    s.assert_contains("Question 1/2 (2 unanswered)");
    s.keys("l");
    s.assert_contains("Question 2/2 (2 unanswered)");
    s.keys("h");
    s.assert_contains("Question 1/2 (2 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2407 left_right_move_between_questions_in_options kind=ui owner=dialogs-pane
#[test]
fn left_right_move_between_questions_in_options() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_options.jsonl"))
        .open();
    s.assert_contains("Question 1/2 (2 unanswered)");
    s.keys("right");
    s.assert_contains("Question 2/2 (2 unanswered)");
    s.keys("left");
    s.assert_contains("Question 1/2 (2 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2431 horizontal_list_keys_move_between_questions_in_options kind=ui owner=dialogs-pane
#[test]
fn horizontal_list_keys_move_between_questions_in_options() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_options.jsonl"))
        .open();
    s.assert_contains("Question 1/2 (2 unanswered)");
    s.keys("ctrl+l");
    s.assert_contains("Question 2/2 (2 unanswered)");
    s.keys("ctrl+h");
    s.assert_contains("Question 1/2 (2 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2455 options_notes_focus_hides_question_navigation_tip kind=ui owner=dialogs-pane
#[test]
fn options_notes_focus_hides_question_navigation_tip() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_options.jsonl"))
        .open();
    s.assert_contains(
        "tab to add notes | enter to submit answer | ←/→ to navigate questions | esc to interrupt",
    );
    s.keys("tab");
    s.assert_contains("tab or esc to clear notes | enter to submit answer");
    s.assert_absent("←/→ to navigate questions");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2584 request_user_input_uses_remapped_interrupt_binding_while_notes_are_visible kind=ui owner=dialogs-pane
#[test]
fn request_user_input_uses_remapped_interrupt_binding_while_notes_are_visible() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .keymap(&[("chat.interrupt_turn", "f12")])
        .open();
    s.keys("tab");
    s.assert_contains("tab or esc to clear notes | enter to submit answer | f12 to interrupt");
    s.keys("f12");
    s.assert_absent("Choose an option.");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2618 tab_opens_notes_when_option_selected kind=ui owner=dialogs-pane
#[test]
fn tab_opens_notes_when_option_selected() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("down tab");
    s.assert_contains("tab or esc to clear notes | enter to submit answer");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2661 switching_from_freeform_with_text_resets_focus_and_keeps_last_option_empty kind=ui owner=dialogs-pane
#[test]
fn switching_from_freeform_with_text_resets_focus_and_keeps_last_option_empty() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!(
            "fixtures/approvals_2/freeform_then_options.jsonl"
        ))
        .open();
    s.type_text("freeform notes");
    // The oracle calls `move_question(next)`; a freeform question's key for it is ctrl+n.
    s.keys("ctrl+n");
    s.assert_contains("Question 2/2 (2 unanswered)");
    s.assert_absent("Add notes");
    s.keys("enter");
    s.assert_contains("Submit with unanswered questions?");
    s.assert_contains("1 unanswered question");
    s.keys("1 enter");
    s.assert_absent("Submit with unanswered questions?");
    s.assert_absent("Question 2/2");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2705 esc_in_notes_mode_without_options_interrupts kind=ui owner=dialogs-pane
#[test]
fn esc_in_notes_mode_without_options_interrupts() {
    kit::na(
        Owner::DialogsPane,
        "asserts the done flag and the Interrupt op on Esc in a freeform form; no drawn text is asserted",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2722 esc_in_options_mode_interrupts kind=ui owner=dialogs-pane
#[test]
fn esc_in_options_mode_interrupts() {
    kit::na(
        Owner::DialogsPane,
        "asserts the done flag and the Interrupt op on Esc in the options form; no drawn text is asserted",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2739 esc_in_notes_mode_clears_notes_and_hides_ui kind=ui owner=dialogs-pane
#[test]
fn esc_in_notes_mode_clears_notes_and_hides_ui() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("tab");
    s.assert_contains("tab or esc to clear notes | enter to submit answer");
    s.keys("esc");
    s.assert_contains("tab to add notes | enter to submit answer | esc to interrupt");
    s.assert_contains("› 1. Option 1");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2767 esc_in_notes_mode_with_text_clears_notes_and_hides_ui kind=ui owner=dialogs-pane
#[test]
fn esc_in_notes_mode_with_text_clears_notes_and_hides_ui() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("tab");
    s.type_text("a");
    s.assert_contains("tab or esc to clear notes | enter to submit answer");
    s.keys("esc");
    s.assert_contains("tab to add notes | enter to submit answer | esc to interrupt");
    s.assert_contains("› 1. Option 1");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2796 esc_drops_committed_answers kind=ui owner=dialogs-pane
#[test]
fn esc_drops_committed_answers() {
    kit::na(
        Owner::DialogsPane,
        "asserts the Interrupt op after Esc with a committed answer (expect_interrupt_only); no drawn text is asserted",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:2824 backspace_in_options_clears_selection kind=ui owner=dialogs-pane
#[test]
fn backspace_in_options_clears_selection() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("down");
    s.assert_contains("› 2. Option 2");
    s.keys("backspace");
    s.assert_absent("› ");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2845 backspace_on_empty_notes_closes_notes_ui kind=ui owner=dialogs-pane
#[test]
fn backspace_on_empty_notes_closes_notes_ui() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("tab");
    s.assert_contains("tab or esc to clear notes | enter to submit answer");
    s.keys("backspace");
    s.assert_contains("tab to add notes | enter to submit answer | esc to interrupt");
    s.assert_contains("› 1. Option 1");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2871 tab_in_notes_clears_notes_and_hides_ui kind=ui owner=dialogs-pane
#[test]
fn tab_in_notes_clears_notes_and_hides_ui() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("tab");
    s.type_text("Some notes");
    s.keys("tab");
    s.assert_absent("Some notes");
    s.assert_contains("tab to add notes | enter to submit answer | esc to interrupt");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2930 freeform_requires_enter_with_text_to_mark_answered kind=ui owner=dialogs-pane
#[test]
fn freeform_requires_enter_with_text_to_mark_answered() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_freeform.jsonl"))
        .open();
    s.assert_contains("Question 1/2 (2 unanswered)");
    s.type_text("Draft");
    s.keys("enter");
    s.assert_contains("(1 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2959 freeform_enter_with_empty_text_is_unanswered kind=ui owner=dialogs-pane
#[test]
fn freeform_enter_with_empty_text_is_unanswered() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_freeform.jsonl"))
        .open();
    s.keys("enter");
    s.assert_contains("Question 1/2 (2 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:2982 freeform_shift_enter_inserts_newline_without_advancing kind=ui owner=dialogs-pane
#[test]
fn freeform_shift_enter_inserts_newline_without_advancing() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_freeform.jsonl"))
        .open();
    s.type_text("Draft");
    s.keys("shift+enter");
    s.assert_contains("Draft");
    s.assert_contains("Question 1/2 (2 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3011 freeform_uses_configured_composer_submit_binding kind=ui owner=dialogs-pane
#[test]
fn freeform_uses_configured_composer_submit_binding() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_freeform.jsonl"))
        .keymap(&[("composer.submit", "ctrl+j")])
        .open();
    s.type_text("Draft");
    s.keys("ctrl+j");
    s.assert_contains("Question 2/2 (1 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3042 freeform_submit_binding_wins_over_question_navigation kind=ui owner=dialogs-pane
#[test]
fn freeform_submit_binding_wins_over_question_navigation() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_freeform.jsonl"))
        .keymap(&[("composer.submit", "ctrl+n")])
        .open();
    s.type_text("Draft");
    s.keys("ctrl+n");
    s.assert_contains("Question 2/2 (1 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3115 freeform_commit_resets_when_draft_changes kind=ui owner=dialogs-pane
#[test]
fn freeform_commit_resets_when_draft_changes() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_freeform.jsonl"))
        .open();
    s.type_text("Committed");
    s.keys("enter");
    s.assert_contains("Question 2/2 (1 unanswered)");
    // The oracle calls `move_question`; the freeform keys for it are ctrl+p / ctrl+n (left and right
    // move the text cursor).
    s.keys("ctrl+p");
    s.type_text("Edited");
    s.keys("ctrl+n");
    s.assert_contains("Question 2/2 (2 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3196 notes_submission_commits_selected_option kind=ui owner=dialogs-pane
#[test]
fn notes_submission_commits_selected_option() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/two_options.jsonl"))
        .open();
    s.keys("down tab");
    s.type_text("Notes");
    s.keys("enter");
    s.assert_contains("Question 2/2 (1 unanswered)");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3228 tab_and_enter_open_notes_for_other_option kind=ui owner=dialogs-pane
#[test]
fn tab_and_enter_open_notes_for_other_option() {
    for key in ["tab", "enter"] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(120, 16)
            .entry("approvals", "user_input")
            .open();
        s.keys(&format!("down down down {key}"));
        s.assert_contains("tab or esc to clear notes | enter to submit answer");
    }
}

// ORACLE bottom_pane/request_user_input/mod.rs:3308 large_paste_is_preserved_when_switching_questions kind=ui owner=dialogs-pane
#[test]
fn large_paste_is_preserved_when_switching_questions() {
    kit::na(
        Owner::DialogsPane,
        "asserts the composer's pending-paste fields and expanded text on a draft; the placeholder is not asserted on the screen",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:3336 pending_paste_placeholder_survives_submission_and_back_navigation kind=ui owner=dialogs-pane
#[test]
fn pending_paste_placeholder_survives_submission_and_back_navigation() {
    kit::na(
        Owner::DialogsPane,
        "asserts the draft's pending-paste fields after submit and back navigation; no drawn text is asserted",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:3367 request_user_input_options_snapshot kind=snap owner=dialogs-pane snap=request_user_input_options
#[test]
fn request_user_input_options_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "user_input")
        .open();
    // The oracle's screen is the pane alone; ours has the debug menu above it. The pane is eleven
    // rows with the menu surface's blank row at the foot (`menu_surface_padding_height`), which the
    // oracle's snapshot trims.
    s.assert_snap_region("request_user_input_options", 5, 0);
}

// ORACLE bottom_pane/request_user_input/mod.rs:3387 request_user_input_options_notes_visible_snapshot kind=snap owner=dialogs-pane snap=request_user_input_options_notes_visible
#[test]
fn request_user_input_options_notes_visible_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "user_input_options")
        .open();
    s.keys("tab");
    s.assert_snap("request_user_input_options_notes_visible");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3410 request_user_input_tight_height_snapshot kind=snap owner=dialogs-pane snap=request_user_input_tight_height
#[test]
fn request_user_input_tight_height_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 10)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.assert_snap("request_user_input_tight_height");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3455 desired_height_keeps_spacers_and_preferred_options_visible kind=ui owner=dialogs-pane
#[test]
fn desired_height_keeps_spacers_and_preferred_options_visible() {
    kit::na(
        Owner::DialogsPane,
        "asserts the overlay's section rectangles and desired height for a wrapped question (rtwk lays out the form); the drawn rows are covered by the snapshot tests",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:3521 request_user_input_wrapped_options_snapshot kind=snap owner=dialogs-pane snap=request_user_input_wrapped_options
#[test]
fn request_user_input_wrapped_options_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(110, 10)
        .script(include_str!("fixtures/approvals_2/wrapped_options.jsonl"))
        .open();
    s.assert_snap("request_user_input_wrapped_options");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3554 request_user_input_long_option_text_snapshot kind=snap owner=dialogs-pane snap=request_user_input_long_option_text
#[test]
fn request_user_input_long_option_text_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "G49 + G39b (rtwk-gaps.md): the label column is capped at 70% and wraps now (`GridTrack::content().within`); left: the oracle's continuation lines hang under the text after `› 1. ` (a Row of prefix and Expanded text), it breaks after `/` (G49: `TextWrap::Word` does not), and its pane is bottom-aligned (dialogs-pane slice)",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 18)
        .entry("approvals", "user_input_very_long_option")
        .open();
    s.assert_snap("request_user_input_long_option_text");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3574 selected_long_wrapped_option_stays_visible kind=ui owner=dialogs-pane
#[test]
fn selected_long_wrapped_option_stays_visible() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 20)
        .script(include_str!("fixtures/approvals_2/long_scroll.jsonl"))
        .open();
    s.keys("down down");
    s.assert_contains("› 3. Use Detailed Hint C");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3597 request_user_input_footer_wrap_snapshot kind=snap owner=dialogs-pane snap=request_user_input_footer_wrap
#[test]
fn request_user_input_footer_wrap_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(52, 11)
        .entry("approvals", "user_input_two_options")
        .open();
    // The oracle's setup selects the second option (`selected_idx = Some(1)`).
    s.keys("down");
    s.assert_snap("request_user_input_footer_wrap");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3625 request_user_input_scroll_options_snapshot kind=snap owner=dialogs-pane snap=request_user_input_scrolling_options
#[test]
fn request_user_input_scroll_options_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 12)
        .script(include_str!("fixtures/approvals_2/what_next.jsonl"))
        .open();
    s.keys("down down down");
    s.assert_snap("request_user_input_scrolling_options");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3677 request_user_input_hidden_options_footer_snapshot kind=snap owner=dialogs-pane snap=request_user_input_hidden_options_footer
#[test]
fn request_user_input_hidden_options_footer_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "G39b (rtwk-gaps.md): the oracle sizes the label column over the rows in view (here `3. Review a diff`) and draws `option 4/5 |` in the footer of a cut list; the port sizes it over all rows (the window of a scroll view is known only after layout; a measure over `ScrollController` pixels was tried and does not settle) and has no tip",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 10)
        .entry("approvals", "user_input_what_next")
        .open();
    s.keys("down down down");
    s.assert_snap("request_user_input_hidden_options_footer");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3729 wrapped_question_preserves_the_complete_url_destination kind=snap owner=dialogs-pane snap=request_user_input_wrapped_url
#[test]
fn wrapped_question_preserves_the_complete_url_destination() {
    kit::pending(
        Owner::DialogsPane,
        "G49 + G72: every row but the wrapped question equals the oracle's (entry approvals/user_input_url_question); rtwk Text does not break after a slash (the oracle: `Review https://github.com/openai/` / `codex/pull/12345?diff=split before`) and a plain string has no autolink (no OSC 8 link on the URL)",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(40, 18)
        .entry("approvals", "user_input_url_question")
        .open();
    s.assert_snap("request_user_input_wrapped_url");
    s.assert_link(
        "https://github.com/openai/codex/pull/12345?diff=split",
        "https://github.com/openai/codex/pull/12345?diff=split",
    );
}

// ORACLE bottom_pane/request_user_input/mod.rs:3770 request_user_input_freeform_snapshot kind=snap owner=dialogs-pane snap=request_user_input_freeform
#[test]
fn request_user_input_freeform_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 10)
        .entry("approvals", "user_input_freeform")
        .open();
    s.assert_snap("request_user_input_freeform");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3787 request_user_input_freeform_chord_only_submit_snapshot kind=snap owner=dialogs-pane
#[test]
fn request_user_input_freeform_chord_only_submit_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 10)
        .entry("approvals", "user_input_freeform")
        .keymap(&[("composer.submit", "ctrl+x enter")])
        .open();
    s.assert_has_row_trimmed("Question 1/1 (1 unanswered)");
    s.assert_has_row_trimmed("Share details.");
    s.assert_has_row_trimmed("› Type your answer (optional)");
    s.assert_has_row_trimmed("⌃x enter to submit answer | esc to interrupt");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3825 request_user_input_freeform_remapped_submit_snapshot kind=snap owner=dialogs-pane snap=request_user_input_freeform_remapped_submit
#[test]
fn request_user_input_freeform_remapped_submit_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 10)
        .entry("approvals", "user_input_freeform")
        .keymap(&[("composer.submit", "ctrl+j")])
        .open();
    s.assert_snap("request_user_input_freeform_remapped_submit");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3845 request_user_input_freeform_remapped_interrupt_snapshot kind=snap owner=dialogs-pane snap=request_user_input_freeform_remapped_interrupt
#[test]
fn request_user_input_freeform_remapped_interrupt_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 10)
        .entry("approvals", "user_input_freeform")
        .keymap(&[("chat.interrupt_turn", "f12")])
        .open();
    s.assert_snap("request_user_input_freeform_remapped_interrupt");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3865 request_user_input_multi_question_first_snapshot kind=snap owner=dialogs-pane snap=request_user_input_multi_question_first
#[test]
fn request_user_input_multi_question_first_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 15)
        .entry("approvals", "user_input_options_then_freeform")
        .open();
    s.assert_snap("request_user_input_multi_question_first");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3888 request_user_input_multi_question_last_snapshot kind=snap owner=dialogs-pane snap=request_user_input_multi_question_last
#[test]
fn request_user_input_multi_question_last_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 12)
        .entry("approvals", "user_input_options_then_freeform")
        .open();
    s.keys("right");
    s.assert_snap("request_user_input_multi_question_last");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3912 request_user_input_unanswered_confirmation_snapshot kind=snap owner=dialogs-pane snap=request_user_input_unanswered_confirmation
#[test]
fn request_user_input_unanswered_confirmation_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 12)
        .entry("approvals", "user_input_unanswered")
        .open();
    s.assert_snap("request_user_input_unanswered_confirmation");
}

// ORACLE bottom_pane/request_user_input/mod.rs:3938 options_scroll_while_editing_notes kind=ui owner=dialogs-pane
#[test]
fn options_scroll_while_editing_notes() {
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .script(include_str!("fixtures/approvals_2/options.jsonl"))
        .open();
    s.keys("tab");
    s.type_text("Notes");
    s.keys("down");
    s.assert_contains("› 2. Option 2");
}

// ---- oracle: bottom_pane/request_user_input/render_tests.rs (1 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE bottom_pane/request_user_input/render_tests.rs:22 halfwidth_sound_marks_are_truncated_and_rendered_at_a_grapheme_boundary kind=snap owner=dialogs-pane snap=request_user_input_halfwidth_sound_marks_truncation
#[test]
fn halfwidth_sound_marks_are_truncated_and_rendered_at_a_grapheme_boundary() {
    kit::pending(
        Owner::DialogsPane,
        "the oracle draws two styled Lines, truncated by the word-boundary helper to width 5, into a bare 5x2 Buffer; no screen seeds a styled line at that width and the port has no such truncation",
    );
    let s = Screen::build(Owner::DialogsPane).size(5, 2).live().open();
    s.assert_snap("request_user_input_halfwidth_sound_marks_truncation");
}

// ---- oracle: bottom_pane/user_verification_tests.rs (7 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE bottom_pane/user_verification_tests.rs:58 prompt_snapshot kind=snap owner=dialogs-pane snap=user_verification_prompt
#[test]
fn prompt_snapshot() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 12)
        .entry("approvals", "user_verification")
        .open();
    s.assert_snap("user_verification_prompt");
}

// ORACLE bottom_pane/user_verification_tests.rs:67 clipped_prompt_exposes_fullscreen_hint kind=snap owner=dialogs-pane snap=user_verification_narrow
#[test]
fn clipped_prompt_exposes_fullscreen_hint() {
    let s = Screen::build(Owner::DialogsPane)
        .size(40, 8)
        .entry("approvals", "user_verification")
        .open();
    s.assert_snap("user_verification_narrow");
}

// ORACLE bottom_pane/user_verification_tests.rs:76 waiting_snapshot_and_duplicate_approval_suppression kind=snap owner=dialogs-pane snap=user_verification_waiting
#[test]
fn waiting_snapshot_and_duplicate_approval_suppression() {
    kit::pending(
        Owner::DialogsPane,
        "Enter on Verify starts the controller (userVerification/verify, the attempt, the waiting flag on the queued prompt): app.rs, handed over; the waiting view itself is the entry user_verification_waiting, pinned in tests/sheets/approvals.rs",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 7)
        .entry("approvals", "user_verification")
        .open();
    s.keys("enter");
    s.assert_snap("user_verification_waiting");
}

// ORACLE bottom_pane/user_verification_tests.rs:93 cancellation_is_available_before_and_during_verification kind=ui owner=dialogs-pane
#[test]
fn cancellation_is_available_before_and_during_verification() {
    kit::na(
        Owner::DialogsPane,
        "asserts the UserVerificationDecision::Cancel event and the view completion for Esc, c and x in both states; no drawn text is asserted",
    );
}

// ORACLE bottom_pane/user_verification_tests.rs:135 full_screen_request_is_available_before_and_during_verification kind=ui owner=dialogs-pane
#[test]
fn full_screen_request_is_available_before_and_during_verification() {
    kit::na(
        Owner::DialogsPane,
        "asserts the FullScreenUserVerificationRequest app event, that no decision is sent and the terminal title state; no drawn text is asserted",
    );
}

// ORACLE bottom_pane/user_verification_tests.rs:159 long_url_tokens_remain_visible_in_a_narrow_verification_prompt kind=snap owner=dialogs-pane snap=user_verification_long_urls
#[test]
fn long_url_tokens_remain_visible_in_a_narrow_verification_prompt() {
    kit::pending(
        Owner::DialogsPane,
        "G72: a URL in a plain string has no autolink (the prompt's OSC 8 links) and TextWrap::Word breaks the URL after a hyphen where the oracle cuts it at the width; the rows differ only there",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(30, 18)
        .entry("approvals", "user_verification_long_urls")
        .open();
    s.assert_snap("user_verification_long_urls");
    s.keys("enter");
    s.assert_link(
        "https://example.com/production/critical-release-candidate",
        "https://example.com/production/critical-release-candidate",
    );
}

// ORACLE bottom_pane/user_verification_tests.rs:206 full_screen_pager_reaches_the_end_of_a_long_request kind=snap owner=dialogs-pane snap=user_verification_full_screen_bottom
#[test]
fn full_screen_pager_reaches_the_end_of_a_long_request() {
    kit::pending(
        Owner::DialogsPane,
        "no full-screen pager for the user-verification request: the oracle pages a static overlay over the prompt header, which the port does not have",
    );
    let mut s = Screen::build(Owner::DialogsPane).size(40, 12).live().open();
    s.keys("end");
    s.assert_snap("user_verification_full_screen_bottom");
}
