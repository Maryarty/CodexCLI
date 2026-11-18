# chat-nav — parity

The chat as the hub of the app: the command registry, dispatch, the key
actions with the oracle's defaults, the composer's popups (`/`, `$`, `@`,
Ctrl+R) and every transition from the chat. Branch `tui-rtwk/chat-nav`.
Layers: core (no rtwk) holds the registry, the parse, the popup filter, the
queue rule and the dispatch decision as a typed `Outcome`; the view holds the
widgets, the key bindings and the opening of screens.

Notes on the brief: the oracle's `SlashCommand` has **65** variants, not 66
(`SlashCommand::ALL` is checked against the oracle's source by
`core/tests/commands.rs::the_registry_has_the_oracles_commands_in_the_oracles_order`).
No shared file was edited: the new test modules are submodules of
`tests/chat.rs` (`#[path]`), the popovers entry is in `src/screens/popovers/`.
Two golden files of the skeleton changed because the chat screen gained the
oracle's `› ` prompt: `tests/snapshots/chat/empty.snap`, `answer.snap`.

## Oracle tests

Oracle files of the slice: `slash_command.rs`, `bottom_pane/{slash_commands,
command_popup,skill_popup,file_search_popup}.rs`, `bottom_pane/mentions_v2/
{render_tests,search_catalog}.rs`, `bottom_pane/chat_composer/history_search.rs`,
`bottom_pane/chat_composer_history.rs`, `chatwidget/tests/slash_commands.rs`.
218 tests: **81 ported**, **7 delta**, **130 n/a** (each with its reason: the
effect is another slice's: session-core, the composer slot, the keymap slice,
the pets and dialogs slots). "ported" of a decision means the typed `Outcome`
(core) equals the oracle's effect; the effect itself (an RPC, a turn) is
session-core's and is shown as a `ChatNotice::NotAvailable` line until then
(`tests/commands.rs::a_command_that_needs_the_session_says_it_is_not_available_yet`).
Tests are named after the oracle's where one oracle test is one test here.

### `slash_command.rs` (5)

| oracle test | ours | status | note |
|---|---|---|---|
| `stop_command_is_canonical_name` | core/tests/commands.rs::stop_command_is_canonical_name | ported |  |
| `clean_alias_parses_to_stop_command` | core/tests/commands.rs::clean_alias_parses_to_stop_command | ported |  |
| `pet_alias_parses_to_pets_command` | core/tests/commands.rs::pet_alias_parses_to_pets_command | ported |  |
| `certain_commands_are_available_during_task` | core/tests/commands.rs::certain_commands_are_available_during_task | ported |  |
| `auto_review_command_is_approve` | core/tests/commands.rs::auto_review_command_is_approve | ported |  |

### `bottom_pane/slash_commands.rs` (15)

| oracle test | ours | status | note |
|---|---|---|---|
| `worktree_command_lookup_requires_feature` | core/tests/commands.rs::worktree_command_lookup_requires_feature | ported |  |
| `debug_command_still_resolves_for_dispatch` | core/tests/commands.rs::debug_command_still_resolves_for_dispatch | ported |  |
| `clear_command_resolves_for_dispatch` | core/tests/commands.rs::clear_command_resolves_for_dispatch | ported |  |
| `goal_command_allows_extra_os_for_dispatch` | core/tests/commands.rs::goal_command_allows_extra_os_for_dispatch | ported |  |
| `stop_command_resolves_for_dispatch` | core/tests/commands.rs::stop_command_resolves_for_dispatch | ported |  |
| `clean_command_alias_resolves_for_dispatch` | core/tests/commands.rs::clean_command_alias_resolves_for_dispatch | ported |  |
| `service_tier_commands_are_hidden_when_disabled` | core/tests/commands.rs::service_tier_commands_are_hidden_when_disabled | ported |  |
| `all_service_tiers_are_exposed_as_commands_after_model` | core/tests/commands.rs::all_service_tiers_are_exposed_as_commands_after_model | ported |  |
| `goal_command_is_hidden_when_disabled` | core/tests/commands.rs::goal_command_is_hidden_when_disabled | ported |  |
| `voice_command_is_hidden_when_disabled` | core/tests/commands.rs::voice_command_is_hidden_when_disabled | ported |  |
| `usage_command_is_hidden_from_input_when_account_token_activity_is_disabled` | core/tests/commands.rs::usage_command_is_hidden_from_input_when_account_token_activity_is_disabled | ported |  |
| `usage_command_exact_lookup_still_resolves_when_account_token_activity_is_disabled` | core/tests/commands.rs::usage_command_exact_lookup_still_resolves_when_account_token_activity_is_disabled | ported |  |
| `side_conversation_hides_commands_without_side_flag` | core/tests/commands.rs::side_conversation_hides_commands_without_side_flag | ported |  |
| `side_conversation_exact_lookup_still_resolves_hidden_commands_for_dispatch_error` | core/tests/commands.rs::side_conversation_exact_lookup_still_resolves_hidden_commands_for_dispatch_error | ported |  |
| `side_conversation_exact_lookup_still_resolves_service_tier_commands_for_dispatch_error` | core/tests/commands.rs::side_conversation_exact_lookup_still_resolves_service_tier_commands_for_dispatch_error | ported |  |

