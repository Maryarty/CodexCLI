# select-dialog parity

Slice S14: the one app-level selection sheet (oracle `bottom_pane/list_selection_view.rs`
`ListSelectionView` / `SelectionViewParams`). Code: `core/src/screens/select.rs` and
`select/list.rs` (pure model and rules), `src/sheets/select/` (`dialog.rs` the sheet,
`rows.rs` the rows grid, `side.rs` the preview, `lists.rs` Menu to List, `strings.rs`
words, `fixtures/*.jsonl` mock data through core `apply`).

## The sheet as later dialogs call it

* `Menu::List(Box<List>)` in core (`core::screens::select::list::List`): title, subtitle,
  footer note and hint, items (id, name, description, selected description, current,
  disabled and reason, child label, shortcut, search value, toggle, footer hint, secondary
  action), tabs, search (placeholder), numbered, column mode (`AutoVisible`,
  `AutoAllRows`), single-line rows, name column width, side preview (beside or stacked),
  initial row, max rows. A dialog builds a `List` from its own state (words from its own
  strings.rs) and opens the menu through `ScreenIntent::Select`/`SeedEv::List`.
* A pick returns an id: `Picked::Item(id)` (`Picked::ItemSecondary(id)` for the secondary
  key); the sheet keeps no copy of app state (ADR-003) and is a `StatefulWidget` that
  reads the open menu from the screen state, owns only the cursor, window, tab, query
  controller and focus node.
* Mouse: a tap on a row picks it, on a tab switches to it. Keys: up/down, j/k (not while searching), ctrl-p/ctrl-n,
  page, home/end, number and row shortcuts, left/right (tabs), space/enter, esc, search
  typing and paste (words joined by one blank).
* The simple parameterisations that are only a `List`: the model picker and permissions
  menu (`Menu::Models`, `Menu::Permissions`) and fixtures with the oracle's snapshots for
  the skills menu, copy picker, apps (loading), the three worktrees menus
  (`tests/sheets/select.rs`). The oracle's review, TUI mode, plugins, goal and subagents
  menus are built by their own dialogs from live data: they need only a `List`.

## Oracle tests (`tests/oracle/picker.rs`, profile `oracle`)

63 tests: 24 green, 39 pending, 0 red. Green ones render the sheet at the oracle's size
through the replay helper (`tests/oracle_snap`); expected text is the oracle's snapshot.

