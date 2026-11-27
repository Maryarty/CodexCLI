//! Shard `transcript_view_5` (screens S10, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: transcript_view/source_cell_tests.rs (6 screen-level tests here; 1 state tests left to chat-live)
// ORACLE transcript_view/source_cell_tests.rs:20 reasoning_copy_joins_soft_wraps_and_retains_the_existing_rendering kind=ui owner=chat-live
#[test]
fn reasoning_copy_joins_soft_wraps_and_retains_the_existing_rendering() {
    kit::pending(
        Owner::ChatLive,
        "no transcript-only reasoning entry: a ReasoningSummaryCell has no script shape or debug entry, and the chat draws no 36-column transcript cell yet",
    );
    let s = Screen::build(Owner::ChatLive).size(36, 24).live().open();
    // The oracle compares the painted cells with the paragraph render of the same lines;
    // until the owner has a snapshot of that render, the literal words of the content are checked in order.
    s.assert_in_order(&["Investigate", "resize."]);
}

// ORACLE transcript_view/source_cell_tests.rs:39 plan_copy_retains_checkboxes_but_omits_wrapping_gutters kind=ui owner=chat-live
#[test]
fn plan_copy_retains_checkboxes_but_omits_wrapping_gutters() {
    kit::pending(
        Owner::ChatLive,
        "no plan-update entry: the chat has no update_plan cell and no script shape for a plan",
    );
    let s = Screen::build(Owner::ChatLive).size(38, 24).live().open();
    s.assert_in_order(&[
        "• Updated Plan",
        "Preserve the existing",
        "✔ Replace the terminal",
    ]);
}

// ORACLE transcript_view/source_cell_tests.rs:118 recap_copy_preserves_hard_lines_and_reflows_only_soft_wraps kind=ui owner=chat-live
#[test]
fn recap_copy_preserves_hard_lines_and_reflows_only_soft_wraps() {
    kit::pending(
        Owner::ChatLive,
        "no thread-recap entry: a ThreadRecapHistoryCell with a next action has no script shape, and the chat draws no recap yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_in_order(&[
        "The café review is paused at https://example.com/review/42.",
        "An indented hard line.",
        "Next: Review https://example.com/integration/report before continuing.",
    ]);
    let before = s.shot();
    s.resize(32, 24);
    s.resize(12, 24);
    s.resize(80, 24);
    s.assert_same_rows(&before);
}

// ORACLE transcript_view/source_cell_tests.rs:187 recap_tabs_preserve_url_wrapping_and_source_across_resize kind=ui owner=chat-live
#[test]
fn recap_tabs_preserve_url_wrapping_and_source_across_resize() {
    kit::pending(
        Owner::ChatLive,
        "no thread-recap entry with a leading tab and a URL: no script shape, and the chat draws no recap yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let before = s.shot();
    s.resize(32, 24);
    s.assert_row(0, "  ↳ Recap:      ");
    s.assert_row(1, "           https://example.com");
    s.resize(80, 24);
    s.assert_same_rows(&before);
}

// ORACLE transcript_view/source_cell_tests.rs:227 selected_recap_remains_visible_when_resize_cannot_fit_its_gutter kind=snap owner=chat-live snap=codex_tui__transcript_view__text__source_cell_tests__owned_recap_selected_narrow
#[test]
fn selected_recap_remains_visible_when_resize_cannot_fit_its_gutter() {
    kit::pending(
        Owner::ChatLive,
        "no thread-recap entry and no drag selection over a transcript cell yet; the oracle joins the narrow frames of two recaps in one snapshot",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.drag(0, 0, 45, 0);
    s.resize(12, 24);
    s.assert_snap("owned_recap_selected_narrow");
}

// ORACLE transcript_view/source_cell_tests.rs:293 resize_restores_the_exact_styles_of_whitespace_omitted_by_wrapping kind=ui owner=chat-live
#[test]
fn resize_restores_the_exact_styles_of_whitespace_omitted_by_wrapping() {
    kit::pending(
        Owner::ChatLive,
        "no styled-span entry (bold, red background, italic, dim and underlined spans across wrapped spaces) and no transcript cell at 24 columns yet",
    );
    let s = Screen::build(Owner::ChatLive).size(24, 2).live().open();
    s.assert_row(0, "alpha  beta gamma");
    s.assert_text_style("alpha", "bold");
    s.assert_style(5, 0, "bg=Red");
    s.assert_text_style("beta", "italic");
    s.assert_text_style("gamma", "underlined");
}

// ---- oracle: transcript_view/table_copy_tests.rs (3 screen-level tests here; 4 state tests left to chat-live)
// ORACLE transcript_view/table_copy_tests.rs:101 table_cells_keep_inline_formatting_alignment_and_literal_pipes kind=ui owner=chat-live
#[test]
fn table_cells_keep_inline_formatting_alignment_and_literal_pipes() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown of a table, re-rendered as HTML, at three widths (the clipboard payload of a selection), nothing drawn",
    );
}

// ORACLE transcript_view/table_copy_tests.rs:118 header_only_and_empty_cells_keep_their_table_positions kind=ui owner=chat-live
#[test]
fn header_only_and_empty_cells_keep_their_table_positions() {
    kit::na(
        Owner::ChatLive,
        "asserts the copied markdown of header-only and empty table cells (the clipboard payload of a selection), nothing drawn",
    );
}

