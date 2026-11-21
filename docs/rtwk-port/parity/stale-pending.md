# stale-pending: the 255 chat oracle tests that passed with their pending marker probed

Probe: `KIT_PROBE_PENDING=1` at claude/tui-rtwk f51e15937 (list: ~/.cache/codexcli/probe-stale-f51e15937.txt, the 3 signin tests are task providers' and left out). Each test was read against its `// ORACLE` test and the port source.

Rule applied: REAL = the body asserts a positive behaviour of the oracle test that cannot pass without the port feature (a snapshot match, rows or styles only the feature draws, a state after keys) and passes without the probe; the marker was removed. VACUOUS = passes because the feature is absent (assert_absent only, untouched placeholder row, typed-text echo), the body is partial in the claim the oracle test is named for, or the check holds trivially. Markers stayed on those. Bodies were not edited.

A recurring VACUOUS cause in the markdown streaming shards: `assert_matches_whole` / `assert_streams_like_whole` compare a chunked feed with the same text fed whole; the port draws the accumulated text through one MarkdownView, so the equality holds with no streaming controller at all. Those tests are REAL only where they also assert a rendered positive (table rule glyph, list rows, resize relayout).

Only change outside the marker lines: markdown_streaming_5.rs lost the now-unused `STREAM` const and its doc line (dead_code warning).

## Totals

| shard | REAL | VACUOUS |
|---|---|---|
| approvals_2 | 1 | 4 |
| cells_1 | 0 | 5 |
| cells_5 | 0 | 7 |
| chat_approvals_1 | 2 | 2 |
| chat_approvals_2 | 0 | 1 |
| chat_commands_1 | 0 | 8 |
| chat_commands_2 | 0 | 4 |
| chat_commands_3 | 1 | 13 |
| chat_live_1 | 2 | 1 |
| chat_live_2 | 0 | 1 |
| chat_live_3 | 0 | 4 |
| chat_live_4 | 1 | 4 |
| chat_live_5 | 0 | 10 |
| chat_popups_1 | 0 | 1 |
| chat_popups_2 | 1 | 2 |
| chat_popups_4 | 0 | 6 |
| chat_status_1 | 0 | 9 |
| chat_status_2 | 0 | 6 |
| composer_1 | 0 | 3 |
| composer_2 | 1 | 4 |
| composer_3 | 2 | 9 |
| composer_4 | 0 | 15 |
| composer_popups | 0 | 4 |
| markdown_streaming_1 | 7 | 9 |
| markdown_streaming_2 | 15 | 7 |
| markdown_streaming_3 | 5 | 3 |
| markdown_streaming_4 | 2 | 4 |
| markdown_streaming_5 | 9 | 23 |
| pane_footer_1 | 0 | 8 |
| pane_footer_2 | 0 | 6 |
| textarea | 0 | 3 |
| transcript_view_1 | 0 | 11 |
| transcript_view_2 | 0 | 7 |
| transcript_view_3 | 0 | 1 |
| transcript_view_5 | 0 | 1 |
| **all** | **49** | **206** |

Per commit family (one commit each):

- approvals: 1 REAL, 4 VACUOUS
- cells: 0 REAL, 12 VACUOUS
- chat: 7 REAL, 72 VACUOUS
- composer: 3 REAL, 35 VACUOUS
- markdown_streaming: 38 REAL, 46 VACUOUS
- pane_footer: 0 REAL, 14 VACUOUS
- textarea: 0 REAL, 3 VACUOUS
- transcript_view: 0 REAL, 20 VACUOUS

## Arrived red (not mine)

composer_2: 10 red before and after (handle_paste_small_inserts_text, mention_popup_type_prefixes_snapshot, kill_buffer_persists_after_slash_command_dispatch, slash_init_dispatches_command_and_does_not_submit_literal_text, slash_key_completes_selected_slash_command_as_text, slash_mention_dispatches_command_and_inserts_at, slash_popup_model_clipping_snapshot, slash_tab_completion_moves_cursor_to_end, slash_tab_completion_wins_over_queueing_while_task_running, tab_submits_when_no_task_running).

## Notes for the owners

- Bodies worth finishing so the marker can go: composer_popups::category_tags (narrow case), markdown_streaming_3::tight_item_with_soft_break (indent), markdown_streaming_2 paragraph_multiple / blockquote blank-row tests (use rows with blanks, assert_row), markdown_streaming_4::table_inside_blockquote (every row starts `> `).
- Stale marker texts on REAL tests said features did not exist (exec approval entry, markdown renderer, error cell); they are gone with the markers.

## Per test


### approvals_2

- `approvals_2::auto_resolution_hides_timer_during_grace_period` VACUOUS — assert_absent only; no auto-resolution timer exists at all
- `approvals_2::freeform_enter_with_empty_text_is_unanswered` VACUOUS — the form is static (present.rs draws all questions unanswered); Enter does nothing, so the unchanged header passes
- `approvals_2::options_scroll_while_editing_notes` VACUOUS — no notes editing; tab and typing are ignored and Down moves the options, which is all the body checks
- `approvals_2::request_user_input_wrapped_options_snapshot` REAL — assert_snap (110x9) against the oracle snapshot passes: wrapped option rows now match
- `approvals_2::tab_in_notes_clears_notes_and_hides_ui` VACUOUS — assert_absent of text that was never drawn, and the static hint line

### cells_1

- `cells_1::code_mode_output_row_budget_applies_after_wrapping_and_to_errors` VACUOUS — partial body: one contains of a word/title; the oracle's budget/snapshot/rows are not asserted and the cell is absent
- `cells_1::mcp_preview_shares_one_limit_across_blocks_and_preserves_transcript` VACUOUS — partial body: one contains of a word/title; the oracle's budget/snapshot/rows are not asserted and the cell is absent
- `cells_1::projected_image_marker_still_requires_a_complete_image` VACUOUS — partial body: one contains of a word/title; the oracle's budget/snapshot/rows are not asserted and the cell is absent
- `cells_1::raw_grouped_history_retains_terminal_input_and_omits_reasoning` VACUOUS — partial body: one contains of a word/title; the oracle's budget/snapshot/rows are not asserted and the cell is absent
- `cells_1::wrap_styled_spans_single_line` VACUOUS — partial body: one contains of a word/title; the oracle's budget/snapshot/rows are not asserted and the cell is absent

### cells_5

- `cells_5::agent_markdown_cell_does_not_split_words_after_inline_markdown` VACUOUS — partial body: in_order of two fragments; the oracle pins row ends and the row start
- `cells_5::agent_markdown_cell_renders_source_at_different_widths` VACUOUS — partial body: the bullet and in_order; the oracle's 32-column-has-more-rows claim is missing
- `cells_5::reasoning_summary_block_hides_empty_html_comment_parts` VACUOUS — assert_absent only, no positive control (no reasoning cell)
- `cells_5::reasoning_summary_block_respects_config_overrides` VACUOUS — assert_absent only; no config seed, no reasoning cell
- `cells_5::user_history_cell_summarizes_inline_data_urls` VACUOUS — partial body: two contains; the oracle's raw_lines snapshot is not reachable
- `cells_5::user_history_cell_trims_trailing_blank_message_lines_with_text_elements` VACUOUS — partial body: only the text; the exactly-one-trailing-blank-row claim is missing
- `cells_5::user_history_cell_trims_trailing_blank_message_lines` VACUOUS — partial body: only the text; the exactly-one-trailing-blank-row claim is missing

### chat_approvals_1

- `chat_approvals_1::apply_patch_request_omits_diff_summary_from_modal` VACUOUS — assert_absent only on an empty live chat (no feed, no modal)
- `chat_approvals_1::approval_modal_exec_without_reason_snapshot` REAL — assert_snap against the oracle snapshot (100x12) passes; the no-reason fixture exists now
- `chat_approvals_1::approval_modal_exec_snapshot` REAL — assert_snap against the oracle snapshot (100x14) passes; the approvals entry now draws the exec request
- `chat_approvals_1::user_message_during_user_shell_command_is_queued_not_steered` VACUOUS — typed text echo only; no feed, no queue row

### chat_approvals_2

- `chat_approvals_2::ordinary_follow_up_clears_unanswered_questions_after_accepted_input` VACUOUS — placeholder row of an untouched composer (no feed, no questions)

### chat_commands_1

- `chat_commands_1::composer_submission_sends_once_and_requests_latest` VACUOUS — passes only because the feature the marker names is absent: kit gap: the sent UserTurn and the FollowTranscript event are not reachable (no probe of sent ops); the composer after Enter has n...
- `chat_commands_1::enqueueing_history_prompt_multiple_times_is_stable` VACUOUS — passes only because the feature the marker names is absent: no seed for a running turn after a submitted prompt (turn-1 started after "repeat me" is sent); the queue is not seeded
- `chat_commands_1::enter_with_only_remote_images_submits_user_turn` VACUOUS — passes only because the feature the marker names is absent: no seed for remote image attachments on an empty composer (set_remote_image_urls); the oracle has no row literal for the cleared c...
- `chat_commands_1::hidden_shell_paste_queued_before_session_submits_literal_prompt` VACUOUS — passes only because the feature the marker names is absent: no seed for submissions queued until the session is configured (set_queue_submissions_until_session_configured); the live chat sta...
- `chat_commands_1::startup_draft_file_search_waits_for_protected_view_and_enabled_input` VACUOUS — passes only because the feature the marker names is absent: no seed for a startup draft waiting behind an open approvals popup and a disabled composer; the file-search request is an event, n...
- `chat_commands_1::startup_draft_handoff_keeps_vim_insert_mode_for_nonempty_drafts` VACUOUS — passes only because the feature the marker names is absent: no seed for vim mode on (vim_mode_default) with a startup draft restored into it
- `chat_commands_1::submission_with_remote_and_local_images_keeps_local_placeholder_numbering` VACUOUS — passes only because the feature the marker names is absent: no seed for a thread session with a remote image URL attached (set_remote_image_urls) beside a local image; the submitted items ar...
- `chat_commands_1::unbound_queued_message_edit_does_not_fall_back_to_alt_up` VACUOUS — passes only because the feature the marker names is absent: no keymap seed (the port reads no key bindings, and an unbound action has no pair in the kit) and no seed for a queued message on...

### chat_commands_2

- `chat_commands_2::queued_slash_review_with_args_restores_for_edit` VACUOUS — passes only because the feature the marker names is absent: Tab does not queue a slash command while a turn runs in the port: the text stays in the composer, so the probe passes on the text...
- `chat_commands_2::slash_compact_eagerly_queues_follow_up_before_turn_start` VACUOUS — passes only because the feature the marker names is absent: not verified: /compact holds no running task in the live chat, so the probe passes on the typed text alone and the queued state (o...
- `chat_commands_2::slash_copy_picker_defers_queued_input_until_selection_or_cancellation_settles` VACUOUS — passes only because the feature the marker names is absent: no /copy picker: the port answers /copy with 'not available yet', so the picker rows are not drawn; Tab does not queue while a tur...
- `chat_commands_2::slash_copy_picker_supports_arrow_navigation_enter_and_scrolling` VACUOUS — passes only because the feature the marker names is absent: no /copy picker: the port answers /copy with 'not available yet', so the picker rows are not drawn

### chat_commands_3

- `chat_commands_3::configured_welcome_opt_out_disables_blossom_and_clicks` VACUOUS — passes only because the feature the marker names is absent: the welcome opt-out settings (config.toml animations / welcome) are not seedable by the kit; the blossom glyph check is a kit gap
- `chat_commands_3::host_switch_snapshot_preserves_buffered_typing` VACUOUS — typed text echo only
- `chat_commands_3::keymap_capture_can_capture_current_copy_shortcut` VACUOUS — passes only because the feature the marker names is absent: the keymap capture view (opened from the keymap editor) is not built, so the copy key cannot be routed to capture black-box
- `chat_commands_3::new_startup_decoration_tracks_draft_and_session_action` VACUOUS — passes only because the feature the marker names is absent: the startup blossom decoration per draft and session action is not seedable black-box; the kit has no any-braille-glyph assertion...
- `chat_commands_3::ordinary_and_prepared_image_submissions_consume_the_sparkle_before_rendering` VACUOUS — passes only because the feature the marker names is absent: the sparkle (braille glyph) animation of a fresh task on gpt-6-astra with starfield is not seeded; the kit has no any-of-glyphs as...
- `chat_commands_3::raw_slash_command_reports_usage_for_invalid_arg` REAL — `/raw status` draws `Usage: /raw [on|off]` (strings.rs usage_raw); oracle text asserted
- `chat_commands_3::slash_archive_cancellation_keeps_task_running` VACUOUS — passes only because the feature the marker names is absent: a running task seed and the archive confirmation popup are not reachable black-box; the popup text is the oracle's slash_archive_r...
- `chat_commands_3::slash_clear_after_ctrl_c_keeps_stashed_draft_recallable` VACUOUS — marker restored by composer-asserts: with the oracle's literals read through `Screen::composer_text` the body is red; the port keeps the draft in the composer on Ctrl+C (oracle: empty, draft recorded) and has no prompt history (Up recalls nothing). The earlier REAL row was a draft still on the screen matching `assert_contains`.
- `chat_commands_3::slash_keymap_invalid_args_show_usage` VACUOUS — marker restored by composer-asserts: body now asserts the oracle's recall (clear the composer, Up, composer is `/keymap nope`); red because the chat composer has no prompt history (Up after the clear shows the placeholder). The port also keeps the draft in the composer after the usage line.
- `chat_commands_3::slash_mcp_invalid_args_show_usage` VACUOUS — marker restored by composer-asserts: same as the keymap row (`/mcp full`); red because the chat composer has no prompt history.
- `chat_commands_3::slash_rollout_handles_missing_path` VACUOUS — `not available` matches the generic `'/rollout' is not available yet.` for every unbuilt command
- `chat_commands_3::startup_submission_edit_or_disconnect_cancels_without_losing_draft` VACUOUS — typed text echo only
- `chat_commands_3::startup_submission_waits_for_protected_view_and_preserves_paste_provenance` VACUOUS — paste echo only; ops not drawn
- `chat_commands_3::startup_submission_waits_for_session_and_sends_exactly_once` VACUOUS — typed echo; the single UserTurn is not asserted

### chat_live_1

- `chat_live_1::live_app_server_failed_turn_does_not_duplicate_error_history` REAL — error cell drawn once, rows unchanged after turn/failed, Working gone: the oracle's three claims
- `chat_live_1::live_app_server_turn_completion_repairs_dropped_message_deltas` REAL — the completed item's dropped text is drawn after the deltas (needs the item projection); the oracle's consolidation snapshot of the source is internal and not asserted
- `chat_live_1::preview_keeps_queue_categories_separate` VACUOUS — typed words echo; no queue/steer previews

### chat_live_2

- `chat_live_2::luna_reserve_prompt_preserves_the_composer_palette_on_exit` VACUOUS — passes only because the feature the marker names is absent: no Luna Reserve status on the chat screen and no set_model kit step; the composer row's background must return to the ordinary use...

### chat_live_3

- `chat_live_3::inline_review_submits_during_mcp_startup` VACUOUS — passes only because the feature the marker names is absent: no MCP startup state on the chat screen: the starting slow server is not applied, so /review is not held back and the submit path...
- `chat_live_3::pending_mcp_startup_dispatches_queued_slash_commands` VACUOUS — passes only because the feature the marker names is absent: no MCP startup state on the chat screen and no resume picker opened by Tab on /resume; the empty-state animation has no feed step
- `chat_live_3::pending_mcp_startup_does_not_drain_follow_up_before_review_starts` VACUOUS — passes only because the feature the marker names is absent: no MCP startup state and no queued-message rows on the chat screen; the queued /review starts a review that no feed step makes
- `chat_live_3::pending_mcp_startup_does_not_unblock_foreground_shell` VACUOUS — passes only because the feature the marker names is absent: no MCP startup state on the chat screen and no queued-message row for a prompt sent while a shell runs; the /fork cell text is not...

### chat_live_4

- `chat_live_4::completed_replay_preserves_messages_and_draft_across_reconstruction` REAL — user/agent rows in order, draft kept, second build same rows: the oracle's inline snapshot rows and equality
- `chat_live_4::plan_implementation_popup_skips_replayed_turn_complete` VACUOUS — passes only because the feature the marker names is absent: no Implement-this-plan prompt in the port (no Plan mode): the absence holds only until the prompt exists
- `chat_live_4::plan_implementation_popup_skips_when_messages_queued` VACUOUS — passes only because the feature the marker names is absent: no Implement-this-plan prompt in the port (no Plan mode): the queued-message skip holds only vacuously until the prompt exists
- `chat_live_4::plan_implementation_popup_skips_when_steer_follows_proposed_plan` VACUOUS — passes only because the feature the marker names is absent: no Implement-this-plan prompt in the port (no Plan mode): the steer-after-plan skip holds only vacuously until the prompt exists
- `chat_live_4::plan_implementation_popup_skips_without_proposed_plan` VACUOUS — passes only because the feature the marker names is absent: no Implement-this-plan prompt in the port (no Plan mode): the no-proposed-plan skip holds only vacuously until the prompt exists

### chat_live_5

- `chat_live_5::ctrl_c_interrupts_without_arming_quit_when_double_press_disabled` VACUOUS — passes only because the feature the marker names is absent: no running turn to interrupt with Ctrl+C on the chat screen (turn/started is not applied to the bottom pane)
- `chat_live_5::enter_submits_steer_while_review_is_running` VACUOUS — passes only because the feature the marker names is absent: no review mode and no pending steer on the chat screen: Enter while /review runs does not show the steer as pending
- `chat_live_5::failed_pending_steer_submit_does_not_add_pending_preview` VACUOUS — passes only because the feature the marker names is absent: kit has no step that makes the backend refuse a submit (the oracle drops the op channel); the refused steer must not stay as a pen...
- `chat_live_5::item_completed_only_pops_front_pending_steer` VACUOUS — passes only because the feature the marker names is absent: no pending steer preview on the chat screen: two steers during a stream are not kept in order, and a completed item does not pop t...
- `chat_live_5::live_agent_message_renders_during_review_mode` VACUOUS — no review mode in the port (entered_review is ignored): a completed agent message draws anyway
- `chat_live_5::steer_enter_during_final_stream_preserves_follow_up_prompts_in_order` VACUOUS — passes only because the feature the marker names is absent: no pending steer preview on the chat screen: two follow-ups entered during a final-answer stream are not kept in order
- `chat_live_5::steer_enter_queues_while_plan_stream_is_active` VACUOUS — passes only because the feature the marker names is absent: no plan stream on the chat screen (item/plan/delta is not drawn) and no Plan collaboration mode seed: Enter during the plan stream...
- `chat_live_5::steer_enter_uses_pending_steers_while_final_answer_stream_is_active` VACUOUS — passes only because the feature the marker names is absent: no pending steer on the chat screen: Enter during a streamed final answer is not sent as a steer, and its item completion is not d...
- `chat_live_5::steer_enter_uses_pending_steers_while_turn_is_running_without_streaming` VACUOUS — passes only because the feature the marker names is absent: no pending steer on the chat screen: Enter while a turn runs without a stream is not sent as a steer, and its item completion is n...
- `chat_live_5::suppressed_interrupted_turn_notice_skips_history_warning` VACUOUS — passes only because the feature the marker names is absent: no side conversation on the chat screen: the interrupted-turn notice mode 'Suppress' is not a seed, so the notices are drawn

### chat_popups_1

- `chat_popups_1::permission_shortcuts_load_once_and_reuse_the_picker_catalog` VACUOUS — the load-once/cache claim is events; the body checks only that F8 opens and Esc closes something

### chat_popups_2

- `chat_popups_2::approvals_selection_popup_snapshot` REAL — assert_snap_region against the oracle snapshot passes (the marker's framed-Dialog delta is gone)
- `chat_popups_2::permission_discovery_discards_stale_results_and_preserves_covering_modal` VACUOUS — assert_absent only; no discovery
- `chat_popups_2::permissions_selection_hides_auto_review_when_feature_disabled_even_if_auto_review_is_active` VACUOUS — assert_absent only; no auto-review option exists

### chat_popups_4

- `chat_popups_4::experimental_features_toggle_saves_on_exit` VACUOUS — passes only because the feature the marker names is absent: no experimental-features popup in the port: the toggle row, the remapped cancel key (F2) that saves and closes, and the save-on-ex...
- `chat_popups_4::model_picker_refresh_rejects_obsolete_and_unusable_replies` VACUOUS — passes only because the feature the marker names is absent: no model picker in the port: a refresh reply the picker rejects (obsolete or unusable) leaves the open list unchanged; no catalog...
- `chat_popups_4::reasoning_shortcut_clears_armed_quit_shortcut` VACUOUS — passes only because the feature the marker names is absent: no reasoning shortcut (alt+.) in the port: the armed quit hint is cleared by the shortcut; no widget and no seed
- `chat_popups_4::session_model_selection_accepts_final_choices_without_saving` VACUOUS — passes only because the feature the marker names is absent: no session-only model selection in the port: the s key applies the final choice for this session and closes the picker; no widget...
- `chat_popups_4::session_model_selection_notifies_the_original_task_after_each_final_astra_choice` VACUOUS — passes only because the feature the marker names is absent: no session-only model selection in the port: after each final choice the picker closes; the Astra notify event is not drawn; no wi...
- `chat_popups_4::single_reasoning_option_skips_selection` VACUOUS — passes only because the feature the marker names is absent: no reasoning-effort popup in the port: a model with one reasoning option applies it without showing the Select Reasoning Level lis...

### chat_status_1

- `chat_status_1::ambient_pet_draw_uses_terminal_screen_area_not_short_inline_viewport` VACUOUS — passes only because the feature the marker names is absent: kit gap: the pet draw is a kitty image placement (x 71, y 18 on a full 80x24 screen), which the text grid cannot assert; no pet on...
- `chat_status_1::ambient_pet_hides_notification_text_overlay` VACUOUS — passes only because the feature the marker names is absent: no ambient pet notification overlay on the chat screen: the notification labels must stay hidden
- `chat_status_1::ambient_pet_screen_bottom_anchor_uses_terminal_bottom` VACUOUS — passes only because the feature the marker names is absent: kit gap: the pet anchor is a kitty image placement (y 14 composer-anchored, y 18 screen-bottom), which the text grid cannot assert...
- `chat_status_1::ambient_pet_stays_hidden_until_a_pet_is_selected` VACUOUS — passes only because the feature the marker names is absent: kit gap: the ambient pet is a kitty image placement (x 51, y 14, 9x5), which the text grid cannot assert; no pet on the chat scree...
- `chat_status_1::final_answer_completion_restores_status_indicator_for_pending_steer` VACUOUS — passes only because the feature the marker names is absent: no pending-steer queue or status-indicator restore on the chat screen
- `chat_status_1::missing_rate_limit_reached_type_does_not_prompt_or_refresh` VACUOUS — passes only because the feature the marker names is absent: no workspace-owner prompt on the chat screen
- `chat_status_1::rate_limit_recovery_prompt_can_be_dismissed_manually` VACUOUS — passes only because the feature the marker names is absent: no rate-limit recovery prompt on the chat screen
- `chat_status_1::streaming_final_answer_keeps_task_running_state` VACUOUS — passes only because the feature the marker names is absent: no final-answer status hiding: the chat screen shows Working while a turn runs and has no Tab-queue row
- `chat_status_1::workspace_owner_limit_states_do_not_prompt_for_owner_nudge` VACUOUS — passes only because the feature the marker names is absent: no workspace-owner nudge prompt on the chat screen

### chat_status_2

- `chat_status_2::quiet_hook_cleanup_starts_when_the_hook_is_revealed` VACUOUS — passes only because the feature the marker names is absent: hook activity rows and hook history cells are not built in the chat (the skeleton projects no hook items)
- `chat_status_2::status_command_omits_thread_usage_for_unsupported_plans` VACUOUS — passes only because the feature the marker names is absent: the `/status` card is not built in the chat (the status screen exists only as a debug entry, and `/status` is not dispatched); the...
- `chat_status_2::status_command_overlapping_refreshes_update_matching_cells_only` VACUOUS — passes only because the feature the marker names is absent: the `/status` card is not built in the chat (the status screen exists only as a debug entry, and `/status` is not dispatched)
- `chat_status_2::status_command_renders_immediately_and_refreshes_rate_limits_for_chatgpt_auth` VACUOUS — passes only because the feature the marker names is absent: the `/status` card is not built in the chat (the status screen exists only as a debug entry, and `/status` is not dispatched)
- `chat_status_2::status_command_renders_native_and_foreign_instruction_sources` VACUOUS — passes only because the feature the marker names is absent: the `/status` card is not built in the chat (the status screen exists only as a debug entry, and `/status` is not dispatched)
- `chat_status_2::status_line_hostname_renders_current_machine_hostname` VACUOUS — passes only because the feature the marker names is absent: the status line (`local_settings.tui.status_line` items) is not built in the chat, and the kit has no config seed for it

### composer_1

- `composer_1::deleting_prefix_keeps_bang_literal_on_submission` VACUOUS — passes only because the feature the marker names is absent: shell mode is not built: a leading `!` does not switch the composer to shell mode, so the `Shell mode` footer label and the shell...
- `composer_1::esc_hint_stays_hidden_with_draft_content` VACUOUS — passes only because the feature the marker names is absent: the composer footer modes are not built: no key or seed sets the esc backtrack hint, the quit shortcut hint, queue submissions, th...
- `composer_1::question_mark_does_not_toggle_during_paste_burst` VACUOUS — passes only because the feature the marker names is absent: the composer footer modes are not built: no key or seed sets the esc backtrack hint, the quit shortcut hint, queue submissions, th...

### composer_2

- `composer_2::disable_paste_burst_flushes_pending_first_char_and_inserts_immediately` VACUOUS — passes only because the feature the marker names is absent: no paste burst to disable on the chat composer: set_disable_paste_burst has no kit seed, and the port types the first char at once
- `composer_2::non_ascii_burst_buffers_large_multiline_mixed_ascii_and_unicode` VACUOUS — passes only because the feature the marker names is absent: no paste burst on the chat composer: each Enter of the payload submits, so the ten-line payload is not one draft
- `composer_2::reopening_identical_unified_mention_restarts_file_search` VACUOUS — passes only because the feature the marker names is absent: no file-search state on the chat composer: the '@' popup lists the popovers candidates only, so a query 'foo' opens no popup and t...
- `composer_2::restored_bound_at_mentions_do_not_open_mention_popup` VACUOUS — passes only because the feature the marker names is absent: no restored draft with mention bindings on the chat composer (set_text_content_with_mention_bindings has no seed): the restored `@...
- `composer_2::shortcut_overlay_persists_while_task_running` REAL — `?` draws the `Keyboard shortcuts` panel and it stays after the turn starts (the oracle's FooterMode::ShortcutOverlay persistence)

### composer_3

- `composer_3::dismissed_file_popup_ignores_token_substrings_in_leading_paste` VACUOUS — passes only because the feature the marker names is absent: no file popup on the chat composer: after Esc Esc on '@ma', a paste at the start does not reopen the popup for the '@ma' inside 'e...
- `composer_3::dismissed_file_popup_tracks_token_across_leading_whitespace_edits` VACUOUS — passes only because the feature the marker names is absent: no file popup on the chat composer: '@ma' opens it, Esc Esc dismisses it, and the dismissal holds across leading-space edits
- `composer_3::history_navigation_restores_remote_only_submissions` VACUOUS — passes only because the feature the marker names is absent: no remote image URLs and no prompt history on the chat composer: a remote-only submission is recorded, and Up restores it with an...
- `composer_3::humanlike_typing_1000_chars_appears_live_no_placeholder` VACUOUS — passes only because the feature the marker names is absent: no paste-burst state on the chat composer: 1000 human-paced 'z' keys appear live with no placeholder
- `composer_3::left_respects_attachments_pastes_and_other_input_surfaces` VACUOUS — passes only because the feature the marker names is absent: no agents command center on the chat composer: Left with text, a paste or a remote image does not open the agents overview, and th...
- `composer_3::platform_specific_altgr_input_remains_text` VACUOUS — passes only because the feature the marker names is absent: no paste-burst state on the chat composer: Ctrl+Alt+'@' is not a burst and inserts nothing on this platform (the oracle's non-Wind...
- `composer_3::slash_path_input_submits_without_command_error` REAL — a `/Users/..` path submits as text through the port's slash dispatcher: placeholder row back, text in the transcript
- `composer_3::slash_popup_not_activated_for_slash_space_text_history_like_input` VACUOUS — typed-text rows only; no assert on the popup or on history
- `composer_3::slash_with_leading_space_submits_as_text` REAL — ` /this-looks-like-a-command` submits as text through the dispatcher: placeholder row back, text in the transcript
- `composer_3::submit_captures_recent_mention_bindings_before_clearing_textarea` VACUOUS — the binding is not a row: only submit-and-clear of plain text
- `composer_3::test_partial_placeholder_deletion` VACUOUS — passes only because the feature the marker names is absent: no large-paste placeholder on the chat composer: Backspace inside a placeholder breaks it, so its text no longer shows the placeho...

### composer_4

- `composer_4::ascii_two_fast_chars_start_buffer_from_pending_and_flush_as_paste` VACUOUS — passes only because the feature the marker names is absent: no paste burst on the chat composer: the fast `a` and `b` are not held back and flushed as one paste
- `composer_4::backspace_deletes_a_body_selection_before_an_absorbed_prefix` VACUOUS — passes only because the feature the marker names is absent: no mouse selection on the chat composer and no `!` shell prefix absorbed into the prompt: the drag does not select `review text`,...
- `composer_4::blockquote_paste_replacing_selected_prefix_stays_literal` VACUOUS — passes only because the feature the marker names is absent: no mouse selection on the chat composer: the drag does not select `> selected`, so the paste does not replace it
- `composer_4::cancelled_history_search_preserves_literal_bang_draft` VACUOUS — literal-vs-shell (the claim) is unobservable without shell/queue; rows hold on any editor
- `composer_4::handoff_edit_discards_stale_command_before_ordinary_enter` VACUOUS — no handoff mode: plain type, ctrl+u, enter
- `composer_4::internal_snapshot_restores_exact_mention_identity` VACUOUS — passes only because the feature the marker names is absent: no Vim mode and no mention elements on the chat composer (no seed for insert_element or plugin bindings): the `@plugin` chips are...
- `composer_4::paste_burst_tab_does_not_accept_a_completion` VACUOUS — passes only because the feature the marker names is absent: no paste-burst state on the chat composer: the Tab after a pasted `review this` is not a literal tab, and a `/` popup accepts its...
- `composer_4::paste_burst_tabs_refresh_idle_timeout` VACUOUS — passes only because the feature the marker names is absent: no paste-burst window on the chat composer: a Tab is a literal tab while a burst is open, and the idle timeout is not drawn
- `composer_4::pasting_fifo_path_keeps_it_editable` VACUOUS — the kit cannot make the FIFO: the body pastes ordinary text, so the oracle's path handling is not exercised
- `composer_4::reconnect_edit_removes_deleted_attachment_and_paste_metadata` VACUOUS — passes only because the feature the marker names is absent: no disconnected composer on the chat screen (no seed) and no image attachment (no kit seed for attach_image): Ctrl+U in the discon...
- `composer_4::reconnect_expands_pastes_preserving_images_and_cursor` VACUOUS — passes only because the feature the marker names is absent: no disconnected composer on the chat screen (no seed), no large-paste placeholder and no image attachment (no kit seed for attach_...
- `composer_4::replace_yank_inserts_rich_objects_without_stale_character_recovery` VACUOUS — passes only because the feature the marker names is absent: no Vim Replace or yank, no mention, image or paste payload on the chat composer (no kit seeds): the rich objects and the Backspace...
- `composer_4::right_click_copy_uses_the_refreshed_editor_bounds` VACUOUS — passes only because the feature the marker names is absent: no mouse selection on the chat composer, and the right-click copy is not in the kit (click and drag are the left button only): the...
- `composer_4::vim_editing_modes_use_the_same_large_and_image_paste_handling` VACUOUS — passes only because the feature the marker names is absent: no Vim mode and no large-paste placeholder on the chat composer; the image-path paste needs a file on disk (no kit call writes one...
- `composer_4::vim_replace_yank_preserves_existing_image_and_recovers_atomically` VACUOUS — passes only because the feature the marker names is absent: no Vim Replace or yank and no image attachment on the chat composer: the existing image and the recovered text are not drawn

### composer_popups

- `composer_popups::category_tags_distinguish_mentions_when_descriptions_hide` VACUOUS — tags drawn, but the claim (tags stay when descriptions hide, four renders) is not asserted: partial; candidate once the narrow case is added
- `composer_popups::filesystem_columns_use_terminal_cell_widths` VACUOUS — contains of the typed `src/` and a conditional loop that may assert nothing
- `composer_popups::long_paths_keep_distinguishing_filenames_and_original_selection` VACUOUS — partial: two file names; the oracle's 18 renders are not
- `composer_popups::vim_normal_history_search_preview_places_cursor_on_last_char` VACUOUS — no vim mode; the cursor column (the claim) is not asserted

### markdown_streaming_1

- `markdown_streaming_1::append_markdown_agent_keeps_markdown_fence_when_content_is_not_table` REAL — `**bold**` kept literal inside the fence and no fence marker drawn: the oracle's one rendered row
- `markdown_streaming_1::append_markdown_agent_keeps_markdown_fence_with_blank_line_between_header_and_delimiter` REAL — fence content shown as source rows (no table grid): the oracle keeps the source unchanged
- `markdown_streaming_1::append_markdown_agent_keeps_non_markdown_fences_as_code` REAL — the three source table rows drawn as code lines
- `markdown_streaming_1::append_markdown_agent_unwraps_blockquoted_markdown_fence_table` VACUOUS — assert_absent of the fence marker only; the oracle's unwrapped table rows are not asserted
- `markdown_streaming_1::append_markdown_keeps_ordered_list_line_unsplit_in_context` REAL — `1. Tight item` on one row after the intro line
- `markdown_streaming_1::append_markdown_preserves_full_text_line` REAL — the whole 110-char line drawn at 120 columns (identical to plain text, but through the renderer)
- `markdown_streaming_1::citations_render_as_plain_text` REAL — citation text kept as plain rows (identical to plain text, but through the renderer)
- `markdown_streaming_1::crlf_code_block_no_extra_blank_lines` VACUOUS — in_order cannot see extra blank rows, the oracle's claim
- `markdown_streaming_1::fenced_code_info_string_with_metadata_highlights` VACUOUS — partial body: text only; the Rgb foreground is a kit gap
- `markdown_streaming_1::file_url_citation_preserves_literal_query_delimiters_snapshot` VACUOUS — no cwd seed (the oracle runs three cwds); only the label text
- `markdown_streaming_1::followups_preserve_literal_and_malformed_source` VACUOUS — partial body: the visible half of eight forms
- `markdown_streaming_1::generic_markdown_keeps_assistant_directives_literal` VACUOUS — does not reach the generic renderer path the oracle tests; plain text passes
- `markdown_streaming_1::list_spacing_preserves_links_and_source_when_removing_separators` VACUOUS — partial body: links only; the oracle compares Compact and Uniform renders
- `markdown_streaming_1::unicode_math_nested_fractions_and_oversized_pending_stay_source` VACUOUS — no math renderer: the source passes because nothing converts it
- `markdown_streaming_1::wrapped_table_url_fragments_keep_complete_web_destination` VACUOUS — one contains `report`; the per-row link destinations are a kit gap
- `markdown_streaming_1::wraps_list_items_containing_blockquotes` REAL — `1. item with quote` / `   > quoted text that` / `   > should wrap` at 24 columns: nested block layout and wrap

### markdown_streaming_2

- `markdown_streaming_2::blockquote_inside_nested_list` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::blockquote_list_then_nested_blockquote` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::blockquote_nested_two_levels` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::blockquote_surrounded_by_blank_lines` VACUOUS — in_order skips the blank rows that are the claim
- `markdown_streaming_2::blockquote_two_paragraphs_inside_ordered_list_has_blank_line` VACUOUS — the blank quote row is the claim; in_order skips it
- `markdown_streaming_2::blockquote_with_heading_and_paragraph` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::blockquote_with_multiline_code_block` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::disabled_mermaid_keeps_unsupported_source_without_notice` VACUOUS — no config seed and no mermaid renderer: source is shown because nothing converts it
- `markdown_streaming_2::disabled_renderers_preserve_source_independently` VACUOUS — no config seed; only the mermaid-off state, which passes because there is no mermaid renderer
- `markdown_streaming_2::headings` REAL — six heading rows with the oracle's bold/underlined/italic styles asserted
- `markdown_streaming_2::label_only_policy_keeps_non_web_links_and_literal_urls_unchanged` VACUOUS — no terminal-kind seed; the oracle compares two renderings, the body checks text order only
- `markdown_streaming_2::links_keep_their_columns_after_styled_fragments` REAL — four links by text and target after a wide glyph and styled spans
- `markdown_streaming_2::list_item_text_then_blockquote` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::list_nested` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::list_unordered_multiple` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::nested_blockquote_with_inline_and_fenced_code` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::nested_ordered_in_unordered` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::mermaid_fences_use_native_renderer_for_every_family` VACUOUS — assert_absent of the fence marker; no mermaid renderer
- `markdown_streaming_2::nested_unordered_in_ordered` REAL — MarkdownView draws the agent message now (transcript_pane.rs Item::Agent): the rows/styles the body names are the renderer's, not plain text
- `markdown_streaming_2::paragraph_multiple` VACUOUS — the blank row between paragraphs is the claim; in_order skips it
- `markdown_streaming_2::paragraph_soft_break` REAL — both rows `Hello` `World`; adjacency not asserted
- `markdown_streaming_2::supporting_terminals_render_only_the_styled_label_and_keep_its_target` REAL — label carries the link, `(url)` not drawn; the terminal kind is the kit's (supports OSC 8), not seeded

### markdown_streaming_3

- `markdown_streaming_3::code_block_known_lang_has_syntax_colors` VACUOUS — the colour is not asserted (kit has no any-coloured check)
- `markdown_streaming_3::emphasis` REAL — style italic asserted
- `markdown_streaming_3::inline_code` VACUOUS — the code style is not asserted; text only
- `markdown_streaming_3::strikethrough` REAL — style crossed_out asserted
- `markdown_streaming_3::strong` REAL — style bold asserted
- `markdown_streaming_3::tight_item_with_soft_break` VACUOUS — the continuation row's two-space indent (the claim) is not asserted: fragment `item line2`
- `markdown_streaming_3::strong_emphasis` REAL — bold and bold italic asserted
- `markdown_streaming_3::web_link_labels_keep_link_style_in_wrapped_prose_and_tables` REAL — underlined link labels asserted at 32 and 80 columns, prose and table

### markdown_streaming_4

- `markdown_streaming_4::escaped_pipes_render_in_table_cells` REAL — `a | b` drawn in a table cell
- `markdown_streaming_4::crossing_a_highlight_limit_removes_previously_highlighted_colors` VACUOUS — stream == whole compare only: the port draws the accumulated text through one renderer, so it holds trivially; colours not asserted
- `markdown_streaming_4::table_inside_blockquote_has_quote_prefix` VACUOUS — the every-row `> ` prefix (the claim) is not asserted
- `markdown_streaming_4::table_falls_back_to_key_value_records_if_grid_cannot_fit` REAL — at 20 columns c1/c10/10 drawn and no rule glyph or pipe: the oracle's fallback (positives are weaker: not on one row)
- `markdown_streaming_4::table_key_value_fallback_preserves_rich_values_and_themed_labels` VACUOUS — texts and Key bold only; the oracle's themed fg and underlined link are not asserted

### markdown_streaming_5

- `markdown_streaming_5::compact_list_spacing_keeps_incremental_and_full_renders_aligned` VACUOUS — passes only because the feature the marker names is absent: the live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is...
- `markdown_streaming_5::controller_handles_table_immediately_after_heading` VACUOUS — passes only because the feature the marker names is absent: the chat draws a table as its plain text: the table renderer (box separators, cell wrapping at the width, the mutable tail held un...
- `markdown_streaming_5::controller_holds_blockquoted_table_tail_until_stable` VACUOUS — passes only because the feature the marker names is absent: the chat draws a table as its plain text: the table renderer (box separators, cell wrapping at the width, the mutable tail held un...
- `markdown_streaming_5::controller_keeps_non_markdown_fenced_tables_as_code` REAL — `| A | B |` kept as code and no table rule glyphs (the stream==whole part is trivial, see VACUOUS notes)

### markdown_streaming_4

- `markdown_streaming_4::ambiguous_and_normalized_fences_use_canonical_semantics` VACUOUS — stream == whole compare only: holds trivially (one renderer over the accumulated text)

### markdown_streaming_5

- `markdown_streaming_5::controller_live_view_matches_render_during_interleaved_table_streaming` VACUOUS — passes only because the feature the marker names is absent: the chat draws a table as its plain text: the table renderer (box separators, cell wrapping at the width, the mutable tail held un...
- `markdown_streaming_5::controller_renders_separators_for_multi_table_response_shape` REAL — table rule glyph `━` drawn for the three tables (stream==whole part trivial)
- `markdown_streaming_5::controller_loose_vs_tight_with_commit_ticks_matches_full` REAL — eight list rows in order incl. numbering and nested bullets (stream==whole part trivial)
- `markdown_streaming_5::controller_set_width_preserves_in_flight_tail` REAL — the tail row `• tail without newline` survives a resize to 24
- `markdown_streaming_5::controller_renders_separators_for_no_outer_pipes_table_shape` REAL — `━` drawn and the no-outer-pipes source row absent (stream==whole part trivial)
- `markdown_streaming_5::controller_set_width_during_confirmed_table_stream_matches_finalize_render` REAL — after the resize to 32 the rows equal a fresh render at 32 (a real relayout check)
- `markdown_streaming_5::controller_set_width_rebuilds_queued_lines` REAL — resize to 24 wraps the long line over rows in order
- `markdown_streaming_5::controller_stabilizes_two_column_no_outer_table_in_response` REAL — `━` drawn and `A | B` source absent (stream==whole part trivial)
- `markdown_streaming_5::controller_stabilizes_first_no_outer_pipes_table_in_response` REAL — `━` drawn and the source row absent (stream==whole part trivial)
- `markdown_streaming_5::controller_streamed_table_matches_full_render_widths` VACUOUS — passes only because the feature the marker names is absent: the chat draws a table as its plain text: the table renderer (box separators, cell wrapping at the width, the mutable tail held un...
- `markdown_streaming_5::empty_list_item_stays_mutable_until_finalization` VACUOUS — passes only because the feature the marker names is absent: the live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is...
- `markdown_streaming_5::growing_single_top_level_blocks_render_and_scan_in_one_pass` VACUOUS — passes only because the feature the marker names is absent: the live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is...
- `markdown_streaming_5::incremental_file_citations_preserve_metadata_unicode_and_markdown` VACUOUS — passes only because the feature the marker names is absent: the chat draws inline visualization directives and reference links as text: the inline visualization context and its canonical ful...
- `markdown_streaming_5::incremental_raw_render_preserves_blank_lines` VACUOUS — passes only because the feature the marker names is absent: the chat has no raw/rich history render mode (HistoryRenderMode) to set or switch: no seed or key reaches it, the chat renders its...
- `markdown_streaming_5::incremental_render_does_not_add_blank_line_before_html_block` VACUOUS — passes only because the feature the marker names is absent: the live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is...
- `markdown_streaming_5::inline_visualization_content_references_use_canonical_full_render` VACUOUS — passes only because the feature the marker names is absent: the chat draws inline visualization directives and reference links as text: the inline visualization context and its canonical ful...
- `markdown_streaming_5::incremental_render_preserves_heading_and_normalized_html_seams` VACUOUS — passes only because the feature the marker names is absent: the live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is...
- `markdown_streaming_5::inline_visualizations_use_canonical_full_render` VACUOUS — passes only because the feature the marker names is absent: the chat draws inline visualization directives and reference links as text: the inline visualization context and its canonical ful...
- `markdown_streaming_5::lists_match_incremental_and_emitted_responses` VACUOUS — passes only because the feature the marker names is absent: no seed for the rendering preferences (tui.rendering: mermaid, math, tables, lists): the live chat starts with every renderer on a...
- `markdown_streaming_5::paragraphs_after_unwrapped_table_fence_advance_stable_source` VACUOUS — passes only because the feature the marker names is absent: the chat draws a table as its plain text: the table renderer (box separators, cell wrapping at the width, the mutable tail held un...
- `markdown_streaming_5::mermaid_holdback_tracks_nested_blocks_after_normalized_markdown` VACUOUS — passes only because the feature the marker names is absent: a mermaid fence draws as its source text: the native mermaid renderer and its holdback are not built in the chat's markdown yet
- `markdown_streaming_5::plan_controller_streamed_table_matches_final_render` VACUOUS — passes only because the feature the marker names is absent: the plan stream has no view in the chat: a plan item (its heading, quote, table and links) is drawn as plain text; PlanStreamContr...
- `markdown_streaming_5::reference_link_definition_recomputes_earlier_and_later_blocks` VACUOUS — passes only because the feature the marker names is absent: the live chat draws an agent message as its plain text: the markdown renderer (rtwk-rich MarkdownView over the streamed answer) is...
- `markdown_streaming_5::reference_link_definition_survives_raw_to_rich_render_mode_switch` VACUOUS — passes only because the feature the marker names is absent: the chat has no raw/rich history render mode (HistoryRenderMode) to set or switch: no seed or key reaches it, the chat renders its...
- `markdown_streaming_5::rendering_preferences_match_incremental_final_and_emitted_responses` VACUOUS — passes only because the feature the marker names is absent: no seed for the rendering preferences (tui.rendering: mermaid, math, tables, lists): the live chat starts with every renderer on a...
- `markdown_streaming_5::rejected_math_openers_allow_bounded_incremental_rendering` VACUOUS — passes only because the feature the marker names is absent: the chat draws math sources as their text: no unicode math renderer (inline, display, named delimiters, the unfinished display pre...
- `markdown_streaming_5::unicode_math_aligned_stream_matches_complete_render` VACUOUS — passes only because the feature the marker names is absent: the chat draws math sources as their text: no unicode math renderer (inline, display, named delimiters, the unfinished display pre...
- `markdown_streaming_5::unicode_math_stream_holds_display_until_closed_and_preserves_source` VACUOUS — passes only because the feature the marker names is absent: the chat draws math sources as their text: no unicode math renderer (inline, display, named delimiters, the unfinished display pre...

### pane_footer_1

- `pane_footer_1::an_attachment_ends_the_sparkle_even_after_the_composer_is_cleared` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...
- `pane_footer_1::arbitrary_commands_preserve_a_never_used_fresh_opportunity` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...
- `pane_footer_1::buffered_commands_and_read_only_inline_arguments_preserve_eligibility` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...
- `pane_footer_1::cancelling_history_search_preserves_a_typed_command_despite_attachment_previews` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...
- `pane_footer_1::commands_hide_a_started_sparkle_without_restarting_its_deadline` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...
- `pane_footer_1::focus_startup_work_and_configuration_do_not_extend_a_started_deadline` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...
- `pane_footer_1::input_history_search_uses_the_final_row` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...
- `pane_footer_1::the_first_buffered_character_and_empty_escape_clear_visible_stars` VACUOUS — passes only because the feature the marker names is absent: the composer sparkle (braille starfield dots in the placeholder of a fresh Astra task, with its 15 s idle deadline and 1 s fade) i...

### pane_footer_2

- `pane_footer_2::ctrl_c_cancels_history_search_without_clearing_draft_or_showing_quit_hint` VACUOUS — passes only because the feature the marker names is absent: no history search in the live chat: Ctrl+R opens the search over the draft `draft`; Ctrl+C must close it without clearing the draf...
- `pane_footer_2::esc_with_agent_command_without_popup_does_not_interrupt_task` VACUOUS — passes only because the feature the marker names is absent: no running task with `/subagents ` typed: no entry chat/running_subagents_command
- `pane_footer_2::esc_with_skill_popup_does_not_interrupt_task` VACUOUS — passes only because the feature the marker names is absent: no running task with a skill popup open: no entry chat/running_skill_popup (the `$` prompt typed while a task runs)
- `pane_footer_2::flush_buffered_typing_in_background_and_covering_editors` VACUOUS — passes only because the feature the marker names is absent: no covering request-user-input overlay in the live chat (question `Which way?`, typed `modal`): the composer's buffered `main draf...
- `pane_footer_2::status_line_theme_colors_at_limited_color_depths` VACUOUS — passes only because the feature the marker names is absent: the kit cannot set FORCE_COLOR for the app: the oracle runs this test in a child process at colour depths 0 and 1; no seed for a c...
- `pane_footer_2::typed_approval_shortcuts_during_delay_stay_in_composer` VACUOUS — passes only because the feature the marker names is absent: no delayed exec approval in the live chat: typed `y` and `a` during the typing-idle delay stay in the composer as `ya kept`

### textarea

- `textarea::dot_repeat_has_visual_snapshot_coverage` VACUOUS — no vim: the typed `0dw.` chars leave text matching the in_order fragments
- `textarea::find_and_navigation_have_visual_snapshot_coverage` VACUOUS — no vim: the motions are typed text; fragments always present
- `textarea::kill_whole_line_keymap_dispatch_uses_linewise_kill` VACUOUS — no keymap seed; default ctrl+u kills to line start and also leaves no `def`: does not tell linewise from line-start kill

### transcript_view_1

- `transcript_view_1::cancelling_owned_backtrack_returns_arrow_keys_to_the_composer` VACUOUS — passes only because the feature the marker names is absent: no owned transcript overlay with a backtrack preview: Esc primes it, an ordinary key cancels the preview and the arrow keys reach...
- `transcript_view_1::composer_paste_releases_selection_and_retains_normal_cursor_movement` VACUOUS — paste and cursor move work; the selection release (the claim) has no key to start one
- `transcript_view_1::copy_feedback_is_right_aligned_and_does_not_claim_terminal_delivery` VACUOUS — passes only because the feature the marker names is absent: no copy feedback row through the kit for Pending, Busy, Confirmed, Unconfirmed and Err results at 80, 32 and 18 columns: the clipb...
- `transcript_view_1::enter_preserves_search_backtrack_and_modal_ownership_while_scrolled` VACUOUS — passes only because the feature the marker names is absent: no owned transcript view: Enter while searching, Esc with an empty selection anchor, the backtrack priming and the open modals (re...
- `transcript_view_1::escape_closes_selection_and_search_before_returning_to_latest` VACUOUS — passes only because the feature the marker names is absent: no transcript selection (begin_selection with three clicks), no Find (begin_search) and no footer on the chat screen: the Esc chai...
- `transcript_view_1::footer_selection_defers_warning_clicks_and_preserves_the_draft` VACUOUS — passes only because the feature the marker names is absent: no warning history cell seed (a warning with a long URL and eight padding lines inserted into the history) and no selection on the...
- `transcript_view_1::inline_confirmation_preserves_compact_viewport_without_replaying_history` VACUOUS — passes only because the feature the marker names is absent: no replace-goal confirmation over the inline chat (show_replace_thread_goal_confirmation with the draft 'A replacement goal'): the...
- `transcript_view_1::owned_drag_stops_when_focus_or_input_ownership_is_lost` VACUOUS — passes only because the feature the marker names is absent: no owned transcript selection: a drag over the scrolled rows selects text, and the selection stops when focus is lost, the session...
- `transcript_view_1::owned_transcript_keeps_text_out_of_the_pet_columns` VACUOUS — passes only because the feature the marker names is absent: no ambient pet column: the kitty pet image support and its reserved columns are not built; the screen draws the long row, the blan...
- `transcript_view_1::provider_error_escape_opens_resume_picker` VACUOUS — passes only because the feature the marker names is absent: no provider-error seed: a model-provider mismatch (show_provider_error) has no kit call; Esc after it opens the resume picker and...
- `transcript_view_1::visible_updates_and_empty_tail_cells_do_not_report_activity` VACUOUS — passes only because the feature the marker names is absent: no unseen-activity footer on the chat screen: the scrolled-up transcript, the new visible output that raises it and the footer tha...

### transcript_view_2

- `transcript_view_2::closing_find_at_bottom_does_not_show_back_to_bottom` VACUOUS — passes only because the feature the marker names is absent: the chat's transcript has no find that closes at the bottom without showing the follow control
- `transcript_view_2::copy_navigation_keeps_bottom_context_when_prompt_header_appears` VACUOUS — passes only because the feature the marker names is absent: the chat's transcript has no copy mode: Home and Down move the copy target, and the prompt header appearing keeps the target at ro...
- `transcript_view_2::copying_selection_at_bottom_does_not_show_back_to_bottom` VACUOUS — passes only because the feature the marker names is absent: the chat's transcript has no follow control and no double-click word selection at the bottom to copy
- `transcript_view_2::decimal_mouse_scroll_speeds_preserve_whole_rows` VACUOUS — passes only because the feature the marker names is absent: the chat's transcript has no mouse scroll speed setting for 0.1 and 0.3; whole rows are drawn only after the accumulated movement
- `transcript_view_2::find_and_shift_click_release_copy_payload` VACUOUS — passes only because the feature the marker names is absent: the chat's transcript has no find that selects a match with a shift-click release, and no copy mode footer for it
- `transcript_view_2::height_changes_backfill_after_empty_click_but_preserve_deliberate_pause` VACUOUS — passes only because the feature the marker names is absent: the chat's transcript has no backfill on a height change after an empty click, and no deliberate-pause memory across heights
- `transcript_view_2::ordinary_click_opens_bare_and_markdown_links_on_release` VACUOUS — link cells asserted, but not the open on release (the claim)

### transcript_view_3

- `transcript_view_3::paused_header_reservation_survives_copy_until_scroll_or_resize` VACUOUS — passes only because the feature the marker names is absent: the chat draws no pinned header and no paused header reservation after a copy

### transcript_view_5

- `transcript_view_5::startup_warning_keeps_first_visible_header_in_place` VACUOUS — passes only because the feature the marker names is absent: no startup-warning entry (a skill manifest notice) and no session header kept in the live tail: no script shape yet
