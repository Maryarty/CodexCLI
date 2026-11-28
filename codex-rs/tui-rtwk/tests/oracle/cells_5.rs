//! Shard `cells_5` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const PLAN_FRAMES: &str = "G49: the Full frame (the inline viewport) breaks `retry/backoff` where the oracle breaks after the slash (`retry/` / `backoff`); the Compact frame equals the oracle's";
const PLAN_URL_TOKENS: &str = "G-urltoken: the oracle wraps a URL token whole (adaptive_wrap_hyperlink_lines keeps it on one row even wider than the screen); TextWrap::Word breaks it after a hyphen or a slash, so the contiguous literal is never on one row";

/// The row of the transcript's first cell: the oracle's snapshot is the bare cell, the chat
/// draws its bottom pane under the transcript, so the cell is looked up by its bullet.
fn first_cell_row(screen: &Screen) -> usize {
    screen
        .rows()
        .iter()
        .position(|row| row.starts_with("• ") && !row.starts_with("• Working"))
        .unwrap_or_else(|| panic!("missing cell row:\n{}", screen.text()))
}

// ---- oracle: history_cell/tests.rs (39 screen-level tests here; 3 state tests left to chat-live)
// ORACLE history_cell/tests.rs:2205 multiline_command_without_wrap_uses_branch_then_eight_spaces kind=snap owner=chat-live snap=multiline_command_without_wrap_uses_branch_then_eight_spaces
#[test]
fn multiline_command_without_wrap_uses_branch_then_eight_spaces() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .inline(&[])
        .script(include_str!(
            "fixtures/cells_5/multiline_command_without_wrap_uses_branch_then_eight_spaces.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "multiline_command_without_wrap_uses_branch_then_eight_spaces",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2228 multiline_command_both_lines_wrap_with_correct_prefixes kind=snap owner=chat-live snap=multiline_command_both_lines_wrap_with_correct_prefixes
#[test]
fn multiline_command_both_lines_wrap_with_correct_prefixes() {
    let s = Screen::build(Owner::ChatLive)
        .size(28, 24)
        .inline(&[])
        .script(include_str!(
            "fixtures/cells_5/multiline_command_both_lines_wrap_with_correct_prefixes.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "multiline_command_both_lines_wrap_with_correct_prefixes",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2252 stderr_tail_more_than_five_lines_snapshot kind=snap owner=chat-live snap=stderr_tail_more_than_five_lines_snapshot
#[test]
fn stderr_tail_more_than_five_lines_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .inline(&[])
        .script(include_str!(
            "fixtures/cells_5/stderr_tail_more_than_five_lines_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "stderr_tail_more_than_five_lines_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2294 ran_cell_multiline_with_stderr_snapshot kind=snap owner=chat-live snap=ran_cell_multiline_with_stderr_snapshot
#[test]
fn ran_cell_multiline_with_stderr_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(28, 24)
        .inline(&[])
        .script(include_str!(
            "fixtures/cells_5/ran_cell_multiline_with_stderr_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "ran_cell_multiline_with_stderr_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2339 user_history_cell_wraps_and_prefixes_each_line_snapshot kind=snap owner=chat-live snap=user_history_cell_wraps_and_prefixes_each_line_snapshot
#[test]
fn user_history_cell_wraps_and_prefixes_each_line_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(12, 24)
        .script(include_str!(
            "fixtures/cells_5/user_history_cell_wraps_and_prefixes_each_line_snapshot.jsonl"
        ))
        .open();
    // The oracle's raw_lines assertion (`["_count_rows"]`) is the transcript's raw mode (step 11).
    let row = s
        .rows()
        .iter()
        .position(|row| row.starts_with("›"))
        .expect("prompt");
    s.assert_snap_region(
        "user_history_cell_wraps_and_prefixes_each_line_snapshot",
        row,
        0,
    );
}

// ORACLE history_cell/tests.rs:2359 user_history_cell_wraps_long_urls_inside_the_message_gutter kind=snap owner=chat-live snap=user_history_cell_wraps_long_urls_inside_the_message_gutter
#[test]
fn user_history_cell_wraps_long_urls_inside_the_message_gutter() {
    let s = Screen::build(Owner::ChatLive)
        .size(64, 24)
        .script(include_str!(
            "fixtures/cells_5/user_history_cell_wraps_long_urls_inside_the_message_gutter.jsonl"
        ))
        .open();
    // The first wrapped fragment of the URL keeps the complete destination (OSC 8).
    s.assert_link(
        "https://example.test/forwarded/threads/10930?page=1",
        "https://example.test/forwarded/threads/10930?page=1&search=&filter=all&queue=customer_support_unprocessed&sort=latest_desc&forwardedScope=all",
    );
    // The snapshot opens with the cell's blank row above the prompt.
    let row = s
        .rows()
        .iter()
        .position(|row| row.starts_with("›"))
        .expect("prompt");
    s.assert_snap_region(
        "user_history_cell_wraps_long_urls_inside_the_message_gutter",
        row - 1,
        0,
    );
}

// ORACLE history_cell/tests.rs:2415 user_history_cell_renders_remote_image_urls kind=snap owner=chat-live snap=user_history_cell_renders_remote_image_urls
#[test]
fn user_history_cell_renders_remote_image_urls() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/user_history_cell_renders_remote_image_urls.jsonl"
        ))
        .open();
    s.assert_contains("[Image #1]");
    s.assert_contains("describe these");
    let row = s
        .rows()
        .iter()
        .position(|row| row.contains("[Image #1]"))
        .expect("image");
    s.assert_snap_region("user_history_cell_renders_remote_image_urls", row - 1, 0);
}

// ORACLE history_cell/tests.rs:2432 user_image_labels_follow_the_painted_prompt_surface kind=snap owner=chat-live snap=user_image_labels_follow_the_painted_prompt_surface
#[test]
fn user_image_labels_follow_the_painted_prompt_surface() {
    // The oracle paints the cell alone at 32x5 on three terminal palettes and dumps the buffers;
    // the chat shows the cell among its own rows, so the cells the dump styles are asserted:
    // the image label and the placeholder in the accent that reads on the painted surface, the
    // prompt mark and the text on the surface itself.
    for (fg, bg, accent, surface) in [
        (0x1e1e1e, 0xffffff, "Rgb(28,100,200)", "Rgb(249,249,249)"),
        (0xebebeb, 0x12141e, "Rgb(99,168,248)", "Rgb(55,57,66)"),
        (0x969696, 0x5f5f5f, "Rgb(5,9,14)", "Rgb(120,120,120)"),
    ] {
        let s = Screen::build(Owner::ChatLive)
            .size(32, 14)
            .terminal_colors(fg, bg)
            .script(include_str!(
                "fixtures/cells_5/user_image_labels_follow_the_painted_prompt_surface.jsonl"
            ))
            .open();
        let on_surface = format!("fg={accent} bg={surface}");
        s.assert_in_order(&["[Image #2]", "[Image #1] describe these"]);
        s.assert_text_style("[Image #2]", &on_surface);
        s.assert_text_style("[Image #1]", &on_surface);
        s.assert_text_style("› ", &format!("bg={surface} bold dim"));
        s.assert_text_style("describe these", &format!("bg={surface}"));
    }
}

// ORACLE history_cell/tests.rs:2472 user_history_cell_summarizes_inline_data_urls kind=snap owner=chat-live snap=portable_image_raw_output
#[test]
fn user_history_cell_summarizes_inline_data_urls() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/user_history_cell_summarizes_inline_data_urls.jsonl"
        ))
        .open();
    s.assert_contains("[Image #1]");
    s.assert_contains("describe inline image");
    // The oracle's second half snapshots raw_lines() of the cell with a `[Image #1]` placeholder
    // (portable_image_raw_output): the transcript's raw mode, not reachable by a key here yet.
}

// ORACLE history_cell/tests.rs:2498 user_history_cell_numbers_multiple_remote_images kind=snap owner=chat-live snap=user_history_cell_numbers_multiple_remote_images
#[test]
fn user_history_cell_numbers_multiple_remote_images() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/user_history_cell_numbers_multiple_remote_images.jsonl"
        ))
        .open();
    s.assert_contains("[Image #1]");
    s.assert_contains("[Image #2]");
    let row = s
        .rows()
        .iter()
        .position(|row| row.contains("[Image #1]"))
        .expect("image");
    s.assert_snap_region(
        "user_history_cell_numbers_multiple_remote_images",
        row - 1,
        0,
    );
}