// ORACLE transcript_view/table_copy_tests.rs:163 empty_selected_rows_keep_copy_spacing_without_newline_highlights kind=snap owner=chat-live snap=codex_tui__transcript_view__markdown_copy_tests__tables__empty_selected_rows_keep_copy_spacing_without_newline_highlights
#[test]
fn empty_selected_rows_keep_copy_spacing_without_newline_highlights() {
    kit::pending(
        Owner::ChatLive,
        "no markdown table entry (an AgentMarkdownCell with a table) and no drag selection that reverses the blank rows yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 20).live().open();
    s.drag(2, 0, 79, 19);
    s.assert_snap("empty_selected_rows_keep_copy_spacing_without_newline_highlights");
}

// ---- oracle: transcript_view/text_tabs_tests.rs (5 screen-level tests here; 0 state tests left to chat-live)
// ORACLE transcript_view/text_tabs_tests.rs:15 styled_tabs_use_row_stops_including_the_gutter kind=snap owner=chat-live snap=codex_tui__transcript_view__text__tab_tests__styled_tab_stops_and_selection
#[test]
fn styled_tabs_use_row_stops_including_the_gutter() {
    kit::pending(
        Owner::ChatLive,
        "no tabbed hyperlink entry with a styled prefix and two links, and no selection of a tab cell yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(35, 4).live().open();
    s.drag(6, 1, 7, 1);
    s.assert_snap("styled_tab_stops_and_selection");
}

// ORACLE transcript_view/text_tabs_tests.rs:69 tab_stops_restart_after_wrapping_and_resize_preserves_source_and_links kind=ui owner=chat-live
#[test]
fn tab_stops_restart_after_wrapping_and_resize_preserves_source_and_links() {
    kit::pending(
        Owner::ChatLive,
        "no tabbed hyperlink entry (a link after a tab, wrapped at ten columns) and no resize of a transcript cell yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(10, 3).live().open();
    let before = s.shot();
    s.resize(40, 3);
    s.resize(10, 3);
    s.assert_same_rows(&before);
    s.assert_link("X", "https://example.com/x");
}

// ORACLE transcript_view/text_tabs_tests.rs:102 leading_and_repeated_tabs_remain_selectable_below_one_tab_stop kind=ui owner=chat-live
#[test]
fn leading_and_repeated_tabs_remain_selectable_below_one_tab_stop() {
    kit::pending(
        Owner::ChatLive,
        "no entry with leading and repeated tabs before a wide grapheme, and no drag selection over tab cells yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(7, 4).live().open();
    for width in 1u16..8 {
        s.resize(width, 4);
        s.assert_contains("X");
        s.drag(0, 0, width - 1, 0);
        for column in 0..width {
            s.assert_style(usize::from(column), 0, "reversed");
        }
    }
}

// ORACLE transcript_view/text_tabs_tests.rs:131 ordinary_spaces_before_tabs_affect_the_stop_without_entering_the_gutter kind=ui owner=chat-live
#[test]
fn ordinary_spaces_before_tabs_affect_the_stop_without_entering_the_gutter() {
    kit::pending(
        Owner::ChatLive,
        "no entry with ordinary spaces before tabs in a 32-column row: no script shape yet",
    );
    let s = Screen::build(Owner::ChatLive).size(32, 4).live().open();
    // The oracle's row 0 is the first row of the cell; the chat's transcript row may sit lower.
    s.assert_text_at(8, 0, "X");
    s.assert_text_at(24, 0, "Y");
}

// ORACLE transcript_view/text_tabs_tests.rs:150 hard_wrapped_tabs_keep_source_characters_and_actual_row_stops kind=snap owner=chat-live snap=codex_tui__transcript_view__text__tab_tests__hard_wrapped_tabs
#[test]
fn hard_wrapped_tabs_keep_source_characters_and_actual_row_stops() {
    kit::pending(
        Owner::ChatLive,
        "no hard-wrap entry (a tabbed line at four columns, hard wrapped) and no transcript cell at that width yet",
    );
    let s = Screen::build(Owner::ChatLive).size(4, 4).live().open();
    s.assert_snap("hard_wrapped_tabs");
}

// ---- oracle: transcript_view/text_tests.rs (11 screen-level tests here; 7 state tests left to chat-live)
// ORACLE transcript_view/text_tests.rs:18 copying_wrapped_text_preserves_only_hard_newlines kind=ui owner=chat-live
#[test]
fn copying_wrapped_text_preserves_only_hard_newlines() {
    kit::na(
        Owner::ChatLive,
        "asserts the selected copy text of a streamed code block at nine columns (the clipboard payload), nothing drawn",
    );
}

// ORACLE transcript_view/text_tests.rs:65 scrolled_plaintext_selection_matches_painted_tab_and_word_boundaries kind=ui owner=chat-live
#[test]
fn scrolled_plaintext_selection_matches_painted_tab_and_word_boundaries() {
    kit::pending(
        Owner::ChatLive,
        "Needs: f-entries-tabs-styles, rtwk paints the tab as one space and puts B at column 4 instead of column 8",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(15, 3)
        .script(include_str!(
            "fixtures/transcript_view_5/scrolled_tab.jsonl"
        ))
        .view_only()
        .open();
    s.wheel_up(0, 0, 20);
    s.wheel_down(0, 0, 1);
    s.assert_text_at(8, 0, "B");
    s.assert_text_at(0, 2, "w");
    s.double_click(8, 0);
    s.assert_text_style("BCD", "reversed");
    s.assert_copy_asked("BCD");
}

// ORACLE transcript_view/text_tests.rs:105 visible_rows_preserve_styles_and_wrapped_hyperlinks kind=ui owner=chat-live
#[test]
fn visible_rows_preserve_styles_and_wrapped_hyperlinks() {
    const SOURCE: &str = r#"
{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"agent","text":"prefix at top **[guide that wraps](https://example.com/docs)** after the link","end":false}
"#;
    let mut s = Screen::build(Owner::ChatLive)
        .size(13, 2)
        .script(SOURCE)
        .view_only()
        .open();
    s.keys("ctrl+home");
    s.assert_contains("prefix at");
    s.assert_absent("that wraps");
    s.keys("down");
    s.assert_row(0, "  top guide");
    s.assert_row(1, "  that wraps");
    for fragment in ["guide", "that wraps"] {
        s.assert_text_style(fragment, "bold");
        s.assert_link(fragment, "https://example.com/docs");
    }
    s.assert_opened_urls(&[]);
    s.click_text("wraps");
    s.assert_opened_urls(&["https://example.com/docs"]);
}

// ORACLE transcript_view/text_tests.rs:123 selection_highlights_wide_graphemes_without_trailing_padding kind=ui owner=chat-live
#[test]
fn selection_highlights_wide_graphemes_without_trailing_padding() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(12, 5)
        .script(include_str!(
            "fixtures/transcript_view_5/wide_grapheme.jsonl"
        ))
        .view_only()
        .open();
    let row = s.row_of("a界b");
    let y = u16::try_from(row).unwrap();
    s.drag(3, y, 4, y);
    s.assert_copy_asked("界");
    for column in [3, 4] {
        s.assert_style(column, row, "reversed");
    }
    for column in [2, 5, 6, 7, 8, 9] {
        s.assert_style(column, row, "!reversed");
    }
}

// ORACLE transcript_view/text_tests.rs:140 selected_newlines_respect_wrapping_and_synthetic_rows kind=ui owner=chat-live
#[test]
fn selected_newlines_respect_wrapping_and_synthetic_rows() {
    kit::pending(
        Owner::ChatLive,
        "no entry with a disclosure control and a leading separator, and no selection that reverses the synthetic newline cell yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(10, 10).live().open();
    s.drag(2, 1, 5, 2);
    // The oracle's width-8 case: the newline cell after "alpha beta" is reversed at (6, 5).
    s.assert_style(6, 5, "reversed");
}

// ORACLE transcript_view/text_tests.rs:171 offsets_can_reach_rows_after_u16_limit kind=ui owner=chat-live
#[test]
fn offsets_can_reach_rows_after_u16_limit() {
    kit::pending(
        Owner::ChatLive,
        "screen feature: no 65,538-row seed (an agent message of 65,538 chars at width 1) and the chat draws nothing at 1x1; the kit wheels any count of notches (`wheel_down(x, y, u16::MAX)` then 2 more)",
    );
    let mut s = Screen::build(Owner::ChatLive).size(1, 1).live().open();
    s.wheel_down(0, 0, u16::MAX);
    s.wheel_down(0, 0, 2);
    s.assert_text_at(0, 0, "x");
}

// ORACLE transcript_view/text_tests.rs:190 aligned_text_excludes_synthetic_leading_columns kind=snap owner=chat-live snap=wide
#[test]
fn aligned_text_excludes_synthetic_leading_columns() {
    kit::pending(
        Owner::ChatLive,
        "no right-aligned hyperlinked entry ('wide') at eight columns: no script shape and no transcript cell yet",
    );
    let s = Screen::build(Owner::ChatLive).size(8, 1).live().open();
    s.assert_snap("wide");
    s.assert_link("wide", "https://example.com");
}

// ORACLE transcript_view/text_tests.rs:234 selected_revision_rewraps_both_directions_with_styles_and_indents kind=ui owner=chat-live
#[test]
fn selected_revision_rewraps_both_directions_with_styles_and_indents() {
    kit::pending(
        Owner::ChatLive,
        "no adaptive-wrapped entry with a link and indents, and no drag selection of a revised entry: the chat has neither yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(10, 3).live().open();
    let before = s.shot();
    s.resize(24, 3);
    s.assert_contains("alpha beta gamma");
    s.assert_link("beta gamma", "https://example.com");
    s.resize(10, 3);
    s.assert_same_rows(&before);
}

// ORACLE transcript_view/text_tests.rs:284 narrow_wrapping_keeps_joined_emoji_whole kind=ui owner=chat-live
#[test]
fn narrow_wrapping_keeps_joined_emoji_whole() {
    const SOURCE: &str = r#"
{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"agent","text":"👩‍💻Z","end":false}
"#;
    let mut s = Screen::build(Owner::ChatLive)
        .size(5, 4)
        .script(SOURCE)
        .view_only()
        .open();
    s.assert_in_order(&["👩‍💻", "Z"]);
    let row = s.row_of("👩‍💻");
    s.assert_row_ends_with(row, "👩‍💻");
    let y = u16::try_from(row).expect("emoji row");
    s.selection_copy_policy(false, false);
    s.drag(2, y, 3, y);
    s.keys("ctrl+c");
    s.assert_copies_asked(&["👩‍💻"]);
}

// ORACLE transcript_view/text_tests.rs:302 fullscreen_markdown_keeps_a_fitting_url_on_one_row kind=snap owner=chat-live
#[test]
fn fullscreen_markdown_keeps_a_fitting_url_on_one_row() {
    kit::pending(
        Owner::ChatLive,
        "no markdown answer entry with a URL at 44 columns, and no fullscreen transcript cell yet",
    );
    let s = Screen::build(Owner::ChatLive).size(44, 2).live().open();
    s.assert_row(0, "• See the documentation at");
    s.assert_row(1, "  https://example.com/docs/reference.");
}

// ORACLE transcript_view/text_tests.rs:356 command_and_output_copy_preserve_hard_lines_across_resize kind=snap owner=chat-live snap=compact_command_copy snap=detailed_command_copy
#[test]
fn command_and_output_copy_preserve_hard_lines_across_resize() {
    const COMMAND: &str = "printf 'alpha beta gamma delta'";
    const OUTPUT: &str = "  output alpha beta gamma delta\nnext line";
    const EXPECTED: &str =
        "printf 'alpha beta gamma delta'\n  output alpha beta gamma delta\nnext line";

    for detailed in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(64, 20).live().open();
        s.selection_copy_policy(false, false);
        s.feed(
            &serde_json::json!({"step":"notify","message":{"method":"item/completed","params":{
            "threadId":"thread-1","turnId":"turn-1","completedAtMs":0,"item":{
                "type":"commandExecution","id":"copy-test","command":COMMAND,"cwd":"/work",
                "processId":null,"source":"agent","status":"completed","commandActions":[],
                "aggregatedOutput":OUTPUT,"exitCode":0,"durationMs":null
            }}}})
            .to_string(),
        );
        if detailed {
            s.keys("ctrl+t");
        }
        let grid = s.grid();
        let (x, _, start) = grid.locate(COMMAND).expect("command starts on screen");
        let (_, after, end) = grid.locate("next line").expect("output is on screen");
        s.drag(x as u16, start as u16, (after - 1) as u16, end as u16);
        s.keys("ctrl+c");
        s.assert_copies_asked(&[EXPECTED]);
        s.answer_copy_unconfirmed();
        s.resize(24, 20);
        if !detailed {
            s.assert_absent(COMMAND);
            s.assert_contains("next line");
        }
        s.keys("ctrl+c");
        s.assert_copies_asked(&[EXPECTED, EXPECTED]);
        s.answer_copy_unconfirmed();
        s.resize(64, 20);
        s.keys("ctrl+c");
        s.assert_copies_asked(&[EXPECTED, EXPECTED, EXPECTED]);
    }
}

// ---- oracle: transcript_view_tests.rs (34 screen-level tests here; 3 state tests left to chat-live)
// ORACLE transcript_view_tests.rs:77 consecutive_user_prompts_share_one_background_block kind=snap owner=chat-live snap=consecutive_user_prompts_share_one_background_block
#[test]
fn consecutive_user_prompts_share_one_background_block() {
    // the oracle's frame is its `TranscriptView` (Ctrl+T), built from cells with no turns
    // around them: two steering prompts, an answer, a blank prompt and a later prompt, one
    // turn each, none with a start, end or duration (no `Worked for` row between them). The
    // chat keeps each prompt's own rows (chat_live_2's replayed review prompts).
    let turn = |id: &str, item: serde_json::Value| {
        let mut turn = kit::replay_turn(id, serde_json::json!([item]));
        turn["startedAt"] = serde_json::Value::Null;
        turn["completedAt"] = serde_json::Value::Null;
        turn["durationMs"] = serde_json::Value::Null;
        turn
    };
    let user = |id: &str, text: &str| {
        serde_json::json!({
            "type": "userMessage", "id": id, "clientId": null,
            "content": [{"type": "text", "text": text, "text_elements": []}],
            "startedAtMs": null, "completedAtMs": null
        })
    };
    let mut turns = vec![
        turn("turn-1", user("user-1", "First steering message")),
        turn("turn-2", user("user-2", "Second steering message")),
        turn(
            "turn-3",
            message_item("msg-3", "Assistant response", "final_answer"),
        ),
        turn("turn-4", user("user-4", "   ")),
        turn("turn-5", user("user-5", "Later user message")),
    ];
    turns.reverse();
    let mut s = Screen::build(Owner::ChatLive)
        .size(32, 24)
        .replay(turns)
        .terminal_colors(0xeb_eb_eb, 0x12_14_1e)
        .live()
        .open();
    s.keys("ctrl+t");
    let first = s
        .rows()
        .iter()
        .position(|row| row.starts_with("› First steering message"))
        .expect("the first prompt is drawn");
    s.assert_snap_region(
        "consecutive_user_prompts_share_one_background_block",
        first - 1,
        0,
    );
}

// ORACLE transcript_view_tests.rs:115 prompt_adjacency_uses_cached_retained_presentations kind=ui owner=chat-live
#[test]
fn prompt_adjacency_uses_cached_retained_presentations() {
    kit::na(
        Owner::ChatLive,
        "asserts how often a test cell's layout is built (a render counter of the layout cache), not a drawn row",
    );
}

// ORACLE transcript_view_tests.rs:186 page_keys_follow_pager_scroll_bindings kind=snap owner=chat-live snap=page_keys_follow_pager_scroll_bindings
#[test]
fn page_keys_follow_pager_scroll_bindings() {
    let configurations: [(&[(&str, &str)], u16); 4] = [
        (&[], 7),
        (
            &[
                ("pager.page_up", "f9"),
                ("pager.page_down", "f10"),
                ("pager.half_page_up", "page-up"),
                ("pager.half_page_down", "page-down"),
            ],
            4,
        ),
        (
            &[
                ("pager.page_up", "f9"),
                ("pager.page_down", "f10"),
                ("pager.scroll_up", "page-up"),
                ("pager.scroll_down", "page-down"),
            ],
            1,
        ),
        (&[("pager.page_up", "f9"), ("pager.page_down", "f10")], 0),
    ];
    for (bindings, delta) in configurations {
        let mut s = Screen::build(Owner::ChatLive)
            .size(36, 8)
            .script(include_str!("fixtures/transcript_view_5/rows30.jsonl"))
            .view_only()
            .keymap(bindings)
            .open();
        s.assert_row_ends_with(0, "row 25");
        let pre_scroll = s.shot();
        s.keys("pageup");
        s.assert_row_ends_with(0, &format!("row {}", 25 - delta));
        s.keys("pagedown");
        s.assert_same_rows(&pre_scroll);
    }
}

// ORACLE transcript_view_tests.rs:257 terminal_output_disclosure_follows_live_history_and_keymap kind=ui owner=chat-live
#[test]
fn terminal_output_disclosure_follows_live_history_and_keymap() {
    let mut command = command_item("output-test", "bash -lc echo");
    command["aggregatedOutput"] = serde_json::json!("1\n2\n3\n4\n5\n6\n7\n8\n");
    let s = live_plain(72, 14, &[item_line("item/completed", command)]);
    s.assert_contains("+ 5 lines (⌃t to expand)");
}

// ORACLE transcript_view_tests.rs:336 terminal_output_disclosure_counts_only_revealable_lines kind=snap owner=chat-live snap=terminal_output_disclosure_counts_only_revealable_lines
// The oracle joins five frames under bare labels the kit's snapshot reader does not split, so
// each frame is compared row by row from the snapshot's text.
#[test]
fn terminal_output_disclosure_counts_only_revealable_lines() {
    /// One frame: its name, the command, the output, the width, whether the output streams, and
    /// the rows the oracle draws.
    type Case = (
        &'static str,
        &'static str,
        String,
        u16,
        bool,
        &'static [&'static str],
    );
    let cases: [Case; 5] = [
        (
            "three visible",
            "echo",
            "a\nb\nc\n".to_owned(),
            64,
            false,
            &["• Ran echo", "  └ a", "    b", "    c"],
        ),
        (
            "one hidden",
            "echo",
            "a\nb\nc\nd\n".to_owned(),
            64,
            false,
            &[
                "• Ran echo",
                "  └ b",
                "    c",
                "    d",
                "    + 1 line (⌃t to expand)",
            ],
        ),
        (
            "clipped line",
            "echo",
            "abcdefghijklmnopqrstuvwxyz\n".to_owned(),
            20,
            false,
            &["• Ran echo", "  └ abcdefghijklmno…", "    + 1 line"],
        ),
        (
            "command only",
            "echo a\necho b",
            String::new(),
            64,
            false,
            &["• Ran echo a …", "  └ (no output)", "    + Show details"],
        ),
        (
            "storage truncated",
            "echo",
            "x\n".repeat(530_000),
            64,
            true,
            &[
                "• Running echo",
                "  └ x",
                "    x",
                "    x",
                "    + 97 lines (⌃t to expand)",
            ],
        ),
    ];
    for (_name, command, output, width, streamed, frame) in cases {
        let command = format!("bash -lc {}", shell_quote(command));
        let mut item = command_item("output-test", &command);
        let mut lines = Vec::new();
        if streamed {
            item["status"] = serde_json::json!("inProgress");
            item["aggregatedOutput"] = serde_json::Value::Null;
            item["exitCode"] = serde_json::Value::Null;
            item["durationMs"] = serde_json::Value::Null;
            lines.push(item_line("item/started", item));
            lines.push(notify_line(
                "item/commandExecution/outputDelta",
                serde_json::json!({
                    "threadId": "thread-1", "turnId": "turn-1", "itemId": "output-test",
                    "delta": output
                }),
            ));
        } else {
            item["aggregatedOutput"] = serde_json::json!(output);
            item["durationMs"] = serde_json::json!(1_000);
            lines.push(item_line("item/completed", item));
        }
        let s = live_plain(width, 14, &lines);
        let top = row_of(&s, if streamed { "• Running" } else { "• Ran" });
        let got: Vec<String> = s.rows().into_iter().skip(top).take(frame.len()).collect();
        kit::assert_rows_are(&got, frame);
    }
}

#[test]
fn a_streamed_shell_line_gains_a_width_aware_f4_stop() {
    let mut s = live_plain(20, 16, &[streaming_command("width-stop")]);
    s.feed(&command_delta("width-stop", "abc"));
    s.assert_absent("+ 1 line");
    s.keys("f4");
    s.assert_absent("enter expand");

    s.feed(&command_delta("width-stop", "defghijklmnopqrstuvwxyz"));
    s.assert_contains("+ 1 line");
    s.keys("f4").keys("enter");
    s.assert_has_row("  └ abcdefghijklmnop");
    s.assert_has_row("    qrstuvwxyz");
    s.assert_contains("Hide details");
}

#[test]
fn retained_command_output_is_searchable_and_copyable_with_its_storage_gap() {
    // The retained item fits in this viewport, independently of over-height reveal.
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 120)
        .live()
        .view_compact()
        .open();
    let id = "bounded-output";
    s.feed(&started_line());
    s.feed(&streaming_command(id));
    s.feed(&command_delta(id, "HEADTOKEN\n"));
    s.feed(&command_delta(id, &"retained head row\n".repeat(49)));
    s.feed(&command_delta(
        id,
        &"LOSTNEEDLE in the middle\n".repeat(45_000),
    ));
    s.feed(&command_delta(id, &"retained tail row\n".repeat(49)));
    s.feed(&command_delta(id, "TAILTOKEN\n"));
    s.assert_contains("+ 97 lines (⌃t to expand)");

    s.keys("/").type_text("LOSTNEEDLE");
    s.assert_find_query("LOSTNEEDLE");
    s.assert_find_footer(80, "No matches · full transcript · esc cancel");
    s.find_query("45000 lines");
    s.assert_text_style("45000 lines", "reversed");
    s.assert_contains("… +45000 lines");
    s.find_query("TAILTOKEN");
    s.assert_text_style("TAILTOKEN", "reversed");
    s.find_query("HEADTOKEN");
    s.assert_text_style("HEADTOKEN", "reversed");
    s.double_click_text("HEADTOKEN");
    s.keys("ctrl+c");
    s.assert_copy_asked("HEADTOKEN");
}

#[test]
fn a_byte_limited_ansi_preview_can_reveal_visible_output_after_the_cut() {
    let mut s = live_plain(64, 16, &[streaming_command("ansi-tail")]);
    let output = format!("{}VisibleSuffix", "\u{1b}[0m".repeat(5_000));
    s.feed(&command_delta("ansi-tail", &output));
    s.assert_absent("VisibleSuffix");
    s.assert_contains("+ 1 line");
    s.keys("f4").keys("enter");
    s.assert_contains("VisibleSuffix");
}

fn streaming_command(id: &str) -> String {
    let mut item = command_item(id, "bash -lc echo");
    item["status"] = serde_json::json!("inProgress");
    item["aggregatedOutput"] = serde_json::Value::Null;
    item["exitCode"] = serde_json::Value::Null;
    item["durationMs"] = serde_json::Value::Null;
    item_line("item/started", item)
}

fn command_delta(id: &str, delta: &str) -> String {
    notify_line(
        "item/commandExecution/outputDelta",
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": id, "delta": delta}),
    )
}

/// `text` as one shell word.
fn shell_quote(text: &str) -> String {
    if text.contains('\n') {
        format!("'{text}'")
    } else {
        text.to_owned()
    }
}

// ORACLE transcript_view_tests.rs:394 startup_warning_keeps_first_visible_header_in_place kind=ui owner=chat-live
#[test]
fn startup_warning_keeps_first_visible_header_in_place() {
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let before = s.shot();
    // The warning arrives below the header and the header is then inserted above it: no script shape yet.
    s.assert_same_rows(&before);
}

// ORACLE transcript_view_tests.rs:425 hidden_initial_entries_do_not_add_a_separator_but_visible_entries_do kind=snap owner=chat-live
#[test]
fn hidden_initial_entries_do_not_add_a_separator_but_visible_entries_do() {
    kit::pending(
        Owner::ChatLive,
        "no seed of hidden empty entries between visible ones with a live row 'Live': the oracle's empty cells have no script shape",
    );
    let mut s = Screen::build(Owner::ChatLive).size(20, 5).live().open();
    s.assert_row(0, "First");
    s.assert_row(1, "");
    s.assert_row(2, "Second");
    s.assert_row(3, "");
    s.assert_row(4, "Live");
    let frame = s.shot();
    s.wheel_up(0, 0, 100);
    s.assert_same_rows(&frame);
}

// ORACLE transcript_view_tests.rs:447 reading_survives_prepend_and_new_output_then_returns_to_latest kind=snap owner=chat-live
#[test]
fn reading_survives_prepend_and_new_output_then_returns_to_latest() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 12)
        .replay(Vec::new())
        .view_only()
        .open();
    s.history_page(
        vec![
            kit::agent_turn("turn-current", &["current", "last line"]),
            kit::agent_turn("turn-older", &["older", "line two"]),
        ],
        Some("cursor-1"),
    );
    s.wheel_up(0, 0, 60);
    let before = s.shot();
    // the older page is loaded and new output arrives: the reader's rows do not move
    s.history_page(kit::turns_of("loaded", 0..1), None);
    s.feed(include_str!("fixtures/transcript_view_5/new_output.jsonl"));
    s.assert_same_rows(&before);
    s.keys("ctrl+end");
    // the rows of a turn close with the `Worked for` row the oracle's cells have not: the latest
    // output and the line before it are in view together
    s.assert_contains("last line");
    s.assert_contains("new output");
    s.assert_absent("loaded 0");
}

// ORACLE transcript_view_tests.rs:495 beginning_wait_keeps_live_content_through_new_output_prepend_and_resize kind=ui owner=chat-live
#[test]
fn beginning_wait_keeps_live_content_through_new_output_prepend_and_resize() {
    let mut s = Screen::build(Owner::ChatLive).size(20, 2).live().open();
    s.keys("ctrl+home");
    let before = s.shot();
    // The older page arrives and the live tail streams new output: no script shape yet.
    s.assert_same_rows(&before);
    s.resize(12, 3);
    s.resize(20, 2);
    s.assert_same_rows(&before);
}

// ORACLE transcript_view_tests.rs:532 transcript_interaction_cancels_pending_beginning_without_losing_received_history kind=ui owner=chat-live
#[test]
fn transcript_interaction_cancels_pending_beginning_without_losing_received_history() {
    for interaction in ["wheel", "pageup", "f3", "ctrl+space"] {
        let mut s = Screen::build(Owner::ChatLive).size(24, 3).live().open();
        s.keys("ctrl+home");
        match interaction {
            "wheel" => {
                s.wheel_up(0, 0, 1);
            }
            other => {
                s.keys(other);
            }
        }
        let before = s.shot();
        // The older page is received after the interaction: no script shape yet.
        s.assert_same_rows(&before);
    }
}

// ORACLE transcript_view_tests.rs:578 returning_to_latest_cancels_beginning_intent_while_retaining_the_loaded_page kind=snap owner=chat-live
#[test]
fn returning_to_latest_cancels_beginning_intent_while_retaining_the_loaded_page() {
    for (beginning, latest) in [
        ("ctrl+home", "ctrl+end"),
        ("alt+<", "alt+>"),
        ("alt+shift+<", "alt+shift+>"),
        ("alt+shift+,", "alt+shift+."),
    ] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(20, 3)
            .replay(Vec::new())
            .view_only()
            .open();
        s.history_page(
            vec![kit::agent_turn("turn-new", &["newest answer", "last line"])],
            Some("cursor-1"),
        );
        let newest = s.shot();
        s.keys(beginning);
        s.keys(latest);
        // the older page arrives after the return: it is kept, and the reader stays at the latest
        s.history_page(kit::turns_of("older", 0..3), Some("cursor-2"));
        s.assert_same_rows(&newest);
        // the intent is gone: the arrival did not ask for the page after it
        s.assert_page_requests(2);
        s.keys(beginning);
        s.assert_page_requests(3);
    }
}

