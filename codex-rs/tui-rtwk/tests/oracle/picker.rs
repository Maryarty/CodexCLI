//! Shard `picker` (screens S14, owner ui-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: bottom_pane/list_selection_view.rs (38 screen-level tests here; 3 state tests left to ui-pane)
// ORACLE bottom_pane/list_selection_view.rs:1982 secondary_action_respects_search_disabled_rows_and_dismissal kind=ui owner=ui-pane
#[test]
fn secondary_action_respects_search_disabled_rows_and_dismissal() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 10)
        .entry("select", "secondary_action")
        .open();
    s.keys("s");
    s.assert_absent("Selection");
    let mut disabled = Screen::build(Owner::UiPane)
        .size(60, 10)
        .entry("select", "secondary_action_disabled")
        .open();
    disabled.keys("s");
    disabled.assert_contains("Selection");
    let mut searching = Screen::build(Owner::UiPane)
        .size(60, 10)
        .entry("select", "secondary_action_searchable")
        .open();
    searching.keys("s");
    searching.assert_contains("Selection");
    // the search line shows the query `s` (the row format is the port's)
}

// ORACLE bottom_pane/list_selection_view.rs:2025 searchable_list_honors_configured_printable_accept_alternative kind=ui owner=ui-pane
#[test]
fn searchable_list_honors_configured_printable_accept_alternative() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "accept_alternative")
        .keymap(&[("list.accept", "z")])
        .open();
    s.keys("z");
    s.assert_absent("z");
}

// ORACLE bottom_pane/list_selection_view.rs:2169 renders_blank_line_between_title_and_items_without_subtitle kind=snap owner=ui-pane snap=list_selection_spacing_without_subtitle
#[test]
fn renders_blank_line_between_title_and_items_without_subtitle() {
    let s = Screen::build(Owner::UiPane)
        .size(48, 7)
        .entry("select", "approval_modes")
        .open();
    s.assert_snap("list_selection_spacing_without_subtitle");
}

// ORACLE bottom_pane/list_selection_view.rs:2178 renders_blank_line_between_subtitle_and_items kind=snap owner=ui-pane snap=list_selection_spacing_with_subtitle
#[test]
fn renders_blank_line_between_subtitle_and_items() {
    let s = Screen::build(Owner::UiPane)
        .size(48, 8)
        .entry("select", "approval_modes_subtitle")
        .open();
    s.assert_snap("list_selection_spacing_with_subtitle");
}

// ORACLE bottom_pane/list_selection_view.rs:2184 theme_picker_subtitle_uses_fallback_text_in_94x35_terminal kind=ui owner=ui-pane
#[test]
fn theme_picker_subtitle_uses_fallback_text_in_94x35_terminal() {
    let s = Screen::build(Owner::UiPane)
        .size(94, 35)
        .entry("settings", "theme_picker")
        .open();
    s.assert_contains("Move up/down to live preview themes");
}

// ORACLE bottom_pane/list_selection_view.rs:2214 preserve_side_content_bg_keeps_rendered_background_colors kind=ui owner=ui-pane
#[test]
fn preserve_side_content_bg_keeps_rendered_background_colors() {
    let s = Screen::build(Owner::UiPane)
        .size(120, 35)
        .entry("select", "side_content_blue")
        .open();
    s.assert_text_style("+", "bg=Blue");
}

// ORACLE bottom_pane/list_selection_view.rs:2257 snapshot_footer_note_wraps kind=snap owner=ui-pane snap=list_selection_footer_note_wraps
#[test]
fn snapshot_footer_note_wraps() {
    let s = Screen::build(Owner::UiPane)
        .size(40, 10)
        .entry("select", "footer_note")
        .open();
    s.assert_snap("list_selection_footer_note_wraps");
}

// ORACLE bottom_pane/list_selection_view.rs:2289 renders_search_query_line_when_enabled kind=ui owner=ui-pane
#[test]
fn renders_search_query_line_when_enabled() {
    let mut s = Screen::build(Owner::UiPane)
        .size(48, 8)
        .entry("select", "searchable_approval")
        .open();
    s.type_text("filters");
    s.assert_contains("filters");
}

// ORACLE bottom_pane/list_selection_view.rs:2320 empty_searchable_list_keeps_search_and_no_matches_visible kind=snap owner=ui-pane snap=list_selection_empty_searchable
#[test]
fn empty_searchable_list_keeps_search_and_no_matches_visible() {
    let s = Screen::build(Owner::UiPane)
        .size(48, 7)
        .entry("select", "empty_searchable")
        .open();
    s.assert_snap("list_selection_empty_searchable");
}