// ORACLE history_cell/tests.rs:2518 user_history_cell_height_matches_rendered_lines_with_remote_images kind=ui owner=chat-live
#[test]
fn user_history_cell_height_matches_rendered_lines_with_remote_images() {
    kit::na(
        Owner::ChatLive,
        "asserts desired_height() equals the length of display_lines() and transcript_lines() equals display_lines(): returned values and method equalities, never drawn as such (the rows the cell draws are checked by the snapshot tests)",
    );
}

// ORACLE history_cell/tests.rs:2541 user_history_cell_trims_trailing_blank_message_lines kind=ui owner=chat-live
#[test]
fn user_history_cell_trims_trailing_blank_message_lines() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/user_history_cell_trims_trailing_blank_message_lines.jsonl"
        ))
        .open();
    s.assert_contains("line one");
    // Missed: the oracle counts the trailing blank rows of the cell (exactly 1); the kit has no
    // "count the blank rows after this text" call.
}

// ORACLE history_cell/tests.rs:2561 user_history_cell_trims_trailing_blank_message_lines_with_text_elements kind=ui owner=chat-live
#[test]
fn user_history_cell_trims_trailing_blank_message_lines_with_text_elements() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/user_history_cell_trims_trailing_blank_message_lines_with_text_elements.jsonl"
        ))
        .open();
    s.assert_contains("tokenized");
    // Missed: the trailing blank-row count (exactly 1), as in the test above.
}