### `bottom_pane/command_popup.rs` (16)

| oracle test | ours | status | note |
|---|---|---|---|
| `unavailable_side_conversation_command_is_shown_only_when_searched` | core/tests/commands.rs::unavailable_side_conversation_command_is_shown_only_when_searched | ported |  |
| `filter_includes_init_when_typing_prefix` | core/tests/commands.rs::filter_includes_init_when_typing_prefix | ported |  |
| `selecting_init_by_exact_match` | core/tests/commands.rs::selecting_init_by_exact_match | ported |  |
| `model_is_first_suggestion_for_mo` | core/tests/commands.rs::model_is_first_suggestion_for_mo | ported |  |
| `service_tier_command_uses_catalog_name_and_description` | core/tests/commands.rs::service_tier_command_uses_catalog_name_and_description | ported |  |
| `command_popup_wrap_boundary_preserves_following_choice` | core/tests/commands.rs::command_popup_wrap_boundary_preserves_following_choice | ported |  |
| `filtered_commands_keep_presentation_order_for_prefix` | core/tests/commands.rs::filtered_commands_keep_presentation_order_for_prefix | ported |  |
| `app_command_popup_snapshot` | tests/popovers.rs::command_popup_app; core/tests/commands.rs::app_and_voice_rows_of_the_popup_snapshots | ported | rows equal; look differs (N1) |
| `voice_command_popup_snapshot` | tests/popovers.rs::command_popup_voice | ported | row equal; look differs (N1) |
| `prefix_filter_limits_matches_for_ac` | core/tests/commands.rs::prefix_filter_limits_matches_for_ac | ported |  |
| `changing_filter_resets_selection_after_scrolling` | core/tests/commands.rs::changing_filter_resets_selection_after_scrolling | ported |  |
| `quit_hidden_in_empty_filter_but_shown_for_prefix` | core/tests/commands.rs::quit_hidden_in_empty_filter_but_shown_for_prefix | ported |  |
| `btw_hidden_in_empty_filter_but_shown_for_prefix` | core/tests/commands.rs::btw_hidden_in_empty_filter_but_shown_for_prefix | ported |  |
| `plan_command_hidden_when_collaboration_modes_disabled` | core/tests/commands.rs::plan_command_hidden_when_collaboration_modes_disabled | ported |  |
| `plan_command_visible_when_collaboration_modes_enabled` | core/tests/commands.rs::plan_command_visible_when_collaboration_modes_enabled | ported |  |
| `debug_commands_are_hidden_from_popup` | core/tests/commands.rs::debug_commands_are_hidden_from_popup | ported |  |

### `bottom_pane/skill_popup.rs` (6)

| oracle test | ours | status | note |
|---|---|---|---|
| `filtered_mentions_preserve_results_beyond_popup_height` | - | n/a | scrolling beyond 8 rows is rtwk Autocomplete's (`rows(8)`); not asserted here |
| `scrolling_mentions_shifts_rendered_window_snapshot` | - | n/a | same: rtwk Autocomplete scrolls its window (N1) |
| `category_tags_distinguish_mentions_when_descriptions_hide` | tests/popovers.rs::skill_popup_category_tags | delta | N1: the oracle hides descriptions at width 32 and keeps the tags; ours lists tag and detail in the full width |
| `lowercase_expansion_preserves_match_ranking_and_highlighting` | core/tests/commands.rs::unicode_dotted_i_istanbul_highlighting | delta | fuzzy matcher ported; mention rows show no match highlight (N1) |
| `display_name_match_sorting_beats_worse_secondary_search_term_matches` | - | n/a | mention ranking is the fuzzy score only; no secondary search terms yet (N2) |
| `match_tier_precedes_preference_and_plugin_rank` | - | n/a | no tier, preference or plugin rank in `popovers::candidates` (N2) |

### `bottom_pane/file_search_popup.rs` (2)

| oracle test | ours | status | note |
|---|---|---|---|
| `long_paths_keep_distinguishing_filenames_and_original_selection` | tests/popovers.rs::long_paths_keep_distinguishing_filenames_and_original_selection | ported |  |
| `set_matches_keeps_only_the_first_page_of_results` | - | n/a | file search backend (session-core); the files are seeded |

### `bottom_pane/mentions_v2/render_tests.rs` (5)

| oracle test | ours | status | note |
|---|---|---|---|
| `live_picker_frame_snapshots` | - | n/a | mentions picker v2 (tabs, header, footer, filesystem columns): N2 |
| `selection_covers_the_row_and_empty_filter_clears_previous_results` | - | n/a | mentions picker v2 (tabs, header, footer, filesystem columns): N2 |
| `filesystem_columns_use_terminal_cell_widths` | - | n/a | mentions picker v2 (tabs, header, footer, filesystem columns): N2 |
| `fuzzy_matches_preserve_graphemes_when_truncated` | - | n/a | mentions picker v2 (tabs, header, footer, filesystem columns): N2 |
| `long_mentions_leave_room_for_distinguishing_parent_paths` | - | n/a | mentions picker v2 (tabs, header, footer, filesystem columns): N2 |