// ORACLE bottom_pane/list_selection_view.rs:2343 paste_appends_to_search_query_and_filters_items kind=ui owner=ui-pane
#[test]
fn paste_appends_to_search_query_and_filters_items() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 8)
        .entry("select", "branch_search")
        .open();
    s.keys("f");
    s.paste("eature/paste-support\n");
    s.assert_contains("main -> feature/paste-support");
    s.assert_absent("main -> feature/other");
}

// ORACLE bottom_pane/list_selection_view.rs:2375 whitespace_only_paste_is_ignored kind=ui owner=ui-pane
#[test]
fn whitespace_only_paste_is_ignored() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 8)
        .entry("select", "single_main_searchable")
        .open();
    let before = s.shot();
    s.paste(" \n\t ");
    s.assert_same_rows(&before);
}

// ORACLE bottom_pane/list_selection_view.rs:2398 switching_tabs_changes_visible_items_and_clears_search kind=ui owner=ui-pane
#[test]
fn switching_tabs_changes_visible_items_and_clears_search() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 8)
        .entry("select", "two_tabs_beta")
        .open();
    s.type_text("beta");
    s.keys("left");
    s.assert_contains("Alpha Item");
    s.assert_absent("Beta Item");
    s.assert_absent("beta");
    s.keys("ctrl+l");
    s.assert_contains("Beta Item");
    s.keys("ctrl+h");
    s.assert_contains("Alpha Item");
}

// ORACLE bottom_pane/list_selection_view.rs:2450 tabbed_view_preserves_current_row_on_initial_selection_and_tab_switch kind=ui owner=ui-pane
#[test]
fn tabbed_view_preserves_current_row_on_initial_selection_and_tab_switch() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 8)
        .entry("select", "two_tabs_current")
        .open();
    s.assert_contains("› 2. Beta Current");
    s.keys("left");
    s.assert_contains("› 2. Alpha Current");
}

// ORACLE bottom_pane/list_selection_view.rs:2510 space_appends_to_active_search_instead_of_toggling_selected_item kind=ui owner=ui-pane
#[test]
fn space_appends_to_active_search_instead_of_toggling_selected_item() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "plugin_toggle")
        .open();
    s.type_text("plugin");
    s.keys("space");
    s.assert_contains("plugin");
    // the toggle stays off: the row shows it unchecked (the glyph is the port's toggle row)
}

// ORACLE bottom_pane/list_selection_view.rs:2550 configured_list_binding_takes_precedence_over_search_toggle_key kind=ui owner=ui-pane
#[test]
fn configured_list_binding_takes_precedence_over_search_toggle_key() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "toggle_then_next")
        .keymap(&[("list.move_down", "tab")])
        .open();
    s.keys("tab");
    s.assert_contains("› Next");
}

// ORACLE bottom_pane/list_selection_view.rs:2598 configured_cancel_binding_takes_precedence_over_space_toggle kind=ui owner=ui-pane
#[test]
fn configured_cancel_binding_takes_precedence_over_space_toggle() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "animations_checkbox")
        .keymap(&[("list.cancel", "space")])
        .open();
    s.keys("space");
    s.assert_absent("Animations");
}

// ORACLE bottom_pane/list_selection_view.rs:2635 single_line_row_display_truncates_instead_of_wrapping kind=ui owner=ui-pane
#[test]
fn single_line_row_display_truncates_instead_of_wrapping() {
    let s = Screen::build(Owner::UiPane)
        .size(36, 5)
        .entry("select", "single_line_long")
        .open();
    s.assert_contains("…");
}

// ORACLE bottom_pane/list_selection_view.rs:2690 name_column_width_override_moves_description_column_right kind=ui owner=ui-pane
#[test]
fn name_column_width_override_moves_description_column_right() {
    let auto = Screen::build(Owner::UiPane)
        .size(48, 4)
        .entry("select", "auto_columns")
        .open();
    let widened = Screen::build(Owner::UiPane)
        .size(48, 4)
        .entry("select", "name_column_18")
        .open();
    auto.assert_contains("1. Short");
    widened.assert_contains("1. Short");
    // missed kit call: the column of `desc` in each frame (description_col); the kit has no column-of-text compare, so widened > auto is not written
}

// ORACLE bottom_pane/list_selection_view.rs:2757 enter_with_no_matches_triggers_cancel_callback kind=ui owner=ui-pane
#[test]
fn enter_with_no_matches_triggers_cancel_callback() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "read_only_searchable")
        .open();
    s.type_text("no-matches");
    s.keys("enter");
    s.assert_absent("Read Only");
}

// ORACLE bottom_pane/list_selection_view.rs:2788 move_down_without_selection_change_does_not_fire_callback kind=ui owner=ui-pane
#[test]
fn move_down_without_selection_change_does_not_fire_callback() {
    kit::na(
        Owner::UiPane,
        "asserts that on_selection_changed sends no app event after a Down in a one-item list: an event sent to the app, not drawn",
    );
}