// ORACLE history_cell/tests.rs:2585 render_uses_wrapping_for_long_url_like_line kind=ui owner=chat-live
#[test]
fn render_uses_wrapping_for_long_url_like_line() {
    kit::pending(
        Owner::ChatLive,
        "G48 TextWrap::Word breaks after a hyphen: the URL's `alpha-team` splits as `alpha-` / `team`, where the oracle cuts only at the width, so the head the test reads is never on one row",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(52, 24)
        .script(include_str!(
            "fixtures/cells_5/render_uses_wrapping_for_long_url_like_line.jsonl"
        ))
        .open();
    s.assert_contains("https://example.test/api/v1/projects/alpha-team");
    // Missed: the oracle rebuilds the wrapped rows into the whole URL and counts more than three
    // visible rows; the kit has no call that joins wrapped rows.
}

// ORACLE history_cell/tests.rs:2647 plan_update_with_note_and_wrapping_snapshot kind=snap owner=chat-live snap=plan_update_with_note_and_wrapping_snapshot
#[test]
fn plan_update_with_note_and_wrapping_snapshot() {
    kit::pending(Owner::ChatLive, PLAN_FRAMES);
    // The owned transcript shows the snapshot's `Compact` rendering; the inline viewport prints
    // the `Full` one (the oracle's `display_lines`).
    let script = include_str!("fixtures/cells_5/plan_update_with_note_and_wrapping_snapshot.jsonl");
    let compact = Screen::build(Owner::ChatLive)
        .size(32, 24)
        .script(script)
        .open();
    let full = Screen::build(Owner::ChatLive)
        .size(32, 40)
        .inline(&[])
        .script(script)
        .open();
    // the oracle test joined the two renders under its own labels
    let labels = ["Compact", "Full"];
    let cell_row = |s: &Screen| {
        s.rows()
            .iter()
            .position(|row| row.starts_with("• Updated Plan"))
            .unwrap_or(usize::MAX)
    };
    compact
        .snap("plan_update_with_note_and_wrapping_snapshot")
        .labelled(&labels)
        .frame("Compact")
        .at(cell_row(&compact), 0)
        .assert();
    full.snap("plan_update_with_note_and_wrapping_snapshot")
        .labelled(&labels)
        .frame("Full")
        .at(cell_row(&full), 0)
        .assert();
}

// ORACLE history_cell/tests.rs:2684 plan_update_without_note_snapshot kind=snap owner=chat-live snap=plan_update_without_note_snapshot
#[test]
fn plan_update_without_note_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(40, 24)
        .inline(&[])
        .script(include_str!(
            "fixtures/cells_5/plan_update_without_note_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region("plan_update_without_note_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:2706 plan_update_does_not_split_url_like_tokens_in_note_or_step kind=ui owner=chat-live
#[test]
fn plan_update_does_not_split_url_like_tokens_in_note_or_step() {
    kit::pending(Owner::ChatLive, PLAN_URL_TOKENS);
    let s = Screen::build(Owner::ChatLive)
        .size(30, 24)
        .inline(&[])
        .script(include_str!(
            "fixtures/cells_5/plan_update_does_not_split_url_like_tokens_in_note_or_step.jsonl"
        ))
        .open();
    s.assert_contains(
        "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890",
    );
    s.assert_contains("example.test/api/v1/projects/beta-team/releases/2026-02-17/builds/0987654321/artifacts/reports/performance");
    // Missed: the oracle counts each URL on exactly one row; the kit has no per-row count.
}

// ORACLE history_cell/tests.rs:2741 reasoning_summary_block kind=ui owner=chat-live
#[test]
fn reasoning_summary_block() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block.jsonl"
        ))
        .open();
    // no history cell: only the status row says it, once
    s.assert_rows_with("Detailed reasoning goes here.", 1);
    s.keys("ctrl+t");
    s.assert_contains("• Detailed reasoning goes here.");
}

