//! Shard `chat_live_2` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: chatwidget/tests/backend_banners_tests.rs (12 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/backend_banners_tests.rs:29 backend_banner_presentation_and_cta_do_not_imply_recovery kind=snap owner=chat-live
#[test]
fn backend_banner_presentation_and_cta_do_not_imply_recovery() {
    kit::pending(
        Owner::ChatLive,
        "no seed for an account rate-limit response that carries a backend banner (rate_limit_upsell: title, presentation none/inline/dismissible, a notify_owner cta); the chat screen has no backend banner view",
    );
    // The oracle, per presentation: the banner shows its title; `esc to dismiss` only when
    // dismissible; key 1 queues the notify-owner email and the popup does not change.
    for dismissible in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
        s.assert_contains("Selected model usage exhausted");
        if dismissible {
            s.assert_contains("esc to dismiss");
        } else {
            s.assert_absent("esc to dismiss");
        }
        s.keys("1");
        s.assert_contains("Selected model usage exhausted");
    }
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:105 backend_banner_zero_actions_preserve_guidance_and_composer_input kind=snap owner=chat-live
#[test]
fn backend_banner_zero_actions_preserve_guidance_and_composer_input() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a backend banner with zero or an unsupported cta (information only, dismissible); the banner view and its snapshots are not built",
    );
    for dismissible in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
        s.assert_contains("Selected model usage exhausted");
        s.assert_contains("switch to another model");
        s.assert_absent("Press a number");
        if dismissible {
            s.assert_contains("esc to dismiss");
            s.assert_snap("backend_banner_information_dismissible");
        } else {
            s.assert_absent("esc to dismiss");
            s.assert_snap("backend_banner_information_only");
        }
        // Key 1 has no action; the next navigation key leaves `1` in the composer.
        s.keys("1 left");
    }
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:142 backend_banner_dismissal_tracks_occurrence_not_copy_or_model kind=ui owner=chat-live
#[test]
fn backend_banner_dismissal_tracks_occurrence_not_copy_or_model() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a dismissible backend banner, nor a model switch (set_model) from the screen; the dismissed occurrence is not kept across the update",
    );
    let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
    s.assert_contains("Selected model usage exhausted");
    s.keys("esc");
    // The oracle then changes the model and the description and checks that the new copy
    // stays hidden until the reset time changes; the seed steps are not expressible yet.
    s.assert_absent("Updated backend copy");
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:165 backend_banner_occurrence_uses_explicit_or_legacy_source_model kind=ui owner=chat-live
#[test]
fn backend_banner_occurrence_uses_explicit_or_legacy_source_model() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a dismissible backend banner with or without blocked_model_slug, nor a model switch from the screen",
    );
    let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
    s.assert_contains("Selected model usage exhausted");
    s.keys("esc");
    // The oracle then moves the banner to test-model-b and sets the model to test-model-b:
    // the banner is shown again only when the explicit source model was not given.
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:189 backend_banner_turn_on_another_model_preserves_hidden_occurrence kind=ui owner=chat-live
#[test]
fn backend_banner_turn_on_another_model_preserves_hidden_occurrence() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a dismissible backend banner, nor a model switch (set_model) from the screen; the turn's start and completion are expressible",
    );
    let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
    s.assert_contains("Selected model usage exhausted");
    s.type_text("hello").keys("enter");
    s.feed(include_str!("fixtures/chat_live_2/turn_done.jsonl"));
    // The oracle returns to test-model-a after the turn: the hidden occurrence shows again.
    s.assert_contains("Selected model usage exhausted");
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:208 backend_banner_restores_only_programmatically_displaced_switch_prompt kind=ui owner=chat-live
#[test]
fn backend_banner_restores_only_programmatically_displaced_switch_prompt() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a 95% rate-limit snapshot that shows the `Approaching rate limits` switch prompt, nor the banner that displaces it",
    );
    let mut s = Screen::build(Owner::ChatLive).size(90, 24).live().open();
    s.assert_contains("Approaching rate limits");
    s.keys("esc");
    // The oracle then shows the banner, removes it, and checks the prompt returns only when
    // the user did not dismiss it (the Esc above).
    s.assert_contains("Selected model usage exhausted");
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:234 owner_notification_completion_cannot_cross_account_change kind=snap owner=chat-live
#[test]
fn owner_notification_completion_cannot_cross_account_change() {
    kit::pending(
        Owner::ChatLive,
        "no add-credits nudge email request on the chat screen (start, account change, finish with Sent): the notification history cell is not built",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // The oracle's history after the current request finishes: one notification, none from the previous account.
    s.assert_snap("owner_notification_after_account_change");
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:262 backend_banner_new_turn_dismisses_only_shown_dismissible_content kind=ui owner=chat-live
#[test]
fn backend_banner_new_turn_dismisses_only_shown_dismissible_content() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a dismissible backend banner (shown before the submit or not); the turn started and completed are expressible",
    );
    let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
    s.assert_contains("Selected model usage exhausted");
    s.type_text("hello").keys("enter");
    s.feed(include_str!("fixtures/chat_live_2/turn_done.jsonl"));
    // The oracle: a dismissible banner shown before the submit is gone after the turn; the others stay.
    s.assert_absent("Selected model usage exhausted");
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:292 backend_banner_invalid_content_and_absence_restore_fallback kind=ui owner=chat-live
#[test]
fn backend_banner_invalid_content_and_absence_restore_fallback() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a backend banner with invalid content or its absence, nor the usage-limit error that shows the workspace owner fallback",
    );
    let s = Screen::build(Owner::ChatLive).size(90, 24).live().open();
    s.assert_contains("Selected model usage exhausted");
    // The oracle removes the banner or breaks its content, then a usage-limit error shows the fallback.
    s.assert_contains("workspace owner");
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:322 backend_banner_fallback_candidates_and_notice_follow_selected_model kind=ui owner=chat-live
#[test]
fn backend_banner_fallback_candidates_and_notice_follow_selected_model() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a model catalog with a hidden model and fallback slugs, a ChatGPT account, nor the fallback switch that the banner offers; the banner is not built",
    );
    let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
    s.assert_contains("Selected model usage exhausted");
    s.keys("esc");
    s.assert_absent("Selected model usage exhausted");
    // The oracle then opens another task on test-model-c: the banner is inherited and
    // follows the selected model (test-model-a hides it, test-model-c shows it).
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:373 backend_banner_sparse_updates_preserve_visible_and_dismissed_occurrences kind=ui owner=chat-live
#[test]
fn backend_banner_sparse_updates_preserve_visible_and_dismissed_occurrences() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a dismissible backend banner nor the rolling rate-limit snapshots (60% and an exhausted credits snapshot) that must not change the popup",
    );
    for dismiss in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(70, 24).live().open();
        s.assert_contains("Selected model usage exhausted");
        if dismiss {
            s.keys("esc");
        }
        let before = s.shot();
        // The oracle feeds two rolling snapshots: the popup stays the same.
        s.assert_same_rows(&before);
    }
}