// ORACLE bottom_pane/list_selection_view.rs:2817 disabled_current_rows_skip_default_selection_and_number_shortcuts kind=ui owner=ui-pane
#[test]
fn disabled_current_rows_skip_default_selection_and_number_shortcuts() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 10)
        .entry("select", "disabled_current")
        .open();
    s.assert_contains("› 1. Alpha");
    s.assert_contains("  2. Beta");
    s.assert_absent("1. Unavailable");
    s.assert_absent("3. Beta");
    s.keys("2");
    s.assert_absent("Beta");
}

// ORACLE bottom_pane/list_selection_view.rs:2875 c0_ctrl_p_respects_unbound_list_move_up kind=ui owner=ui-pane
#[test]
fn c0_ctrl_p_respects_unbound_list_move_up() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "first_second_searchable_second")
        .keymap(&[("list.move_up", "")])
        .open();
    s.keys("ctrl+p");
    s.assert_contains("› Second");
}

// ORACLE bottom_pane/list_selection_view.rs:2907 c0_ctrl_n_respects_unbound_list_move_down kind=ui owner=ui-pane
#[test]
fn c0_ctrl_n_respects_unbound_list_move_down() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "first_second_searchable")
        .keymap(&[("list.move_down", "")])
        .open();
    s.keys("ctrl+n");
    s.assert_contains("› First");
}

// ORACLE bottom_pane/list_selection_view.rs:2938 c0_ctrl_p_respects_remapped_list_move_down kind=ui owner=ui-pane
#[test]
fn c0_ctrl_p_respects_remapped_list_move_down() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "first_second_searchable")
        .keymap(&[("list.move_up", ""), ("list.move_down", "ctrl+p")])
        .open();
    s.keys("ctrl+p");
    s.assert_contains("› Second");
}

// ORACLE bottom_pane/list_selection_view.rs:2969 list_selection_view_page_and_jump_navigation_use_list_keymap kind=ui owner=ui-pane
#[test]
fn list_selection_view_page_and_jump_navigation_use_list_keymap() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 12)
        .entry("select", "twelve_items")
        .keymap(&[
            ("list.page_down", "ctrl+d"),
            ("list.page_up", "ctrl+u"),
            ("list.jump_bottom", "ctrl+e"),
            ("list.jump_top", "ctrl+a"),
        ])
        .open();
    s.keys("pagedown");
    s.assert_contains("› 1. Item 0");
    s.keys("ctrl+d");
    s.assert_contains("› 9. Item 8");
    s.keys("ctrl+u");
    s.assert_contains("› 1. Item 0");
    s.keys("ctrl+e");
    s.assert_contains("› 12. Item 11");
    s.keys("ctrl+a");
    s.assert_contains("› 1. Item 0");
}

// ORACLE bottom_pane/list_selection_view.rs:3008 page_and_jump_navigation_skip_trailing_disabled_rows_without_wrapping kind=ui owner=ui-pane
#[test]
fn page_and_jump_navigation_skip_trailing_disabled_rows_without_wrapping() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 10)
        .entry("select", "twelve_trailing_disabled")
        .open();
    s.keys("pagedown");
    s.assert_contains("› 8. Item 7");
    s.keys("end");
    s.assert_contains("› 8. Item 7");
}

// ORACLE bottom_pane/list_selection_view.rs:3040 wraps_long_option_without_overflowing_columns kind=ui owner=ui-pane
#[test]
fn wraps_long_option_without_overflowing_columns() {
    let s = Screen::build(Owner::UiPane)
        .size(60, 10)
        .entry("select", "approval_long_command")
        .open();
    s.assert_contains("     `python -mpre_commit run");
    s.assert_contains("eslint-plugin/no-");
    s.assert_contains("mixed-const-enum-exports.js");
}

// ORACLE bottom_pane/list_selection_view.rs:3081 width_changes_do_not_hide_rows kind=ui owner=ui-pane
#[test]
fn width_changes_do_not_hide_rows() {
    let mut s = Screen::build(Owner::UiPane)
        .size(90, 12)
        .entry("select", "three_models_effort")
        .open();
    for width in 60..=90 {
        s.resize(width, 12);
        s.assert_contains("3.");
    }
}

// ORACLE bottom_pane/list_selection_view.rs:3135 narrow_width_keeps_all_rows_visible kind=ui owner=ui-pane
#[test]
fn narrow_width_keeps_all_rows_visible() {
    let s = Screen::build(Owner::UiPane)
        .size(24, 10)
        .entry("select", "three_long_descriptions")
        .open();
    s.assert_contains("3.");
}

// ORACLE bottom_pane/list_selection_view.rs:3163 snapshot_model_picker_width_80 kind=snap owner=ui-pane snap=list_selection_model_picker_width_80
#[test]
fn snapshot_model_picker_width_80() {
    let s = Screen::build(Owner::UiPane)
        .size(80, 10)
        .entry("select", "three_models_effort")
        .open();
    s.assert_snap("list_selection_model_picker_width_80");
}

