//! Shard `transcript_view_3` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: transcript_view/markdown_copy_tests.rs (23 screen-level tests here; 5 state tests left to chat-live)
// ORACLE transcript_view/markdown_copy_tests.rs:18 selected_markdown_preserves_lists_and_inline_code kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__selected_list_html snap=codex_tui__transcript_view__markdown_copy_tests__selected_markdown_preserves_lists_and_inline_code
#[test]
fn selected_markdown_preserves_lists_and_inline_code() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:124 inline_code_selection_copies_only_selected_content kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__inline_code_selection_copies_only_selected_content
#[test]
fn inline_code_selection_copies_only_selected_content() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:186 inline_boundaries_preserve_selected_text_and_formatting kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__inline_boundaries_preserve_selected_text_and_formatting
#[test]
fn inline_boundaries_preserve_selected_text_and_formatting() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:251 lists_keep_nesting_and_paragraphs_without_sibling_spacers kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__lists_keep_nesting_and_paragraphs_without_sibling_spacers
#[test]
fn lists_keep_nesting_and_paragraphs_without_sibling_spacers() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:268 code_only_is_literal_and_mixed_code_has_safe_fences kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__code_only_is_literal_and_mixed_code_has_safe_fences
#[test]
fn code_only_is_literal_and_mixed_code_has_safe_fences() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:286 mixed_plain_output_remains_literal_in_html kind=ui owner=chat-live
#[test]
fn mixed_plain_output_remains_literal_in_html() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:404 mixed_user_prose_preserves_literal_text_without_code_fences kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__mixed_user_prose_preserves_literal_text_without_code_fences
#[test]
fn mixed_user_prose_preserves_literal_text_without_code_fences() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:442 copied_selection_keeps_its_revision_and_format_across_resize kind=ui owner=chat-live
#[test]
fn copied_selection_keeps_its_revision_and_format_across_resize() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:491 literal_punctuation_is_not_reinterpreted_and_selected_breaks_survive kind=ui owner=chat-live
#[test]
fn literal_punctuation_is_not_reinterpreted_and_selected_breaks_survive() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:506 explicit_hard_breaks_survive_only_when_selected kind=ui owner=chat-live
#[test]
fn explicit_hard_breaks_survive_only_when_selected() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:517 task_lists_and_transformed_tables_keep_their_meaning kind=ui owner=chat-live
#[test]
fn task_lists_and_transformed_tables_keep_their_meaning() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:538 link_entities_are_not_decoded_twice kind=ui owner=chat-live
#[test]
fn link_entities_are_not_decoded_twice() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:551 selected_local_link_keeps_one_visible_destination kind=ui owner=chat-live
#[test]
fn selected_local_link_keeps_one_visible_destination() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:562 complete_rules_keep_structure_and_partial_rules_keep_selected_text kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__complete_rules_keep_structure_and_partial_rules_keep_selected_text
#[test]
fn complete_rules_keep_structure_and_partial_rules_keep_selected_text() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:581 table_spillover_keeps_prose_outside_the_literal_table kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__table_spillover_keeps_prose_outside_the_literal_table
#[test]
fn table_spillover_keeps_prose_outside_the_literal_table() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:594 partial_prose_keeps_leading_whitespace_outside_code_blocks kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__partial_prose_keeps_leading_whitespace_outside_code_blocks
#[test]
fn partial_prose_keeps_leading_whitespace_outside_code_blocks() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:618 nested_tables_keep_their_copy_containers kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__nested_tables_keep_their_copy_containers
#[test]
fn nested_tables_keep_their_copy_containers() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:635 selected_hard_breaks_survive_surrounding_blocks kind=ui owner=chat-live
#[test]
fn selected_hard_breaks_survive_surrounding_blocks() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:674 selected_open_code_fence_keeps_streamed_rows_in_one_block kind=snap owner=chat-live
#[test]
fn selected_open_code_fence_keeps_streamed_rows_in_one_block() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:717 cross_cell_selection_keeps_boundary_breaks kind=ui owner=chat-live
#[test]
fn cross_cell_selection_keeps_boundary_breaks() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:741 adjacent_top_level_code_blocks_keep_blank_lines_and_boundaries kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__adjacent_top_level_code_blocks_keep_blank_lines_and_boundaries
#[test]
fn adjacent_top_level_code_blocks_keep_blank_lines_and_boundaries() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:754 adjacent_code_containers_remain_separate kind=ui owner=chat-live
#[test]
fn adjacent_code_containers_remain_separate() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_copy_tests.rs:780 selected_list_and_code_containers_keep_their_structure kind=ui owner=chat-live
#[test]
fn selected_list_and_code_containers_keep_their_structure() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ---- oracle: transcript_view/markdown_element_copy_tests.rs (6 screen-level tests here; 1 state tests left to chat-live)
// ORACLE transcript_view/markdown_element_copy_tests.rs:7 supported_elements_preserve_structure_at_narrow_and_wide_widths kind=ui owner=chat-live
#[test]
fn supported_elements_preserve_structure_at_narrow_and_wide_widths() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_element_copy_tests.rs:35 transformed_content_copies_visible_text_without_active_html_or_images kind=ui owner=chat-live
#[test]
fn transformed_content_copies_visible_text_without_active_html_or_images() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_element_copy_tests.rs:60 math_and_mermaid_copy_the_visible_rendering kind=ui owner=chat-live
#[test]
fn math_and_mermaid_copy_the_visible_rendering() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_element_copy_tests.rs:78 blockquote_selection_copies_only_selected_content kind=snap owner=chat-live
#[test]
fn blockquote_selection_copies_only_selected_content() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_element_copy_tests.rs:163 quoted_task_lists_preserve_checkbox_states kind=snap owner=chat-live
#[test]
fn quoted_task_lists_preserve_checkbox_states() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ORACLE transcript_view/markdown_element_copy_tests.rs:193 mouse_selected_blockquote_copies_without_markers kind=ui owner=chat-live
#[test]
fn mouse_selected_blockquote_copies_without_markers() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown or text of a selection and its HTML rendering (the clipboard payload of copy_selected_text_with), nothing drawn",
    );
}

