# Parity pack: screens-dialogs (S13, S14, S19, S23 done in part; S15 core only; S24 done)

State: the core halves, fixture scripts, `strings.rs` and core tests of S15, S23
and S24 are done; the S23 and S24 views are built; the S15 views and every
oracle-row comparison that depends on the oracle's bottom-pane look are OPEN
(listed below, the tests are `#[ignore]`d with a reason, none is red). The
look moves to a shared bottom pane in the `dialogs-pane` slice; the views of
S13/S14/S23 are drawn in an rtwk `Dialog` until then.

Method of every row: the variant's fixture script (JSONL, the oracle's test
data) plays through core `apply` (`Sandbox::play` / the `screen` step); the rtwk
view renders the model at the oracle's terminal size; `dialogs::oracle::assert_rows`
(`codex-rs/tui-rtwk/tests/dialogs.rs`) compares the oracle `.snap` rows, in
order, trimmed, with the `›` marker kept and no width tuning.

## Open (ignored tests; the dialogs-pane slice un-ignores them)
| Test | Why open |
|---|---|
| `approvals::approval_modal_exec`, `approval_overlay_patch_destination_unavailable`, `permissions_prompt_snapshot`, `approval_form_tool_approval_snapshot`, `request_user_input_options_snapshot` | the oracle's rows (a pane without frame, `›` marker, 80-column wrap) differ from a `Dialog` render (N01, N02) |
| `select::snapshot_model_picker_width_80`, `approvals_selection_popup_snapshot` | same (N01, N02) |
| `dialogs::shortcut_overlay_draws_base_keyboard_and_falls_back_to_the_list`, `footer_mode_shortcut_overlay`, `feedback_selection_popup_snapshot` | same (N01) |
| S15 screen tests (text in git history: commit `82e21cc81`, `tests/settings.rs`): `config_panel_renders_each_group_at_a_stable_height`, `config_global_search`, `right_changes_the_group_and_esc_clears_the_search`, `enter_on_a_toggle_flips_it`, `setup_view_snapshot_uses_runtime_preview_values`, `space_toggles_the_item_and_the_preview_follows`, `model_theme_files_preview_select_and_restore`, `picker_content_snapshot` | the four settings views are not built in this slice (stub slot); a draft of a frameless version (5 of 8 of these passed) was left in the shared notes folder `settings-view-draft/`; the open points of the draft: Left/Right and Space must reach the page while a search `TextField` has the focus |

## Deltas
- N01 (REJECTED, no visual delta is accepted): codex-tui draws the dialogs as
  frameless bottom-pane panes with a `›` cursor marker; tui-rtwk draws S13/S14/S23 in a
  `Dialog` (frame, title, highlighted cursor row). Oracle (`list_selection_model_picker_width_80.snap`):
  ```
    Select Model and Effort
  › 1. gpt-5.1-codex (current)  Optimized for Codex. Balance of reasoning quality
                                and coding ability.
  ```
  Ours (Dialog, at 100 columns, over the debug menu):
  ```
   Select Model and Effort          (frame row, title in the frame)
  ▎1. gpt-5.1-codex (current)  Optimized for Codex. Balance of reasoning quality
                               and coding ability.
  ```
  Proposal: the `dialogs-pane` slice (a shared bottom pane drawn like the oracle's);
  the tests stay, ignored, until it lands.
- N02 (REJECTED): the oracle wraps at the pane width (80); a `Dialog` has an own inner width,
  so the earlier `max_width` 83/84 tuning is gone: wrap points differ. Oracle:
  `and coding ability.` under the description at column 30; ours wraps at the dialog's width.
  Proposal: as N01.
- N03 (NEW kind, S23 warnings): the oracle marks a warnings page visited when it becomes
  current (navigation); a draft that is never drawn (a page skipped past) counts as visited in
  the model too, as `warnings_keep_current_and_next_without_dismissing_unvisited_pages` needs.
  Oracle: visited on `show`; ours: visited on becoming current, not on being drawn.
- D1 (S13): the patch-approval snapshot is the variant where the request carries no changes
  (`Destination: unavailable`); a patch with changes (diff preview) has no required oracle row.
- D2: seeds are the oracle's test data, not protocol JSON, for S14, S19, S15, S23, S24 (`menu`,
  `card`, ... events); S13 seeds carry the protocol's own `ServerRequest` JSON.