// ORACLE bottom_pane/list_selection_view.rs:3210 snapshot_narrow_width_preserves_third_option kind=snap owner=ui-pane snap=list_selection_narrow_width_preserves_rows
#[test]
fn snapshot_narrow_width_preserves_third_option() {
    let s = Screen::build(Owner::UiPane)
        .size(24, 7)
        .entry("select", "three_long_descriptions")
        .open();
    s.assert_snap("list_selection_narrow_width_preserves_rows");
}

// ORACLE bottom_pane/list_selection_view.rs:3237 snapshot_narrow_width_counts_halfwidth_sound_marks kind=snap owner=ui-pane snap=list_selection_halfwidth_sound_marks_narrow
#[test]
fn snapshot_narrow_width_counts_halfwidth_sound_marks() {
    kit::delta(
        Owner::UiPane,
        "G56 (G44 closed by TextMode::Legacy, `Open::legacy_text`): the oracle wraps a row's name and description as one line (`word_wrap_line`), so a description word too long for what is left beside the name (`handakuten`, 8 cells) starts on the next line and the name line stands alone; the Grid's description cell breaks the word at its width on the name line",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(20, 12)
        .entry("select", "halfwidth_marks")
        .legacy_text()
        .open();
    s.assert_snap_section("list_selection_halfwidth_sound_marks_narrow", "width 20:");
    s.resize(24, 10);
    s.assert_snap_section("list_selection_halfwidth_sound_marks_narrow", "width 24:");
}

// ORACLE bottom_pane/list_selection_view.rs:3272 snapshot_auto_visible_col_width_mode_scroll_behavior kind=snap owner=ui-pane snap=list_selection_col_width_mode_auto_visible_scroll
#[test]
fn snapshot_auto_visible_col_width_mode_scroll_behavior() {
    kit::delta(
        Owner::UiPane,
        "G39b: AutoVisible measures the label column over the rows in the list's window and rtwk's ListView has no layout-time hook for it; the kit also cannot address the `before scroll:` and `after scroll:` frames; the probe is tests/sheets/select.rs probe_g39b_auto_visible_measures_the_column_over_every_row",
    );
    let mut s = Screen::build(Owner::UiPane)
        .size(96, 12)
        .entry("select", "col_width_auto_visible")
        .open();
    s.assert_snap_section(
        "list_selection_col_width_mode_auto_visible_scroll",
        "before scroll:",
    );
    s.keys(["down"; 8].join(" ").as_str());
    s.assert_snap_section(
        "list_selection_col_width_mode_auto_visible_scroll",
        "after scroll:",
    );
}

// ORACLE bottom_pane/list_selection_view.rs:3280 snapshot_auto_all_rows_col_width_mode_scroll_behavior kind=snap owner=ui-pane snap=list_selection_col_width_mode_auto_all_rows_scroll
#[test]
fn snapshot_auto_all_rows_col_width_mode_scroll_behavior() {
    let mut s = Screen::build(Owner::UiPane)
        .size(96, 12)
        .entry("select", "col_width_auto_all_rows")
        .open();
    s.assert_snap_section(
        "list_selection_col_width_mode_auto_all_rows_scroll",
        "before scroll:",
    );
    s.keys(["down"; 8].join(" ").as_str());
    s.assert_snap_section(
        "list_selection_col_width_mode_auto_all_rows_scroll",
        "after scroll:",
    );
}

// ORACLE bottom_pane/list_selection_view.rs:3288 auto_all_rows_col_width_does_not_shift_when_scrolling kind=ui owner=ui-pane
#[test]
fn auto_all_rows_col_width_does_not_shift_when_scrolling() {
    let mut s = Screen::build(Owner::UiPane)
        .size(96, 12)
        .entry("select", "col_width_auto_all_rows")
        .open();
    s.keys(["down"; 8].join(" ").as_str());
    s.assert_contains("9. Item 9 with an intentionally much longer name");
    // missed kit call: the description column of `8. Item 8` / `desc 8` before and after the scroll (no column-of-text compare)
}

// ORACLE bottom_pane/list_selection_view.rs:3375 stacked_side_content_is_used_when_side_by_side_does_not_fit kind=ui owner=ui-pane
#[test]
fn stacked_side_content_is_used_when_side_by_side_does_not_fit() {
    let s = Screen::build(Owner::UiPane)
        .size(70, 8)
        .entry("select", "stacked_side_content")
        .open();
    s.assert_contains("N");
    s.assert_absent("W");
}

// ORACLE bottom_pane/list_selection_view.rs:3413 side_content_clearing_resets_symbols_and_style kind=ui owner=ui-pane
#[test]
fn side_content_clearing_resets_symbols_and_style() {
    kit::na(
        Owner::UiPane,
        "ratatui renders into a Buffer that already holds stale cells (red X cells, a row offset) and must clear them under the preview; rtwk paints the dialog's whole region every frame and diffs cells, so no stale cell can survive: nothing in the screen to assert (the preview beside the rows is a sheet test, a_preview_sits_beside_the_rows_when_it_fits)",
    );
}

// ORACLE bottom_pane/list_selection_view.rs:3472 side_content_clearing_handles_non_zero_buffer_origin kind=ui owner=ui-pane
#[test]
fn side_content_clearing_handles_non_zero_buffer_origin() {
    kit::na(
        Owner::UiPane,
        "ratatui renders into a Buffer that already holds stale cells (red X cells, a row offset) and must clear them under the preview; rtwk paints the dialog's whole region every frame and diffs cells, so no stale cell can survive: nothing in the screen to assert (the preview beside the rows is a sheet test, a_preview_sits_beside_the_rows_when_it_fits)",
    );
}

// ---- oracle: bottom_pane/multi_select_picker.rs (7 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/multi_select_picker.rs:888 non_orderable_items_cannot_move_or_be_crossed kind=ui owner=ui-pane
#[test]
fn non_orderable_items_cannot_move_or_be_crossed() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 12)
        .entry("select", "multi_orderable")
        .open();
    s.keys("down");
    s.assert_in_order(&["theme-colors", "model", "branch"]);
}