// ---- oracle: transcript_view/mutations_tests.rs (11 screen-level tests here; 1 state tests left to chat-live)
// ORACLE transcript_view/mutations_tests.rs:35 page_join_holds_the_reading_revision_until_navigation_reaches_a_surviving_cell kind=ui owner=chat-live
#[test]
fn page_join_holds_the_reading_revision_until_navigation_reaches_a_surviving_cell() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 2)
        .script(include_str!("fixtures/transcript_view_3/page_join.jsonl"))
        .open();
    s.wheel_up(0, 0, 10);
    s.wheel_down(0, 0, 1);
    let before = s.shot();
    // the oracle prepends two groups and replaces the middle one: the join is not a kit step yet
    s.assert_same_rows(&before);
    s.wheel_down(0, 0, 1);
    s.assert_same_rows(&before);
    s.wheel_down(0, 0, 100);
    let latest = s.shot();
    let mut fresh = Screen::build(Owner::ChatLive)
        .size(20, 2)
        .script(include_str!("fixtures/transcript_view_3/page_join.jsonl"))
        .open();
    fresh.wheel_down(0, 0, 100);
    fresh.assert_same_rows(&latest);
}

// ORACLE transcript_view/mutations_tests.rs:78 page_join_keeps_selection_and_clearing_it_retains_the_reading_position kind=ui owner=chat-live
#[test]
fn page_join_keeps_selection_and_clearing_it_retains_the_reading_position() {
    // the oracle's rows `one`..`four` are four one-row turns here; its two pages of groups join
    // above them as one page (a page after the first is asked for where the reader is at the top,
    // and the selected row is not there), and nothing regroups: the page's rows are its own
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 8)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["one", "two", "three", "four"]);
    s.wheel_up(0, 0, 100);
    // three clicks on the row select `two`
    s.click_text("two");
    s.click_text("two");
    s.click_text("two");
    s.assert_text_style("two", "reversed");
    let before = s.shot();
    // the joins keep the selection and the rows
    s.join_older_page(&["earliest page", "ancient", "older group"], false);
    s.assert_same_rows(&before);
    s.resize(40, 8);
    s.assert_text_style("two", "reversed");
    // clearing the selection keeps the reading position
    s.keys("esc");
    s.assert_text_style("two", "!reversed");
    s.assert_contains("two");
    s.assert_absent("earliest page");
}