### `bottom_pane/mentions_v2/search_catalog.rs` (3)

| oracle test | ours | status | note |
|---|---|---|---|
| `plugin_mention_name_uses_display_segments_when_they_match_plugin_name` | - | n/a | mention data derived by the backend (session-core) |
| `generated_app_mentions_use_bounded_display_names` | - | n/a | mention data derived by the backend (session-core) |
| `plugin_mention_name_falls_back_to_title_cased_plugin_name` | - | n/a | mention data derived by the backend (session-core) |

### `bottom_pane/chat_composer/history_search.rs` (19)

| oracle test | ours | status | note |
|---|---|---|---|
| `vim_normal_j_k_navigate_history_at_history_boundaries` | - | n/a | Vim mode of the composer: composer slot |
| `remapped_vim_normal_history_navigation_does_not_fall_back_to_j_k` | - | n/a | Vim mode of the composer: composer slot |
| `vim_normal_j_k_fall_back_to_multiline_cursor_movement` | - | n/a | Vim mode of the composer: composer slot |
| `vim_normal_operator_motion_does_not_navigate_history` | - | n/a | Vim mode of the composer: composer slot |
| `history_search_opens_without_previewing_latest_entry` | core/tests/popovers.rs::history_search_shows_the_newest_match_and_walks_older_and_newer | ported | `HistorySearch::new` is Idle, preview none |
| `unavailable_history_search_restores_idle_draft` | - | n/a | persistent history fetch (session-core) |
| `history_search_match_ranges_are_case_insensitive` | core/tests/popovers.rs::history_search_is_case_insensitive_and_reports_no_match | delta | matching is case-insensitive; the ranges are not highlighted (N5) |
| `history_search_match_ranges_preserve_unicode_boundaries` | - | n/a | no highlight ranges (N5) |
| `history_search_accepts_matching_entry` | tests/popovers.rs::enter_accepts_the_match_into_the_composer | ported |  |
| `vim_normal_history_search_preview_places_cursor_on_last_char` | - | n/a | Vim mode of the composer: composer slot |
| `history_search_stays_on_single_match_at_boundaries` | core/tests/popovers.rs::history_search_shows_the_newest_match_and_walks_older_and_newer | ported | bitten (see Bites) |
| `history_search_footer_action_hints_are_emphasized` | tests/popovers.rs::footer_mode_history_search | delta | N5: the footer is one dim line, the key hints are not chips |
| `history_search_highlights_matches_until_accepted` | - | delta | N5: no match highlight in the preview |
| `history_search_esc_restores_original_draft` | tests/popovers.rs::esc_cancels_and_restores_the_draft | ported |  |
| `history_search_ctrl_c_restores_original_draft` | - | delta | N6: Ctrl+C in search is not caught: it quits the app as on the chat (oracle: restores the draft) |
| `history_search_flushes_pending_first_char_before_snapshot` | - | n/a | paste-burst buffering is not in rtwk's TextArea; the composer slot |
| `history_search_flushes_buffered_paste_before_snapshot` | - | n/a | paste burst: composer slot |
| `history_search_esc_resets_normal_history_navigation` | - | n/a | Up/Down history recall belongs to the composer slot |
| `history_search_no_match_restores_preview_but_keeps_search_open` | core/tests/popovers.rs::history_search_is_case_insensitive_and_reports_no_match | ported | preview none, the draft shows, search stays open |

### `bottom_pane/chat_composer_history.rs` (12)