// ORACLE history_cell/tests.rs:2755 reasoning_summary_height_matches_wrapped_rendering_for_url_like_content kind=ui owner=chat-live
#[test]
fn reasoning_summary_height_matches_wrapped_rendering_for_url_like_content() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(24, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_height_matches_wrapped_rendering_for_url_like_content.jsonl"
        ))
        .open();
    // The oracle's cell here is built with `transcript_only: false`; the app's reasoning block is
    // the transcript-only one, so the detailed view (Ctrl+T) is where it draws.
    s.keys("ctrl+t");
    // The first row keeps the summary bullet; the wrapped height (desired_height) is a value, not drawn.
    s.assert_contains("• example.test/api/v1/");
}

// ORACLE history_cell/tests.rs:2799 reasoning_summary_block_returns_reasoning_cell_when_feature_disabled kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_returns_reasoning_cell_when_feature_disabled() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_returns_reasoning_cell_when_feature_disabled.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• Detailed reasoning goes here.");
}

// ORACLE history_cell/tests.rs:2810 reasoning_summary_block_respects_config_overrides kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_respects_config_overrides() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block.jsonl"
        ))
        .open();
    // no history cell: only the status row says it, once
    s.assert_rows_with("Detailed reasoning goes here.", 1);
}

// ORACLE history_cell/tests.rs:2823 reasoning_summary_block_falls_back_when_header_is_missing kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_falls_back_when_header_is_missing() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_falls_back_when_header_is_missing.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• **High level reasoning without closing");
}

// ORACLE history_cell/tests.rs:2834 reasoning_summary_block_falls_back_when_summary_is_missing kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_falls_back_when_summary_is_missing() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_falls_back_when_summary_is_missing_a.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• High level reasoning without closing");
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_falls_back_when_summary_is_missing_b.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• High level reasoning without closing");
}

// ORACLE history_cell/tests.rs:2853 reasoning_summary_block_keeps_title_only_summary_in_expanded_transcript kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_keeps_title_only_summary_in_expanded_transcript() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_keeps_title_only_summary_in_expanded_transcript.jsonl"
        ))
        .open();
    // no history cell: only the status row says it, once
    s.assert_rows_with("Confirming backend JSONL source", 1);
    s.keys("ctrl+t");
    s.assert_contains("• Confirming backend JSONL source");
}

// ORACLE history_cell/tests.rs:2870 reasoning_summary_block_splits_header_and_summary_when_present kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_splits_header_and_summary_when_present() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_splits_header_and_summary_when_present.jsonl"
        ))
        .open();
    // no history cell: only the status row says it, once
    s.assert_rows_with("We should fix the bug next.", 1);
    s.keys("ctrl+t");
    s.assert_contains("• We should fix the bug next.");
    s.assert_absent("High level plan");
}

// ORACLE history_cell/tests.rs:2884 reasoning_summary_block_hides_empty_html_comment_parts kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_hides_empty_html_comment_parts() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_hides_empty_html_comment_parts.jsonl"
        ))
        .open();
    s.assert_absent("Checking the first thing");
    s.assert_absent("Checking the second thing");
    s.keys("ctrl+t");
    s.assert_absent("Checking the first thing");
    s.assert_absent("<!--");
    // The block draws nothing (no bullet row of its own, no blank row above it): the Working row
    // is the only bullet of the screen.
    s.assert_rows_starting("•", 1);
}

// ORACLE history_cell/tests.rs:2901 reasoning_summary_block_preserves_bold_content_after_empty_html_comment_part kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_preserves_bold_content_after_empty_html_comment_part() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_preserves_bold_content_after_empty_html_comment_part_a.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• Important conclusion");
    s.assert_absent("<!--");
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_preserves_bold_content_after_empty_html_comment_part_b.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• Result: keep this");
}