// ORACLE transcript_view/mutations_tests.rs:110 page_join_merges_unseen_groups_while_an_unrelated_selection_is_held kind=snap owner=chat-live
#[test]
fn page_join_merges_unseen_groups_while_an_unrelated_selection_is_held() {
    kit::na(
        Owner::ChatLive,
        "the page joins the oracle's `older group` and the `newer group` it merges into one `merged summary` cell (`replace_group`), shown at the top of the reader's snapshot; a page of the server holds whole turns and a group of tool calls is of one turn, so no entry of the first page merges with one of the older: the page joins above with its own rows and the selection held elsewhere stays (`pages::history_pages::a_page_joining_above_keeps_a_selection_held_elsewhere`)",
    );
}

// ORACLE transcript_view/mutations_tests.rs:151 page_join_keeps_groups_scrolled_into_view_after_selecting_elsewhere kind=ui owner=chat-live
#[test]
fn page_join_keeps_groups_scrolled_into_view_after_selecting_elsewhere() {
    kit::na(
        Owner::ChatLive,
        "the oracle replaces the two middle groups by `merged summary` while the reader scrolled to the top keeps the rows it drew and a selection of `last` elsewhere; nothing of the first page merges with another (a group of tool calls is of one turn, a page holds whole turns), so there is no replacement under the reader to hold: the rows and the selection stay when a page joins (`pages::history_pages::a_page_joining_above_keeps_a_selection_held_elsewhere`)",
    );
}

// ORACLE transcript_view/mutations_tests.rs:183 scrolling_forward_out_of_a_replaced_group_resolves_the_surviving_cell_index kind=ui owner=chat-live
#[test]
fn scrolling_forward_out_of_a_replaced_group_resolves_the_surviving_cell_index() {
    kit::na(
        Owner::ChatLive,
        "asserts the reading position and the held snapshot fields of the view after a scroll, nothing drawn",
    );
}

// ORACLE transcript_view/mutations_tests.rs:226 regrouping_the_tail_for_older_pages_does_not_report_new_activity kind=ui owner=chat-live
#[test]
fn regrouping_the_tail_for_older_pages_does_not_report_new_activity() {
    kit::na(
        Owner::ChatLive,
        "asserts the unseen_activity field after regrouping, a state field (its footer label is not asserted), nothing drawn",
    );
}

// ORACLE transcript_view/mutations_tests.rs:248 regrouping_preserves_new_activity_that_has_not_yet_been_drawn kind=ui owner=chat-live
#[test]
fn regrouping_preserves_new_activity_that_has_not_yet_been_drawn() {
    kit::na(
        Owner::ChatLive,
        "asserts the unseen_activity field after regrouping, a state field (its footer label is not asserted), nothing drawn",
    );
}

// ORACLE transcript_view/mutations_tests.rs:263 absorbing_older_history_keeps_live_readers_and_selections_on_their_displayed_revision kind=ui owner=chat-live
#[test]
fn absorbing_older_history_keeps_live_readers_and_selections_on_their_displayed_revision() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 2)
        .script(include_str!("fixtures/transcript_view_3/page_join.jsonl"))
        .open();
    s.wheel_up(0, 0, 2);
    let before = s.shot();
    // absorbing the older history and hydrating the live tail are not kit steps yet
    s.assert_same_rows(&before);
    s.assert_absent("updated live tail");
}