- D3 (S23): the warnings footer's scroll hint and the width-adaptive trimming of the hints are
  not carried (the footer shows the full hint).
- D4 (S23): the feedback note view (free text) is not built; the category and consent popups are.
- D5 (S23): the consent popups compare through `assert_rows_except` (the chat's composer
  rows are not the dialog's).
- D6 (S15): theme picker: the style dump, the syntax highlighting of the preview, the wide
  preview, custom themes and the theme search ("Type to filter themes...") are not carried;
  the model keeps the theme in force for the preview.
- D7 (S23): the shortcut overlay's keyboard diagram is static.
- D8 (S15): the keymap catalogue is a 13-row fixture (the full 156 actions come with the
  `keymap` task); the tab descriptions and the `*` custom marker are not carried; its search is a
  plain all-words match in the catalogue's order (`KeymapPicker::visible`), not the oracle's fuzzy
  ranking.
- D9 (S24): the sprite preview image and the disabled row's preview text are not carried; pet
  descriptions are not shown (the oracle's picker shows names only as well).
- D10 (S15): the status-line setup filter is a subsequence match on the name, not the oracle's
  ranked fuzzy match.
- D11 (S15): the config panel's search keys use the English core constants (the oracle's
  `search_value` is English too); core `search_matches_displayed_values` uses the English value labels.
- D12 (S15): the sub-dialogs of the config panel (condition picker, vim, effects, recaps,
  notification states) are not opened; Enter on such a row records `Edit::Editor`.


## Done rows
### S13 approvals
| Variant (debug entry) | Script | Oracle snapshot | rtwk test |
|---|---|---|---|
| exec_command | `src/screens/approvals/fixtures/exec_command.jsonl` | `chatwidget/snapshots/...approval_modal_exec.snap` | `approvals::approval_modal_exec` (open) |
| patch_edits | `.../patch_edits.jsonl` | `...approval_overlay_patch_destination_unavailable.snap` | `approval_overlay_patch_destination_unavailable` (open) |
| permissions | `.../permissions.jsonl` | `...approval_overlay_permissions_prompt.snap` | `permissions_prompt_snapshot` (open) |
| mcp_elicitation | `.../mcp_elicitation.jsonl` | `bottom_pane/snapshots/...mcp_server_elicitation_approval_form_without_schema.snap` | `approval_form_tool_approval_snapshot` (open) |
| user_input | `.../user_input.jsonl` | `...request_user_input_options.snap` | `request_user_input_options_snapshot` (open) |
| exec_approval (skeleton) | `exec_approval.jsonl` | none (fallback) | `a_request_without_a_prompt_falls_back_to_the_dismiss_dialog` |

Core state tests: `core/tests/approvals.rs`.

### S14 selection
| Variant | Script | Oracle snapshot | rtwk test |
|---|---|---|---|
| model_picker | `src/screens/select/fixtures/model_picker.jsonl` | `bottom_pane/snapshots/...list_selection_model_picker_width_80.snap` | `select::snapshot_model_picker_width_80` (open) |
| permissions_menu | `.../permissions_menu.jsonl` | `chatwidget/snapshots/...approvals_selection_popup.snap` | `select::approvals_selection_popup_snapshot` (open) |

Own tests: `enter_picks_the_highlighted_row_and_closes`, `a_number_key_picks_its_row`,
`esc_closes_without_a_pick`; core: `core/tests/select.rs`.

### S19 status
| Variant | Script | Oracle snapshot | rtwk test |
|---|---|---|---|
| reasoning_details | `src/screens/status/fixtures/reasoning_details.jsonl` | `status/snapshots/...status_snapshot_includes_reasoning_details.snap` | `status::status_snapshot_includes_reasoning_details` |

### S23 small dialogs
Scripts: `src/screens/dialogs/fixtures/{shortcut_overlay,warnings,warnings_long,feedback_categories,feedback_consent,feedback_consent_windows}.jsonl`;
views in a `Dialog` (`src/screens/dialogs/`); tests `tests/dialogs.rs`, core `core/tests/dialogs.rs`
(the oracle names of the table below). The rows that compare against the oracle's look are open.

### S15 settings (core only)
Scripts: `src/screens/settings/fixtures/{config_panel,theme_picker,status_line,keymap}.jsonl`; core
`core/src/screens/settings.rs` (panel, theme picker, status line setup, keymap picker; intents Tab, Move,
Search, Activate, Toggle, Reorder, Close); `strings.rs` with the oracle's text (EN); state tests
`core/tests/settings.rs` (19, named after the oracle's: `config_panel_renders_each_group_at_a_stable_height`,
`search_finds_settings_across_groups_and_preserves_the_selected_group`, `setup_view_snapshot_uses_runtime_preview_values`,
`picker_content_snapshot` compares against the oracle `.snap` text, ...). Views: open.

### S24 pets
Scripts: `src/screens/pets/fixtures/{picker,picker_custom,picker_disabled}.jsonl`; view `src/screens/pets/`
(a frameless page: title, subtitle, search, list with the `›` row, preview status, footer; the oracle snapshot
rows match row for row: `pets::slash_pets_picker`); tests `tests/pets.rs` (`the_disabled_row_is_marked_current`,
`typing_filters_the_list`, `enter_picks_the_pet_and_closes`, `the_arrows_move_the_cursor_with_the_search_field_focused`,
`esc_clears_the_search_then_leaves`); core `core/tests/pets.rs` (`picker_lists_app_bundled_and_custom_pets`,
`picker_preselects_codex_without_marking_it_current_when_no_pet_is_configured`, `picker_marks_disabled_pet_as_current`,
`picker_content_snapshot`).

## Oracle tests of the files of this task (every test: ported by name, or the reason)
| Slice | Oracle file | Tests | Ported (by name) | Not ported |
|---|---|---|---|---|
| S14 | `bottom_pane/list_selection_view.rs` | 42 | `snapshot_model_picker_width_80` (tests/select.rs) | 41: The list widget's own behaviours (search, scrolling, side content, footer notes, narrow layouts) beyond the two required parameterisations; the view is an rtwk list. |
| S19 | `status/tests.rs` | 42 | `status_snapshot_includes_reasoning_details` (tests/status.rs) | 41: The `/status` card on other configurations (rate limits, credits, collaboration modes, forks, errors): the required variant is `reasoning_details`; the card model carries one fixture. |
| S13 | `bottom_pane/approval_overlay.rs` | 37 | `configured_list_cancel_aborts_exec_approval` (tests/approvals.rs), `shortcut_triggers_selection` (tests/approvals.rs), `deny_shortcut_submits_denied_exec_decision` (tests/approvals.rs), `exec_prefix_option_emits_execpolicy_amendment` (core/tests/approvals.rs), `generic_exec_options_can_offer_allow_for_session` (core/tests/approvals.rs), `permissions_prompt_snapshot` (tests/approvals.rs) | 31: Key-routing and decision tests of the exec/patch/permissions/network overlays; the required variants and the decision defaults are ported, the rest need the app-server answer command (handover). |
| S13 | `bottom_pane/request_user_input/mod.rs` | 76 | `request_user_input_options_snapshot` (tests/approvals.rs) | 75: The question form's composer, notes, multi-question and wrap tests; the options snapshot is the required variant. |
| S13 | `bottom_pane/mcp_server_elicitation.rs` | 23 | `approval_form_tool_approval_snapshot` (tests/approvals.rs) | 22: The elicitation form's schema, field and parsing tests; the approval form without a schema is the required variant. |
| S23 | `bottom_pane/shortcut_overlay_tests.rs` | 6 | `shortcut_overlay_draws_base_keyboard_and_falls_back_to_the_list` (tests/dialogs.rs) | 5: Footer-mode tests of the composer's footer (the composer is S10). |
| S23 | `bottom_pane/warnings_view_tests.rs` | 6 | `warning_pages_copy_only_the_current_diagnostic` (core/tests/dialogs.rs), `warning_pages_wrap_and_scroll_long_diagnostics` (tests/dialogs.rs), `warnings_keep_current_and_next_without_dismissing_unvisited_pages` (core/tests/dialogs.rs) | 3: Footer scroll hint and width-adaptive hint trimming (delta D3). |
| S23 | `bottom_pane/feedback_view.rs` | 8 | `feedback_upload_consent_lists_doctor_report` (tests/dialogs.rs), `feedback_upload_consent_lists_windows_sandbox_log_when_included` (tests/dialogs.rs) | 6: The note view (free-text feedback) is not built (delta D4); the category and consent popups are. |
| S15 | `config_panel/config_panel_tests.rs` | 15 | `config_panel_renders_each_group_at_a_stable_height` (core/tests/settings.rs), `search_finds_settings_across_groups_and_preserves_the_selected_group` (core/tests/settings.rs), `search_matches_displayed_values` (core/tests/settings.rs) | 12: Sub-dialogs (condition picker, vim, effects, recaps, notification states) are a later transition; core records `Edit::Editor`. |
| S15 | `theme_picker.rs` | 9 | - | 9: Style dump, syntax highlighting of the preview, wide preview and custom themes (delta D6). |
| S15 | `theme_picker_model_tests.rs` | 1 | `model_theme_files_preview_select_and_restore` (core/tests/settings.rs) | 0: all ported or own |
| S15 | `bottom_pane/status_line_setup.rs` | 16 | `preview_uses_placeholders_when_runtime_values_are_missing` (core/tests/settings.rs), `picker_previews_the_pr_link_and_preserves_the_color_preference` (core/tests/settings.rs), `setup_view_snapshot_uses_runtime_preview_values` (core/tests/settings.rs) | 13: Catalogue, parsing and persistence tests of the status-line items (the app/config layer), fuzzy ranking. |
| S15 | `keymap_setup/picker_tests.rs` | 1 | - | 1: Needs the full 156-action catalogue (the keymap task); the fixture has 13 rows. |
| S15 | `keymap_setup/menu_tests.rs` | 2 | - | 2: Action menu and conflict resolution: the keymap task. |
| S15 | `keymap_setup/capture_tests.rs` | 5 | - | 5: Key capture: the keymap task. |
| S15 | `keymap_setup.rs` | 50 | `picker_content_snapshot` (core/tests/settings.rs) | 49: Binding edit, conflict and config-writing tests: the keymap task. |
| S24 | `pets/picker.rs` | 4 | `picker_lists_app_bundled_and_custom_pets` (core/tests/pets.rs), `picker_preselects_codex_without_marking_it_current_when_no_pet_is_configured` (core/tests/pets.rs), `picker_marks_disabled_pet_as_current` (core/tests/pets.rs) | 1: all ported or own |

### Names of the oracle tests not ported, by file
- `bottom_pane/list_selection_view.rs`: secondary_action_respects_search_disabled_rows_and_dismissal, searchable_list_honors_configured_printable_accept_alternative, renders_blank_line_between_title_and_items_without_subtitle, renders_blank_line_between_subtitle_and_items, theme_picker_subtitle_uses_fallback_text_in_94x35_terminal, theme_picker_enables_side_content_background_preservation, preserve_side_content_bg_keeps_rendered_background_colors, snapshot_footer_note_wraps, renders_search_query_line_when_enabled, empty_searchable_list_keeps_search_and_no_matches_visible, paste_appends_to_search_query_and_filters_items, whitespace_only_paste_is_ignored, switching_tabs_changes_visible_items_and_clears_search, tabbed_view_preserves_current_row_on_initial_selection_and_tab_switch, space_appends_to_active_search_instead_of_toggling_selected_item, configured_list_binding_takes_precedence_over_search_toggle_key, configured_cancel_binding_takes_precedence_over_space_toggle, single_line_row_display_truncates_instead_of_wrapping, name_column_width_override_moves_description_column_right, enter_with_no_matches_triggers_cancel_callback, move_down_without_selection_change_does_not_fire_callback, disabled_current_rows_skip_default_selection_and_number_shortcuts, automatic_shortcuts_stop_at_nine, c0_ctrl_p_respects_unbound_list_move_up, c0_ctrl_n_respects_unbound_list_move_down, c0_ctrl_p_respects_remapped_list_move_down, page_and_jump_navigation_use_list_keymap, page_and_jump_navigation_skip_trailing_disabled_rows_without_wrapping, wraps_long_option_without_overflowing_columns, width_changes_do_not_hide_rows, narrow_width_keeps_all_rows_visible, snapshot_narrow_width_preserves_third_option, snapshot_narrow_width_counts_halfwidth_sound_marks, snapshot_auto_visible_col_width_mode_scroll_behavior, snapshot_auto_all_rows_col_width_mode_scroll_behavior, auto_all_rows_col_width_does_not_shift_when_scrolling, side_layout_width_half_uses_exact_split, side_layout_width_half_falls_back_when_list_would_be_too_narrow, stacked_side_content_is_used_when_side_by_side_does_not_fit, side_content_clearing_resets_symbols_and_style, side_content_clearing_handles_non_zero_buffer_origin
- `status/tests.rs`: stale_monthly_limit_marks_fresh_rolling_snapshot_stale, status_snapshot_shows_chatgpt_plan_without_email, status_permissions_non_default_workspace_write_uses_workspace_label, status_permissions_named_read_only_profile_shows_builtin_label, status_permissions_read_only_profile_shows_additional_writable_roots, status_permissions_named_workspace_profile_shows_builtin_label, status_permissions_workspace_auto_review_shows_reviewer_label, status_permissions_named_profile_shows_additional_writable_roots, status_permissions_workspace_roots_show_additional_directories, status_permissions_workspace_roots_include_profile_defined_directories, status_permissions_broadened_workspace_profile_shows_builtin_label, status_permissions_user_defined_profile_shows_name, status_snapshot_shows_active_user_defined_profile, status_uses_server_provider_id_and_auth_requirement, status_snapshot_shows_auto_review_permissions, status_permissions_full_disk_managed_with_network_is_danger_full_access, status_permissions_full_disk_managed_without_network_is_external_sandbox, status_snapshot_includes_forked_from, status_snapshot_includes_monthly_limit, status_snapshot_includes_enterprise_monthly_credit_limit, status_snapshot_uses_generic_limit_labels_for_unsupported_windows, status_snapshot_shows_unlimited_credits, status_snapshot_shows_positive_credits, status_snapshot_shows_available_credits_without_display_balance, status_snapshot_respects_unlimited_without_has_credits_flag, status_card_token_usage_excludes_cached_tokens, status_wraps_long_paths_and_session_ids_without_losing_text, status_snapshot_wraps_in_narrow_terminal, status_snapshot_wraps_halfwidth_kana_in_narrow_terminal, status_snapshot_shows_missing_limits_message, status_snapshot_uses_default_reasoning_when_config_empty, status_snapshot_shows_refreshing_limits_notice, transcript_overlay_remeasures_status_after_rate_limit_refresh, status_snapshot_includes_credits_and_limits, status_snapshot_shows_unavailable_limits_message, status_snapshot_treats_refreshing_empty_limits_as_unavailable, status_snapshot_shows_stale_limits_message, status_snapshot_cached_limits_hide_credits_without_flag, status_context_window_uses_last_usage, status_permissions_include_executor_profile_root, reset_timestamps_follow_clock_preference
- `bottom_pane/approval_overlay.rs`: approval_shortcuts_display_chords_in_configured_order, ctrl_c_aborts_and_clears_queue, configured_list_cancel_cancels_mcp_elicitation, approval_headers_preserve_wrapped_url_destinations, network_deny_shortcut_submits_policy_deny_decision, resolved_request_dismisses_overlay_without_emitting_abort, o_opens_source_thread_for_cross_thread_approval, configured_open_thread_shortcut_opens_source_thread, cross_thread_footer_hint_mentions_o_shortcut, network_deny_forever_shortcut_is_not_bound, header_includes_command_snippet, network_exec_options_use_expected_labels_and_hide_execpolicy_amendment, additional_permissions_exec_options_hide_execpolicy_amendment, permissions_options_use_expected_labels, additional_permissions_rule_shows_non_path_file_system_entries, additional_permissions_rule_uses_workspace_roots_label, permissions_session_shortcut_submits_session_scope, permissions_deny_shortcut_uses_deny_keymap, permissions_strict_auto_review_shortcut_submits_turn_scope_with_strict_review, additional_permissions_prompt_shows_permission_rule_line, additional_permissions_prompt_snapshot, apply_patch_prompt_with_thread_label_omits_command_line, apply_patch_prompt_without_changes_shows_unavailable_destination, network_exec_prompt_title_includes_host, ctrl_shift_a_opens_fullscreen, exec_history_cell_wraps_with_two_space_indent, exec_history_cell_does_not_render_blank_action_for_empty_command, network_access_command_history_uses_target_without_structured_context, esc_cancels_mcp_elicitation, esc_still_cancels_elicitation_with_custom_overlap, enter_sets_last_selected_index_without_dismissing
- `bottom_pane/request_user_input/mod.rs`: queued_requests_are_fifo, interrupt_discards_queued_requests_and_emits_interrupt, auto_resolution_absent_has_no_timer, auto_resolution_uses_is_blocking_without_auto_resolution_ms, auto_resolution_hides_timer_during_grace_period, auto_resolution_visible_countdown_snapshot, auto_resolution_visible_countdown_is_red, auto_resolution_expiry_emits_empty_answer, auto_resolution_key_interaction_snoozes_timer, auto_resolution_paste_interaction_snoozes_timer, auto_resolution_resets_for_queued_request, resolved_request_dismisses_overlay_without_emitting_events, resolved_current_request_advances_to_next_same_turn_prompt, resolved_queued_request_removes_only_that_prompt, options_can_submit_empty_when_unanswered, enter_commits_default_selection_on_last_option_question, enter_commits_default_selection_on_non_last_option_question, number_keys_select_and_submit_options, vim_keys_move_option_selection, typing_in_options_does_not_open_notes, h_l_move_between_questions_in_options, left_right_move_between_questions_in_options, horizontal_list_keys_move_between_questions_in_options, options_notes_focus_hides_question_navigation_tip, freeform_shows_ctrl_p_and_ctrl_n_question_navigation_tip, freeform_footer_shows_configured_submit_binding, freeform_footer_displays_configured_chords_without_internal_dispatch_keys, request_user_input_uses_remapped_interrupt_binding_while_notes_are_visible, tab_opens_notes_when_option_selected, switching_to_options_resets_notes_focus_when_notes_hidden, switching_from_freeform_with_text_resets_focus_and_keeps_last_option_empty, esc_in_notes_mode_without_options_interrupts, esc_in_options_mode_interrupts, esc_in_notes_mode_clears_notes_and_hides_ui, esc_in_notes_mode_with_text_clears_notes_and_hides_ui, esc_drops_committed_answers, backspace_in_options_clears_selection, backspace_on_empty_notes_closes_notes_ui, tab_in_notes_clears_notes_and_hides_ui, skipped_option_questions_count_as_unanswered, highlighted_option_questions_are_unanswered, freeform_requires_enter_with_text_to_mark_answered, freeform_enter_with_empty_text_is_unanswered, freeform_shift_enter_inserts_newline_without_advancing, freeform_uses_configured_composer_submit_binding, freeform_submit_binding_wins_over_question_navigation, freeform_questions_submit_empty_when_empty, freeform_draft_is_not_submitted_without_enter, freeform_commit_resets_when_draft_changes, notes_are_captured_for_selected_option, notes_submission_commits_selected_option, tab_and_enter_open_notes_for_other_option, is_other_adds_none_of_the_above_and_submits_it, large_paste_is_preserved_when_switching_questions, pending_paste_placeholder_survives_submission_and_back_navigation, request_user_input_options_notes_visible_snapshot, request_user_input_tight_height_snapshot, layout_allocates_all_wrapped_options_when_space_allows, desired_height_keeps_spacers_and_preferred_options_visible, footer_wraps_tips_without_splitting_individual_tips, request_user_input_wrapped_options_snapshot, request_user_input_long_option_text_snapshot, selected_long_wrapped_option_stays_visible, request_user_input_footer_wrap_snapshot, request_user_input_scroll_options_snapshot, request_user_input_hidden_options_footer_snapshot, wrapped_question_preserves_the_complete_url_destination, request_user_input_freeform_snapshot, request_user_input_freeform_chord_only_submit_snapshot, request_user_input_freeform_remapped_submit_snapshot, request_user_input_freeform_remapped_interrupt_snapshot, request_user_input_multi_question_first_snapshot, request_user_input_multi_question_last_snapshot, request_user_input_unanswered_confirmation_snapshot, options_scroll_while_editing_notes
- `bottom_pane/mcp_server_elicitation.rs`: parses_boolean_form_request, unsupported_numeric_form_falls_back, empty_object_schema_uses_approval_actions, empty_tool_approval_schema_uses_approval_actions, tool_suggestion_meta_is_parsed_into_request_payload, plugin_tool_suggestion_meta_without_install_url_is_parsed_into_request_payload, tool_approval_display_params_prefer_explicit_display_order, submit_sends_accept_with_typed_content, horizontal_list_keys_move_between_select_fields, switching_fields_clears_length_validation_flash, text_fields_inherit_composer_chord_context_and_submit_binding, empty_tool_approval_schema_session_choice_sets_persist_meta, empty_tool_approval_schema_always_allow_sets_persist_meta, ctrl_c_cancels_elicitation, queues_requests_fifo, resolved_request_dismisses_overlay_without_emitting_events, form_prompt_preserves_the_complete_wrapped_url_destination, boolean_form_snapshot, message_only_form_snapshot, message_only_form_with_persist_options_snapshot, approval_form_tool_approval_with_persist_options_snapshot, approval_form_tool_approval_with_param_summary_snapshot
- `bottom_pane/shortcut_overlay_tests.rs`: shortcut_overlay_uses_runtime_bindings_and_wsl_paste, shortcut_overlay_leaves_remapped_tab_and_function_keys_available, shortcut_keyboard_adapts_to_probed_and_unknown_terminal_colors, shortcut_keyboard_keeps_diagram_for_keys_outside_us_layout, shortcut_overlay_clipping_preserves_customization_in_small_areas
- `bottom_pane/warnings_view_tests.rs`: scroll_hint_tracks_wheel_and_keys_across_wrapped_rows, warnings_dismiss_only_drawn_pages_after_navigation_skips_a_frame, warning_links_preserve_destinations_when_wrapped_and_scrolled
- `bottom_pane/feedback_view.rs`: should_show_feedback_connectivity_details_only_for_non_good_result_with_diagnostics, issue_url_available_for_bug_bad_result_safety_check_and_other, feedback_success_cell_matches_external_bug_copy, feedback_success_cell_matches_employee_bug_copy, feedback_success_cell_matches_good_result_copy, feedback_success_cell_uses_issue_links_for_remaining_categories
- `config_panel/config_panel_tests.rs`: effects_picker_groups_animation_settings, deselect_after_copy_requires_effective_copy_on_select, notification_controls_render_dependency_states_and_condition_picker, on_demand_search_preserves_printable_list_bindings, session_picker_layout_uses_editor_handoff_and_saves_choice, mouse_scroll_speed_picker_preserves_a_custom_value_and_saves_a_preset, copy_on_select_uses_editor_handoff_and_saves_choice, unavailable_right_click_paste_remains_editable, resume_directory_disables_current_without_remote_cwd_override, refresh_preserves_search_and_updates_visible_state, config_panel_wraps_complete_save_error, editor_handoff_blocks_input_until_acknowledged
- `theme_picker.rs`: theme_picker_uses_half_width_with_stacked_fallback_preview, theme_picker_items_include_search_values_for_preview_mapping, wide_preview_renders_all_lines_with_vertical_center_and_left_inset, narrow_preview_renders_single_add_and_single_remove_in_four_lines, subtitle_uses_tilde_path_when_codex_home_under_home_directory, subtitle_falls_back_when_tilde_path_subtitle_is_too_wide, subtitle_falls_back_to_preview_instructions_without_tilde_path, subtitle_falls_back_for_94_column_terminal_side_by_side_layout, unavailable_configured_theme_falls_back_to_configured_or_default_selection
- `bottom_pane/status_line_setup.rs`: context_used_accepts_context_usage_legacy_id, context_remaining_is_selectable_id, thread_usage_items_are_independently_selectable, project_name_is_canonical_and_accepts_legacy_ids, model_is_canonical_and_accepts_model_name_legacy_id, reasoning_is_selectable_id, run_state_is_canonical_and_accepts_status_legacy_id, git_summary_items_are_selectable_ids, parse_status_line_items_accepts_title_only_variants, preview_uses_runtime_values, preview_includes_thread_title, setup_view_snapshot_includes_thread_usage_items, setup_view_snapshot_includes_hostname
- `keymap_setup/picker_tests.rs`: tabs_and_search_keep_production_picker_geometry_stable
- `keymap_setup/menu_tests.rs`: child_menu_hints_and_actions_follow_configured_list_bindings, disabled_reason_wraps_in_the_description_column
- `keymap_setup/capture_tests.rs`: chord_capture_instruction_snapshots, chord_capture_instructions_wrap_to_narrow_panes, captures_repeated_strokes_but_ignores_auto_repeat, modified_escape_is_captured_as_a_chord_stroke, replacing_one_list_binding_with_chord_preserves_other_alternatives
- `keymap_setup.rs`: picker_covers_every_replaceable_action, picker_hides_fast_mode_action_when_feature_is_disabled, picker_shows_fast_mode_action_when_feature_is_enabled, keymap_picker_fast_mode_enabled_snapshot, picker_common_tab_lists_curated_actions, picker_approval_tab_lists_all_approval_actions, picker_customized_tab_contains_root_overrides, picker_unbound_tab_lists_default_unbound_actions, picker_debug_tab_is_last_and_opens_inspector, picker_selected_action_starts_on_matching_all_tab_row, picker_repeat_and_redo_render_snapshots, picker_voice_toggle_snapshot, picker_question_actions_snapshot, picker_all_tab_items_remain_searchable, picker_wide_render_snapshot, picker_narrow_render_snapshot, picker_editor_render_snapshot, picker_custom_render_snapshot, picker_global_chord_render_snapshot, configured_global_chord_alternatives_remain_in_order, picker_narrow_uses_compact_tabs, action_menu_content_snapshot, action_menu_responsive_render_snapshot, action_menu_disables_clear_when_action_has_no_custom_binding, capture_view_snapshot, debug_view_initial_snapshot, debug_view_shows_delayed_missing_key_hint, debug_view_reports_detected_key_and_matching_actions, debug_view_uses_custom_binding_source, debug_view_labels_custom_global_fallback_source, capture_completion_returns_to_selected_keymap_picker_row, clear_completion_returns_to_selected_keymap_picker_row, replace_one_completion_drops_focused_keymap_submenus, key_capture_serializes_modifier_order_for_config, key_capture_serializes_special_keys, key_capture_serializes_function_keys_through_f24, key_capture_serializes_c0_control_chars_as_ctrl_bindings, key_capture_serializes_minus_as_named_key, replacement_sets_single_binding, replace_all_collapses_multi_binding_to_single, add_alternate_grows_single_binding, add_alternate_grows_default_multi_binding, add_alternate_preserves_super_default_binding, add_alternate_duplicate_is_noop, replace_one_preserves_other_bindings, replace_one_deduplicates_replacement, replace_one_rejects_stale_old_key, clear_removes_custom_binding, replacement_rejects_unknown_action
- `pets/picker.rs`: picker_imports_legacy_avatar_manifests

## Rules
- R14 (widget `State` holds no application data): the dialogs read the model in `build` through a
  provider projection and keep only ids, a shown flag and UI positions (`tests/rules.rs`:
  `r14_widget_state_fields_hold_no_screen_model_types` scans the State fields for the screen model
  types; `r14_the_screen_scan_catches_a_copied_menu` shows it biting: a copied `Menu` is reported).
  The earlier copying of the dialog views (batch-1 G5) is removed (commit `88ef48c94`).
- ADR-002 (core without rtwk): `core/src/screens/{dialogs,settings,pets}.rs` import no rtwk crate; a screen's
  state is changed only by `apply`/`apply_intent`; UI-only state (windows, field keys) lives in the views.
- ADR-010: user-visible text only in `strings.rs` (EN = the oracle's text; `Msg<N>` with a `Fmt` method for interpolation).

## Scenarios (pty)
n/a until `scenario-harness`.

## Handover
- Warnings decisions, copy effects and the feedback note live only in the model (`decided`, `copied`,
  `note`); the real triggers `/warnings`, `/feedback`, `/rename`, F2 open the unseeded dialog because
  `dialogs::open` has no seed.
- App and core `chat` must open the settings and pets pickers with seeded models (`settings::open`,
  the pets page read `ScreenStates`).
- `tests/skeleton.rs` was left unchanged: the unseeded pets page keeps the skeleton's empty-slot text.
- `PendingRequest` holds no params, so approvals has its own `Ev::Presented` plus a `screen` seed step with the request JSON;
  `apply_app` (app.rs) should forward server requests to approvals, and a real answer command is needed.
- rtwk: `Dialog` has no frameless / bottom-pane mode (N01, N02); the `dialogs-pane` slice un-ignores the open tests.
- `core/src/screens/settings.rs`: `ActionItem::is_unbound`, `KeymapPicker::{query, visible}` (search of the keymap picker).