// ORACLE bottom_pane/multi_select_picker.rs:926 horizontal_list_keys_reorder_orderable_items kind=ui owner=ui-pane
#[test]
fn horizontal_list_keys_reorder_orderable_items() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 8)
        .entry("select", "multi_orderable_two")
        .open();
    s.keys("ctrl+l");
    s.assert_in_order(&["branch", "model"]);
    s.keys("ctrl+h");
    s.assert_in_order(&["model", "branch"]);
}

// ORACLE bottom_pane/multi_select_picker.rs:958 section_break_after_item_renders_separator_row kind=ui owner=ui-pane
#[test]
fn section_break_after_item_renders_separator_row() {
    let s = Screen::build(Owner::UiPane)
        .size(60, 9)
        .entry("select", "section_break")
        .open();
    s.assert_in_order(&[
        "› [ ] theme-colors",
        "  ───────────────────────",
        "  [ ] model",
    ]);
}

// ORACLE bottom_pane/multi_select_picker.rs:983 searchable_plain_j_updates_query_instead_of_navigating kind=ui owner=ui-pane
#[test]
fn searchable_plain_j_updates_query_instead_of_navigating() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 6)
        .entry("select", "multi_alpha_jupiter")
        .open();
    s.keys("j");
    s.assert_contains("j");
    s.assert_absent("alpha");
    s.assert_contains("› [ ] jupiter");
}

// ORACLE bottom_pane/multi_select_picker.rs:1001 multi_select_picker_page_and_jump_navigation_use_list_keymap kind=ui owner=ui-pane
#[test]
fn multi_select_picker_page_and_jump_navigation_use_list_keymap() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 18)
        .entry("select", "multi_twelve")
        .keymap(&[
            ("list.page_down", "ctrl+d"),
            ("list.move_left", "ctrl+d"),
            ("list.page_up", "ctrl+u"),
            ("list.jump_bottom", "ctrl+e"),
            ("list.jump_top", "ctrl+a"),
        ])
        .open();
    s.keys("pagedown");
    s.assert_contains("› [ ] item-0");
    s.keys("ctrl+d");
    s.assert_contains("› [ ] item-8");
    s.keys("ctrl+u");
    s.assert_contains("› [ ] item-0");
    s.keys("ctrl+e");
    s.assert_contains("› [ ] item-11");
    s.keys("ctrl+a");
    s.assert_contains("› [ ] item-0");
}

// ORACLE bottom_pane/multi_select_picker.rs:1045 short_picker_preserves_selected_row_when_overflow_spacers_yield kind=snap owner=ui-pane snap=short_picker_preserves_selected_row_when_overflow_spacers_yield
#[test]
fn short_picker_preserves_selected_row_when_overflow_spacers_yield() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 7)
        .entry("select", "multi_fields_twelve")
        .open();
    s.keys("end");
    for height in 1..=7 {
        s.resize(80, height);
        if height >= 4 {
            s.assert_contains("› [ ] field-11");
        }
        s.assert_snap_section(
            "short_picker_preserves_selected_row_when_overflow_spacers_yield",
            &format!("height={height}"),
        );
    }
    s.keys("space");
    s.assert_contains("[x] field-11");
}

