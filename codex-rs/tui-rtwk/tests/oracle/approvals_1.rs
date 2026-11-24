//! Shard `approvals_1` (screens S13, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The first screen row that holds `text`: where a sheet's rows start when it
/// is drawn over the debug menu.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or(usize::MAX)
}

// ---- oracle: bottom_pane/approval_overlay.rs (28 screen-level tests here; 9 state tests left to dialogs-pane)
// ORACLE bottom_pane/approval_overlay.rs:1348 configured_list_cancel_aborts_exec_approval kind=ui owner=dialogs-pane
#[test]
fn configured_list_cancel_aborts_exec_approval() {
    kit::pending(
        Owner::DialogsPane,
        "no keymap seed (`.keymap` is missing until the port reads key bindings; entry approvals/exec_request exists)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("approvals", "exec_request")
        .keymap(&[("list.cancel", "q")])
        .open();
    s.keys("q");
    // The oracle's SubmitThreadOp(Cancel) is an event, not drawn; the visible half is the closed prompt.
    s.assert_absent("Would you like to run the following command?");
}

// ORACLE bottom_pane/approval_overlay.rs:1379 configured_list_cancel_cancels_mcp_elicitation kind=ui owner=dialogs-pane
#[test]
fn configured_list_cancel_cancels_mcp_elicitation() {
    kit::pending(
        Owner::DialogsPane,
        "no keymap seed (`.keymap` is missing until the port reads key bindings) and no entry with make_elicitation_request (server test-server, message Need more information)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("approvals", "elicitation_need_more_info")
        .keymap(&[("list.cancel", "q")])
        .open();
    s.keys("q");
    // The oracle's ResolveElicitation(Cancel) is an event, not drawn; the visible half is the closed form.
    s.assert_absent("Need more information");
}

// ORACLE bottom_pane/approval_overlay.rs:1410 shortcut_triggers_selection kind=ui owner=dialogs-pane
#[test]
fn shortcut_triggers_selection() {
    kit::na(
        Owner::DialogsPane,
        "asserts that y emits a SubmitThreadOp approval op (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:1428 deny_shortcut_submits_denied_exec_decision kind=ui owner=dialogs-pane
#[test]
fn deny_shortcut_submits_denied_exec_decision() {
    kit::na(
        Owner::DialogsPane,
        "asserts the Decline op that d emits (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:1469 approval_headers_preserve_wrapped_url_destinations kind=snap owner=dialogs-pane snap=approval_header_wrapped_url
#[test]
fn approval_headers_preserve_wrapped_url_destinations() {
    kit::pending(
        Owner::DialogsPane,
        "G72: the wrapped URL is plain text, so no cell carries its OSC 8 link (rtwk Text links only a Span::link; the rows equal the oracle's snapshot, pinned in tests/sheets/approvals.rs)",
    );
    // The oracle draws the header alone at 32 columns; the sheet draws it two
    // cells in, so 36 columns, and the header's rows are the region.
    let s = Screen::build(Owner::DialogsPane)
        .size(36, 16)
        .entry("approvals", "exec_url_reason")
        .open();
    s.assert_snap_region("approval_header_wrapped_url", row_of(&s, "Reason:"), 2);
    // The URL is one OSC 8 link over its two wrapped rows.
    s.assert_link(
        "https://github.com/openai/codex/",
        "https://github.com/openai/codex/pull/12345?diff=split",
    );
    s.assert_link(
        "pull/12345?diff=split",
        "https://github.com/openai/codex/pull/12345?diff=split",
    );
    // The oracle also loops the permission, elicitation and patch headers for their links; those are not in the snapshot and are not written here yet.
}

// ORACLE bottom_pane/approval_overlay.rs:1523 network_deny_shortcut_submits_policy_deny_decision kind=ui owner=dialogs-pane
#[test]
fn network_deny_shortcut_submits_policy_deny_decision() {
    kit::na(
        Owner::DialogsPane,
        "asserts the network policy deny op that d emits (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:1612 o_opens_source_thread_for_cross_thread_approval kind=ui owner=dialogs-pane
#[test]
fn o_opens_source_thread_for_cross_thread_approval() {
    kit::na(
        Owner::DialogsPane,
        "asserts the SelectAgentThread app event that o emits, nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:1646 configured_open_thread_shortcut_opens_source_thread kind=ui owner=dialogs-pane
#[test]
fn configured_open_thread_shortcut_opens_source_thread() {
    kit::na(
        Owner::DialogsPane,
        "asserts the SelectAgentThread app event after the configured key, nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:1689 cross_thread_footer_hint_mentions_o_shortcut kind=snap owner=dialogs-pane snap=approval_overlay_cross_thread_prompt
#[test]
fn cross_thread_footer_hint_mentions_o_shortcut() {
    Screen::build(Owner::DialogsPane)
        .size(80, 12)
        .entry("approvals", "exec_cross_thread")
        .open()
        .assert_snap("approval_overlay_cross_thread_prompt");
}

// ORACLE bottom_pane/approval_overlay.rs:1719 exec_prefix_option_emits_execpolicy_amendment kind=ui owner=dialogs-pane
#[test]
fn exec_prefix_option_emits_execpolicy_amendment() {
    kit::na(
        Owner::DialogsPane,
        "asserts the AcceptWithExecpolicyAmendment op that p emits (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:1773 network_deny_forever_shortcut_is_not_bound kind=ui owner=dialogs-pane
#[test]
fn network_deny_forever_shortcut_is_not_bound() {
    kit::na(
        Owner::DialogsPane,
        "asserts that no approval event is emitted for d, an app event, nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:1814 header_includes_command_snippet kind=ui owner=dialogs-pane
#[test]
fn header_includes_command_snippet() {
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("approvals", "exec_snippet")
        .open();
    s.assert_contains("echo hello world");
}

// ORACLE bottom_pane/approval_overlay.rs:2027 permissions_session_shortcut_submits_session_scope kind=ui owner=dialogs-pane
#[test]
fn permissions_session_shortcut_submits_session_scope() {
    kit::na(
        Owner::DialogsPane,
        "asserts the RequestPermissionsResponse op with session scope (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:2053 permissions_deny_shortcut_uses_deny_keymap kind=ui owner=dialogs-pane
#[test]
fn permissions_deny_shortcut_uses_deny_keymap() {
    kit::na(
        Owner::DialogsPane,
        "asserts the empty RequestPermissionsResponse op (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:2090 permissions_strict_auto_review_shortcut_submits_turn_scope_with_strict_review kind=ui owner=dialogs-pane
#[test]
fn permissions_strict_auto_review_shortcut_submits_turn_scope_with_strict_review() {
    kit::na(
        Owner::DialogsPane,
        "asserts the strict auto review RequestPermissionsResponse op (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:2117 additional_permissions_prompt_shows_permission_rule_line kind=ui owner=dialogs-pane
#[test]
fn additional_permissions_prompt_shows_permission_rule_line() {
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .entry("approvals", "exec_additional_permissions")
        .open();
    s.assert_contains("Permission rule:");
    s.assert_contains("network;");
}

// ORACLE bottom_pane/approval_overlay.rs:2175 additional_permissions_prompt_snapshot kind=snap owner=dialogs-pane snap=approval_overlay_additional_permissions_prompt
#[test]
fn additional_permissions_prompt_snapshot() {
    // The sheet is bottom-aligned: 14 rows tall, the oracle's buffer.
    Screen::build(Owner::DialogsPane)
        .size(120, 14)
        .entry("approvals", "exec_additional_permissions")
        .open()
        .assert_snap("approval_overlay_additional_permissions_prompt");
}

// ORACLE bottom_pane/approval_overlay.rs:2213 permissions_prompt_snapshot kind=snap owner=dialogs-pane snap=approval_overlay_permissions_prompt
#[test]
fn permissions_prompt_snapshot() {
    Screen::build(Owner::DialogsPane)
        .size(120, 18)
        .entry("approvals", "permissions")
        .open()
        // The oracle's screen is the bottom pane alone; ours has the debug menu above it.
        .assert_snap_region("approval_overlay_permissions_prompt", 4, 0);
}

// ORACLE bottom_pane/approval_overlay.rs:2224 apply_patch_prompt_with_thread_label_omits_command_line kind=ui owner=dialogs-pane
#[test]
fn apply_patch_prompt_with_thread_label_omits_command_line() {
    let s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .entry("approvals", "patch_thread_label")
        .open();
    s.assert_contains("Thread: Banach [worker]");
    s.assert_contains("Description: Apply proposed file edits");
    s.assert_contains("Destination: /tmp/bug1.txt");
    s.assert_contains("Destination: /tmp/new.txt");
    s.assert_contains("Destination: /tmp/old.txt");
    s.assert_contains("Destination: C:\\workspace\\old.txt");
    s.assert_contains("Destination: C:\\workspace\\new.txt");
    s.assert_contains("o to open thread");
    s.assert_absent("$ apply_patch");
}

// ORACLE bottom_pane/approval_overlay.rs:2282 apply_patch_prompt_without_changes_shows_unavailable_destination kind=snap owner=dialogs-pane snap=approval_overlay_patch_destination_unavailable
#[test]
fn apply_patch_prompt_without_changes_shows_unavailable_destination() {
    // The sheet is bottom-aligned: 12 rows tall, the oracle's buffer.
    Screen::build(Owner::DialogsPane)
        .size(120, 12)
        .entry("approvals", "patch_no_changes")
        .open()
        .assert_snap("approval_overlay_patch_destination_unavailable");
}

// ORACLE bottom_pane/approval_overlay.rs:2301 network_exec_prompt_title_includes_host kind=snap owner=dialogs-pane snap=network_exec_prompt
#[test]
fn network_exec_prompt_title_includes_host() {
    kit::pending(
        Owner::DialogsPane,
        "screen feature (approval sheet): the selected option row `› 1. Yes, just this once` is drawn bold magenta, the oracle draws it bold reversed with the default colour; the cell after the title is bold in ours, plain in the oracle (the kit reads the snapshot, quote and all)",
    );
    // The sheet is bottom-aligned: 13 rows tall, the oracle's buffer.
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 13)
        .entry("approvals", "exec_network")
        .open();
    s.assert_snap("network_exec_prompt");
    s.assert_contains("Do you want to approve network access to \"example.com\"?");
    s.assert_absent("$ curl");
    s.assert_absent("don't ask again");
}

// ORACLE bottom_pane/approval_overlay.rs:2363 ctrl_shift_a_opens_fullscreen kind=ui owner=dialogs-pane
#[test]
fn ctrl_shift_a_opens_fullscreen() {
    kit::na(
        Owner::DialogsPane,
        "asserts the FullScreenApprovalRequest app event that ctrl+shift+a emits, nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:2384 exec_history_cell_wraps_with_two_space_indent kind=ui owner=dialogs-pane
#[test]
fn exec_history_cell_wraps_with_two_space_indent() {
    kit::pending(
        Owner::DialogsPane,
        "the decision cell (`✔ You approved ...`) is a transcript row: its model (core `approvals::decided::Decided`) and widget (`sheets::approvals::DecisionCell`) are done and pinned in core/tests/approvals.rs `decided` and tests/sheets/approvals.rs `decision_cell`; open is the thread holding it (NEED threads.rs `Item::Decision` from `ThreadEv::Decided`, app.rs recording it on resolve, transcript_pane.rs drawing it), and an approvals entry draws the sheet over the debug menu, not over a chat (entry approvals/exec_zsh_git_add exists); and G49: TextWrap::Word does not break after a slash (the cell's rows are pinned apart from that in tests/sheets/approvals.rs)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(28, 8)
        .entry("approvals", "exec_zsh_git_add")
        .open();
    s.keys("y");
    // The oracle compares four exact rows; the kit checks them in order until the port's rows are known.
    s.assert_in_order(&[
        "✔ You approved codex to run",
        "  git add tui/src/render/",
        "  mod.rs tui/src/render/",
        "  renderable.rs this time",
    ]);
}

// ORACLE bottom_pane/approval_overlay.rs:2415 exec_history_cell_does_not_render_blank_action_for_empty_command kind=ui owner=dialogs-pane
#[test]
fn exec_history_cell_does_not_render_blank_action_for_empty_command() {
    kit::pending(
        Owner::DialogsPane,
        "the decision cell (`✔ You approved ...`) is a transcript row: its model (core `approvals::decided::Decided`) and widget (`sheets::approvals::DecisionCell`) are done and pinned in core/tests/approvals.rs `decided` and tests/sheets/approvals.rs `decision_cell`; open is the thread holding it (NEED threads.rs `Item::Decision` from `ThreadEv::Decided`, app.rs recording it on resolve, transcript_pane.rs drawing it), and an approvals entry draws the sheet over the debug menu, not over a chat (entry approvals/exec_empty_command exists)",
    );
    let mut approved = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("approvals", "exec_empty_command")
        .open();
    approved.keys("y");
    approved.assert_contains("✔ You approved this request this time");

    let mut for_session = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("approvals", "exec_empty_command")
        .open();
    for_session.keys("a");
    for_session.assert_contains("✔ You approved this request every time this session");
}

// ORACLE bottom_pane/approval_overlay.rs:2438 network_access_command_history_uses_target_without_structured_context kind=ui owner=dialogs-pane
#[test]
fn network_access_command_history_uses_target_without_structured_context() {
    kit::pending(
        Owner::DialogsPane,
        "the decision cell (`✔ You approved ...`) is a transcript row: its model (core `approvals::decided::Decided`) and widget (`sheets::approvals::DecisionCell`) are done and pinned in core/tests/approvals.rs `decided` and tests/sheets/approvals.rs `decision_cell`; open is the thread holding it (NEED threads.rs `Item::Decision` from `ThreadEv::Decided`, app.rs recording it on resolve, transcript_pane.rs drawing it), and an approvals entry draws the sheet over the debug menu, not over a chat (and no entry approvals/exec_network_access yet)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("approvals", "exec_network_access")
        .open();
    s.keys("y");
    s.assert_contains("✔ You approved codex network access to https://example.com:8443 this time");
}

// ORACLE bottom_pane/approval_overlay.rs:2484 esc_cancels_mcp_elicitation kind=ui owner=dialogs-pane
#[test]
fn esc_cancels_mcp_elicitation() {
    kit::na(
        Owner::DialogsPane,
        "asserts the ResolveElicitation Cancel op that esc emits (an app event), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:2506 esc_still_cancels_elicitation_with_custom_overlap kind=ui owner=dialogs-pane
#[test]
fn esc_still_cancels_elicitation_with_custom_overlap() {
    kit::na(
        Owner::DialogsPane,
        "asserts the ResolveElicitation ops for esc and n with a keymap overlap (app events), nothing drawn",
    );
}

// ORACLE bottom_pane/approval_overlay.rs:2570 enter_sets_last_selected_index_without_dismissing kind=ui owner=dialogs-pane
#[test]
fn enter_sets_last_selected_index_without_dismissing() {
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("approvals", "exec_request")
        .open();
    s.keys("enter");
    // The oracle asserts the prompt is complete and the Accept op (an event); the visible half is the closed prompt.
    s.assert_absent("Would you like to run the following command?");
}

// ---- oracle: bottom_pane/approval_overlay/clipping_tests.rs (1 screen-level tests here; 0 state tests left to dialogs-pane)
// ORACLE bottom_pane/approval_overlay/clipping_tests.rs:8 clipped_exec_approval_opens_the_complete_command kind=snap owner=dialogs-pane snap=codex_tui__bottom_pane__approval_overlay__clipping_tests__clipped_exec_approval_opens_the_complete_command
#[test]
fn clipped_exec_approval_opens_the_complete_command() {
    kit::pending(
        Owner::DialogsPane,
        "no keymap seed for open_fullscreen (ctrl+g, then unbound): the hint shows the default `⌃a` (entry approvals/exec_clipped; sheet tests `a_header_taller_than_the_pane_is_cut_with_its_line_count` and `a_cut_header_gives_up_its_rows_to_the_options_and_the_hint_last` hold the oracle's four bound frames with ⌃a), and ctrl+g opens no pager yet (the transcript chunk's static pager hook)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 14)
        .entry("approvals", "exec_clipped")
        .keymap(&[("approval.open_fullscreen", "ctrl+g")])
        .open();
    s.assert_snap_section(
        "codex_tui__bottom_pane__approval_overlay__clipping_tests__clipped_exec_approval_opens_the_complete_command",
        "bound=true 80x14",
    );
    s.assert_contains("⌃g view all");
    s.assert_absent("destructive_suffix");
    s.assert_absent("reason_suffix");
    s.assert_contains("Yes, proceed");
    s.keys("ctrl+g");
    // The oracle's FullScreenApprovalRequest event is not drawn; the prompt stays open.
    s.assert_contains("Yes, proceed");
    // The other frames (40x10, 40x8, 40x7, and the unbound variant) are not written yet.
}

// ---- oracle: bottom_pane/async_questions/state_tests.rs (19 screen-level tests here; 1 state tests left to dialogs-pane)
// ORACLE bottom_pane/async_questions/state_tests.rs:31 taking_pending_drafts_preserves_order_expands_pastes_and_skips_blank_drafts kind=ui owner=dialogs-pane
#[test]
fn taking_pending_drafts_preserves_order_expands_pastes_and_skips_blank_drafts() {
    kit::na(
        Owner::DialogsPane,
        "asserts the drafts that take_pending_drafts returns, the unanswered count and the expanded flag, none drawn",
    );
}

// ORACLE bottom_pane/async_questions/state_tests.rs:54 taking_pending_drafts_bounds_question_titles_without_truncating_answers kind=ui owner=dialogs-pane
#[test]
fn taking_pending_drafts_bounds_question_titles_without_truncating_answers() {
    kit::na(
        Owner::DialogsPane,
        "asserts the draft strings that take_pending_drafts returns (the title bound), not drawn",
    );
}

// ORACLE bottom_pane/async_questions/state_tests.rs:74 replay_preserves_drafts_and_does_not_reopen_handled_questions kind=ui owner=dialogs-pane
#[test]
fn replay_preserves_drafts_and_does_not_reopen_handled_questions() {
    kit::na(
        Owner::DialogsPane,
        "asserts capture, restore and unanswered_count (internal state), not drawn",
    );
}

// ORACLE bottom_pane/async_questions/state_tests.rs:95 arrival_preserves_vim_undo_and_navigation_flushes_buffered_input kind=ui owner=dialogs-pane
#[test]
fn arrival_preserves_vim_undo_and_navigation_flushes_buffered_input() {
    kit::pending(
        Owner::DialogsPane,
        "no vim mode in the question prompt, and the arrival of a question while a draft is typed needs a live feed (Screen::feed does not reach an entry screen); the paste burst is internal; the discard of typed-ahead keys when a request arrives is G8 (rtwk has no input discard)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .entry("approvals", "user_input_two_questions")
        .open();
    s.keys("i");
    s.paste("draft");
    s.keys("esc");
    s.keys("u");
    s.assert_absent("draft");
    s.keys("ctrl+r");
    s.assert_contains("draft");
    // The oracle's buffered input and the final `Second` question with an empty composer are checked once the arrival can be fed.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:133 long_prompt_keeps_the_active_input_visible kind=snap owner=dialogs-pane snap=long_prompt_active_input
#[test]
fn long_prompt_keeps_the_active_input_visible() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with a question whose title is the 30-times repeated A lengthy prompt. and a pasted answer visible answer (the slice registers it as approvals/user_input_long_prompt)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 8)
        .entry("approvals", "user_input_long_prompt")
        .open();
    s.paste("visible answer");
    s.assert_contains("visible answer");
    s.assert_snap("long_prompt_active_input");
}

// ORACLE bottom_pane/async_questions/state_tests.rs:144 wrapped_question_url_keeps_its_complete_destination kind=snap owner=dialogs-pane snap=question_wrapped_url
#[test]
fn wrapped_question_url_keeps_its_complete_destination() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with a second question titled Review this change (https://github.com/openai/codex/pull/12345?diff=split)? (the slice registers it as approvals/user_input_wrapped_url)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 12)
        .entry("approvals", "user_input_wrapped_url")
        .open();
    s.keys("ctrl+]");
    s.assert_snap("question_wrapped_url");
    s.assert_link(
        "https://github.com/openai/codex/",
        "https://github.com/openai/codex/pull/12345?diff=split",
    );
    // The oracle's 4-row height keeps a partial link: its check on the clipped render is not written yet.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:184 markdown_question_preserves_formatting_and_link_destinations kind=snap owner=dialogs-pane snap=question_multiline
#[test]
fn markdown_question_preserves_formatting_and_link_destinations() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with the markdown question Run **this command** with a sh fence and a [diagnostics] link (the slice registers it as approvals/user_input_markdown)",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(60, 15)
        .entry("approvals", "user_input_markdown")
        .open();
    s.assert_snap("question_multiline");
    s.assert_link("diagnostics", "https://example.com/diagnostics?view=full");
    // The oracle's bold cells (this command) and the CRLF variant are not written yet.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:232 selected_other_renders_as_a_dim_placeholder kind=snap owner=dialogs-pane snap=selected_other_placeholder
#[test]
fn selected_other_renders_as_a_dim_placeholder() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with two questions (First, Second with the option Named) and a long second title; the other placeholder is drawn once a draft is typed then cleared (the slice registers it as approvals/user_input_other)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(50, 10)
        .entry("approvals", "user_input_other")
        .open();
    s.keys("down");
    s.keys("2");
    s.assert_snap("selected_other_placeholder");
    // The oracle's DIM cells of the placeholder (5 columns) and the vim section are not written yet.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:294 question_choice_remaps_paging_and_inline_feedback kind=snap owner=dialogs-pane snap=question_remapped_choice_footer
#[test]
fn question_choice_remaps_paging_and_inline_feedback() {
    kit::pending(
        Owner::DialogsPane,
        "no keymap seed (list.move_down x, list.accept f9, list.jump_top 7) and no entry with twelve long options (Option 1 .. Option 12 is long enough to wrap) (the slice registers it as approvals/user_input_twelve)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(50, 12)
        .entry("approvals", "user_input_twelve")
        .keymap(&[
            ("list.move_down", "x"),
            ("list.accept", "f9"),
            ("list.jump_top", "7"),
        ])
        .open();
    s.keys("down");
    s.keys("x");
    s.keys("pagedown");
    s.keys("7");
    s.assert_snap("question_remapped_choice_footer");
}

// ORACLE bottom_pane/async_questions/state_tests.rs:329 question_inline_history_search kind=snap owner=dialogs-pane snap=question_inline_history_search
#[test]
fn question_inline_history_search() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with twelve options Option 1 .. Option 12, a replayed history entry earlier answer and a pasted original answer (the slice registers it as approvals/user_input_history)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(50, 12)
        .entry("approvals", "user_input_history")
        .open();
    s.keys("ctrl+r");
    s.keys("e");
    s.assert_snap("question_inline_history_search");
    // The oracle's REVERSED cell of the match is not written yet (no cell position in the snapshot).
}

// ORACLE bottom_pane/async_questions/state_tests.rs:356 question_other_remains_visible_at_nine_columns_and_distinguishes_named_other kind=snap owner=dialogs-pane snap=question_named_other snap=question_narrow_other
#[test]
fn question_other_remains_visible_at_nine_columns_and_distinguishes_named_other() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with a question whose only option is Other and a pasted other draft (the slice registers it as approvals/user_input_named_other)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(50, 12)
        .entry("approvals", "user_input_named_other")
        .open();
    s.assert_snap("question_named_other");
    s.resize(9, 8);
    s.assert_snap("question_narrow_other");
    // The oracle's cursor and the x draft in the narrow render are not written yet.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:375 question_options_reserve_wrapped_height_at_the_width_boundary kind=snap owner=dialogs-pane snap=question_options_width_boundary
#[test]
fn question_options_reserve_wrapped_height_at_the_width_boundary() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with options abcdefgh *ijklmno* and pqrstuvw xyzabcd at width 24; the joined snapshot labels Full height: and Clipped: are not frame headers the kit reads yet (oracle_snap header(), kit call missing)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(24, 12)
        .entry("approvals", "user_input_wrap_boundary")
        .open();
    s.assert_snap_section("question_options_width_boundary", "Full height:");
    s.resize(24, 4);
    s.assert_snap_section("question_options_width_boundary", "Clipped:");
}

// ORACLE bottom_pane/async_questions/state_tests.rs:410 question_other_excludes_clipped_options_at_the_width_boundary kind=snap owner=dialogs-pane snap=question_other_clipped_width_boundary
#[test]
fn question_other_excludes_clipped_options_at_the_width_boundary() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with options abcdefgh ijklmno and pqrstuvw xyzabcd with Other selected in a 20 by 2 region (the slice registers it as approvals/user_input_other_clipped)",
    );
    Screen::build(Owner::DialogsPane)
        .size(20, 2)
        .entry("approvals", "user_input_other_clipped")
        .open()
        .assert_snap("question_other_clipped_width_boundary");
}

// ORACLE bottom_pane/async_questions/state_tests.rs:431 question_wrapped_options_and_other_share_the_text_indent kind=snap owner=dialogs-pane snap=question_clipped_choice_rejected snap=question_clipped_prompt snap=question_wrapped_named_option snap=question_wrapped_selected_option
#[test]
fn question_wrapped_options_and_other_share_the_text_indent() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with the wrapped suggested answer A suggested answer that is long enough to wrap across multiple rows, plus a second wrapped option (the slice registers it as approvals/user_input_wrapped_options)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(36, 16)
        .entry("approvals", "user_input_wrapped_options")
        .open();
    s.assert_snap("question_wrapped_named_option");
    s.resize(36, 10);
    s.assert_snap("question_wrapped_selected_option");
    s.resize(50, 8);
    s.assert_snap("question_clipped_choice_rejected");
    s.resize(50, 3);
    s.assert_snap("question_clipped_prompt");
    // The oracle's option selection, Enter and bounded-list steps between the renders are not written yet.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:509 question_wrapped_other_shares_the_text_indent kind=snap owner=dialogs-pane snap=question_capped_other snap=question_wrapped_other
#[test]
fn question_wrapped_other_shares_the_text_indent() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with the wrapped suggested answer and a pasted wrapping Other answer, then a long draft of a thousand lines (the slice registers it as approvals/user_input_wrapped_other)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(36, 16)
        .entry("approvals", "user_input_wrapped_other")
        .open();
    s.assert_snap("question_wrapped_other");
    s.resize(50, 18);
    s.assert_snap("question_capped_other");
    s.assert_cursor(7, 12);
    // The oracle's other_label length bounds (128 and 512 bytes) are internal, not drawn.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:552 existing_cross_context_keymaps_load_without_misleading_submit_hints kind=snap owner=dialogs-pane
#[test]
fn existing_cross_context_keymaps_load_without_misleading_submit_hints() {
    kit::pending(
        Owner::DialogsPane,
        "no keymap seed for chat.interrupt_turn, chat.edit_queued_message and list.accept bound to f12, and no second question to navigate to",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .entry("approvals", "user_input_two_questions")
        .keymap(&[
            ("chat.interrupt_turn", "f12"),
            ("chat.edit_queued_message", "f12"),
            ("list.accept", "f12"),
        ])
        .open();
    s.keys("ctrl+]");
    s.assert_has_row("⌃] skip   ⇧→ prev question");
    s.keys("f12");
    // The oracle's submission (no reply sent on F12) is not drawn.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:567 question_navigation_resets_history_recall kind=ui owner=dialogs-pane
#[test]
fn question_navigation_resets_history_recall() {
    kit::pending(
        Owner::DialogsPane,
        "no seed for the composer history (older, newer replayed user messages) on a question prompt (the slice registers it as approvals/user_input_history_recall)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("approvals", "user_input_history_recall")
        .open();
    s.keys("up");
    s.assert_contains("newer");
    s.keys("ctrl+]");
    s.keys("shift+tab");
    s.keys("up");
    s.assert_contains("newer");
}

// ORACLE bottom_pane/async_questions/state_tests.rs:583 named_choices_share_selection_while_other_keeps_its_inline_cursor kind=ui owner=dialogs-pane
#[test]
fn named_choices_share_selection_while_other_keeps_its_inline_cursor() {
    kit::pending(
        Owner::DialogsPane,
        "no entry with the named option *Named* [guide](https://example.com) and a linked g at widths 40 and 80 (the slice registers it as approvals/user_input_linked_option)",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(40, 16)
        .entry("approvals", "user_input_linked_option")
        .open();
    s.keys("down");
    s.assert_link("guide", "https://example.com");
    s.resize(80, 16);
    s.assert_link("guide", "https://example.com");
    // The oracle's selection background (selection_style) and the italic N cell are compared by colour from the oracle's code; not written yet.
}

// ORACLE bottom_pane/async_questions/state_tests.rs:645 oversized_question_ids_keep_questions_answerable_without_echoing_the_id kind=ui owner=dialogs-pane
#[test]
fn oversized_question_ids_keep_questions_answerable_without_echoing_the_id() {
    kit::na(
        Owner::DialogsPane,
        "asserts the submitted reply (sent to the model), not drawn",
    );
}

// ---- oracle: bottom_pane/mcp_server_elicitation.rs (10 screen-level tests here; 13 state tests left to dialogs-pane)
// ORACLE bottom_pane/mcp_server_elicitation.rs:2206 horizontal_list_keys_move_between_select_fields kind=ui owner=dialogs-pane
#[test]
fn horizontal_list_keys_move_between_select_fields() {
    kit::pending(
        Owner::DialogsPane,
        "no keymap seed (`.keymap` is missing until the port reads key bindings: list.accept cleared); the entry approvals/mcp_two_booleans exists and the default h/l and Left/Right keys are pinned in tests/sheets/approvals.rs",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("approvals", "mcp_two_booleans")
        .keymap(&[("list.accept", "")])
        .open();
    s.keys("ctrl+l");
    s.assert_has_row("Second");
    s.keys("ctrl+h");
    s.assert_has_row("First");
    s.assert_absent("submit");
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2250 switching_fields_clears_length_validation_flash kind=ui owner=dialogs-pane
#[test]
fn switching_fields_clears_length_validation_flash() {
    let mut s = Screen::build(Owner::DialogsPane)
        .entry("approvals", "mcp_two_strings")
        .open();
    // The oracle pastes one character over the input limit: Enter flashes the length error, ctrl+n goes on.
    s.paste_over_the_input_limit();
    s.keys("enter");
    s.keys("ctrl+n");
    // The oracle's frame delay and the flash timing are not drawn; the composer is empty after the switch, so no x run is on the screen.
    s.assert_absent("xxxxxxxxxx");
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2278 text_fields_inherit_composer_chord_context_and_submit_binding kind=snap owner=dialogs-pane snap=mcp_server_elicitation_text_configured_key_chords
#[test]
fn text_fields_inherit_composer_chord_context_and_submit_binding() {
    kit::pending(
        Owner::DialogsPane,
        "no keymap seed (`.keymap` is missing until the port reads key bindings: composer.submit = ctrl-x enter); the entry approvals/mcp_text_chords exists and its default Enter submit is pinned in tests/sheets/approvals.rs",
    );
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 10)
        .entry("approvals", "mcp_text_chords")
        .keymap(&[("composer.submit", "ctrl-x enter")])
        .open();
    s.assert_snap("mcp_server_elicitation_text_configured_key_chords");
    s.keys("ctrl+x");
    s.keys("enter");
    // The oracle's keymap-context check (program status) is internal, not drawn.
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2637 form_prompt_preserves_the_complete_wrapped_url_destination kind=snap owner=dialogs-pane snap=mcp_form_wrapped_url
#[test]
fn form_prompt_preserves_the_complete_wrapped_url_destination() {
    kit::pending(
        Owner::DialogsPane,
        "G49 + G72: the entry approvals/mcp_wrapped_url_form exists and every row but the message equals the oracle snapshot; rtwk Text does not break after a slash (the oracle breaks `https://github.com/openai/` / `codex/pull/12345?diff=split before`) and the URL is plain text, so no cell carries its OSC 8 link",
    );
    let s = Screen::build(Owner::DialogsPane)
        .size(40, 20)
        .entry("approvals", "mcp_wrapped_url_form")
        .open();
    s.assert_snap("mcp_form_wrapped_url");
    s.assert_link(
        "https://github.com/openai/codex/",
        "https://github.com/openai/codex/pull/12345?diff=split",
    );
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2674 boolean_form_snapshot kind=snap owner=dialogs-pane snap=mcp_server_elicitation_boolean_form
#[test]
fn boolean_form_snapshot() {
    Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "mcp_boolean_form")
        .open()
        .assert_snap("mcp_server_elicitation_boolean_form");
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2711 approval_form_tool_approval_snapshot kind=snap owner=dialogs-pane snap=mcp_server_elicitation_approval_form_without_schema
#[test]
fn approval_form_tool_approval_snapshot() {
    Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "mcp_elicitation")
        .open()
        .assert_snap("mcp_server_elicitation_approval_form_without_schema");
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2742 message_only_form_snapshot kind=snap owner=dialogs-pane snap=mcp_server_elicitation_message_only_form
#[test]
fn message_only_form_snapshot() {
    Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "mcp_message_only")
        .open()
        .assert_snap("mcp_server_elicitation_message_only_form");
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2765 message_only_form_with_persist_options_snapshot kind=snap owner=dialogs-pane snap=mcp_server_elicitation_message_only_form_with_persist_options
#[test]
fn message_only_form_with_persist_options_snapshot() {
    Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "mcp_message_only_persist")
        .open()
        .assert_snap("mcp_server_elicitation_message_only_form_with_persist_options");
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2793 approval_form_tool_approval_with_persist_options_snapshot kind=snap owner=dialogs-pane snap=mcp_server_elicitation_approval_form_with_session_persist
#[test]
fn approval_form_tool_approval_with_persist_options_snapshot() {
    Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "mcp_tool_persist")
        .open()
        .assert_snap("mcp_server_elicitation_approval_form_with_session_persist");
}

// ORACLE bottom_pane/mcp_server_elicitation.rs:2823 approval_form_tool_approval_with_param_summary_snapshot kind=snap owner=dialogs-pane snap=mcp_server_elicitation_approval_form_with_param_summary
#[test]
fn approval_form_tool_approval_with_param_summary_snapshot() {
    Screen::build(Owner::DialogsPane)
        .size(120, 16)
        .entry("approvals", "mcp_param_summary")
        .open()
        .assert_snap("mcp_server_elicitation_approval_form_with_param_summary");
}