// ORACLE chatwidget/tests/backend_banners_tests.rs:396 backend_banner_changed_remedy_keeps_fallback_until_applicable_replacement kind=ui owner=chat-live
#[test]
fn backend_banner_changed_remedy_keeps_fallback_until_applicable_replacement() {
    kit::pending(
        Owner::ChatLive,
        "no seed for a credits-depleted rate-limit snapshot, a usage-cap error, nor a banner whose remedy changes (notify owner, then request more usage); the banner is not built",
    );
    for rolling_update in [false, true] {
        let s = Screen::build(Owner::ChatLive).size(90, 24).live().open();
        s.assert_contains("Notify owner");
        if rolling_update {
            s.assert_contains("Request a limit increase from your owner");
        }
        s.assert_contains("Request a limit increase from your owner");
        // The oracle replaces the banner with the spending-cap title and request-more-usage cta.
        s.assert_contains("Workspace spending cap reached");
        s.assert_contains("Request more usage");
        s.assert_absent("Request a limit increase from your owner");
    }
}

// ---- oracle: chatwidget/tests/compaction_tests.rs (4 screen-level tests here; 2 state tests left to chat-live)
// ORACLE chatwidget/tests/compaction_tests.rs:31 compaction_status_survives_follow_up_and_preserves_turn_time kind=snap owner=chat-live snap=compaction_completed snap=compaction_running snap=compaction_running_narrow
#[test]
fn compaction_status_survives_follow_up_and_preserves_turn_time() {
    kit::pending(
        Owner::ChatLive,
        "the status line's compaction header and turn timer are not built (`Compacting context` over the turn's 600s timer, a compaction started 83s ago; the narrow render at 40 cells)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_live_2/compaction_running.jsonl"
        ))
        .open();
    s.assert_snap("compaction_running");
    s.resize(40, 24);
    s.assert_snap("compaction_running_narrow");
    s.resize(80, 24);
    s.type_text("keep going").keys("enter");
    s.assert_contains("Compacting context");
    s.feed(include_str!(
        "fixtures/chat_live_2/compaction_completed.jsonl"
    ));
    s.assert_snap("compaction_completed");
    s.assert_contains("Working");
    s.assert_absent("Compacting context");
}