// ORACLE bottom_pane/multi_select_picker.rs:1096 picker_appearance_renders_checkboxes_preview_and_overflow kind=snap owner=ui-pane snap=picker_appearance_renders_checkboxes_preview_and_overflow
#[test]
fn picker_appearance_renders_checkboxes_preview_and_overflow() {
    kit::delta(
        Owner::UiPane,
        "G39b (select-dialog.md): frame 2 (`Last page:`) has the details column measured over the rows in the window, ours over every row (the rule row's width keeps it at column 27); the three frames' other rows are equal (tests/sheets/select.rs the_multi_select_appearance_matches_the_oracle_in_three_frames); the dump's cells say `bg: Reset`, the debug host paints its own background under the sheet",
    );
    // The oracle renders three frames at width 48 into the height `desired_height` asks for (17, 17, 10).
    const SNAP: &str = "picker_appearance_renders_checkboxes_preview_and_overflow";
    let mut s = Screen::build(Owner::UiPane)
        .size(48, 17)
        .entry("select", "multi_appearance")
        .open();
    s.assert_snap_section(SNAP, "First page:");
    s.keys("end");
    s.keys("space");
    s.keys("left");
    s.assert_snap_section(SNAP, "Last page:");
    s.resize(48, 10);
    s.type_text("z");
    s.assert_snap_section(SNAP, "No matches:");
}

// ---- oracle: bottom_pane/selection_picker_tests.rs (6 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/selection_picker_tests.rs:62 shared_menu_presentation_at_wide_and_narrow_sizes kind=snap owner=ui-pane snap=codex_tui__bottom_pane__list_selection_view__picker_tests__shared_menu_presentation_at_wide_and_narrow_sizes
#[test]
fn shared_menu_presentation_at_wide_and_narrow_sizes() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .entry("select", "shared_menu")
        .open();
    // The oracle draws the view into the whole area (the footer on the last row); the dialog is as tall as its content.
    const SNAP: &str = "codex_tui__bottom_pane__list_selection_view__picker_tests__shared_menu_presentation_at_wide_and_narrow_sizes";
    s.snap(SNAP).frame("80x24").dialog().assert();
    s.assert_contains("enter select · esc back");
    s.resize(40, 16);
    s.snap(SNAP).frame("40x16").dialog().assert();
    s.assert_contains("enter select · esc back");
}

// ORACLE bottom_pane/selection_picker_tests.rs:111 tiny_viewports_preserve_query_and_selection_when_enlarged kind=ui owner=ui-pane
#[test]
fn tiny_viewports_preserve_query_and_selection_when_enlarged() {
    let mut s = Screen::build(Owner::UiPane)
        .size(80, 24)
        .entry("select", "browser_keymap")
        .open();
    s.keys("2 down");
    let before = s.shot();
    for (w, h) in [(0, 0), (1, 1), (4, 1), (12, 3), (40, 8)] {
        s.resize(w, h);
    }
    s.resize(80, 24);
    s.assert_same_rows(&before);
}

// ORACLE bottom_pane/selection_picker_tests.rs:130 short_browser_keeps_header_tabs_search_and_footer kind=snap owner=ui-pane snap=codex_tui__bottom_pane__list_selection_view__picker_tests__short_browser_keeps_header_tabs_search_and_footer
#[test]
fn short_browser_keeps_header_tabs_search_and_footer() {
    let mut s = Screen::build(Owner::UiPane)
        .size(48, 6)
        .entry("select", "browser_keymap")
        .open();
    for (w, h) in [(48, 6), (48, 7), (48, 13), (24, 6), (24, 7)] {
        s.resize(w, h);
        s.assert_contains("Keymap");
        s.assert_contains("Common");
        s.assert_contains("Type to search");
        s.assert_contains("left/right group");
        s.assert_snap_section("codex_tui__bottom_pane__list_selection_view__picker_tests__short_browser_keeps_header_tabs_search_and_footer", &format!("{w}×{h}:"));
    }
}

// ORACLE bottom_pane/selection_picker_tests.rs:146 browser_page_navigation_uses_rows_that_fit kind=snap owner=ui-pane snap=codex_tui__bottom_pane__list_selection_view__picker_tests__browser_page_navigation_uses_rows_that_fit
#[test]
fn browser_page_navigation_uses_rows_that_fit() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 18)
        .entry("select", "browser_keymap")
        .open();
    s.keys("pagedown");
    s.assert_snap("codex_tui__bottom_pane__list_selection_view__picker_tests__browser_page_navigation_uses_rows_that_fit");
    s.assert_contains("›");
}

// ORACLE bottom_pane/selection_picker_tests.rs:167 shrinking_the_browser_keeps_selection_visible_without_navigation kind=ui owner=ui-pane
#[test]
fn shrinking_the_browser_keeps_selection_visible_without_navigation() {
    for entry in ["browser_single_line", "browser_wrapped"] {
        let mut s = Screen::build(Owner::UiPane)
            .size(60, 24)
            .entry("select", entry)
            .open();
        s.keys(["down"; 7].join(" ").as_str());
        s.resize(60, 13);
        s.assert_contains("Action 07");
    }
}