// ORACLE history_cell/tests.rs:2930 reasoning_summary_block_strips_header_after_leading_empty_part kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_strips_header_after_leading_empty_part() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_strips_header_after_leading_empty_part.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• Tests passed");
    s.assert_absent("Checking tests");
}

// ORACLE history_cell/tests.rs:2947 reasoning_summary_block_drops_empty_part_after_real_content kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_drops_empty_part_after_real_content() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_drops_empty_part_after_real_content.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• done");
    s.assert_absent("Checking tests");
}

// ORACLE history_cell/tests.rs:2964 reasoning_summary_block_preserves_literal_html_comment kind=ui owner=chat-live
#[test]
fn reasoning_summary_block_preserves_literal_html_comment() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/reasoning_summary_block_preserves_literal_html_comment.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_contains("• Use <!-- --> in JSX.");
}

// ORACLE history_cell/tests.rs:2978 deprecation_notice_renders_summary_with_details kind=ui owner=chat-live
#[test]
fn deprecation_notice_renders_summary_with_details() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/deprecation_notice_renders_summary_with_details.jsonl"
        ))
        .open();
    s.assert_absent("Feature flag `foo`");
    s.keys("ctrl+t");
    s.assert_has_row("⚠ Feature flag `foo`");
    s.assert_has_row("Use flag `bar` instead.");
}

// ORACLE history_cell/tests.rs:2997 legacy_zsh_migration_notice_snapshot kind=snap owner=chat-live snap=legacy_zsh_migration_notice_snapshot
#[test]
fn legacy_zsh_migration_notice_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/legacy_zsh_migration_notice_snapshot.jsonl"
        ))
        .open();
    // The oracle's cell is built from the feature registry's legacy usage (core); the kit feeds its
    // summary and details as a deprecation notice.
    s.keys("ctrl+t");
    // The detailed transcript is the chat's own mode, bottom-aligned: the cell stands where its
    // last row meets the status row, not at the top.
    let row = s
        .rows()
        .iter()
        .position(|row| row.starts_with("⚠ `[features]"))
        .expect("the notice");
    s.assert_snap_region("legacy_zsh_migration_notice_snapshot", row, 0);
}

// ORACLE history_cell/tests.rs:3009 agent_markdown_cell_renders_source_at_different_widths kind=ui owner=chat-live
#[test]
fn agent_markdown_cell_renders_source_at_different_widths() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/agent_markdown_cell_renders_source_at_different_widths.jsonl"
        ))
        .open();
    s.assert_contains("• A long agent message that should wrap");
    s.resize(32, 24);
    s.assert_in_order(&["A long agent message", "changes."]);
    // Missed: the oracle asserts the 32-column rendering has more rows than the 80-column one.
}

// ORACLE history_cell/tests.rs:3029 agent_markdown_cell_does_not_split_words_after_inline_markdown kind=ui owner=chat-live
#[test]
fn agent_markdown_cell_does_not_split_words_after_inline_markdown() {
    let s = Screen::build(Owner::ChatLive)
        .size(190, 24)
        .script(include_str!(
            "fixtures/cells_5/agent_markdown_cell_does_not_split_words_after_inline_markdown.jsonl"
        ))
        .open();
    s.assert_in_order(&["inline code,", "strikethrough,"]);
    // Missed: the oracle pins the row ends (`inline code,`) and the row start (`  strikethrough,`);
    // the kit's row-position calls need the cell's first row, which the chat layout does not fix yet.
}

// ORACLE history_cell/tests.rs:3045 streamed_agent_list_paragraph_preserves_item_indent_when_wrapped kind=snap owner=chat-live snap=streamed_agent_list_paragraph_preserves_item_indent_when_wrapped
#[test]
fn streamed_agent_list_paragraph_preserves_item_indent_when_wrapped() {
    kit::na(
        Owner::ChatLive,
        "builds an `AgentMessageCell` from pre-rendered lines whose first line is a bare `1. ...` text (so it wraps under the gutter only: `  rejected.`), a state no flow produces: the app renders an agent message through Markdown, where a list item hangs under its marker (`     rejected.`); the paragraph under the item keeps its five columns in both",
    );
}

// ORACLE history_cell/tests.rs:3069 agent_markdown_cell_narrow_width_shows_prefix_only kind=ui owner=chat-live
#[test]
fn agent_markdown_cell_narrow_width_shows_prefix_only() {
    // 60 rows: at two columns the composer's wrapped placeholder takes the screen's first rows
    let s = Screen::build(Owner::ChatLive)
        .size(2, 60)
        .script(include_str!(
            "fixtures/cells_5/agent_markdown_cell_narrow_width_shows_prefix_only.jsonl"
        ))
        .open();
    s.assert_has_row("•");
    s.assert_absent("narrow width coverage");
}