// ORACLE transcript_view_tests.rs:629 transcript_jumps_leave_typing_and_effort_shortcuts_to_the_composer kind=ui owner=chat-live
#[test]
fn transcript_jumps_leave_typing_and_effort_shortcuts_to_the_composer() {
    kit::na(
        Owner::ChatLive,
        "asserts that the transcript does not take the jump and typing keys (the handle_key result) and that its position stays latest, no row drawn",
    );
}

// ORACLE transcript_view_tests.rs:654 scrolling_to_latest_cancels_beginning_and_keeps_selection_when_the_final_page_arrives kind=ui owner=chat-live
#[test]
fn scrolling_to_latest_cancels_beginning_and_keeps_selection_when_the_final_page_arrives() {
    // the oracle's two cells are eight one-row turns here, so that the first row of the thread
    // and its latest row are not in view together; its replaced second cell is a regrouped
    // entry (`replace_group`, the state tests' own)
    for wheel in [false, true] {
        for select in [false, true] {
            let mut s = Screen::build(Owner::ChatLive)
                .size(30, 8)
                .replay(Vec::new())
                .view_only()
                .open();
            s.history_page(kit::turns_of("entry", 0..8), Some("cursor-1"));
            s.keys("ctrl+home");
            if select {
                s.click_text("entry 7");
                s.click_text("entry 7");
                s.click_text("entry 7");
            }
            if wheel {
                s.wheel_down(0, 0, 1);
            } else {
                s.keys("pagedown");
            }
            // the final older page arrives: the reader stays at the latest rows, and the
            // selection is kept
            s.history_page(kit::turns_of("final", 0..2), None);
            s.assert_contains("entry 7");
            s.assert_absent(">_ OpenAI Codex");
            if select {
                s.assert_text_style("entry 7", "reversed");
            }
        }
    }
}