// ORACLE bottom_pane/selection_picker_tests.rs:192 overflow_arrows_follow_the_painted_picker_surface kind=ui owner=ui-pane
#[test]
fn overflow_arrows_follow_the_painted_picker_surface() {
    let mut s = Screen::build(Owner::UiPane)
        .size(60, 18)
        .entry("select", "browser_keymap")
        .open();
    s.keys(["down"; 15].join(" ").as_str());
    s.assert_contains("↑");
    s.assert_contains("↓");
    // missed kit call: the palette-seeded default colours for the arrow cell colours and the selected row background
}

// ---- oracle: bottom_pane/selection_popup_common.rs (7 screen-level tests here; 2 state tests left to ui-pane)
// ORACLE bottom_pane/selection_popup_common.rs:743 narrow_description_columns_hide_without_stacking kind=snap owner=ui-pane snap=description_column_visibility
#[test]
fn narrow_description_columns_hide_without_stacking() {
    let mut s = Screen::build(Owner::UiPane)
        .size(48, 2)
        .entry("select", "description_hide_numbered")
        .open();
    for width in [48, 49, 80] {
        s.resize(width, 2);
        if width >= 49 {
            s.assert_contains("Details");
            s.assert_contains("Unavailable");
        } else {
            s.assert_absent("Details");
            s.assert_absent("Unavailable");
        }
        s.assert_contains("Blocked (disabled)");
        s.assert_snap_section("description_column_visibility", &format!("width={width}"));
    }
}

// ORACLE bottom_pane/selection_popup_common.rs:816 exact_fit_description_reserves_only_the_rendered_row kind=snap owner=ui-pane
#[test]
fn exact_fit_description_reserves_only_the_rendered_row() {
    kit::pending(
        Owner::UiPane,
        "no entry with the one row `/model` and description `fits exactly` at width 20: the select screen has no such row",
    );
    let s = Screen::build(Owner::UiPane)
        .size(20, 1)
        .entry("select", "exact_fit")
        .open();
    s.assert_row(0, "/model  fits exactly");
}

// ORACLE bottom_pane/selection_popup_common.rs:886 fuzzy_matched_emoji_graphemes_keep_description_alignment kind=snap owner=ui-pane snap=popup_fuzzy_matched_emoji_graphemes
#[test]
fn fuzzy_matched_emoji_graphemes_keep_description_alignment() {
    kit::pending(
        Owner::UiPane,
        "no entry with the fuzzy-matched emoji rows (thumbs-up with skin tone, family) at width 20: the select screen has no fuzzy match highlight",
    );
    let s = Screen::build(Owner::UiPane)
        .size(20, 2)
        .entry("select", "fuzzy_emoji")
        .open();
    s.assert_snap("popup_fuzzy_matched_emoji_graphemes");
    s.assert_text_at(4, 0, "d");
    s.assert_text_at(4, 1, "d");
}

// ORACLE bottom_pane/selection_popup_common.rs:928 wrapped_two_column_rows_count_halfwidth_sound_marks kind=snap owner=ui-pane
#[test]
fn wrapped_two_column_rows_count_halfwidth_sound_marks() {
    kit::pending(
        Owner::UiPane,
        "no entry with the halfwidth sound-mark row (`abｶﾞc` name, `abﾊﾟc` description) wrapped at width 9: the select screen has no two-column wrap",
    );
    let s = Screen::build(Owner::UiPane)
        .size(9, 2)
        .entry("select", "wrap_halfwidth")
        .open();
    s.assert_row(0, "abｶﾞ  ab");
    s.assert_row(1, "c     ﾊﾟ c");
}

// ORACLE bottom_pane/selection_popup_common.rs:965 wrapped_two_column_rows_preserve_hard_line_breaks kind=snap owner=ui-pane
#[test]
fn wrapped_two_column_rows_preserve_hard_line_breaks() {
    kit::pending(
        Owner::UiPane,
        "no entry with a two-line name and description (first/second, alpha/beta) at width 24: the select screen has no hard line break rows",
    );
    let s = Screen::build(Owner::UiPane)
        .size(24, 2)
        .entry("select", "hard_breaks")
        .open();
    s.assert_row(0, "first   alpha");
    s.assert_row(1, "second  beta");
}

// ORACLE bottom_pane/selection_popup_common.rs:986 selection_preserves_semantic_spans_and_secondary_hierarchy kind=ui owner=ui-pane
#[test]
fn selection_preserves_semantic_spans_and_secondary_hierarchy() {
    let s = Screen::build(Owner::UiPane)
        .size(48, 1)
        .entry("select", "semantic_spans")
        .open();
    s.assert_text_style("L", "bold underlined");
    s.assert_text_style("Ready", "fg=Green bold italic");
    s.assert_text_style("secondary", "!bold");
    // missed kit call: the palette-seeded colours (with_test_default_colors) and the selection style cases
}