// ORACLE chatwidget/tests/compaction_tests.rs:106 compaction_status_clears_when_turn_ends_without_item_completion kind=ui owner=chat-live
#[test]
fn compaction_status_clears_when_turn_ends_without_item_completion() {
    kit::pending(
        Owner::ChatLive,
        "the status line's compaction header is not built: it must clear on a turn that ends completed, interrupted or failed without the item completing",
    );
    for script in [
        include_str!("fixtures/chat_live_2/turn_end_completed.jsonl"),
        include_str!("fixtures/chat_live_2/turn_end_interrupted.jsonl"),
        include_str!("fixtures/chat_live_2/turn_end_failed.jsonl"),
    ] {
        let s = Screen::build(Owner::ChatLive)
            .size(80, 24)
            .script(script)
            .open();
        s.assert_absent("Compacting context");
        s.assert_absent("Context compacted");
        s.assert_contains("Working");
    }
}

// ORACLE chatwidget/tests/compaction_tests.rs:138 compaction_history_does_not_start_a_timer_or_finish_live_compaction kind=ui owner=chat-live
#[test]
fn compaction_history_does_not_start_a_timer_or_finish_live_compaction() {
    kit::pending(
        Owner::ChatLive,
        "no replay path in the script (a replayed compaction, ReplayKind): the replayed item must not start a timer or finish the live compaction; the `Context compacted` history cell is not built",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_absent("Compacting context");
    s.feed(include_str!(
        "fixtures/chat_live_2/compaction_running.jsonl"
    ));
    s.assert_contains("Compacting context");
    // The oracle's replayed completion of another item leaves the live compaction running and
    // writes one `• Context compacted` history cell.
    s.assert_contains("Context compacted");
}

// ORACLE chatwidget/tests/compaction_tests.rs:162 compaction_snapshot_restores_elapsed_time_and_clears_on_replayed_completion kind=ui owner=chat-live
#[test]
fn compaction_snapshot_restores_elapsed_time_and_clears_on_replayed_completion() {
    kit::pending(
        Owner::ChatLive,
        "no replay path in the script (a thread snapshot) and no elapsed time for a compaction started 12s before; the header and the `Context compacted` cell are not built",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.feed(include_str!(
        "fixtures/chat_live_2/compaction_running.jsonl"
    ));
    s.assert_contains("Compacting context");
    s.feed(include_str!(
        "fixtures/chat_live_2/compaction_completed.jsonl"
    ));
    s.assert_absent("Compacting context");
    s.assert_contains("Working");
    s.assert_contains("Context compacted");
}

// ---- oracle: chatwidget/tests/completion_styling_tests.rs (2 screen-level tests here; 4 state tests left to chat-live)
// ORACLE chatwidget/tests/completion_styling_tests.rs:72 completion_follows_plain_and_streamed_tool_answers kind=snap owner=chat-live snap=completion_after_plain_answer snap=completion_after_streamed_tool_answer
#[test]
fn completion_follows_plain_and_streamed_tool_answers() {
    kit::pending(
        Owner::ChatLive,
        "no script step for the completed turn's final separator (`Worked for` duration, completion time) nor for a streamed command execution before the answer; the plain answer is expressible",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/chat_live_2/completion_plain.jsonl"))
        .open();
    s.assert_snap("completion_after_plain_answer");
    // The oracle's streamed branch adds an echo command with its output and the answer
    // `The change is ready.`, then completes the turn with the same separator.
    s.assert_snap("completion_after_streamed_tool_answer");
}

// ORACLE chatwidget/tests/completion_styling_tests.rs:255 completion_snapshot_normalization_preserves_clock_only_message_lines kind=snap owner=chat-live snap=completion_like_transcript_text
#[test]
fn completion_snapshot_normalization_preserves_clock_only_message_lines() {
    kit::na(
        Owner::ChatLive,
        "asserts the snapshot tool's timestamp normalization over a cell the test builds itself (a PlainHistoryCell and a FinalMessageSeparator), not a turn the screen draws: a unit test of the helper",
    );
}

// ---- oracle: chatwidget/tests/config_errors_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/config_errors_tests.rs:4 chained_config_error_wraps_in_history_snapshot kind=snap owner=chat-live snap=chained_config_error_wraps_in_history_snapshot
#[test]
fn chained_config_error_wraps_in_history_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no script step for a UI error message (a failed save of the default model, `config/batchWrite failed`) in the history; the error cell is not built",
    );
    let s = Screen::build(Owner::ChatLive).size(56, 8).live().open();
    s.assert_snap("chained_config_error_wraps_in_history_snapshot");
}

// ---- oracle: chatwidget/tests/goal_validation.rs (2 screen-level tests here; 4 state tests left to chat-live)
// ORACLE chatwidget/tests/goal_validation.rs:117 goal_slash_command_preserves_large_pasted_objective kind=ui owner=chat-live
#[test]
fn goal_slash_command_preserves_large_pasted_objective() {
    kit::pending(
        Owner::ChatLive,
        "no `/goal` command on the chat screen (feature Goals); a paste over the 4000-char objective limit shows its `[Pasted Content` placeholder in the composer, and submit opens the goal draft, not built",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/goal ");
    // The oracle pastes MAX_THREAD_GOAL_OBJECTIVE_CHARS + 1 (4000 + 1) `x` cells.
    s.paste(&"x".repeat(4_001));
    s.assert_contains("[Pasted Content");
    s.keys("enter");
    // The oracle then holds the draft's pending paste to the pasted text and sends no submit.
}

// ORACLE chatwidget/tests/goal_validation.rs:159 goal_slash_command_emits_only_inserted_paste_text_element kind=ui owner=chat-live
#[test]
fn goal_slash_command_emits_only_inserted_paste_text_element() {
    kit::pending(
        Owner::ChatLive,
        "no `/goal` command on the chat screen (feature Goals) and no goal draft; the literal placeholder typed in the objective and the inserted paste placeholder are composer text, not built",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.type_text("/goal keep literal [Pasted Content 1001 chars] and ");
    s.paste(&"x".repeat(1_001));
    s.assert_contains("keep literal [Pasted Content 1001 chars] and [Pasted Content 1001 chars]");
    s.keys("enter");
    // The oracle then checks the draft holds one pending paste (the inserted one) and no submit.
}
// ---- oracle: chatwidget/tests/history_projection.rs (6 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/history_projection.rs:12 older_tool_projection_matches_initial_replay kind=snap owner=chat-live
#[test]
fn older_tool_projection_matches_initial_replay() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_projection.rs:180 snapshot_formatter_reasoning_matches_compact_and_detailed_replay kind=snap owner=chat-live
#[test]
fn snapshot_formatter_reasoning_matches_compact_and_detailed_replay() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_projection.rs:250 raw_reasoning_keeps_its_own_heading kind=snap owner=chat-live
#[test]
fn raw_reasoning_keeps_its_own_heading() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_projection.rs:269 snapshot_formatter_completed_patch_needs_no_started_notification kind=snap owner=chat-live
#[test]
fn snapshot_formatter_completed_patch_needs_no_started_notification() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_projection.rs:307 snapshot_formatter_live_and_buffered_patch_notifications_render_once kind=snap owner=chat-live
#[test]
fn snapshot_formatter_live_and_buffered_patch_notifications_render_once() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_projection.rs:344 snapshot_formatter_empty_success_and_failed_patch_keep_their_outcomes kind=snap owner=chat-live snap=empty
#[test]
fn snapshot_formatter_empty_success_and_failed_patch_keep_their_outcomes() {
    kit::todo();
}

// ---- oracle: chatwidget/tests/history_replay.rs (24 screen-level tests here; 11 state tests left to chat-live)
// ORACLE chatwidget/tests/history_replay.rs:18 resumed_initial_messages_render_history kind=snap owner=chat-live
#[test]
fn resumed_initial_messages_render_history() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:130 replayed_failed_turns_preserve_overload_warnings_between_retries kind=snap owner=chat-live
#[test]
fn replayed_failed_turns_preserve_overload_warnings_between_retries() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:181 restored_conversation_ultra_remains_selected_after_switching_to_plan kind=ui owner=chat-live
#[test]
fn restored_conversation_ultra_remains_selected_after_switching_to_plan() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:219 replayed_user_messages_seed_composer_history kind=ui owner=chat-live
#[test]
fn replayed_user_messages_seed_composer_history() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:309 replayed_review_prompt_does_not_seed_composer_history kind=ui owner=chat-live
#[test]
fn replayed_review_prompt_does_not_seed_composer_history() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:333 replayed_delegated_tool_output_is_attributed_without_seeding_composer_history kind=snap owner=chat-live snap=replayed_delegated_tool_output
#[test]
fn replayed_delegated_tool_output_is_attributed_without_seeding_composer_history() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:410 replayed_nested_review_prompts_do_not_render_or_seed_composer_history kind=snap owner=chat-live
#[test]
fn replayed_nested_review_prompts_do_not_render_or_seed_composer_history() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:853 replayed_user_message_with_only_remote_images_renders_history_cell kind=ui owner=chat-live
#[test]
fn replayed_user_message_with_only_remote_images_renders_history_cell() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:914 replayed_user_message_with_only_local_images_renders_history_cell kind=ui owner=chat-live
#[test]
fn replayed_user_message_with_only_local_images_renders_history_cell() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:973 forked_thread_history_line_includes_name_and_id_snapshot kind=snap owner=chat-live snap=forked_thread_history_line
#[test]
fn forked_thread_history_line_includes_name_and_id_snapshot() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1003 forked_thread_history_line_without_name_shows_id_once_snapshot kind=snap owner=chat-live snap=forked_thread_history_line_without_name
#[test]
fn forked_thread_history_line_without_name_shows_id_once_snapshot() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1091 prompt_edit_thread_history_line_snapshot kind=snap owner=chat-live snap=prompt_edit_thread_history_line
#[test]
fn prompt_edit_thread_history_line_snapshot() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1113 app_server_forked_thread_history_line_uses_app_server_title_snapshot kind=snap owner=chat-live snap=app_server_forked_thread_history_line
#[test]
fn app_server_forked_thread_history_line_uses_app_server_title_snapshot() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1153 app_server_forked_thread_history_line_without_app_server_name_ignores_local_snapshot kind=snap owner=chat-live snap=app_server_forked_thread_history_line_without_app_server_name
#[test]
fn app_server_forked_thread_history_line_without_app_server_name_ignores_local_snapshot() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1195 thread_snapshot_replay_preserves_agent_message_during_review_mode kind=ui owner=chat-live
#[test]
fn thread_snapshot_replay_preserves_agent_message_during_review_mode() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1276 replayed_reasoning_item_preserves_summary_parts_and_hides_raw_reasoning_when_disabled kind=ui owner=chat-live
#[test]
fn replayed_reasoning_item_preserves_summary_parts_and_hides_raw_reasoning_when_disabled() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1328 replayed_reasoning_item_shows_raw_reasoning_when_enabled kind=ui owner=chat-live
#[test]
fn replayed_reasoning_item_shows_raw_reasoning_when_enabled() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1376 replayed_mcp_tool_call_stays_active_until_completion kind=ui owner=chat-live
#[test]
fn replayed_mcp_tool_call_stays_active_until_completion() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1419 failed_repl_mcp_tool_call_preserves_status_and_result kind=snap owner=chat-live
#[test]
fn failed_repl_mcp_tool_call_preserves_status_and_result() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1482 deferred_mcp_lifecycle_events_keep_fifo_after_stream_finishes kind=ui owner=chat-live
#[test]
fn deferred_mcp_lifecycle_events_keep_fifo_after_stream_finishes() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1544 live_reasoning_summary_is_not_rendered_twice_when_item_completes kind=ui owner=chat-live
#[test]
fn live_reasoning_summary_is_not_rendered_twice_when_item_completes() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1605 live_reasoning_summary_drops_empty_parts_without_losing_content kind=ui owner=chat-live
#[test]
fn live_reasoning_summary_drops_empty_parts_without_losing_content() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1685 thread_snapshot_replayed_turn_started_marks_task_running kind=snap owner=chat-live
#[test]
fn thread_snapshot_replayed_turn_started_marks_task_running() {
    kit::todo();
}

// ORACLE chatwidget/tests/history_replay.rs:1723 replayed_in_progress_turn_marks_task_running kind=snap owner=chat-live snap=replayed_in_progress_turn
#[test]
fn replayed_in_progress_turn_marks_task_running() {
    kit::todo();
}

// ---- oracle: chatwidget/tests/list_spacing_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/list_spacing_tests.rs:9 list_spacing_streams_compact_then_reflows_on_completion_or_interruption kind=snap owner=chat-live
#[test]
fn list_spacing_streams_compact_then_reflows_on_completion_or_interruption() {
    kit::pending(
        Owner::ChatLive,
        "no seed for the transcript mode (fullscreen transcript, alternate screen) or the Plan collaboration mode, and no streamed numbered list; the joined snapshot (12 stages, labelled `owned=.., plan=..`) has no section form in the kit (kit gap)",
    );
    // The oracle streams a four-item numbered list at 26 or 28 cells, interrupted or completed,
    // in owned and retained transcript modes; the snapshot holds every stage.
    let s = Screen::build(Owner::ChatLive).size(28, 24).live().open();
    s.assert_contains("1. One");
}

// ---- oracle: chatwidget/tests/luna_reserve_usage_tests.rs (4 screen-level tests here; 0 state tests left to chat-live)
// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:30 luna_reserve_selector_supports_arrows_enter_shortcuts_and_escape_without_losing_draft kind=snap owner=chat-live snap=luna_reserve_selector_second_choice
#[test]
fn luna_reserve_selector_supports_arrows_enter_shortcuts_and_escape_without_losing_draft() {
    kit::pending(
        Owner::ChatLive,
        "no Luna Reserve banner (dismissible, two ctas, `esc to continue working`) with its selector and key routing; the selector owns focus over the composer draft",
    );
    let mut s = Screen::build(Owner::ChatLive).size(90, 24).live().open();
    s.type_text("saved draft");
    s.assert_contains("esc to continue working");
    s.assert_absent("saved draft");
    s.keys("down");
    s.assert_snap("luna_reserve_selector_second_choice");
    // The oracle repeats this for the other key paths (down enter, down up enter, 1, 2, down esc),
    // each opening one URL (or none on esc), and then keeps the draft `saved draft`.
}

// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:109 luna_reserve_status_tracks_the_active_model kind=snap owner=chat-live snap=luna_reserve_usage_wide snap=luna_reserve_usage_narrow
#[test]
fn luna_reserve_status_tracks_the_active_model() {
    kit::pending(
        Owner::ChatLive,
        "no Luna Reserve status on the chat screen: `Luna Reserve default` shows only while the active model is gpt-reserve, and set_model is not a kit step",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_absent("Luna Reserve");
    s.assert_snap("luna_reserve_usage_wide");
    s.resize(34, 24);
    s.assert_snap("luna_reserve_usage_narrow");
}

// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:141 luna_reserve_usage_survives_banner_dismissal_and_typing_during_a_turn kind=snap owner=chat-live snap=luna_reserve_usage_with_upsell snap=luna_reserve_usage_running
#[test]
fn luna_reserve_usage_survives_banner_dismissal_and_typing_during_a_turn() {
    kit::pending(
        Owner::ChatLive,
        "no Luna Reserve upsell (ordinary usage not allowed, `You’re now using Luna`) on the chat screen, nor a running task seed; the status rows of the reserve are not built",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_snap("luna_reserve_usage_with_upsell");
    s.keys("esc");
    s.assert_absent("You’re now using Luna");
    // The oracle sets a task running here (no kit step yet), then types x and presses left.
    s.type_text("x");
    s.keys("left");
    s.assert_snap("luna_reserve_usage_running");
}

// ORACLE chatwidget/tests/luna_reserve_usage_tests.rs:217 luna_reserve_prompt_preserves_the_composer_palette_on_exit kind=ui owner=chat-live
#[test]
fn luna_reserve_prompt_preserves_the_composer_palette_on_exit() {
    kit::pending(
        Owner::ChatLive,
        "no Luna Reserve status on the chat screen and no set_model kit step; the composer row's background must return to the ordinary user-message style when the model leaves gpt-reserve, under two default palettes (no kit call switches the default colours)",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_absent("Luna Reserve");
}