// ORACLE transcript_view_tests.rs:708 single_row_scrolling_crosses_an_entry_separator_in_both_directions kind=ui owner=chat-live
#[test]
fn single_row_scrolling_crosses_an_entry_separator_in_both_directions() {
    let entries = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"entry","kind":"agent","text":"first","thread":"thread-1","turn":"t","item":"i","start":true,"end":false}
{"step":"entry","kind":"agent","text":"second","thread":"thread-1","turn":"t","item":"j","start":false,"end":false}"#;
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 1)
        .script(entries)
        .view_only()
        .open();
    s.mouse_scroll_speed(Some(1.0));
    s.assert_row(0, "• second");
    s.wheel_up(0, 0, 1);
    s.assert_row(0, "");
    s.wheel_up(0, 0, 1);
    s.assert_row(0, "• first");
    s.wheel_down(0, 0, 1);
    s.assert_row(0, "");
    s.wheel_down(0, 0, 1);
    s.assert_row(0, "• second");
}

// ORACLE transcript_view_tests.rs:723 selecting_within_an_edge_row_does_not_start_selection_autoscroll kind=snap owner=chat-live snap=horizontal_selection_at_edge_0
#[test]
fn selecting_within_an_edge_row_does_not_start_selection_autoscroll() {
    kit::pending(
        Owner::ChatLive,
        "Needs: G-new-sel-edge-zone: a horizontal drag on the top row starts vertical scrolling in the reverse ListView. The pin treats that terminal edge row as outside the viewport",
    );
    for edge in [0, 5] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(20, 6)
            .script(include_str!("fixtures/transcript_view_5/rows40.jsonl"))
            .view_only()
            .open();
        s.wheel_up(0, 0, 12);
        let before = s.shot();
        s.mouse_down(2, edge);
        s.wait(500);
        s.assert_same_rows(&before);
        s.mouse_drag(4, edge);
        s.assert_same_rows(&before);
        s.assert_style(3, usize::from(edge), "reversed");
        s.wait(500);
        s.assert_same_rows(&before);
        s.mouse_up(4, edge);
        s.assert_copy_asked("row");
    }
}