// ORACLE bottom_pane/selection_popup_common.rs:1089 selected_rows_use_the_shared_accent_style kind=ui owner=ui-pane
#[test]
fn selected_rows_use_the_shared_accent_style() {
    let s = Screen::build(Owner::UiPane)
        .size(16, 1)
        .entry("select", "selected_row")
        .open();
    s.assert_style(0, 0, "bold");
    // missed kit call: the accent foreground (accent_style().fg) needs the palette, which the kit does not state
}

// ---- oracle: bottom_pane/selection_row_links_tests.rs (1 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/selection_row_links_tests.rs:4 choice_labels_and_descriptions_keep_complete_url_destinations kind=snap owner=ui-pane snap=choice_wrapped_urls
#[test]
fn choice_labels_and_descriptions_keep_complete_url_destinations() {
    kit::pending(
        Owner::UiPane,
        "no entry with the wrapped URL rows (a 日本語 label with a long URL, a long URL description) in the eight prefix, wrap and single-line combinations: the select screen has no hyperlink rows",
    );
    let s = Screen::build(Owner::UiPane)
        .size(48, 12)
        .entry("select", "wrapped_urls")
        .open();
    s.assert_snap_section(
        "choice_wrapped_urls",
        "prefix label: false, wrapped label: true, single line: false",
    );
    // missed kit calls: the other seven sections need their own rows, and the kit cannot address the `prefix label:` frame headers (oracle_snap::frames reads no such header)
}

// ---- oracle: bottom_pane/selection_tabs_tests.rs (3 screen-level tests here; 0 state tests left to ui-pane)
// ORACLE bottom_pane/selection_tabs_tests.rs:22 filled_tabs_window_around_active_tab kind=snap owner=ui-pane snap=filled_tabs_window_around_active_tab
#[test]
fn filled_tabs_window_around_active_tab() {
    kit::na(
        Owner::UiPane,
        "a frame per active tab and per width is not addressable by the kit (oracle_snap::frames reads no such header); the strips are asserted against the oracle's snapshot text through the sheet in tests/sheets/select.rs filled_tabs_window_around_active_tab",
    );
}

// ORACLE bottom_pane/selection_tabs_tests.rs:60 filled_tabs_truncate_unicode_without_hiding_active_tab kind=snap owner=ui-pane snap=filled_tabs_truncate_unicode_without_hiding_active_tab
#[test]
fn filled_tabs_truncate_unicode_without_hiding_active_tab() {
    kit::na(
        Owner::UiPane,
        "a frame per width is not addressable by the kit (oracle_snap::frames reads no such header); the strips are asserted against the oracle's snapshot text through the sheet in tests/sheets/select.rs filled_tabs_truncate_unicode_without_hiding_active_tab",
    );
}

// ORACLE bottom_pane/selection_tabs_tests.rs:69 filled_tab_unknown_palette_uses_readable_underlined_defaults kind=ui owner=ui-pane
#[test]
fn filled_tab_unknown_palette_uses_readable_underlined_defaults() {
    kit::pending(
        Owner::UiPane,
        "G54 (select-dialog.md): the oracle's `Style::default().fg(Reset).bg(Reset)` resets the cells under the tab; rtwk's `Style` has no explicit reset (a Text with no colour blends the cell beneath, `ColoredBox(Color::DEFAULT)` resets only the bg), so the blank of the tab keeps the fg (and underline colour) of what the dialog covers; the sheet's strip is bold and underlined as the oracle's fallback",
    );
    let s = Screen::build(Owner::UiPane)
        .size(20, 6)
        .entry("select", "filled_tab_all")
        .open();
    s.assert_text_style(" All ", "fg=Reset bg=Reset bold underlined");
}

// ---- oracle: bottom_pane/tests/picker_hint_tests.rs (1 screen-level tests here; 1 state tests left to ui-pane)
// ORACLE bottom_pane/tests/picker_hint_tests.rs:7 popup_hints_follow_configured_bindings kind=snap owner=ui-pane
#[test]
fn popup_hints_follow_configured_bindings() {
    // the picker preset draws one blank row above the title: seven rows
    for (name, entry, height) in [("default", "hint_default", 6), ("picker", "hint_picker", 7)] {
        let mut s = Screen::build(Owner::UiPane)
            .size(60, height)
            .entry("select", entry)
            .keymap(&[("list.accept", "f3"), ("list.cancel", "f2")])
            .open();
        s.assert_contains("f3 select · f2 back");
        s.assert_snap(&format!("popup_hint_{name}"));
        s.keys("f2");
        s.assert_absent("Continue");
    }
}