// ORACLE transcript_view/mutations_tests.rs:309 absorbing_a_read_canonical_tail_holds_it_until_navigation_leaves kind=ui owner=chat-live
#[test]
fn absorbing_a_read_canonical_tail_holds_it_until_navigation_leaves() {
    // the oracle's hydrated tail replaces the live one; here the live answer is replaced while
    // the reader is reading the older page, and the rows stay until the reader goes back
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 10)
        .replay(Vec::new())
        .view_only()
        .open();
    s.partial_history_of(&["header"]);
    s.live_tail("current live tail");
    s.join_older_page(&["older one", "older two", "older three"], false);
    s.wheel_up(0, 0, 100);
    let before = s.shot();
    s.replace_live_tail("hydrated group\n\ncurrent preview");
    s.assert_same_rows(&before);
    s.keys("ctrl+end");
    s.assert_contains("current preview");
}

// ORACLE transcript_view/mutations_tests.rs:335 absorbing_older_history_excludes_only_the_hydration_revision_from_new_activity kind=ui owner=chat-live
#[test]
fn absorbing_older_history_excludes_only_the_hydration_revision_from_new_activity() {
    kit::na(
        Owner::ChatLive,
        "asserts the unseen_activity field and the live tail text, state fields, nothing drawn",
    );
}

// ORACLE transcript_view/mutations_tests.rs:360 folding_a_reasoning_only_page_retains_its_find_and_copy_revision kind=ui owner=chat-live
#[test]
fn folding_a_reasoning_only_page_retains_its_find_and_copy_revision() {
    kit::na(
        Owner::ChatLive,
        "the oracle's `replace_group` folds a joined MCP call and the reasoning page into one cell that stands in place of two entries, and the view keeps the revision it found and copied in; the port's rows are the thread's items by key (a joined page adds rows above, nothing is replaced under the reader), so there is no revision to retain: the rows, the match and the copy do not move when an older page joins under Find (`transcript_view_4::empty_find_preserves_reading_inside_a_joined_group`, `pages::find_live_kit`)",
    );
}