// ORACLE transcript_view_tests.rs:811 wheel_scrolling_pauses_selection_autoscroll_until_the_next_drag kind=snap owner=chat-live snap=wheel_stops_selection_autoscroll
#[test]
fn wheel_scrolling_pauses_selection_autoscroll_until_the_next_drag() {
    kit::pending(
        Owner::ChatLive,
        "Needs: G-new-sel-wheel-pause: the native wheel handler leaves the edge timer and its last drag position armed. The visible rows move during the 500 ms pause",
    );
    for up in [true, false] {
        let mut s = Screen::build(Owner::ChatLive)
            .size(20, 6)
            .script(include_str!("fixtures/transcript_view_5/rows40.jsonl"))
            .view_only()
            .open();
        s.wheel_up(0, 0, 12);
        s.mouse_down(2, 2);
        let edge = if up { 0 } else { 5 };
        let wheel = |s: &mut Screen| {
            if up {
                s.wheel_up(4, edge, 1);
            } else {
                s.wheel_down(4, edge, 1);
            }
        };
        wheel(&mut s);
        s.mouse_drag(4, edge);
        let scrolling = s.shot();
        s.wait(100);
        s.assert_rows_differ(&scrolling);
        wheel(&mut s);
        let paused = s.shot();
        s.wait(500);
        s.assert_same_rows(&paused);
        s.mouse_drag(4, edge);
        s.wait(100);
        s.assert_rows_differ(&paused);
        s.mouse_up(4, edge);
    }
}