// ORACLE history_cell/tests.rs:3078 wrapped_and_prefixed_cells_handle_tiny_widths kind=ui owner=chat-live
#[test]
fn wrapped_and_prefixed_cells_handle_tiny_widths() {
    for width in 1..=4u16 {
        // 60 rows: at 1..4 columns the composer's wrapped placeholder takes the first rows
        let blank = Screen::build(Owner::ChatLive)
            .size(width, 60)
            .live()
            .open()
            .shot();
        let s = Screen::build(Owner::ChatLive)
            .size(width, 60)
            .script(include_str!(
                "fixtures/cells_5/wrapped_and_prefixed_cells_handle_tiny_widths.jsonl"
            ))
            .open();
        // The oracle asserts each of the four cells draws a non-empty row: the screen differs from the empty chat.
        s.assert_rows_differ(&blank);
    }
}

// ORACLE history_cell/tests.rs:3120 render_clears_area_when_cell_content_shrinks kind=ui owner=chat-live
#[test]
fn render_clears_area_when_cell_content_shrinks() {
    kit::na(
        Owner::ChatLive,
        "renders two cells into the same ratatui `Buffer` and asserts the second leaves no glyph of the first: a retained buffer a cell draws over; rtwk paints each frame from the widget tree into a fresh surface, so no stale glyph can outlive its cell (the cells' `render` has no counterpart)",
    );
}

// ORACLE history_cell/tests.rs:3157 agent_markdown_cell_survives_insert_history_rewrap kind=ui owner=chat-live
#[test]
fn agent_markdown_cell_survives_insert_history_rewrap() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_5/agent_markdown_cell_survives_insert_history_rewrap.jsonl"
        ))
        .open();
    s.assert_contains("Canary rollout remained at limited traffic longer than planned because p95");
    s.assert_contains("latency briefly regressed during cold-cache periods.");
    s.assert_contains("analytics lagged temporarily.");
}

// ---- oracle: history_cell/warnings_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/warnings_tests.rs:7 usage_warnings_render_in_both_transcripts_including_composites kind=ui owner=chat-live
#[test]
fn usage_warnings_render_in_both_transcripts_including_composites() {
    let s = Screen::build(Owner::ChatLive)
        .size(28, 24)
        .script(include_str!(
            "fixtures/cells_5/usage_warnings_render_in_both_transcripts_including_composites.jsonl"
        ))
        .open();
    // the usage warning is the rate-limit update's visible `new_visible_warning_event` (the fixture's
    // last line); the `warning` notification before it is the hidden diagnostic
    s.assert_in_order(&[
        "Earlier output",
        "⚠ Heads up, you have less",
        "than 10% of your usage",
        "limit left. Run /status",
        "for a breakdown.",
    ]);
    s.assert_absent("Hidden diagnostic");
    // Missed: the oracle compares the rich and raw transcripts of the same cell and a composite of
    // cells (returned values); the raw mode's key is not in the kit yet.
}

// ORACLE history_cell/warnings_tests.rs:47 warning_count_deduplicates_messages_mcp_summaries_and_composites kind=ui owner=chat-live
#[test]
fn warning_count_deduplicates_messages_mcp_summaries_and_composites() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 24)
        .script(include_str!(
            "fixtures/cells_5/warning_count_deduplicates_messages_mcp_summaries_and_composites.jsonl"
        ))
        .open();
    // the fixture: `Repeated warning` twice, the MCP server `alpha` failing (its own message and
    // the summary) and `Another warning`: three identities
    s.assert_contains("⚠ 3 · f2");
    // the warning events are hidden diagnostics: the viewer shows them
    s.assert_absent("Another warning");
    s.keys("f2");
    s.assert_contains("Warnings · 1 of 3 · Warning");
    s.assert_contains("Repeated warning");
    s.keys("k");
    s.assert_contains("Warnings · 2 of 3 · MCP · alpha");
    s.assert_contains("MCP alpha failed");
    s.assert_in_order(&["MCP alpha failed", "MCP startup incomplete (failed:", "alpha)"]);
    // Missed: the oracle compares replayed cells and composites (returned values), and that the
    // raw transcript of each cell is empty.
}