// ---- oracle: transcript_view/prompt_header_tests.rs (5 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/prompt_header_tests.rs:28 prompt_header_tracks_reading_and_yields_to_visible_prompts_and_short_screens kind=snap owner=chat-live snap=prompt_header_tracks_reading_and_yields_to_visible_prompts_and_short_screens
#[test]
fn prompt_header_tracks_reading_and_yields_to_visible_prompts_and_short_screens() {
    // Use the real owned chat, with enough room below the reader for its composer.
    // Hard paragraph boundaries express the oracle's independent display rows.
    let question = "Second café question";
    let response = (0..35)
        .map(|row| format!("response paragraph {row:02}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let turn = kit::replay_turn(
        "header-turn",
        serde_json::json!([
            {"type":"userMessage","id":"prompt","content":[
                {"type":"text","text":question,"text_elements":[]}
            ]},
            {"type":"agentMessage","id":"response","text":response,"phase":"final_answer"}
        ]),
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(68, 16)
        .owned_transcript()
        .replay(vec![turn])
        .open();
    s.wait(300);
    s.assert_contains("response paragraph 34");
    s.assert_text_at(2, 0, question);
    s.wheel_up(3, 3, 1);
    s.assert_contains("Back to bottom");
    s.assert_text_at(2, 0, question);

    s.resize(68, 6);
    s.assert_absent(question);
    s.resize(68, 16);
    s.wait(300);
    s.assert_text_at(2, 0, question);
    s.click_text(question);
    s.wait(300);
    s.assert_contains("response paragraph 00");
    s.assert_text_count(question, 1);
}

// ORACLE transcript_view/prompt_header_tests.rs:92 header_sanitizes_controls_and_truncates_by_display_width kind=snap owner=chat-live snap=header_sanitizes_controls_and_truncates_by_display_width
#[test]
fn header_sanitizes_controls_and_truncates_by_display_width() {
    kit::pending(
        Owner::ChatLive,
        "rtwk gap G-new-usage-2 (a scrolled ListView does not report which items it shows, so a header pinned to the reading position cannot follow it: wanted `ListController::visible_range()`): the chat draws no pinned header row, so the sanitized and truncated header line (the oracle's line helper) is not on any screen yet",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(18, 4)
        .script(include_str!(
            "fixtures/transcript_view_3/sanitized_prompt.jsonl"
        ))
        .open();
    // the header line is sanitized (no control sequence) and truncated by display width at 18 columns
    s.assert_row(0, "› 界 café next q…");
}

// ORACLE transcript_view/prompt_header_tests.rs:103 pinned_prompt_matches_the_original_prompt_style kind=ui owner=chat-live
#[test]
fn pinned_prompt_matches_the_original_prompt_style() {
    kit::pending(
        Owner::ChatLive,
        "rtwk gap G-new-usage-2 (a scrolled ListView does not report which items it shows, so a header pinned to the reading position cannot follow it: wanted `ListController::visible_range()`): the chat draws no pinned prompt header, and the kit cannot compare the style of two cells: the header's style is written as the oracle's prompt style",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(30, 7)
        .script(include_str!("fixtures/transcript_view_3/keep_going.jsonl"))
        .open();
    s.wheel_down(0, 1, 6);
    s.assert_row(0, "› keep going");
    // the header paints the prompt's own style: the prompt's fg and bg on its row
    s.assert_style(0, 0, "fg=Rgb(238,238,238) bg=Rgb(68,68,68)");
}

// ORACLE transcript_view/prompt_header_tests.rs:133 mouse_selection_preserves_header_reservation_at_turn_boundaries kind=ui owner=chat-live
#[test]
fn mouse_selection_preserves_header_reservation_at_turn_boundaries() {
    kit::pending(
        Owner::ChatLive,
        "rtwk gap G-new-usage-2 (a scrolled ListView does not report which items it shows, so a header pinned to the reading position cannot follow it: wanted `ListController::visible_range()`): the chat draws no pinned header and no header reservation at a turn boundary; a mouse selection of the header row is not built",
    );
    for (width, height) in [(40, 6), (40, 7), (40, 8), (15, 6), (15, 7), (15, 8)] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(width, height)
            .script(include_str!(
                "fixtures/transcript_view_3/header_pairs.jsonl"
            ))
            .open();
        let before = s.shot();
        s.mouse_down(5, height - 1);
        s.assert_same_rows(&before);
        s.mouse_up(5, height - 1);
        s.assert_same_rows(&before);
        let mut d = Screen::build(Owner::ChatLive)
            .size(width, height)
            .script(include_str!(
                "fixtures/transcript_view_3/header_pairs.jsonl"
            ))
            .open();
        d.drag(5, height - 1, 13, height - 1);
        d.assert_same_rows(&before);
        d.assert_text_style("10:16 AM", "reversed");
    }
}

// ORACLE transcript_view/prompt_header_tests.rs:215 paused_header_reservation_survives_copy_until_scroll_or_resize kind=ui owner=chat-live
#[test]
fn paused_header_reservation_survives_copy_until_scroll_or_resize() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 7)
        .script(include_str!(
            "fixtures/transcript_view_3/header_pairs.jsonl"
        ))
        .open();
    let before = s.shot();
    s.double_click(0, 6);
    s.assert_same_rows(&before);
    // the copy confirmation is not drawn by the oracle's assertions; the reservation survives the copy
    s.resize(40, 8);
    s.assert_rows_differ(&before);
}

// ---- oracle: transcript_view/prompt_style_tests.rs (3 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/prompt_style_tests.rs:18 visible_rows_fill_inherited_styles_without_painting_outside_the_viewport kind=ui owner=chat-live
#[test]
fn visible_rows_fill_inherited_styles_without_painting_outside_the_viewport() {
    kit::pending(
        Owner::ChatLive,
        "the oracle lays out four styled rows directly (a synthetic layout no chat input produces): no seed for an arbitrary styled layout yet",
    );
    let s = Screen::build(Owner::ChatLive).size(13, 6).live().open();
    // the viewport (2,1) 9x4 shows three styled rows; nothing outside it is painted
    s.assert_style(2, 1, "fg=Green bg=DarkGray italic");
    s.assert_style(2, 2, "fg=Green bg=DarkGray italic bold");
    s.assert_style(3, 2, "fg=Cyan bg=DarkGray italic");
    s.assert_style(10, 3, "bg=DarkGray");
    s.assert_style(1, 1, "bg=Reset");
    s.assert_style(0, 0, "bg=Reset");
    s.assert_style(2, 5, "bg=Reset");
}

// ORACLE transcript_view/prompt_style_tests.rs:51 prompt_margin_and_blank_background_survive_resize_without_changing_copy kind=snap owner=chat-live snap=codex_tui__transcript_view__text__prompt_style_tests__prompt_margin_narrow
#[test]
fn prompt_margin_and_blank_background_survive_resize_without_changing_copy() {
    kit::pending(
        Owner::ChatLive,
        "the chat draws no prompt margin and blank background at 20 columns yet: the snapshot is a Buffer dump of the prompt layout",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 8)
        .script(include_str!(
            "fixtures/transcript_view_3/prompt_margin.jsonl"
        ))
        .open();
    s.resize(20, 8);
    s.assert_snap("prompt_margin_narrow");
}

// ORACLE transcript_view/prompt_style_tests.rs:111 reserved_margin_uses_the_same_alignment_for_painting_and_selection kind=ui owner=chat-live
#[test]
fn reserved_margin_uses_the_same_alignment_for_painting_and_selection() {
    kit::na(
        Owner::ChatLive,
        "rtwk uses the same render object to paint and provide selection offsets; a right-aligned Text with an inset is exercised in selection_units",
    );
}

// ---- oracle: transcript_view/right_click_copy_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/right_click_copy_tests.rs:12 right_click_retains_selection_until_confirmed_and_preserves_reading_position kind=snap owner=chat-live snap=codex_tui__transcript_view__input__right_click_copy_tests__right_click_retains_selection_until_confirmed_and_preserves_reading_position
#[test]
fn right_click_retains_selection_until_confirmed_and_preserves_reading_position() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 8)
        .script(include_str!("fixtures/transcript_view_3/right_click.jsonl"))
        .open();
    s.selection_copy_policy(false, true);
    s.wheel_up(0, 0, 1);
    let row = s.row_of("selected text");
    s.triple_click(0, u16::try_from(row).unwrap());
    s.assert_text_style("selected text", "reversed");
    let before = s.shot();
    for result in [Err("clipboard unavailable"), Ok(false), Ok(true)] {
        s.right_click(4, u16::try_from(row).unwrap());
        s.assert_copy_asked("selected text");
        let (feedback, selected) = match result {
            Err(error) => {
                s.answer_copy(Err(error));
                ("Copy failed", true)
            }
            Ok(false) => {
                s.answer_copy_unconfirmed();
                ("Copy sent to terminal", true)
            }
            Ok(true) => {
                s.answer_copy(Ok(()));
                ("Copied 13 chars", false)
            }
        };
        s.assert_contains(feedback);
        s.assert_same_rows_in(&before, row..row + 3);
        s.assert_text_style(
            "selected text",
            if selected { "reversed" } else { "!reversed" },
        );
    }
    s.right_click(4, u16::try_from(row).unwrap());
    s.assert_copies_asked(&["selected text", "selected text", "selected text"]);
}

// ORACLE transcript_view/right_click_copy_tests.rs:80 right_click_requires_a_nonempty_selection_and_a_press_inside_the_transcript kind=ui owner=chat-live
#[test]
fn right_click_requires_a_nonempty_selection_and_a_press_inside_the_transcript() {
    kit::na(
        Owner::ChatLive,
        "asserts the returned copy action (none) and the selected text, returned values, nothing drawn",
    );
}