// ORACLE transcript_view_tests.rs:879 only_visible_history_is_laid_out_and_repeated_frames_reuse_it kind=ui owner=chat-live
#[test]
fn only_visible_history_is_laid_out_and_repeated_frames_reuse_it() {
    kit::na(
        Owner::ChatLive,
        "asserts how many history entries are laid out (a render counter over ten thousand test cells), not a drawn row",
    );
}

// ORACLE transcript_view_tests.rs:906 selection_copies_source_across_wraps_and_survives_prepend kind=ui owner=chat-live
#[test]
fn selection_copies_source_across_wraps_and_survives_prepend() {
    kit::na(
        Owner::ChatLive,
        "asserts the selected copy text across wraps and a prepend (the clipboard payload), nothing drawn",
    );
}

// ORACLE transcript_view_tests.rs:925 resize_keeps_the_logical_reading_anchor kind=ui owner=chat-live
#[test]
fn resize_keeps_the_logical_reading_anchor() {
    kit::pending(
        Owner::ChatLive,
        "no entry replacement and no new output arriving while reading, and no reading anchor kept across a resize in the chat",
    );
    let mut s = Screen::build(Owner::ChatLive).size(10, 2).live().open();
    s.wheel_up(0, 0, 2);
    s.resize(20, 2);
    // The entry is replaced by "final answer" and new output arrives: no script shape yet.
    s.assert_row(0, "final answer");
    s.assert_row(1, "");
    s.assert_contains("New activity");
}

// ORACLE transcript_view_tests.rs:954 zero_sized_viewport_is_safe_and_empty_history_accepts_navigation kind=ui owner=chat-live
#[test]
fn zero_sized_viewport_is_safe_and_empty_history_accepts_navigation() {
    kit::na(
        Owner::ChatLive,
        "asserts that a zero-sized viewport and empty history take navigation without a panic and select nothing, no row drawn",
    );
}

// ORACLE transcript_view_tests.rs:987 mutable_history_formats_once_per_frame_and_refreshes_the_next_frame kind=snap owner=chat-live
#[test]
fn mutable_history_formats_once_per_frame_and_refreshes_the_next_frame() {
    kit::pending(
        Owner::ChatLive,
        "no mutable transcript cell (its text changes between frames) and no transcript search (begin, paste, advance) in the chat yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(24, 5).live().open();
    s.assert_row(0, "version 2 row 3");
    s.assert_row(1, "version 2 row 4");
    s.assert_row(2, "version 2 row 5");
    s.assert_row(3, "version 2 row 6");
    s.assert_row(4, "version 2 row 7");
    // The transcript search of "version 2", then ctrl+n: the chat has no search yet.
    s.keys("ctrl+f");
    s.type_text("version 2");
    s.keys("enter");
    s.keys("ctrl+n");
    s.assert_contains("version 3");
}

// ORACLE transcript_view_tests.rs:1037 selected_latest_content_keeps_new_activity_visible kind=snap owner=chat-live
#[test]
fn selected_latest_content_keeps_new_activity_visible() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 13)
        .script(include_str!(
            "fixtures/transcript_view_4/sl_command_c.jsonl"
        ))
        .open();
    s.selection_copy_policy(false, false);
    s.double_click_text("selected");
    s.assert_text_style("selected", "reversed");
    s.wheel_down(4, s.row_of("selected") as u16, 10);
    s.live_tail("new output");
    s.assert_contains("New activity");
    s.assert_absent("new output");
    s.assert_contains("⌃c copy");
    s.assert_absent("Back to bottom");
    s.assert_text_style("selected", "reversed");
    let reading_row = s.row_of("selected");
    s.live_tail(" another chunk");
    kit::assert_same(
        s.row_of("selected"),
        reading_row,
        "streaming leaves the reader in place",
    );
    s.assert_absent("another chunk");
    s.assert_text_style("selected", "reversed");
    s.keys("ctrl+c");
    s.assert_copies_asked(&["selected"]);
    s.answer_copy(Ok(()));
    s.assert_text_style("selected", "!reversed");
    s.assert_contains("New activity");
    s.assert_absent("new output");
    s.keys("ctrl+end");
    s.assert_contains("new output another chunk");
    s.assert_absent("New activity");
}

// ORACLE transcript_view_tests.rs:1050 live_reader_survives_commit_new_tail_resize_and_selection_then_rejoins_latest kind=snap owner=chat-live
#[test]
fn live_reader_survives_commit_new_tail_resize_and_selection_then_rejoins_latest() {
    kit::pending(
        Owner::ChatLive,
        "G-new-sel-reveal: resizing a reading anchor inside a multiline entry loses its in-item display row; the viewport returns to the preceding entry instead of the same live line",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 4)
        .replay(Vec::new())
        .view_only()
        .open();
    s.selection_copy_policy(false, false);
    s.partial_history_of(&["before"]);
    s.live_tail("live zero\nlive one\nlive two\nlive three\nlive four\nlive five");
    s.wheel_up(0, 0, 2);
    let before = s.shot();
    s.commit_live_tail();
    s.live_tail("new tail");
    s.assert_same_rows(&before);
    s.resize(9, 4);
    let row = u16::try_from(s.row_of("live two")).unwrap();
    s.triple_click(4, row);
    s.assert_text_style("live two", "reversed");
    s.keys("esc");
    s.assert_text_style("live two", "!reversed");
    s.keys("ctrl+end");
    s.assert_contains("new tail");
}

// ORACLE transcript_view_tests.rs:1109 scrolling_out_of_held_live_content_releases_its_revision kind=ui owner=chat-live
#[test]
fn scrolling_out_of_held_live_content_releases_its_revision() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(20, 2)
        .view_only()
        .live()
        .open();
    s.mouse_scroll_speed(Some(1.0));
    s.live_tail("before").commit_live_tail();
    let prefix = "zero  \none  \ntwo  \nthree  \nfour  \n";
    s.live_tail(&format!("{prefix}five"));
    s.assert_contains("five");

    s.wheel_up(0, 0, 2);
    s.assert_absent("five");
    let held = s.shot();
    // The live item leaves the stream for the committed transcript while the reader is parked.
    let complete = |last: &str| {
        notify_line(
            "item/completed",
            serde_json::json!({
                "threadId": "thread-1", "turnId": "live-tail-1", "completedAtMs": 1,
                "item": {"type": "agentMessage", "id": "live-tail-1-answer",
                    "text": format!("{prefix}{last}"), "phase": null}
            }),
        )
    };
    s.feed(&complete("five"));
    s.assert_same_rows(&held);
    s.wheel_up(0, 0, 100);
    s.assert_contains("before");
    s.assert_absent("five");

    s.wheel_down(0, 0, 100);
    s.assert_contains("five");
    // The reader follows the next revision as soon as it arrives, without another wheel event.
    s.feed(&complete("new end"));
    s.assert_contains("new end");
    s.assert_absent("five");
}