| oracle test | ours | status | note |
|---|---|---|---|
| `duplicate_submissions_are_not_recorded` | - | n/a | `Record` skips an immediate repeat in `popovers::apply_intent`; not asserted: missing test, follow-up |
| `initial_metadata_preserves_startup_history_but_session_changes_clear_it` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |
| `persistent_restore_gates_at_mentions` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |
| `navigation_with_async_fetch` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |
| `search_matches_local_history_and_stops_at_boundaries` | core/tests/popovers.rs::history_search_shows_the_newest_match_and_walks_older_and_newer | ported |  |
| `search_skips_duplicate_local_matches` | - | n/a | duplicates are not skipped while walking; follow-up |
| `repeated_boundary_search_does_not_refetch_persistent_history` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |
| `search_fetches_persistent_history_until_match` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |
| `search_skips_duplicate_persistent_matches` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |
| `search_is_case_insensitive_and_empty_query_finds_latest` | core/tests/popovers.rs::history_search_is_case_insensitive_and_reports_no_match | delta | an empty query previews nothing here (the oracle's composer test `history_search_opens_without_previewing_latest_entry` says the same; the store test says latest): Idle |
| `reset_navigation_resets_cursor` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |
| `should_handle_navigation_when_cursor_is_at_line_boundaries` | - | n/a | persistent history store and Up/Down recall: session-core / composer slot |

### `chatwidget/tests/slash_commands.rs` (137)

| oracle test | ours | status | note |
|---|---|---|---|
| `service_tier_commands_and_saved_selection_respect_independent_speed_policy` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `service_tier_commands_lowercase_catalog_names` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `slash_compact_eagerly_queues_follow_up_before_turn_start` | - | n/a | queue + session effect (session-core) |
| `slash_recap_requests_generation_for_current_thread` | core/tests/commands.rs::a_command_that_needs_the_session_is_a_typed_intent_never_a_silent_noop | n/a | typed session intent |
| `queued_slash_compact_dispatches_after_active_turn` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_slash_review_with_args_dispatches_after_active_turn` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_slash_review_with_args_restores_for_edit` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_bang_shell_dispatches_after_active_turn` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_empty_bang_shell_reports_help_when_dequeued_and_drains_next_input` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_bang_shell_waits_for_user_shell_completion_before_next_input` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_slash_menu_cancel_drains_next_input` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_settings_selection_applies_before_next_input` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_bare_rename_drains_next_input_after_name_update` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_inline_rename_does_not_drain_again_before_turn_started` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_unknown_slash_reports_error_when_dequeued` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `ctrl_d_quits_without_prompt` | - | n/a | Ctrl+D is an editor/keymap action (keymap slice) |
| `ctrl_d_with_modal_open_does_not_quit` | - | n/a | keymap slice |
| `slash_init_does_not_depend_on_loaded_instruction_sources` | core/tests/commands.rs::a_command_that_needs_the_session_is_a_typed_intent_never_a_silent_noop | n/a | typed session intent |
| `bare_slash_command_is_available_from_local_recall_after_dispatch` | - | n/a | composer slot |
| `inline_slash_command_is_available_from_local_recall_after_dispatch` | - | n/a | composer slot |
| `goal_slash_command_with_extra_os_emits_set_goal_event` | core/tests/commands.rs::goal_slash_command_with_extra_os_emits_set_goal_event | ported |  |
| `goal_slash_command_uses_plain_text_for_mentions` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `goal_slash_command_emits_attached_images` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `bare_goal_slash_command_drains_pending_submission_state` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `goal_control_slash_commands_emit_goal_events` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `goal_control_slash_command_without_thread_shows_full_usage` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `goal_edit_slash_command_opens_goal_editor` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `queued_goal_slash_command_emits_set_goal_event_after_thread_starts` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_goal_slash_command_preserves_large_paste` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_goal_slash_command_restores_large_paste_for_edit` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `interrupt_disambiguates_same_sized_goal_pastes` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `queued_goal_slash_command_preserves_current_draft_metadata` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `restored_queued_goal_slash_command_emits_set_goal_event` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `merged_history_record_preserves_raw_text_and_rebased_elements` | - | n/a | notifications, paste: other slices |
| `merged_history_record_remaps_override_image_placeholders` | - | n/a | notifications, paste: other slices |
| `interrupted_merged_message_history_encodes_mentions_once` | - | n/a | notifications, paste: other slices |
| `slash_rename_prefills_existing_thread_name` | - | n/a | rename prompt: dialogs slot (the command opens it: tests/transitions.rs::slash_rename_opens_the_dialogs) |
| `slash_rename_requests_and_prefills_an_editable_title_suggestion` | - | n/a | dialogs slot |
| `slash_rename_preserves_manual_edits_when_suggestion_arrives` | - | n/a | dialogs slot |
| `slash_rename_without_existing_thread_name_starts_empty` | - | n/a | dialogs slot |
| `usage_error_slash_command_is_available_from_local_recall` | - | n/a | composer slot |
| `signed_out_usage_command_reports_chatgpt_login_requirement` | core/tests/commands.rs::signed_out_usage_command_reports_chatgpt_login_requirement | ported |  |
| `signed_out_usage_command_with_args_reports_chatgpt_login_requirement` | core/tests/commands.rs::signed_out_usage_command_reports_chatgpt_login_requirement | ported | same decision |
| `usage_command_with_invalid_view_reports_usage_snapshot` | core/tests/commands.rs::usage_command_with_invalid_view_reports_usage_snapshot | ported |  |
| `usage_views_open_the_requested_summary_mode` | core/tests/commands.rs::usage_views_open_the_requested_summary_mode | ported |  |
| `usage_command_runs_with_backend_auth_without_chatgpt_account_flag` | core/tests/commands.rs::usage_views_open_the_requested_summary_mode | ported | `has_backend_auth` |
| `usage_command_runs_with_backend_auth_from_widget_init` | core/tests/commands.rs::usage_views_open_the_requested_summary_mode | ported | `signed_in` seeds it |
| `unrecognized_slash_command_is_not_added_to_local_recall` | core/tests/commands.rs::unrecognized_slash_command_is_not_added_to_local_recall | ported |  |
| `unavailable_slash_command_is_available_from_local_recall` | - | n/a | Up-arrow recall: composer slot |
| `no_op_stub_slash_command_is_available_from_local_recall` | - | n/a | composer slot |
| `slash_exit_aliases_request_exit` | core/tests/commands.rs::slash_exit_aliases_request_exit | ported |  |
| `slash_logout_requests_app_server_logout` | core/tests/commands.rs::slash_logout_requests_app_server_logout | ported |  |
| `slash_copy_state_tracks_turn_complete_final_reply` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_uses_completed_commentary_during_active_turn` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_state_tracks_plan_item_completion` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_preserves_completed_source_whitespace_and_hides_directives` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_defers_queued_input_until_selection_or_cancellation_settles` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_previews_whole_response_code_blocks_and_blockquotes` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_copies_status_fields_and_preserves_source_after_copying` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_status_omits_missing_fields_and_copies_full_directory` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_status_yields_to_later_turns_and_commands_even_after_refresh` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_status_yields_to_queued_commands_after_queued_status` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_waits_for_submission_after_typing_or_autocomplete` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_waits_for_submission_after_paste` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_numeric_shortcuts_copy_whole_response_and_exact_code` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_renders_only_whole_responses_as_markdown` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_preserves_nested_blockquote_markdown_and_code` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_supports_arrow_navigation_enter_and_scrolling` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_escape_dismisses_without_copying` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_picker_remains_available_from_parent_owned_threads` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_daybreak_offers_an_application_when_unavailable` | - | n/a | session effect; the popup row is core/tests/commands.rs (daybreak description override) |
| `slash_daybreak_custom_provider_requires_capability` | - | n/a | session effect |
| `slash_copy_reports_when_no_agent_response_exists` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_export_opens_destination_picker` | core/tests/commands.rs::a_command_that_needs_the_session_is_a_typed_intent_never_a_silent_noop | n/a | typed session intent; the picker is not built |
| `copy_action_reports_when_no_agent_response_exists` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `keymap_capture_can_capture_current_copy_shortcut` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_keymap_capture_can_capture_app_shortcuts` | - | n/a | keymap slice |
| `slash_keymap_debug_opens_keypress_inspector` | core/tests/commands.rs::slash_keymap_opens_the_picker_debug_opens_the_inspector_invalid_shows_usage | ported | opens `settings` (the inspector is the keymap slice) |
| `slash_keymap_debug_can_inspect_app_shortcuts` | - | n/a | keymap slice |
| `slash_keymap_invalid_args_show_usage` | core/tests/commands.rs::slash_keymap_opens_the_picker_debug_opens_the_inspector_invalid_shows_usage; tests/commands.rs::usage_notices_of_the_oracle | ported |  |
| `copy_shortcut_can_be_remapped` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `legacy_image_paste_ignores_buffered_press_until_other_input` | - | n/a | notifications, paste: other slices |
| `copy_shortcut_submits_markdown_and_reports_completion_once` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_state_is_preserved_during_running_task` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `slash_copy_uses_agent_message_item_when_turn_complete_omits_final_text` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `agent_turn_complete_notification_does_not_reuse_stale_copy_source` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `active_goal_without_follow_up_suppresses_agent_turn_complete_notification` | - | n/a | /goal events and paste elements: session-core (dispatch ported: core goal_slash_command_with_extra_os_emits_set_goal_event) |
| `queued_follow_up_suppresses_agent_turn_complete_notification` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `queued_menu_slash_keeps_agent_turn_complete_notification` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `slash_stop_submits_background_terminal_cleanup` | core/tests/commands.rs::slash_stop_submits_background_terminal_cleanup | ported |  |
| `slash_clear_requests_ui_clear_when_idle` | core/tests/commands.rs::slash_clear_requests_ui_clear_when_idle | ported |  |
| `slash_new_with_name_requests_named_session` | core/tests/commands.rs::slash_new_and_clear_with_name_request_a_named_session | ported | dispatch decision (typed intent); the session effect is session-core |
| `slash_clear_with_name_requests_named_session` | core/tests/commands.rs::slash_new_and_clear_with_name_request_a_named_session | ported | same |
| `slash_clear_after_ctrl_c_keeps_stashed_draft_recallable` | - | n/a | composer slot (stash) |
| `slash_clear_is_disabled_while_task_running` | core/tests/commands.rs::slash_clear_is_disabled_while_task_running | ported |  |
| `slash_archive_cancellation_keeps_task_running` | - | n/a | the dialog's cancel: select slot |
| `slash_memory_maintenance_reports_stubbed_feature` | core/tests/commands.rs::slash_memory_maintenance_reports_stubbed_feature | ported |  |
| `slash_mcp_requests_inventory_via_app_server` | core/tests/commands.rs::slash_mcp_requests_inventory_and_verbose_and_login | ported | dispatch decision |
| `slash_mcp_verbose_requests_full_inventory_via_app_server` | core/tests/commands.rs::slash_mcp_requests_inventory_and_verbose_and_login | ported | dispatch decision |
| `slash_mcp_login_targets_the_active_thread` | core/tests/commands.rs::slash_mcp_requests_inventory_and_verbose_and_login | ported | dispatch decision |
| `slash_mcp_invalid_args_show_usage` | core/tests/commands.rs::slash_mcp_invalid_args_show_usage | ported |  |
| `slash_memories_opens_memory_menu` | core/tests/commands.rs::slash_memories_opens_memory_menu | ported |  |
| `slash_resume_opens_picker_while_mcp_startup_is_running` | core/tests/commands.rs::slash_resume_opens_picker_while_mcp_startup_is_running | ported |  |
| `slash_import_opens_claude_code_import_picker` | core/tests/commands.rs::slash_import_opens_claude_code_import_picker | ported |  |
| `slash_archive_confirmation_requests_current_thread_archive` | core/tests/commands.rs::slash_archive_and_delete_open_the_confirmation_dialog | ported | opens the dialog (select slot); its confirm/RPC is that slot's |
| `slash_delete_confirmation_requests_current_thread_delete` | core/tests/commands.rs::slash_archive_and_delete_open_the_confirmation_dialog | ported | same |
| `slash_resume_with_arg_requests_named_session_while_mcp_startup_is_running` | core/tests/commands.rs::slash_resume_with_arg_requests_named_session | ported |  |
| `slash_pets_opens_picker` | core/tests/commands.rs::slash_pets_opens_picker | ported |  |
| `slash_pets_with_arg_selects_named_pet` | core/tests/commands.rs::slash_pets_opens_picker | ported | dispatch decision (args carried to the pets slice) |
| `slash_pet_disable_aliases_work_on_unsupported_terminal` | core/tests/commands.rs::pet_alias_parses_to_pets_command | n/a | alias parse ported; the terminal check is the pets slice |
| `slash_pets_in_tmux_shows_notice_and_preserves_draft` | - | n/a | pets slice: terminal capability notices |
| `slash_pets_with_arg_on_unsupported_terminal_shows_notice_without_selection` | - | n/a | pets slice |
| `slash_pets_on_unsupported_terminal_shows_terminal_warning` | - | n/a | pets slice |
| `slash_pets_on_old_iterm2_shows_upgrade_warning` | - | n/a | pets slice |
| `slash_fork_requests_current_fork` | core/tests/commands.rs::slash_fork_requests_current_fork_and_named_fork | ported | same |
| `slash_fork_with_name_requests_named_fork` | core/tests/commands.rs::slash_fork_requests_current_fork_and_named_fork | ported | same |
| `slash_app_requests_desktop_handoff` | core/tests/commands.rs::a_command_that_needs_the_session_is_a_typed_intent_never_a_silent_noop | ported | typed session intent |
| `slash_app_without_thread_id_shows_starting_error` | core/tests/commands.rs::slash_app_without_a_thread_shows_the_starting_error | ported |  |
| `slash_pwd_and_cwd_alias_display_current_working_directory_from_composer` | core/tests/commands.rs::slash_pwd_and_cwd_alias_display_current_working_directory; tests/commands.rs::usage_notices_of_the_oracle | ported | the cwd line itself is session-core |
| `slash_cd_changes_current_session_after_replay_and_defaults_to_home` | - | n/a | session effect (session-core) |
| `rejected_queued_cd_drains_following_input` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `slash_cd_rejects_pending_input_and_unsupported_session_ownership` | - | n/a | session effect (session-core) |
| `slash_rollout_displays_current_path` | core/tests/commands.rs::slash_rollout_displays_current_path | ported |  |
| `slash_rollout_handles_missing_path` | - | n/a | session effect (session-core) |
| `fast_slash_command_updates_and_persists_local_service_tier` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `fast_keybinding_toggle_uses_same_events_as_fast_slash_command` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `fast_keybinding_toggle_requires_feature_and_idle_surface` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `user_turn_carries_service_tier_after_fast_toggle` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `user_turn_preserves_flex_without_catalog_support` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `model_switch_recomputes_catalog_default_service_tier` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `queued_fast_slash_applies_before_next_queued_message` | - | n/a | queue effects: the drain rule is ported (core/tests/commands.rs::queued_commands_continue_or_stop_the_queue_as_the_oracle_says); the queue and its restore are session-core |
| `user_turn_sends_standard_override_after_fast_is_turned_off` | - | n/a | service tier in turn parameters: session-core (the popup rows of tiers are ported: core service_tier_command_uses_catalog_name_and_description) |
| `raw_slash_command_toggles_and_accepts_on_off_args` | core/tests/commands.rs::raw_slash_command_toggles_and_accepts_on_off_args | ported |  |
| `raw_slash_command_reports_usage_for_invalid_arg` | core/tests/commands.rs::raw_slash_command_reports_usage_for_invalid_arg | ported |  |
| `compact_queues_user_messages_snapshot` | - | n/a | queue snapshot: session-core |
| `transcript_copy_feedback_stays_in_the_footer_without_history_or_interrupts` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |
| `owned_copy_guard_defers_queued_input_until_released` | - | n/a | /copy picker and copy state: cells/transcript slice (`/copy` dispatches a typed intent) |

Tests of ours with no oracle twin: `core/tests/commands.rs` (registry order and
strings read from the oracle's source; every command names its target;
side-conversation and review gates; voice arguments), `core/tests/popovers.rs`
(triggers, candidates, history search), `tests/transitions.rs` (below),
`tests/commands.rs::{unrecognized..., a_path_is_a_message_not_a_command,
usage_notices_of_the_oracle, slash_mention_inserts_an_at_sign,
a_command_is_disabled_while_a_task_runs, every_description_is_the_oracles_text,
the_german_table_inherits_the_descriptions}`.

Counts: core `commands` 82 + `popovers` 9; UI `chat::` 44 (transitions 36,
commands 8) and `popovers::` 17. Whole crate: 293 tests, all green, run three
times in a row (no flake).

## Trigger table

Every screen-opening command and key opens its screen from the chat and Esc
returns with the transcript and the composer's text intact
(`tests/transitions.rs`, one test per row; the check is the target's slot text
or title, then `assert_chat_back`).

| trigger | target dir (id) | opener | test |
|---|---|---|---|
| `/model` `/permissions` `/review` `/archive` `/delete` (also `/presets`, `/auto-review`, `/daemon` by the same decision) | select (S14) | dialog | `slash_{model,permissions,review,archive,delete}_opens_...` |
| `/skills` `/config` `/theme` `/title` `/statusline` `/experimental` `/memories` `/keymap` (`/tui`) | settings (S15) | dialog | `slash_<name>_opens_the_settings` |
| `/agents` `/subagents` | agents (S21) | route | `slash_agents_...`, `slash_subagents_...` |
| `/resume` | resume (S18) | route | `slash_resume_opens_the_picker` |
| `/import` | import (S06) | route | `slash_import_opens_the_import` |
| `/diff`, Ctrl+T, F3 | pager (S16) | route | `slash_diff_...`, `ctrl_t_...`, `f3_...` |
| `/status` (also picked in the popup) | status (S19) | dialog | `slash_status_...`, `a_command_picked_in_the_popup_opens_its_screen` |
| `/usage`, `/usage weekly` | usage (S20) | route | `slash_usage_...`, `slash_usage_weekly_...` |
| `/warnings` `/feedback` `/rename`, F2 | dialogs (S23) | dialog | `slash_warnings_...`, `slash_feedback_...`, `slash_rename_...`, `f2_opens_the_warnings` |
| `/pets`, `/pet` | pets (S24) | route (part) | `slash_pets_...`, `slash_pet_alias_...` |
| F8 (`toggle_voice`) | voice (S25) | route (part) | `f8_toggles_voice_which_opens_the_voice_strip` |
| `/test-approval` | approvals (S13) | dialog | `slash_test_approval_...` |
| Esc Esc on an empty composer | backtrack (S17) | route (part) | `esc_esc_starts_backtrack_and_esc_leaves_it`, `esc_with_a_draft_does_not_start_backtrack` |
| `/quit` `/exit` | (leaves the app) | | `quit_and_exit_leave_the_app` |
| `/mention` | (inserts `@`) | | `slash_mention_inserts_an_at_sign` |
| Ctrl+R / Ctrl+S | history search (composer) | | `popovers::ctrl_r_and_ctrl_s_walk_older_and_newer_matches` |
| Ctrl+/ | notice "not available yet" (side conversation = session-core) | | core `side_conversation_and_review_gates` |

The default chords are `src/screens/chat/keys.rs::defaults` (the oracle's
`RuntimeKeymap::defaults`, `keymap.rs:1908-1953`); the keymap slice replaces
`defaults` with the configured keymap, the actions stay. Defaults with no
action here (owner in brackets): F4 `focus_activity`, Ctrl+G external editor,
Ctrl+O copy, Ctrl+L clear terminal, Alt+R raw output, F9 voice mute (session /
cells / keymap slices); Esc while a turn runs interrupts (session-core).

## Snapshots

The oracle's snapshots are read at test time from `tui/src/bottom_pane/
snapshots/`; the rows (text and order, after collapsing blanks and dropping the
selection and scrollbar markers) are compared with the rtwk view's screen in
the seeded debug entry `popovers/composer_popups` (80x24).

| snapshot | size | equal | delta |
|---|---|---|---|
| `command_popup_app` | 72x2 | rows | N1 (look) |
| `command_popup_voice` | | row | N1 |
| `command_popup_wrap_boundary...` (tiers `/tier-one`, `/tier-two`) | | rows | N1 |
| `skill_popup_category_tags` | 32x5 | rows at full width | N1 (the oracle hides the descriptions at 32 columns) |
| `default_unified_mention_popup` | 100x24 | the plugin row | N2 (picker v2 chrome) |
| `file_search_popup...long_paths_keep_distinguishing_filenames` | 80 | both rows | N1 |
| `chat_composer...footer_mode_history_search` | 100x9 | footer text and preview | N5 |

Not compared (missing, with the picker): `scrolling_mentions_shifts_rendered_window_snapshot`
and the `mentions_v2` frame snapshots (N2).

## Visual deltas

### N1 — composer popups: rtwk Autocomplete look
kind: NEW. cause: rtwk widget (`Autocomplete` popup).
oracle: `› /app      continue this session in the Desktop app` /
`  /approve  approve one retry of a recent auto-review denial`, in the bottom
pane, which grows; match characters highlighted; unavailable rows (side
conversation) shown greyed.
ours:
```
  ▎/app              continue this session in the Desktop app
   /approve  approve one retry of a recent auto-review denial
```
The popup floats above the composer over the transcript; `▎` marks the
selection; the detail column is right-aligned; no match highlight; rows that a
side conversation makes unavailable are left out. Tab completes the shared
prefix, else picks (the oracle completes the name and keeps the composer; ours
then runs the command). proposal: accept.

### N2 — `@` mentions picker v2 is a flat list
kind: NEW. cause: other (not built).
oracle:
```
  Mentions
   All Results   Filesystem Only   Plugins
  sa
› Sample Plugin   Plugin with skills and an MCP server                                        Plugin
  enter/tab insert · esc close · ↑/↓ select · ←/→ filter
```
ours:
```
▎Sample Plugin                    Plugin with skills and an MCP server Plugin
 Shared                                               [App] Secondary details
 src/shared/long_directory/parser_alpha.rs
```
No header, tabs, filter modes, footer hints or tier/preference/plugin rank;
files and plugins share one list ordered by fuzzy score. A picked plugin
inserts `@<insert> `, not the oracle's link form. proposal: follow-up task
(mentions picker), accept until then.

### N5 — history search: plain footer and preview
kind: NEW. cause: other.
oracle: footer `  reverse-i-search: c  enter accept · esc cancel` with the query
cyan, key hints as chips, the matches highlighted in the preview.
ours: the same words in one dim line, no highlight. Ctrl+C in search is not
caught (quits the app; oracle restores the draft, N6 in the table above).
proposal: accept.

### N7 — a command's notice is a line above the composer
kind: NEW. cause: other (cells not built).
The oracle adds an error cell to the transcript for `Unrecognized command`,
usage lines and "disabled while a task is in progress"; ours shows the same
text on a line above the composer until the next submit
(`Model::notice`, core). The words are the oracle's byte for byte (EN); the
line wraps at the width as the cell does. proposal: follow-up when the cells
slice lands (the notice becomes a history cell).

### N8 — Esc priming has no hint
kind: NEW. cause: other.
Esc on an empty composer primes backtrack; the oracle shows a footer hint
("Esc again to edit previous message"), ours shows nothing. proposal: accept.

### N9 — `/model` and the others open stub screens
kind: K01 (slot screens of other slices render a stub until they land). The
transitions are tested against the slots' text or title, so the tests keep
passing when the slices land (the slot text goes, the title stays).

## Scenarios (pty)

n/a until scenario-harness lands. The chat opens its screens from the `/`
popup and by chords; a first scenario would be: type `/status`, Enter, Esc.

## Rules

- R10: the decision is core `dispatch` (a pure function of the line and
  `DispatchCx`); the view only turns an `Outcome` into an opener, an intent or
  a notice. The queue rule is core (`queued_command_drain_result`).
- R14: `ChatState` holds ids and `esc_primed: bool`; the search state
  (query, draft, status) is core `popovers::Model::search`; the composer text
  is the thread's controller. `r14_*` rules green.
- ADR-010: descriptions, notices, usage lines, the search footer and the tags
  are in `src/screens/chat/strings.rs` (`description_en` is an exhaustive
  `match`, so a new command cannot go without text); `rules` test green; Ru
  falls back to En (`the_german_table_inherits_the_descriptions`).
- Layers: core files name no rtwk, UI, IO; `core_sources_hold_no_user_visible_prose` green.
- Popup data is core state seeded by a fixture script
  (`src/screens/popovers/fixtures/composer_popups.jsonl`) through each dir's
  `seed()`; no widget is given data.
- Lints: the new test files carry the `disallowed_methods` allow of the other
  test files (they read the oracle's source and snapshots).

## Bites

Tests added after the red commit (`3198be81e`): `core/tests/popovers.rs`.
- Break: a blank no longer closes the slash trigger
  (`composer.rs`: `&& !query.contains(char::is_whitespace)` -> `&& true`):
  `a_blank_after_the_command_name_closes_the_popup` fails. Restored.
- Break: the end of the matches reports `NoMatch` instead of keeping the match
  (`popovers.rs`): `history_search_shows_the_newest_match_and_walks_older_and_newer`
  fails. Restored.
The other tests were red in the first commit (the UI ones failed on the
`todo!()` description and the missing screens) and are green now.

## rtwk needs

- `Autocomplete::on_selected` is told a pick only after the text is replaced
  and later (a message): a command picked in the popup cannot be vetoed before
  the replacement, so the chat runs the composer's text as it stands after the
  pick (and ignores an empty one). A way to take a pick instead of inserting it
  (`on_pick` returning handled) would let the chat run the command without a
  replacement. Also: a per-row `disabled`, a match-highlight and a marker
  style (N1).
- `Text` takes a trailing blank off its intrinsic width: a prompt `"› "` laid
  out in a `Row` is one cell wide; the chat wraps it in a `SizedBox` of its
  character count.
- `Focus::bind(Esc)` takes the Esc of the route below: the chat pops itself
  (`Navigator::maybe_pop`) when it is a page over another.
- Effects of the shell (intents) answer on the runtime thread: a test of a
  notice waits (`wait_for`), it cannot pump the fake clock to it.

## Risks

- `/app` is in the popup on macOS and Windows only (the oracle's `cfg`); the
  test `command_popup_app` runs there only in the oracle's sense: on Linux the
  oracle's own snapshot test is cfg-gated too, ours is not (the row text is
  read from the snapshot, so it passes either way).
- The "target is shown" check of `transitions` reads the slot text or the title
  of the target; a slice that renames its title must keep `strings::title`.
- Intents from the chat (`Intent::Session`) end in a notice until session-core
  gives each command its effect: every `SessionIntent` is typed, none is a
  silent no-op (`a_command_that_needs_the_session_is_a_typed_intent_never_a_silent_noop`).