| oracle (file::name) | ours | status | why pending | owner |
|---|---|---|---|---|
| bottom_pane/list_selection_view.rs::secondary_action_respects_search_disabled_rows_and_dismissal | `tests/oracle/picker.rs::secondary_action_respects_search_disabled_rows_and_dismissal` | green | |  |
| bottom_pane/list_selection_view.rs::searchable_list_honors_configured_printable_accept_alternative | `tests/oracle/picker.rs::searchable_list_honors_configured_printable_accept_alternative` | pending | no keymap seed: the port reads no key bindings from a config yet (the keymap slice), so the configured accept `z` cannot be set | keymap slice (bindings from a config) |
| bottom_pane/list_selection_view.rs::renders_blank_line_between_title_and_items_without_subtitle | `tests/oracle/picker.rs::renders_blank_line_between_title_and_items_without_subtitle` | green | |  |
| bottom_pane/list_selection_view.rs::renders_blank_line_between_subtitle_and_items | `tests/oracle/picker.rs::renders_blank_line_between_subtitle_and_items` | green | |  |
| bottom_pane/list_selection_view.rs::theme_picker_subtitle_uses_fallback_text_in_94x35_terminal | `tests/oracle/picker.rs::theme_picker_subtitle_uses_fallback_text_in_94x35_terminal` | pending | no theme picker entry: the port has no theme picker (build_theme_picker_params), so the fallback subtitle is not drawn | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/list_selection_view.rs::preserve_side_content_bg_keeps_rendered_background_colors | `tests/oracle/picker.rs::preserve_side_content_bg_keeps_rendered_background_colors` | pending | the preview keeps the dialog surface; no side-content background colour (a blue-background marker) is carried into the sheet yet | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/list_selection_view.rs::snapshot_footer_note_wraps | `tests/oracle/picker.rs::snapshot_footer_note_wraps` | green | |  |
| bottom_pane/list_selection_view.rs::renders_search_query_line_when_enabled | `tests/oracle/picker.rs::renders_search_query_line_when_enabled` | green | |  |
| bottom_pane/list_selection_view.rs::empty_searchable_list_keeps_search_and_no_matches_visible | `tests/oracle/picker.rs::empty_searchable_list_keeps_search_and_no_matches_visible` | green | |  |
| bottom_pane/list_selection_view.rs::paste_appends_to_search_query_and_filters_items | `tests/oracle/picker.rs::paste_appends_to_search_query_and_filters_items` | green | |  |
| bottom_pane/list_selection_view.rs::whitespace_only_paste_is_ignored | `tests/oracle/picker.rs::whitespace_only_paste_is_ignored` | green | |  |
| bottom_pane/list_selection_view.rs::switching_tabs_changes_visible_items_and_clears_search | `tests/oracle/picker.rs::switching_tabs_changes_visible_items_and_clears_search` | green | |  |
| bottom_pane/list_selection_view.rs::tabbed_view_preserves_current_row_on_initial_selection_and_tab_switch | `tests/oracle/picker.rs::tabbed_view_preserves_current_row_on_initial_selection_and_tab_switch` | green | |  |
| bottom_pane/list_selection_view.rs::space_appends_to_active_search_instead_of_toggling_selected_item | `tests/oracle/picker.rs::space_appends_to_active_search_instead_of_toggling_selected_item` | green | |  |
| bottom_pane/list_selection_view.rs::configured_list_binding_takes_precedence_over_search_toggle_key | `tests/oracle/picker.rs::configured_list_binding_takes_precedence_over_search_toggle_key` | pending | no keymap seed: list.move_down = tab cannot be set (the keymap slice); no entry with a toggle row and `Next` | keymap slice (bindings from a config) |
| bottom_pane/list_selection_view.rs::configured_cancel_binding_takes_precedence_over_space_toggle | `tests/oracle/picker.rs::configured_cancel_binding_takes_precedence_over_space_toggle` | pending | no keymap seed: list.cancel = space cannot be set (the keymap slice); no entry with a checkbox toggle `Animations` | keymap slice (bindings from a config) |
| bottom_pane/list_selection_view.rs::single_line_row_display_truncates_instead_of_wrapping | `tests/oracle/picker.rs::single_line_row_display_truncates_instead_of_wrapping` | green | |  |
| bottom_pane/list_selection_view.rs::name_column_width_override_moves_description_column_right | `tests/oracle/picker.rs::name_column_width_override_moves_description_column_right` | green | |  |
| bottom_pane/list_selection_view.rs::enter_with_no_matches_triggers_cancel_callback | `tests/oracle/picker.rs::enter_with_no_matches_triggers_cancel_callback` | green | |  |
| bottom_pane/list_selection_view.rs::move_down_without_selection_change_does_not_fire_callback | `tests/oracle/picker.rs::move_down_without_selection_change_does_not_fire_callback` | green | |  |
| bottom_pane/list_selection_view.rs::disabled_current_rows_skip_default_selection_and_number_shortcuts | `tests/oracle/picker.rs::disabled_current_rows_skip_default_selection_and_number_shortcuts` | green | |  |
| bottom_pane/list_selection_view.rs::c0_ctrl_p_respects_unbound_list_move_up | `tests/oracle/picker.rs::c0_ctrl_p_respects_unbound_list_move_up` | pending | no keymap seed: list.move_up unbound cannot be set (the keymap slice); no entry with a searchable two-item list (`First`, `Second`) | keymap slice (bindings from a config) |
| bottom_pane/list_selection_view.rs::c0_ctrl_n_respects_unbound_list_move_down | `tests/oracle/picker.rs::c0_ctrl_n_respects_unbound_list_move_down` | pending | no keymap seed: list.move_down unbound cannot be set (the keymap slice); no entry with a searchable two-item list (`First`, `Second`) | keymap slice (bindings from a config) |
| bottom_pane/list_selection_view.rs::c0_ctrl_p_respects_remapped_list_move_down | `tests/oracle/picker.rs::c0_ctrl_p_respects_remapped_list_move_down` | pending | no keymap seed: list.move_up unbound and list.move_down = ctrl+p cannot be set (the keymap slice); no entry with a searchable two-item list | keymap slice (bindings from a config) |
| bottom_pane/list_selection_view.rs::list_selection_view_page_and_jump_navigation_use_list_keymap | `tests/oracle/picker.rs::list_selection_view_page_and_jump_navigation_use_list_keymap` | pending | no keymap seed: list.page_down, page_up, jump_bottom and jump_top remapped to ctrl keys cannot be set (the keymap slice); no entry with twelve items | keymap slice (bindings from a config) |
| bottom_pane/list_selection_view.rs::page_and_jump_navigation_skip_trailing_disabled_rows_without_wrapping | `tests/oracle/picker.rs::page_and_jump_navigation_skip_trailing_disabled_rows_without_wrapping` | green | |  |
| bottom_pane/list_selection_view.rs::wraps_long_option_without_overflowing_columns | `tests/oracle/picker.rs::wraps_long_option_without_overflowing_columns` | green | |  |
| bottom_pane/list_selection_view.rs::width_changes_do_not_hide_rows | `tests/oracle/picker.rs::width_changes_do_not_hide_rows` | green | |  |
| bottom_pane/list_selection_view.rs::narrow_width_keeps_all_rows_visible | `tests/oracle/picker.rs::narrow_width_keeps_all_rows_visible` | green | |  |
| bottom_pane/list_selection_view.rs::snapshot_model_picker_width_80 | `tests/oracle/picker.rs::snapshot_model_picker_width_80` | green | |  |
| bottom_pane/list_selection_view.rs::snapshot_narrow_width_preserves_third_option | `tests/oracle/picker.rs::snapshot_narrow_width_preserves_third_option` | green | |  |
| bottom_pane/list_selection_view.rs::snapshot_narrow_width_counts_halfwidth_sound_marks | `tests/oracle/picker.rs::snapshot_narrow_width_counts_halfwidth_sound_marks` | pending | G44: rtwk's text engine clusters a halfwidth katakana and its sound mark into one cell (TextMode::Graphemes), the oracle counts the mark as its own cell, so the label column is one cell narrower and the description wraps one cell later; and the kit cannot address the `width 20:` and `width 24:` frames (oracle_snap::frames reads only `width=`) | rtwk gap G44 |
| bottom_pane/list_selection_view.rs::snapshot_auto_visible_col_width_mode_scroll_behavior | `tests/oracle/picker.rs::snapshot_auto_visible_col_width_mode_scroll_behavior` | pending | the kit cannot address the `before scroll:` and `after scroll:` frames (oracle_snap::frames reads no such header; tests/kit is not this slice's); the same rows are checked against the oracle's snapshot text in tests/sheets/select.rs | kit (frame addressing) |
| bottom_pane/list_selection_view.rs::snapshot_auto_all_rows_col_width_mode_scroll_behavior | `tests/oracle/picker.rs::snapshot_auto_all_rows_col_width_mode_scroll_behavior` | pending | the kit cannot address the `before scroll:` and `after scroll:` frames (oracle_snap::frames reads no such header; tests/kit is not this slice's); the same rows are checked against the oracle's snapshot text in tests/sheets/select.rs | kit (frame addressing) |
| bottom_pane/list_selection_view.rs::auto_all_rows_col_width_does_not_shift_when_scrolling | `tests/oracle/picker.rs::auto_all_rows_col_width_does_not_shift_when_scrolling` | green | |  |
| bottom_pane/list_selection_view.rs::stacked_side_content_is_used_when_side_by_side_does_not_fit | `tests/oracle/picker.rs::stacked_side_content_is_used_when_side_by_side_does_not_fit` | green | |  |
| bottom_pane/list_selection_view.rs::side_content_clearing_resets_symbols_and_style | `tests/oracle/picker.rs::side_content_clearing_resets_symbols_and_style` | pending | the sheet draws the preview beside the rows (side_content fixture, tests/sheets/select.rs); the kit cannot paint the screen behind the widget first (no red-filled X cells) to prove the clearing | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/list_selection_view.rs::side_content_clearing_handles_non_zero_buffer_origin | `tests/oracle/picker.rs::side_content_clearing_handles_non_zero_buffer_origin` | pending | the sheet is bottom-aligned in the screen; the kit cannot place it at a row offset | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/multi_select_picker.rs::non_orderable_items_cannot_move_or_be_crossed | `tests/oracle/picker.rs::non_orderable_items_cannot_move_or_be_crossed` | pending | no entry with a multi-select list (theme-colors not orderable with a section break after it, then model and branch orderable): the select screen has no multi-select picker | multi-select / browser picker (keymap screen) |
| bottom_pane/multi_select_picker.rs::horizontal_list_keys_reorder_orderable_items | `tests/oracle/picker.rs::horizontal_list_keys_reorder_orderable_items` | pending | no entry with a multi-select list of two orderable items (model, branch): the select screen has no reorder keys | multi-select / browser picker (keymap screen) |
| bottom_pane/multi_select_picker.rs::section_break_after_item_renders_separator_row | `tests/oracle/picker.rs::section_break_after_item_renders_separator_row` | pending | no entry with a multi-select list whose first item has a section break after it: the select screen has no section break row | multi-select / browser picker (keymap screen) |
| bottom_pane/multi_select_picker.rs::searchable_plain_j_updates_query_instead_of_navigating | `tests/oracle/picker.rs::searchable_plain_j_updates_query_instead_of_navigating` | pending | no entry with a searchable multi-select list (alpha, jupiter): the select screen has no multi-select picker | multi-select / browser picker (keymap screen) |
| bottom_pane/multi_select_picker.rs::multi_select_picker_page_and_jump_navigation_use_list_keymap | `tests/oracle/picker.rs::multi_select_picker_page_and_jump_navigation_use_list_keymap` | pending | no keymap seed: list.page_down, move_left, page_up, jump_top and jump_bottom cannot be set (the keymap slice); no entry with twelve items | keymap slice (bindings from a config) |
| bottom_pane/multi_select_picker.rs::short_picker_preserves_selected_row_when_overflow_spacers_yield | `tests/oracle/picker.rs::short_picker_preserves_selected_row_when_overflow_spacers_yield` | pending | no entry with twelve orderable fields (field-00 .. field-11) in a multi-select list: the select screen has no multi-select picker; the kit cannot address the `height=N` frames as written by the test | multi-select / browser picker (keymap screen) |
| bottom_pane/multi_select_picker.rs::picker_appearance_renders_checkboxes_preview_and_overflow | `tests/oracle/picker.rs::picker_appearance_renders_checkboxes_preview_and_overflow` | pending | no entry with twelve multi-select fields (the first enabled, a section break after it), ordering enabled and a preview line (`Field 0`): the select screen has no multi-select preview; the render size is the test's, to confirm | multi-select / browser picker (keymap screen) |
| bottom_pane/selection_picker_tests.rs::shared_menu_presentation_at_wide_and_narrow_sizes | `tests/oracle/picker.rs::shared_menu_presentation_at_wide_and_narrow_sizes` | pending | no entry with the shared menu (Choose an option, three items, footer `enter select · esc back`): the select screen has no such menu; the kit cannot address the `80x24` and `40x16` frames (oracle_snap::frames reads no such header) | kit (frame addressing) |
| bottom_pane/selection_picker_tests.rs::tiny_viewports_preserve_query_and_selection_when_enlarged | `tests/oracle/picker.rs::tiny_viewports_preserve_query_and_selection_when_enlarged` | pending | no entry with the browser list (Keymap, thirty actions per tab, search) at 80x24: the select screen has no browser | multi-select / browser picker (keymap screen) |
| bottom_pane/selection_picker_tests.rs::short_browser_keeps_header_tabs_search_and_footer | `tests/oracle/picker.rs::short_browser_keeps_header_tabs_search_and_footer` | pending | no entry with the browser list (Keymap, five tabs, search placeholder `Type to search shortcuts`, footer `left/right group · enter edit · esc close`): the select screen has no browser; the kit cannot address the `48×6:` frames (oracle_snap::frames reads no such header) | multi-select / browser picker (keymap screen) |
| bottom_pane/selection_picker_tests.rs::browser_page_navigation_uses_rows_that_fit | `tests/oracle/picker.rs::browser_page_navigation_uses_rows_that_fit` | pending | no entry with the browser list at 60x18 (Keymap, thirty actions): the select screen has no browser | multi-select / browser picker (keymap screen) |
| bottom_pane/selection_picker_tests.rs::shrinking_the_browser_keeps_selection_visible_without_navigation | `tests/oracle/picker.rs::shrinking_the_browser_keeps_selection_visible_without_navigation` | pending | no entry with the browser list in single-line and wrapped row displays: the select screen has no browser or row display mode | multi-select / browser picker (keymap screen) |
| bottom_pane/selection_picker_tests.rs::overflow_arrows_follow_the_painted_picker_surface | `tests/oracle/picker.rs::overflow_arrows_follow_the_painted_picker_surface` | pending | no entry with the browser list on a palette-seeded surface (the test sets the terminal default colours for the Panel and Terminal surfaces): the kit has no palette seed and the select screen has no browser | multi-select / browser picker (keymap screen) |
| bottom_pane/selection_popup_common.rs::narrow_description_columns_hide_without_stacking | `tests/oracle/picker.rs::narrow_description_columns_hide_without_stacking` | pending | no entry with the rows `日本語` (details) and `Blocked` (disabled) at widths 48, 49 and 80: the select screen has no description column layout | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_popup_common.rs::exact_fit_description_reserves_only_the_rendered_row | `tests/oracle/picker.rs::exact_fit_description_reserves_only_the_rendered_row` | pending | no entry with the one row `/model` and description `fits exactly` at width 20: the select screen has no such row | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_popup_common.rs::fuzzy_matched_emoji_graphemes_keep_description_alignment | `tests/oracle/picker.rs::fuzzy_matched_emoji_graphemes_keep_description_alignment` | pending | no entry with the fuzzy-matched emoji rows (thumbs-up with skin tone, family) at width 20: the select screen has no fuzzy match highlight | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_popup_common.rs::wrapped_two_column_rows_count_halfwidth_sound_marks | `tests/oracle/picker.rs::wrapped_two_column_rows_count_halfwidth_sound_marks` | pending | no entry with the halfwidth sound-mark row (`abｶﾞc` name, `abﾊﾟc` description) wrapped at width 9: the select screen has no two-column wrap | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_popup_common.rs::wrapped_two_column_rows_preserve_hard_line_breaks | `tests/oracle/picker.rs::wrapped_two_column_rows_preserve_hard_line_breaks` | pending | no entry with a two-line name and description (first/second, alpha/beta) at width 24: the select screen has no hard line break rows | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_popup_common.rs::selection_preserves_semantic_spans_and_secondary_hierarchy | `tests/oracle/picker.rs::selection_preserves_semantic_spans_and_secondary_hierarchy` | pending | no entry with the provider row (underlined L, Ready green italic, secondary details) under the default palette and the selection styles: the select screen has no semantic-span row and the kit has no palette seed | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_popup_common.rs::selected_rows_use_the_shared_accent_style | `tests/oracle/picker.rs::selected_rows_use_the_shared_accent_style` | pending | no entry with one selected row `selected` at width 16: the select screen has no selected-row accent style | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_row_links_tests.rs::choice_labels_and_descriptions_keep_complete_url_destinations | `tests/oracle/picker.rs::choice_labels_and_descriptions_keep_complete_url_destinations` | pending | no entry with the wrapped URL rows (a 日本語 label with a long URL, a long URL description) in the eight prefix, wrap and single-line combinations: the select screen has no hyperlink rows | rows-only popup widget (`widgets/options.rs`, ui-pane) / palette |
| bottom_pane/selection_tabs_tests.rs::filled_tabs_window_around_active_tab | `tests/oracle/picker.rs::filled_tabs_window_around_active_tab` | pending | G45: rtwk's TabBar scrolls by columns and cuts a tab at the edge, the oracle's strip (render_filled_tab_bar) shows whole tabs, keeps the active one and reserves two cells per arrow (probe: tests/sheets/select.rs probe_g45_tab_bar_cuts_a_tab_at_the_edge); the sheet draws its tabs as a plain row of the oracle's labels; and the kit cannot address one frame per active tab | rtwk gap G45 |
| bottom_pane/selection_tabs_tests.rs::filled_tabs_truncate_unicode_without_hiding_active_tab | `tests/oracle/picker.rs::filled_tabs_truncate_unicode_without_hiding_active_tab` | pending | G45: rtwk's TabBar scrolls by columns and cuts a tab at the edge, the oracle's strip (render_filled_tab_bar) shows whole tabs, keeps the active one and reserves two cells per arrow (probe: tests/sheets/select.rs probe_g45_tab_bar_cuts_a_tab_at_the_edge); the sheet draws its tabs as a plain row of the oracle's labels; and the kit cannot address one frame per width | rtwk gap G45 |
| bottom_pane/selection_tabs_tests.rs::filled_tab_unknown_palette_uses_readable_underlined_defaults | `tests/oracle/picker.rs::filled_tab_unknown_palette_uses_readable_underlined_defaults` | pending | G45: rtwk's TabBar scrolls by columns and cuts a tab at the edge, the oracle's strip (render_filled_tab_bar) shows whole tabs, keeps the active one and reserves two cells per arrow (probe: tests/sheets/select.rs probe_g45_tab_bar_cuts_a_tab_at_the_edge); the sheet draws its tabs as a plain row of the oracle's labels; and the strip is drawn inside the dialog, not alone at 5x1 (the kit screen is the whole dialog) | rtwk gap G45 |
| bottom_pane/tests/picker_hint_tests.rs::popup_hints_follow_configured_bindings | `tests/oracle/picker.rs::popup_hints_follow_configured_bindings` | pending | no keymap seed: accept = F3 and cancel = F2 cannot be set (the keymap slice); the footer hint `f3 select · f2 back` comes from those bindings | keymap slice (bindings from a config) |

## Sheet tests (`tests/sheets/select.rs`, binary `screens`) and core tests

* `snapshot_model_picker_width_80`, `approvals_selection_popup_snapshot` (the two
  snapshots the slice had), `snapshot_auto_visible_col_width_mode_scroll_behavior` and
  `snapshot_auto_all_rows_col_width_mode_scroll_behavior` (literal rows of the oracle's
  two frames, because the kit cannot address a frame header), enter / number key picks,
  esc closes, `a_preview_sits_beside_the_rows_when_it_fits`,
  `skills_menu_snapshot`, `copy_picker_snapshot`, `apps_loading_snapshot`,
  `worktrees_conversation_snapshot`, `worktrees_fork_snapshot`, `worktrees_new_snapshot`
  (the last six compare the oracle's rows in order with `assert_rows`, which accepts a row
  that contains the oracle row).
* `probe_g45_tab_bar_cuts_a_tab_at_the_edge`: the probe of G45.
* Core (`core/src/screens/select/list.rs`): search, settle, step wrap and disabled skip,
  page and jump, number shortcuts, row prefixes, window follow, paste, toggle flip, and
  `Side::beside` with the oracle's `side_layout_width_*` values. Bite: removing the
  disabled skip in `Browse::step` fails `step_wraps_round_the_ends_and_skips_disabled_rows`.

## rtwk gaps

* G44 (new): halfwidth sound mark clusters into one cell in rtwk, the oracle counts it as
  a cell. Probe: the oracle test under `KIT_PROBE_PENDING=1`.
* G45 (new): `TabBar` scrolls by columns and cuts a tab, the oracle windows whole tabs.
  Probe in `tests/sheets/select.rs`.
* G39b: the label column over the rows in view needs no rtwk hook: the sheet windows the
  items itself (note added to the G39b row).

## Deltas, risks

* No visual delta in the green tests. Not done: tab strip windowing (G45; a strip wider
  than the room overflows), the active tab's fill colour from the terminal palette (the
  fallback bold underline is drawn), side preview background (`Side::bg`), vertical
  centring inside the preview, multi-select, reorder, section breaks, row hyperlinks,
  fuzzy match highlight, configurable key bindings (no keymap config yet).
* The kit asserts a whole screen, so tests size the screen to the dialog; the debug page
  shows through above a bottom-aligned sheet.