// ORACLE transcript_view_tests.rs:1134 repeated_resize_preserves_plain_and_markdown_passages_before_the_next_draw kind=ui owner=chat-live
#[test]
fn repeated_resize_preserves_plain_and_markdown_passages_before_the_next_draw() {
    kit::pending(
        Owner::ChatLive,
        "G-new-reader-reflow-anchor: a paused production Markdown paragraph loses café28 on 18 → 34 → 12 columns and returns to café38 at the tail",
    );
    let passage = (0..40)
        .map(|index| format!("café{index:02}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut s = Screen::build(Owner::ChatLive).size(18, 10).live().open();
    s.live_tail("older entry").commit_live_tail();
    s.live_tail(&passage).commit_live_tail();
    s.keys("pageup");
    let original = s.shot();
    let first = s
        .rows()
        .into_iter()
        .find_map(|row| {
            row.split_whitespace()
                .find(|word| word.starts_with("café"))
                .map(str::to_owned)
        })
        .expect("the visible Unicode passage");
    s.resize(34, 10);
    s.assert_contains(&first);
    s.resize(12, 10);
    s.assert_contains(&first);
    s.resize(18, 10);
    s.assert_same_rows_in(&original, 0..5);
}

// ORACLE transcript_view_tests.rs:1175 resizing_repeated_lines_keeps_the_same_occurrence kind=snap owner=chat-live
#[test]
fn resizing_repeated_lines_keeps_the_same_occurrence() {
    kit::pending(
        Owner::ChatLive,
        "no entry of repeated lines and no replaced older group in the chat yet; the oracle's wide and narrow rows are inline snapshots",
    );
    let mut s = Screen::build(Owner::ChatLive).size(12, 2).live().open();
    s.wheel_down(0, 0, 3);
    s.resize(24, 2);
    s.assert_row(0, "alpha beta gamma delta");
    s.assert_row(1, "second occurrence ends");
    s.resize(12, 2);
    s.assert_row(0, "gamma delta");
    s.assert_row(1, "second");
}

// ORACLE transcript_view_tests.rs:1208 resize_uses_the_passage_from_the_current_live_or_replaced_entry kind=ui owner=chat-live
#[test]
fn resize_uses_the_passage_from_the_current_live_or_replaced_entry() {
    for _kind in ["live", "replaced"] {
        let mut s = Screen::build(Owner::ChatLive).size(18, 3).live().open();
        s.wheel_up(0, 0, 5);
        // The passage is revised as the live tail or as the replaced entry: no script shape yet.
        let current = s.shot();
        let first_word = s
            .rows()
            .first()
            .and_then(|row| row.split_whitespace().next())
            .unwrap_or("")
            .to_string();
        s.resize(34, 3);
        s.assert_contains(&first_word);
        s.resize(18, 3);
        s.assert_same_rows(&current);
    }
}

// ORACLE transcript_view_tests.rs:1268 history_prefetch_counts_rows_in_preceding_entries kind=ui owner=chat-live
#[test]
fn history_prefetch_counts_rows_in_preceding_entries() {
    kit::na(
        Owner::ChatLive,
        "asserts the near-start flag of the history prefetch (near_start), not a drawn row",
    );
}

// ORACLE transcript_view_tests.rs:1289 user_prompt_has_a_background_without_a_terminal_palette kind=snap owner=chat-live snap=user_prompt_has_a_background_without_a_terminal_palette
#[test]
fn user_prompt_has_a_background_without_a_terminal_palette() {
    kit::pending(
        Owner::ChatLive,
        "the chat draws user prompts without a background block when the terminal has no palette yet",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(32, 8)
        .script(include_str!("fixtures/transcript_view_5/one_prompt.jsonl"))
        .open();
    s.assert_snap("user_prompt_has_a_background_without_a_terminal_palette");
}

// ORACLE transcript_view_tests.rs:1315 spoken_prompt_selection_keeps_its_fallback_contrast kind=ui owner=chat-live
#[test]
fn spoken_prompt_selection_keeps_its_fallback_contrast() {
    kit::pending(
        Owner::ChatLive,
        "no spoken prompt entry ('one' then 'next') and no drag selection over a prompt's rows with the fallback palette in the chat",
    );
    for detailed in [false, true] {
        let mut s = Screen::build(Owner::ChatLive).size(20, 5).live().open();
        if detailed {
            s.keys("ctrl+t");
        }
        let rows = s.rows();
        let first = rows
            .iter()
            .position(|row| row.contains("› one"))
            .unwrap_or(0);
        let second = rows
            .iter()
            .position(|row| row.contains("next"))
            .unwrap_or(0);
        let first16 = u16::try_from(first).unwrap_or(0);
        let second16 = u16::try_from(second).unwrap_or(0);
        s.drag(2, first16, 2, second16);
        s.assert_style(0, first, "fg=Red");
        s.assert_style(5, first, "fg=Rgb(238,238,238) bg=Rgb(68,68,68) reversed");
    }
}

// ---- the finished turn's disclosures: helpers

/// A chat of a resumed thread of one turn without a start, end or duration (as the oracle's
/// `app_server_turn` makes one: no `Worked for` separator, so the folded turn reads
/// `N previous messages`), the turn details folded.
fn folded_turn(items: serde_json::Value, width: u16, height: u16) -> Screen {
    let mut turn = kit::replay_turn("turn-1", items);
    turn["startedAt"] = serde_json::Value::Null;
    turn["completedAt"] = serde_json::Value::Null;
    turn["durationMs"] = serde_json::Value::Null;
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .replay(vec![turn])
        .live()
        .open();
    s.collapse_turn_details();
    s
}

fn user_item(text: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "userMessage", "id": "user-1", "clientId": null,
        "content": [{"type": "text", "text": text, "text_elements": []}],
        "startedAtMs": null, "completedAtMs": null
    })
}

fn message_item(id: &str, text: &str, phase: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "agentMessage", "moderation": null, "id": id, "text": text, "phase": phase,
        "memoryCitation": null, "delivery": null, "questions": null,
        "startedAtMs": null, "completedAtMs": null
    })
}

/// A legacy message: the server named no phase.
fn message_item_without_phase(id: &str, text: &str) -> serde_json::Value {
    let mut item = message_item(id, text, "");
    item["phase"] = serde_json::Value::Null;
    item
}

fn command_item(id: &str, command: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "commandExecution", "id": id, "command": command, "cwd": "/workspace",
        "processId": null, "source": "agent", "status": "completed", "commandActions": [],
        "aggregatedOutput": "output sentinel", "exitCode": 0, "durationMs": 0
    })
}

fn search_item(id: &str, query: &str) -> serde_json::Value {
    serde_json::json!({"type": "webSearch", "id": id, "query": query, "action": {"type": "other"}})
}

/// The screen row of the first row that starts with `text` (leading spaces ignored).
fn row_of(s: &Screen, text: &str) -> usize {
    let Some(at) = s
        .rows()
        .iter()
        .position(|row| row.trim_start().starts_with(text))
    else {
        panic!("no row starts with {text:?} on the screen:\n{}", s.text());
    };
    at
}

/// A `notify` line of the server: `method` with `params`.
fn notify_line(method: &str, params: serde_json::Value) -> String {
    format!(
        "{}\n",
        serde_json::json!({"step": "notify", "message": {"method": method, "params": params}})
    )
}

/// `turn/started` of `turn-1` in `thread-1`.
fn started_line() -> String {
    notify_line(
        "turn/started",
        serde_json::json!({"threadId": "thread-1", "turn": {
            "id": "turn-1", "rootTurnId": null, "items": [], "itemsView": "full",
            "status": "inProgress", "error": null, "startedAt": null, "completedAt": null,
            "durationMs": null
        }}),
    )
}

/// `turn/completed` of `turn-1`, which took `duration_ms`.
fn completed_line(duration_ms: u64) -> String {
    notify_line(
        "turn/completed",
        serde_json::json!({"threadId": "thread-1", "turn": {
            "id": "turn-1", "rootTurnId": null, "items": [], "itemsView": "summary",
            "status": "completed", "error": null, "startedAt": null, "completedAt": null,
            "durationMs": duration_ms
        }}),
    )
}

/// `item/started` or `item/completed` of `item` in `turn-1`.
fn item_line(method: &str, item: serde_json::Value) -> String {
    notify_line(
        method,
        serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "item": item,
            "startedAtMs": 0, "completedAtMs": 0}),
    )
}

/// A live chat (thread `thread-1`) with the turn details folded, fed its turn's start and
/// then `lines`.
fn live_with(width: u16, height: u16, lines: &[String]) -> Screen {
    let mut s = live_plain(width, height, lines);
    s.collapse_turn_details();
    s
}

/// A live chat (thread `thread-1`) fed its turn's start and then `lines`.
fn live_plain(width: u16, height: u16, lines: &[String]) -> Screen {
    let mut s = Screen::build(Owner::ChatLive)
        .size(width, height)
        .script("{\"step\": \"seed\", \"seed\": \"session\", \"thread\": \"thread-1\"}\n")
        .open();
    s.feed(&started_line());
    for line in lines {
        s.feed(line);
    }
    s
}

// ORACLE transcript_view_tests.rs:1391 tool_disclosure_hides_output kind=snap owner=chat-live
// F4 + Enter is a click on the control (the activity focus is a session-page key).
#[test]
fn tool_disclosure_hides_output() {
    let mut s = live_with(
        40,
        14,
        &[item_line(
            "item/completed",
            serde_json::json!({
                "type": "imageGeneration", "id": "image", "status": "failed",
                "revisedPrompt": "private prompt", "failure": null, "savedPath": null,
                "generationId": null, "result": ""
            }),
        )],
    );
    row_of(&s, "▸ Image generation failed");
    s.assert_absent("private prompt");
    s.click_text("▸ Image generation failed");
    // The list keeps its end in view, so the opened rows grew upward.
    let at = row_of(&s, "▾ Image generation failed");
    s.assert_row(at + 1, "      Image generation failed");
}

// ORACLE transcript_view_tests.rs:1425 tool_groups_expand_independently_inside_a_completed_turn kind=snap owner=chat-live
// The oracle's F4/Enter steps are clicks here (the pager's activity focus is a
// session-page key), and its tail (find expands only the match, a selection
// across the fold, the pager shows the output) is the find, selection and
// pager chunks' (transcript-2-find, -select-mode, pager). The chat draws the
// transcript above the composer, so the rows are the oracle's from the prompt's row on.
#[test]
fn tool_groups_expand_independently_inside_a_completed_turn() {
    let mut s = folded_turn(
        serde_json::json!([
            user_item("Check why the service writes a diagnostic to stderr each time it starts."),
            message_item("msg-1", "Checking stderr.", "commentary"),
            command_item("hello", "echo hello"),
            command_item("world", "echo world"),
            message_item("msg-2", "The path is valid.", "partial_answer"),
            message_item("msg-3", "The fixture is missing.", "final_answer"),
        ]),
        52,
        22,
    );
    let top = row_of(&s, "› Check why the service writes a diagnostic to");
    s.assert_row(top + 1, "  stderr each time it starts.");
    s.assert_row(top + 4, "▸ 3 previous messages");
    s.assert_row(top + 6, "• The path is valid.");
    s.assert_row(top + 8, "• The fixture is missing.");
    s.assert_absent("Checking stderr.");
    s.click_text("▸ 3 previous messages");
    s.assert_contains("▾ 3 previous messages");
    s.assert_contains("Checking stderr.");
    s.assert_contains("▸ Ran 2 commands");
    s.assert_absent("Ran echo hello");
    s.click_text("▸ Ran 2 commands");
    s.assert_contains("▾ Ran 2 commands");
    s.assert_contains("Ran echo hello");
    s.assert_contains("Ran echo world");
    s.assert_absent("output sentinel");
}

// ORACLE transcript_view_tests.rs:1533 legacy_progress_stays_visible_until_a_final_answer kind=snap owner=chat-live
// F4 + Enter is a click on the control. Not here (the history page and selection chunks'): the
// older image call prepended to the group while a double-click selection is held, and the
// second turn after the review separator.
#[test]
fn legacy_progress_stays_visible_until_a_final_answer() {
    let mut s = live_with(
        52,
        22,
        &[
            item_line("item/started", user_item("Check the fixture")),
            item_line(
                "item/completed",
                message_item_without_phase("msg-1", "Intermediate progress"),
            ),
            item_line("item/completed", search_item("search", "fixture")),
        ],
    );
    let top = row_of(&s, "› Check the fixture");
    s.assert_row(top + 3, "• Intermediate progress");
    s.assert_row(top + 5, "• Searched the web for fixture");
    s.feed(&item_line(
        "item/completed",
        search_item("search-2", "docs"),
    ));
    s.assert_contains("⌕ 2 tool calls · Searched the web for docs");
    s.click_text("⌕ 2 tool calls");
    s.assert_in_order(&[
        "      Searched the web for fixture",
        "      Searched the web for docs",
    ]);
    s.feed(&item_line(
        "item/completed",
        message_item_without_phase("msg-2", "Fixture verified"),
    ));
    s.feed(&completed_line(2_000));
    s.assert_contains("▸ Worked for 2s");
    s.assert_contains("Fixture verified");
    s.assert_absent("Intermediate progress");
}

// ORACLE transcript_view_tests.rs:1626 live_search_group_keeps_its_summary_in_the_parked_pane kind=snap owner=chat-live
// The disclosure is clicked at its row (the oracle's column 4 of the parked pane's row 0);
// the selection held across the completion is the select-mode chunk's.
#[test]
fn live_search_group_keeps_its_summary_in_the_parked_pane() {
    let search = |id: &str, query: &str| {
        serde_json::json!({
            "type": "commandExecution", "id": id, "command": format!("rg -l {query} src"),
            "cwd": "/workspace", "processId": null, "source": "agent", "status": "inProgress",
            "commandActions": [{
                "type": "search", "command": format!("rg -l {query} src"),
                "query": query, "path": "src"
            }],
            "aggregatedOutput": null, "exitCode": null, "durationMs": null
        })
    };
    let mut s = live_with(
        40,
        14,
        &[item_line("item/started", search("first", "TODO"))],
    );
    s.assert_has_row("    ⌕ Searching for TODO in src");
    s.feed(&item_line("item/started", search("second", "FIXME")));
    // The control sits a quarter of the width in (four columns) and is cut at the edge.
    let at = row_of(&s, "⌕ 2 tool calls");
    s.assert_row(at, "    ⌕ 2 tool calls · Searching for FIXME");
    s.click(4, u16::try_from(at).unwrap_or(0));
    let at = row_of(&s, "▾ 2 tool calls");
    s.assert_row(at, "    ▾ 2 tool calls · Searching for FIXME");
    s.assert_row(at + 1, "      Searching for TODO in src");
    s.assert_row(at + 2, "      Searching for FIXME in src");
}
