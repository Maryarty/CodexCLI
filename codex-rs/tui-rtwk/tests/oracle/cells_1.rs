//! Shard `cells_1` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const RT_SEED: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}
"#;

/// The oracle's diff tests render the whole block (`create_diff_summary`): what the detailed
/// transcript (Ctrl+T) shows; the chat itself shows the compact preview and its control.
fn show_details(s: &mut Screen) {
    s.keys("ctrl+t");
}

/// The screen row the patch cell starts at (the transcript sticks to the bottom, the oracle's
/// snapshot is the cell alone).
fn cell_row(s: &Screen) -> usize {
    let Some(at) = s.rows().iter().position(|row| row.starts_with("• ")) else {
        panic!("no patch cell header on the screen:\n{}", s.text());
    };
    at
}

/// `count` screen rows from the first cell row, trailing blanks off (a literal region compare for a
/// snapshot the oracle joined of several labelled sections).
fn cell_rows(s: &Screen, count: usize) -> Vec<String> {
    let at = cell_row(s);
    s.rows()
        .into_iter()
        .skip(at)
        .take(count)
        .map(|row| row.trim_end().to_owned())
        .collect()
}

// ---- oracle: diff_render.rs (23 screen-level tests here; 28 state tests left to chat-live)
// ORACLE diff_render.rs:1379 syntax_colors_follow_the_actual_diff_background kind=snap owner=chat-live snap=syntax_colors_follow_the_actual_diff_background
#[test]
fn syntax_colors_follow_the_actual_diff_background() {
    kit::na(
        Owner::ChatLive,
        "the oracle test renders spans into a ratatui Buffer and compares the foreground it resolves \
         against the row background; no screen text carries that, the colours are \
         assert_text_style territory and the kit has no span-style seed for a theme background yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 20)
        .script(include_str!(
            "fixtures/cells_1/syntax_colors_follow_the_actual_diff_background.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_contains("let value = 1; plain reset");
}

// ORACLE diff_render.rs:1594 display_path_prefers_cwd_without_git_repo kind=ui owner=chat-live
#[test]
fn display_path_prefers_cwd_without_git_repo() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 14)
        .script(include_str!(
            "fixtures/cells_1/display_path_prefers_cwd_without_git_repo.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_contains("tui/example.png");
}

// ORACLE diff_render.rs:1614 ui_snapshot_wrap_behavior_insert kind=snap owner=chat-live snap=wrap_behavior_insert
#[test]
fn ui_snapshot_wrap_behavior_insert() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(84, 16)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_wrap_behavior_insert.jsonl"
        ))
        .open();
    show_details(&mut s);
    // The oracle snapshot is one diff line (width 80) below the header, under the 4-column indent.
    s.assert_snap_region("wrap_behavior_insert", cell_row(&s) + 1, 4);
}

// ORACLE diff_render.rs:1638 ui_snapshot_apply_update_block kind=snap owner=chat-live snap=apply_update_block
#[test]
fn ui_snapshot_apply_update_block() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 20)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_snap_region("apply_update_block", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1663 ui_snapshot_apply_update_with_rename_block kind=snap owner=chat-live snap=apply_update_with_rename_block
#[test]
fn ui_snapshot_apply_update_with_rename_block() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 20)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_with_rename_block.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_snap_region("apply_update_with_rename_block", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1688 ui_snapshot_apply_multiple_files_block kind=snap owner=chat-live snap=apply_multiple_files_block
#[test]
fn ui_snapshot_apply_multiple_files_block() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 15)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_multiple_files_block.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_snap_region("apply_multiple_files_block", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1721 ui_snapshot_apply_add_block kind=snap owner=chat-live snap=apply_add_block
#[test]
fn ui_snapshot_apply_add_block() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_add_block.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_snap_region("apply_add_block", cell_row(&s), 0);
    // The oracle wraps the body at 3 columns and shows `1 +a` / `   l` (the gutter plus one
    // column of text). The chat's rows carry the cell's four-column indent: the same gutter
    // and one text column is a screen eight columns wide.
    let mut n = Screen::build(Owner::ChatLive)
        .size(8, 30)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_add_block_narrow.jsonl"
        ))
        .open();
    show_details(&mut n);
    n.assert_has_row("    1 +a");
    n.assert_has_row("       l");
}

// ORACLE diff_render.rs:1747 ui_snapshot_apply_delete_block kind=snap owner=chat-live snap=apply_delete_block
#[test]
fn ui_snapshot_apply_delete_block() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 20)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_delete_block.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_snap_region("apply_delete_block", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1766 ui_snapshot_apply_update_block_wraps_long_lines kind=snap owner=chat-live snap=apply_update_block_wraps_long_lines
#[test]
fn ui_snapshot_apply_update_block_wraps_long_lines() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(72, 20)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_wraps_long_lines.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_snap_region("apply_update_block_wraps_long_lines", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1793 ui_snapshot_apply_update_block_wraps_long_lines_text kind=snap owner=chat-live snap=apply_update_block_wraps_long_lines_text
#[test]
fn ui_snapshot_apply_update_block_wraps_long_lines_text() {
    kit::na(
        Owner::ChatLive,
        "the oracle snapshot is the header line unwrapped (28 columns wide at most by the test's \
         wrap_cols) above the body; the chat wraps the 30-column header at its width, so the \
         header row is a word-wrap row and the body rows follow below it",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(28, 20)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_wraps_long_lines_text.jsonl"
        ))
        .open();
    show_details(&mut s);
    // The body of the oracle's snapshot, from `1  1` on, rows equal to the oracle's.
    s.assert_in_order(&[
        "    1  1\n    2 -2\n    2 +added long line which\n        wraps and_if_there_i\n       s_a_long_token_it_wil\n       l_be_broken\n    3  3\n    4 -4\n    4 +4 context line which\n       also wraps across",
    ]);
}

// ORACLE diff_render.rs:1814 ui_snapshot_apply_update_block_line_numbers_three_digits_text kind=snap owner=chat-live snap=apply_update_block_line_numbers_three_digits_text
#[test]
fn ui_snapshot_apply_update_block_line_numbers_three_digits_text() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 32)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_line_numbers_three_digits_text.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_snap_region(
        "apply_update_block_line_numbers_three_digits_text",
        cell_row(&s),
        0,
    );
}

// ORACLE diff_render.rs:1841 ui_snapshot_apply_update_block_relativizes_path kind=snap owner=chat-live snap=apply_update_block_relativizes_path
#[test]
fn ui_snapshot_apply_update_block_relativizes_path() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_relativizes_path.jsonl"
        ))
        .cwd("/workspace")
        .open();
    show_details(&mut s);
    s.assert_snap_region("apply_update_block_relativizes_path", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1870 ui_snapshot_syntax_highlighted_insert_wraps kind=snap owner=chat-live snap=syntax_highlighted_insert_wraps
#[test]
fn ui_snapshot_syntax_highlighted_insert_wraps() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_syntax_highlighted_insert_wraps.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_in_order(&["fn very_long_function_name", "Ok(arg_one) }"]);
}

// ORACLE diff_render.rs:1921 ui_snapshot_diff_gallery_80x24 kind=snap owner=chat-live snap=diff_gallery_80x24
#[test]
fn ui_snapshot_diff_gallery_80x24() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 70)
        .script(include_str!("fixtures/cells_1/diff_gallery_80x24.jsonl"))
        .open();
    show_details(&mut s);
    s.assert_snap_region("diff_gallery_80x24", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1926 ui_snapshot_diff_gallery_94x35 kind=snap owner=chat-live snap=diff_gallery_94x35
#[test]
fn ui_snapshot_diff_gallery_94x35() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(94, 70)
        .script(include_str!("fixtures/cells_1/diff_gallery_94x35.jsonl"))
        .open();
    show_details(&mut s);
    s.assert_snap_region("diff_gallery_94x35", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1931 ui_snapshot_diff_gallery_120x40 kind=snap owner=chat-live snap=diff_gallery_120x40
#[test]
fn ui_snapshot_diff_gallery_120x40() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(120, 70)
        .script(include_str!("fixtures/cells_1/diff_gallery_120x40.jsonl"))
        .open();
    show_details(&mut s);
    s.assert_snap_region("diff_gallery_120x40", cell_row(&s), 0);
}

// ORACLE diff_render.rs:1940 ui_snapshot_ansi16_insert_delete_no_background kind=snap owner=chat-live snap=ansi16_insert_delete_no_background
#[test]
fn ui_snapshot_ansi16_insert_delete_no_background() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(44, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_ansi16_insert_delete_no_background.jsonl"
        ))
        .open();
    show_details(&mut s);
    // The oracle snapshot is two diff lines (width 80, ANSI-16: foreground only) under the 4-column indent.
    s.assert_snap_region("ansi16_insert_delete_no_background", cell_row(&s) + 1, 4);
}

// ORACLE diff_render.rs:2115 ui_snapshot_theme_scope_background_resolution kind=snap owner=chat-live snap=theme_scope_background_resolution
#[test]
fn ui_snapshot_theme_scope_background_resolution() {
    // The oracle hands the resolver `inserted: Rgb(12, 34, 56)` and no deleted fill; no bundled
    // theme has that pair, so the chat runs under `github`, whose `markup.inserted` fill is
    // `Rgb(221, 255, 221)`: the theme's own fill replaces the default tint (Rgb(33, 58, 43)).
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 14)
        .syntax_theme("github")
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_theme_scope_background_resolution.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_text_style("1 +scope", "bg=Rgb(221,255,221)");
}

// ORACLE diff_render.rs:2381 cpp_module_extensions_use_cpp_highlighting kind=snap owner=chat-live snap=cpp_module_extension_highlighting
#[test]
fn cpp_module_extensions_use_cpp_highlighting() {
    kit::pending(
        Owner::ChatLive,
        "pending(G-syntax, number by the lander; decided 2026-11-22 D4: wait for rtwk): the oracle's snapshot is \
         the span splits syntect's C++ grammar makes for nine extensions (cpp cu cuh cppm CPPM cxxm CxXm \
         ixx IXX); rtwk's highlighter has no lexer for the module extensions and none at syntect's \
         level (patch/exec cells and fences); no syntect lexer is carried",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 60)
        .script(include_str!(
            "fixtures/cells_1/cpp_module_extensions_use_cpp_highlighting.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_contains("export module math;");
}

// ORACLE diff_render.rs:2487 wrap_styled_spans_single_line kind=snap owner=chat-live snap=short
#[test]
fn wrap_styled_spans_single_line() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/cells_1/wrap_styled_spans_single_line.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_contains("short");
}

// ORACLE diff_render.rs:2592 fallback_wrapping_uses_display_width_for_tabs_and_wide_chars kind=ui owner=chat-live
#[test]
fn fallback_wrapping_uses_display_width_for_tabs_and_wide_chars() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(12, 16)
        .script(include_str!(
            "fixtures/cells_1/fallback_wrapping_uses_display_width_for_tabs_and_wide_chars.jsonl"
        ))
        .open();
    show_details(&mut s);
    // `abcd`, the tab as four spaces, `界` and `🙂` wrap by display width: two or more rows,
    // none wider than the screen, nothing split in the middle of a wide character.
    s.assert_in_order(&["1 +abcd", "界", "🙂"]);
    s.assert_rows_fit(12);
}

// ORACLE diff_render.rs:2613 large_update_diff_skips_highlighting kind=ui owner=chat-live
#[test]
fn large_update_diff_skips_highlighting() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let lines = 10_001;
    let patch = format!(
        "--- a/huge.rs\\n+++ b/huge.rs\\n@@ -0,0 +1,{lines} @@\\n{}",
        "+let value = 1;\\n".repeat(lines)
    );
    let item = format!(
        r#"{{"step":"notify","message":{{"method":"item/completed","params":{{"item":{{"type":"fileChange","id":"p1","changes":[{{"path":"huge.rs","kind":{{"type":"update","move_path":null}},"diff":"{patch}"}}],"status":"completed"}},"threadId":"thread-1","turnId":"turn-1","completedAtMs":0}}}}}}"#
    );
    s.feed(&format!("{RT_SEED}{item}"));
    s.assert_contains("huge.rs");
    s.assert_contains("let value = 1;");
}

// ORACLE diff_render.rs:2699 update_diff_preserves_multiline_highlight_state_within_hunk kind=ui owner=chat-live
#[test]
fn update_diff_preserves_multiline_highlight_state_within_hunk() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(120, 24)
        .script(include_str!(
            "fixtures/cells_1/update_diff_preserves_multiline_highlight_state_within_hunk.jsonl"
        ))
        .open();
    show_details(&mut s);
    s.assert_contains("world\";");
}

// ---- oracle: exec_cell/live_output_tests.rs (1 screen-level tests here; 4 state tests left to chat-live)
// ORACLE exec_cell/live_output_tests.rs:70 truncated_ansi_sequence_does_not_hide_the_retained_tail kind=ui owner=chat-live
#[test]
fn truncated_ansi_sequence_does_not_hide_the_retained_tail() {
    kit::pending(
        Owner::ChatLive,
        "the oracle checks the inline `display_lines` preview (head and tail with `… +N lines (⌃t to view transcript)`); the chat shows the owned transcript's compact preview (last three rows, `+ N lines (⌃t to expand)`), equal to the oracle's rows in the pty scenario exec-command",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/truncated_ansi_sequence_does_not_hide_the_retained_tail.jsonl"
        ))
        .open();
    s.assert_contains("bytes omitted");
    s.assert_contains("visible-tail");
}

// ---- oracle: exec_cell/render.rs (13 screen-level tests here; 1 state tests left to chat-live)
// ORACLE exec_cell/render.rs:904 user_shell_output_is_limited_by_screen_lines kind=ui owner=chat-live
#[test]
fn user_shell_output_is_limited_by_screen_lines() {
    kit::pending(
        Owner::ChatLive,
        "S27 (inline mode): the oracle asserts the exec cell's display_lines limit (100 rows of user-shell output); the owned transcript draws a collapsed call as its last three rows with `+ N lines` and an expanded one with the whole output (core projects commandExecution items into ToolKind::Shell)",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(20, 24)
        .script(include_str!(
            "fixtures/cells_1/user_shell_output_is_limited_by_screen_lines.jsonl"
        ))
        .open();
    s.assert_contains("alpha-team");
}

// ORACLE exec_cell/render.rs:1005 truncate_lines_middle_keeps_omitted_count_in_line_units kind=ui owner=chat-live
#[test]
fn truncate_lines_middle_keeps_omitted_count_in_line_units() {
    kit::pending(
        Owner::ChatLive,
        "the oracle checks the inline `display_lines` preview (head and tail with `… +N lines (⌃t to view transcript)`); the chat shows the owned transcript's compact preview (last three rows, `+ N lines (⌃t to expand)`), equal to the oracle's rows in the pty scenario exec-command",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 12)
        .script(include_str!(
            "fixtures/cells_1/truncate_lines_middle_keeps_omitted_count_in_line_units.jsonl"
        ))
        .open();
    s.assert_contains("… +6 lines (⌃t to view transcript)");
}

// ORACLE exec_cell/render.rs:1037 output_lines_ellipsis_includes_transcript_hint kind=ui owner=chat-live
#[test]
fn output_lines_ellipsis_includes_transcript_hint() {
    kit::pending(
        Owner::ChatLive,
        "the oracle checks the inline `display_lines` preview (head and tail with `… +N lines (⌃t to view transcript)`); the chat shows the owned transcript's compact preview (last three rows, `+ N lines (⌃t to expand)`), equal to the oracle's rows in the pty scenario exec-command",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .script(include_str!(
            "fixtures/cells_1/output_lines_ellipsis_includes_transcript_hint.jsonl"
        ))
        .open();
    s.assert_in_order(&["1", "2", "… +3 lines (⌃t to view transcript)", "6", "7"]);
}

// ORACLE exec_cell/render.rs:1064 output_lines_handles_newline_dense_output_without_materializing_every_line kind=ui owner=chat-live
#[test]
fn output_lines_handles_newline_dense_output_without_materializing_every_line() {
    kit::pending(
        Owner::ChatLive,
        "the oracle checks the inline `display_lines` preview (head and tail with `… +N lines (⌃t to view transcript)`); the chat shows the owned transcript's compact preview (last three rows, `+ N lines (⌃t to expand)`), equal to the oracle's rows in the pty scenario exec-command",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let item = format!(
        r#"{{"step":"notify","message":{{"method":"item/completed","params":{{"item":{{"type":"commandExecution","id":"call-id","command":"bash -lc printf","cwd":"/workspace","processId":null,"source":"agent","status":"completed","commandActions":[],"aggregatedOutput":"{}","exitCode":0,"durationMs":null}},"threadId":"thread-1","turnId":"turn-1","completedAtMs":0}}}}}}"#,
        "\\n".repeat(100_000)
    );
    s.feed(&format!("{RT_SEED}{item}"));
    s.assert_contains("… +99990 lines");
}

// ORACLE exec_cell/render.rs:1082 streamed_output_renders_head_tail_previews kind=ui owner=chat-live
#[test]
fn streamed_output_renders_head_tail_previews() {
    kit::pending(
        Owner::ChatLive,
        "the oracle checks the inline `display_lines` preview (head and tail with `… +N lines (⌃t to view transcript)`); the chat shows the owned transcript's compact preview (last three rows, `+ N lines (⌃t to expand)`), equal to the oracle's rows in the pty scenario exec-command",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/streamed_output_renders_head_tail_previews.jsonl"
        ))
        .open();
    s.assert_contains("line 1");
    s.assert_contains("line 160");
    s.assert_contains("… +150 lines");
}

// ORACLE exec_cell/render.rs:1129 truncated_live_output_preview_and_transcript_snapshot kind=snap owner=chat-live snap=truncated_live_output_preview_and_transcript
#[test]
fn truncated_live_output_preview_and_transcript_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "the oracle checks the inline `display_lines` preview (head and tail with `… +N lines (⌃t to view transcript)`); the chat shows the owned transcript's compact preview (last three rows, `+ N lines (⌃t to expand)`), equal to the oracle's rows in the pty scenario exec-command",
    );
    let mut s = Screen::build(Owner::ChatLive).size(60, 24).live().open();
    let hidden = "\\u001b[2m".repeat(300_000);
    let item = r#"{"step":"notify","message":{"method":"item/started","params":{"item":{"type":"commandExecution","id":"call-id","command":"bash -lc 'echo output'","cwd":"/workspace","processId":null,"source":"agent","status":"inProgress","commandActions":[],"aggregatedOutput":null,"exitCode":null,"durationMs":null},"threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;
    let out = format!(
        r#"{{"step":"notify","message":{{"method":"item/commandExecution/outputDelta","params":{{"threadId":"thread-1","turnId":"turn-1","itemId":"call-id","delta":"\u001b[31mhead error that wraps onto the next row\u001b[0m{hidden}\u001b[32mtail output that also wraps\u001b[0m"}}}}}}"#
    );
    s.feed(&format!("{RT_SEED}{item}"));
    s.feed(&out);
    s.assert_contains("head error that wraps");
    s.assert_contains("tail output that also wraps");
}

// ORACLE exec_cell/render.rs:1157 powershell_skill_read_snapshot kind=snap owner=chat-live
#[test]
fn powershell_skill_read_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/powershell_skill_read_snapshot.jsonl"
        ))
        .open();
    s.assert_in_order(&["• Exploring", "└ Read SKILL.md"]);
}

// ORACLE exec_cell/render.rs:1185 command_truncation_ellipsis_does_not_include_transcript_hint kind=ui owner=chat-live
#[test]
fn command_truncation_ellipsis_does_not_include_transcript_hint() {
    kit::pending(
        Owner::ChatLive,
        "the oracle checks the inline `display_lines` preview (head and tail with `… +N lines (⌃t to view transcript)`); the chat shows the owned transcript's compact preview (last three rows, `+ N lines (⌃t to expand)`), equal to the oracle's rows in the pty scenario exec-command",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .script(include_str!(
            "fixtures/cells_1/command_truncation_ellipsis_does_not_include_transcript_hint.jsonl"
        ))
        .open();
    s.assert_absent("to view transcript");
    s.assert_in_order(&["first", "second", "… +1 line"]);
}

// ORACLE exec_cell/render.rs:1227 command_display_does_not_split_long_url_token kind=ui owner=chat-live
#[test]
fn command_display_does_not_split_long_url_token() {
    kit::na(
        Owner::ChatLive,
        "D5: the oracle test reads logical lines, where adaptive wrapping keeps a URL-like token whole and the terminal folds it; a grid row is as wide as the terminal, so a token wider than the screen is never contiguous on a screen row (the owned transcript clips it with `…` in the compact preview and folds it in the expanded one, as the oracle does). The body asserts the token's head.",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(36, 24)
        .script(include_str!(
            "fixtures/cells_1/command_display_does_not_split_long_url_token.jsonl"
        ))
        .open();
    s.assert_contains("http://example.com/");
}

// ORACLE exec_cell/render.rs:1261 active_command_without_animations_is_stable kind=ui owner=chat-live
#[test]
fn active_command_without_animations_is_stable() {
    kit::pending(
        Owner::ChatLive,
        "the oracle compares the cell alone; the chat's `Working` indicator spins, so whole-screen rows change within a second while the cell's bullet row is static (page test exec_bullet_is_static)",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/active_command_without_animations_is_stable.jsonl"
        ))
        .open();
    s.assert_contains("• Running echo done");
    let before = s.shot();
    s.wait(1000);
    s.assert_same_rows(&before);
}

// ORACLE exec_cell/render.rs:1290 exploring_display_does_not_split_long_url_like_search_query kind=ui owner=chat-live
#[test]
fn exploring_display_does_not_split_long_url_like_search_query() {
    kit::na(
        Owner::ChatLive,
        "D5: the oracle test reads logical lines, where adaptive wrapping keeps a URL-like token whole and the terminal folds it; a grid row is as wide as the terminal, so a token wider than the screen is never contiguous on a screen row (the owned transcript clips it with `…` in the compact preview and folds it in the expanded one, as the oracle does). The body asserts the token's head.",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(36, 24)
        .script(include_str!(
            "fixtures/cells_1/exploring_display_does_not_split_long_url_like_search_query.jsonl"
        ))
        .open();
    s.assert_contains("example.test/api/v1/");
}

// ORACLE exec_cell/render.rs:1330 output_display_does_not_split_long_url_like_token_without_scheme kind=ui owner=chat-live
#[test]
fn output_display_does_not_split_long_url_like_token_without_scheme() {
    kit::na(
        Owner::ChatLive,
        "D5: the oracle test reads logical lines, where adaptive wrapping keeps a URL-like token whole and the terminal folds it; a grid row is as wide as the terminal, so a token wider than the screen is never contiguous on a screen row (the owned transcript clips it with `…` in the compact preview and folds it in the expanded one, as the oracle does). The body asserts the token's head.",
    );
    let s = Screen::build(Owner::ChatLive).size(36, 24).script(include_str!("fixtures/cells_1/output_display_does_not_split_long_url_like_token_without_scheme.jsonl")).open();
    s.assert_contains("example.test/api/v1/");
}

// ORACLE exec_cell/render.rs:1364 transcript_view_renders_wrapped_url_like_rows_without_clipping kind=ui owner=chat-live
#[test]
fn transcript_view_renders_wrapped_url_like_rows_without_clipping() {
    kit::na(
        Owner::ChatLive,
        "D5: the oracle test reads logical lines, where adaptive wrapping keeps a URL-like token whole and the terminal folds it; a grid row is as wide as the terminal, so a token wider than the screen is never contiguous on a screen row (the owned transcript clips it with `…` in the compact preview and folds it in the expanded one, as the oracle does). The body asserts the token's head.",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(36, 24)
        .script(include_str!(
            "fixtures/cells_1/transcript_view_renders_wrapped_url_like_rows_without_clipping.jsonl"
        ))
        .open();
    s.assert_contains("https://example.test");
}

// ---- oracle: exec_cell/transcript_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE exec_cell/transcript_tests.rs:34 raw_grouped_history_retains_terminal_input_and_omits_reasoning kind=snap owner=chat-live snap=raw_grouped_history_retains_terminal_input_and_omits_reasoning
#[test]
fn raw_grouped_history_retains_terminal_input_and_omits_reasoning() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/raw_grouped_history_retains_terminal_input_and_omits_reasoning.jsonl"
        ))
        .open();
    s.assert_absent("Inspecting the next file");
    s.assert_contains("first.txt");
}

// ---- oracle: history_cell/computer_activity_tests.rs (4 screen-level tests here; 1 state tests left to chat-live)
// ORACLE history_cell/computer_activity_tests.rs:68 computer_activity_active_and_interrupted kind=snap owner=chat-live snap=computer_activity_active snap=computer_activity_interrupted
#[test]
fn computer_activity_active_and_interrupted() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/computer_activity_active_and_interrupted.jsonl"
        ))
        .open();
    s.assert_snap_region("computer_activity_active", cell_row(&s), 0);
    s.feed(include_str!(
        "fixtures/cells_1/computer_activity_active_and_interrupted_interrupt.jsonl"
    ));
    s.assert_snap_region("computer_activity_interrupted", cell_row(&s), 0);
}

// ORACLE history_cell/computer_activity_tests.rs:87 computer_activity_prioritizes_errors_and_images_without_reordering kind=snap owner=chat-live snap=computer_activity_compact snap=computer_activity_expanded snap=computer_activity_mixed snap=computer_activity_narrow
#[test]
fn computer_activity_prioritizes_errors_and_images_without_reordering() {
    kit::pending(
        Owner::ChatLive,
        "S27: five calls; the oracle asserts the inline `display_lines` (`└ 3 more · ⌃t` row), the owned transcript draws the group's two shown calls and the shared `+ Show details` control instead of that row",
    );
    let mut s = Screen::build(Owner::ChatLive).size(90, 12).script(include_str!("fixtures/cells_1/computer_activity_prioritizes_errors_and_images_without_reordering.jsonl")).open();
    s.assert_snap_region("computer_activity_mixed", cell_row(&s), 0);
    s.resize(24, 12);
    s.assert_snap_region("computer_activity_narrow", cell_row(&s), 0);
}

// ORACLE history_cell/computer_activity_tests.rs:146 computer_activity_completed_and_transport_errors_keep_full_details kind=snap owner=chat-live snap=computer_activity_failed_result snap=computer_activity_success
#[test]
fn computer_activity_completed_and_transport_errors_keep_full_details() {
    // Tall enough that the bottom pane of a running turn leaves the whole cell on the screen.
    let mut s = Screen::build(Owner::ChatLive).size(80, 22).script(include_str!("fixtures/cells_1/computer_activity_completed_and_transport_errors_keep_full_details.jsonl")).open();
    s.assert_snap_region("computer_activity_success", cell_row(&s), 0);
    s.feed(include_str!("fixtures/cells_1/computer_activity_completed_and_transport_errors_keep_full_details_failure.jsonl"));
    s.resize(90, 22);
    s.assert_snap_region("computer_activity_failed_result", cell_row(&s), 0);
}

// ORACLE history_cell/computer_activity_tests.rs:179 computer_activity_preview_handles_unicode_and_long_diagnostics kind=snap owner=chat-live snap=computer_activity_unicode
#[test]
fn computer_activity_preview_handles_unicode_and_long_diagnostics() {
    let s = Screen::build(Owner::ChatLive)
        .size(24, 12)
        .script(include_str!(
            "fixtures/cells_1/computer_activity_preview_handles_unicode_and_long_diagnostics.jsonl"
        ))
        .open();
    s.assert_snap_region("computer_activity_unicode", cell_row(&s), 0);
}

// ---- oracle: history_cell/dynamic_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/dynamic_tests.rs:29 dynamic_status_and_output_match_persisted_presentations kind=snap owner=chat-live snap=dynamic_status_and_output_match_persisted_presentations
#[test]
fn dynamic_status_and_output_match_persisted_presentations() {
    // The oracle joins the compact rows of four calls, then the success call's full and raw forms,
    // under bare labels, which a snapshot region cannot split: the compact rows are literals here
    // (the owned transcript draws them as `compact_hyperlink_lines`); full is the expanded form
    // below, raw is copy text (S27).
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/dynamic_status_and_output_match_persisted_presentations.jsonl"
        ))
        .open();
    kit::assert_rows_are(
        &cell_rows(&s, 14),
        &[
            "• Calling example.inspect",
            "    + Show details",
            "",
            "• Called example.inspect",
            "  └ Found the definition",
            "    + Show details",
            "",
            "• Failed example.inspect",
            "  └ Permission denied",
            "    + Show details",
            "",
            "• Called example.inspect",
            "  └ (output unavailable)",
            "    + Show details",
        ],
    );
    // success, full
    show_details(&mut s);
    s.assert_has_row("  └ Arguments: {");
    s.assert_has_row("      \"path\": \"src/main.rs\"");
    s.assert_has_row("    }");
    s.assert_has_row("    Found the definition");
}

// ORACLE history_cell/dynamic_tests.rs:80 dynamic_preview_reports_hidden_lines_and_retains_full_output kind=snap owner=chat-live snap=dynamic_preview_reports_hidden_lines_and_retains_full_output
#[test]
fn dynamic_preview_reports_hidden_lines_and_retains_full_output() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/dynamic_preview_reports_hidden_lines_and_retains_full_output.jsonl"
        ))
        .open();
    // The oracle's snapshot is the 3-line and the 4-line output at widths 20 and 80 (labelled
    // sections): a screen holds both calls at one width, 80 then 20; the control row is the shared
    // disclosure's.
    kit::assert_rows_are(
        &cell_rows(&s, 12),
        &[
            "• Called example.inspect",
            "  └ Result line 1: retained content",
            "    Result line 2: retained content",
            "    Result line 3: retained content",
            "    + Show details",
            "",
            "• Called example.inspect",
            "  … 1 earlier lines hidden",
            "  └ Result line 2: retained content",
            "    Result line 3: retained content",
            "    Result line 4: retained content",
            "    + Show details",
        ],
    );
    s.resize(20, 24);
    kit::assert_rows_are(
        &cell_rows(&s, 12),
        &[
            "• Called example.in…",
            "  └ Result line 1: …",
            "    Result line 2: …",
            "    Result line 3: …",
            "    + Show details",
            "",
            "• Called example.in…",
            "  … 1 earlier lines…",
            "  └ Result line 2: …",
            "    Result line 3: …",
            "    Result line 4: …",
            "    + Show details",
        ],
    );
}

// ---- oracle: history_cell/hook_cell.rs (6 screen-level tests here; 1 state tests left to chat-live)
// ORACLE history_cell/hook_cell.rs:545 completed_hook_system_message_uses_dim_provenance_and_hides_context kind=ui owner=chat-live
#[test]
fn completed_hook_system_message_uses_dim_provenance_and_hides_context() {
    let s = Screen::build(Owner::ChatLive).size(80, 24).script(include_str!("fixtures/cells_1/completed_hook_system_message_uses_dim_provenance_and_hides_context.jsonl")).open();
    s.assert_contains("↳ Hook · Heads up from the hook");
    s.assert_absent("Private model-facing");
    s.assert_text_style("↳ Hook ·", "dim");
    s.assert_text_style("Heads up from the hook", "plain");
}

// ORACLE history_cell/hook_cell.rs:571 completed_hook_system_message_renders_ansi_styles kind=snap owner=chat-live snap=completed_hook_system_message_ansi_styles
#[test]
fn completed_hook_system_message_renders_ansi_styles() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/completed_hook_system_message_renders_ansi_styles.jsonl"
        ))
        .open();
    s.assert_contains("↳ Hook · Styled output");
    s.assert_contains("across lines plain");
}

// ORACLE history_cell/hook_cell.rs:594 completed_hook_with_only_context_is_quiet_on_every_tui_surface kind=ui owner=chat-live
#[test]
fn completed_hook_with_only_context_is_quiet_on_every_tui_surface() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/completed_hook_with_only_context_is_quiet_on_every_tui_surface.jsonl"
        ))
        .open();
    // The script's user message is the anchor: the absences are not a blank screen.
    s.assert_contains("hello");
    s.assert_absent("↳ Hook");
    s.assert_absent("Working Memory Recall");
}

// ORACLE history_cell/hook_cell.rs:661 completed_hook_non_context_entries_are_not_truncated kind=ui owner=chat-live
#[test]
fn completed_hook_non_context_entries_are_not_truncated() {
    let s_warning = Screen::build(Owner::ChatLive)
        .size(20, 24)
        .script(include_str!(
            "fixtures/cells_1/completed_hook_non_context_entries_are_not_truncated_warning.jsonl"
        ))
        .open();
    s_warning.assert_in_order(&[
        "• Hook stopped",
        "└ first",
        "second",
        "third",
        "fourth",
        "fifth",
    ]);
    let s_stop = Screen::build(Owner::ChatLive)
        .size(20, 24)
        .script(include_str!(
            "fixtures/cells_1/completed_hook_non_context_entries_are_not_truncated_stop.jsonl"
        ))
        .open();
    s_stop.assert_in_order(&[
        "• Hook stopped",
        "└ first",
        "second",
        "third",
        "fourth",
        "fifth",
    ]);
    let s_feedback = Screen::build(Owner::ChatLive)
        .size(20, 24)
        .script(include_str!(
            "fixtures/cells_1/completed_hook_non_context_entries_are_not_truncated_feedback.jsonl"
        ))
        .open();
    s_feedback.assert_in_order(&[
        "• Hook stopped",
        "└ first",
        "second",
        "third",
        "fourth",
        "fifth",
    ]);
    let s_error = Screen::build(Owner::ChatLive)
        .size(20, 24)
        .script(include_str!(
            "fixtures/cells_1/completed_hook_non_context_entries_are_not_truncated_error.jsonl"
        ))
        .open();
    s_error.assert_in_order(&[
        "• Hook stopped",
        "└ first",
        "second",
        "third",
        "fourth",
        "fifth",
    ]);
}

// ORACLE history_cell/hook_cell.rs:686 unsuccessful_hooks_use_bold_red_bullets_and_actionable_details kind=ui owner=chat-live
#[test]
fn unsuccessful_hooks_use_bold_red_bullets_and_actionable_details() {
    let s_failed = Screen::build(Owner::ChatLive).size(80, 24).script(include_str!("fixtures/cells_1/unsuccessful_hooks_use_bold_red_bullets_and_actionable_details_failed.jsonl")).open();
    s_failed.assert_in_order(&["• Hook failed", "└ Policy prevented this action."]);
    s_failed.assert_text_style("•", "fg=Red bold");
    let s_blocked = Screen::build(Owner::ChatLive).size(80, 24).script(include_str!("fixtures/cells_1/unsuccessful_hooks_use_bold_red_bullets_and_actionable_details_blocked.jsonl")).open();
    s_blocked.assert_in_order(&["• Blocked by hook", "└ Policy prevented this action."]);
    s_blocked.assert_text_style("•", "fg=Red bold");
    let s_stopped = Screen::build(Owner::ChatLive).size(80, 24).script(include_str!("fixtures/cells_1/unsuccessful_hooks_use_bold_red_bullets_and_actionable_details_stopped.jsonl")).open();
    s_stopped.assert_in_order(&["• Hook stopped", "└ Policy prevented this action."]);
    s_stopped.assert_text_style("•", "fg=Red bold");
}

// ORACLE history_cell/hook_cell.rs:718 completed_stop_hook_multiline_system_message_prefixes_first_line_only kind=ui owner=chat-live
#[test]
fn completed_stop_hook_multiline_system_message_prefixes_first_line_only() {
    let s = Screen::build(Owner::ChatLive).size(80, 24).script(include_str!("fixtures/cells_1/completed_stop_hook_multiline_system_message_prefixes_first_line_only.jsonl")).open();
    s.assert_in_order(&["↳ Hook · Heads up", "Review generated files"]);
}

// ---- oracle: history_cell/mcp_tests.rs (10 screen-level tests here; 2 state tests left to chat-live)
// ORACLE history_cell/mcp_tests.rs:12 mcp_inventory_connection_states kind=snap owner=chat-live snap=mcp_inventory_connection_states
#[test]
fn mcp_inventory_connection_states() {
    kit::pending(
        Owner::ChatLive,
        "no seed for the MCP server inventory: the screen takes the mcpServerStatus/list reply, which a script step cannot carry",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_snap_region("mcp_inventory_connection_states", 0, 0);
}

// ORACLE history_cell/mcp_tests.rs:73 mcp_inventory_older_app_server_authentication kind=snap owner=chat-live snap=mcp_inventory_older_app_server_authentication
#[test]
fn mcp_inventory_older_app_server_authentication() {
    kit::pending(
        Owner::ChatLive,
        "no seed for the MCP server inventory: the screen takes the mcpServerStatus/list reply, which a script step cannot carry",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_snap_region("mcp_inventory_older_app_server_authentication", 0, 0);
}

// ORACLE history_cell/mcp_tests.rs:140 mcp_preview_shares_one_limit_across_blocks_and_preserves_transcript kind=snap owner=chat-live snap=mcp_preview_shares_one_limit_across_blocks_and_preserves_transcript
#[test]
fn mcp_preview_shares_one_limit_across_blocks_and_preserves_transcript() {
    let s = Screen::build(Owner::ChatLive).size(80, 24).script(include_str!("fixtures/cells_1/mcp_preview_shares_one_limit_across_blocks_and_preserves_transcript.jsonl")).open();
    s.assert_contains("Called search.lookup");
}

// ORACLE history_cell/mcp_tests.rs:195 code_mode_output_shares_a_row_budget_across_blocks kind=snap owner=chat-live
#[test]
fn code_mode_output_shares_a_row_budget_across_blocks() {
    kit::pending(
        Owner::ChatLive,
        "step 6 (left; design in the pack, chunk-cells partial 3) and S27: the oracle asserts the MCP cell's display_lines (code mode: the title with no verb, one row budget over the result blocks), the owned transcript draws compact_mcp_lines (`• Called|Failed <title>` and the last three result rows) and, expanded, the transcript lines; core keeps an MCP result as joined text, the typed result blocks are not built",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 24)
        .script(include_str!(
            "fixtures/cells_1/code_mode_output_shares_a_row_budget_across_blocks.jsonl"
        ))
        .open();
    s.assert_contains("Script completed");
}

// ORACLE history_cell/mcp_tests.rs:238 code_mode_output_preserves_trailing_failure_diagnostics_in_transcript kind=snap owner=chat-live
#[test]
fn code_mode_output_preserves_trailing_failure_diagnostics_in_transcript() {
    // The oracle asserts the inline `display_lines` snapshot (`+6 lines (⌃t to view transcript)`,
    // S27: no screen) and then that the transcript lines end with the diagnostics: the detailed
    // transcript (Ctrl+T) draws `transcript_hyperlink_lines`.
    let mut s = Screen::build(Owner::ChatLive).size(60, 24).script(include_str!("fixtures/cells_1/code_mode_output_preserves_trailing_failure_diagnostics_in_transcript.jsonl")).open();
    show_details(&mut s);
    s.assert_in_order(&["Script failed", "Script error:", "permission denied"]);
    let rows: Vec<String> = cell_rows(&s, 12)
        .into_iter()
        .take_while(|row| !row.is_empty())
        .collect();
    kit::assert_rows_are(
        &rows[rows.len() - 2..],
        &["    Script error:", "    permission denied"],
    );
}

// ORACLE history_cell/mcp_tests.rs:282 code_mode_output_row_budget_applies_after_wrapping_and_to_errors kind=ui owner=chat-live
#[test]
fn code_mode_output_row_budget_applies_after_wrapping_and_to_errors() {
    let s = Screen::build(Owner::ChatLive).size(40, 24).script(include_str!("fixtures/cells_1/code_mode_output_row_budget_applies_after_wrapping_and_to_errors.jsonl")).open();
    s.assert_contains("transcript tail");
}

// ORACLE history_cell/mcp_tests.rs:345 projected_content_preserves_full_rendering kind=ui owner=chat-live
#[test]
fn projected_content_preserves_full_rendering() {
    kit::pending(
        Owner::ChatLive,
        "step 6 (left; design in the pack, chunk-cells partial 3) and S27: the oracle asserts the MCP cell's display_lines (code mode: the title with no verb, one row budget over the result blocks), the owned transcript draws compact_mcp_lines (`• Called|Failed <title>` and the last three result rows) and, expanded, the transcript lines; core keeps an MCP result as joined text, the typed result blocks are not built",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/projected_content_preserves_full_rendering.jsonl"
        ))
        .open();
    s.assert_contains("resource body");
}

// ORACLE history_cell/mcp_tests.rs:388 projected_image_marker_still_requires_a_complete_image kind=ui owner=chat-live
#[test]
fn projected_image_marker_still_requires_a_complete_image() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/projected_image_marker_still_requires_a_complete_image.jsonl"
        ))
        .open();
    s.assert_contains("Returned image");
}

// ORACLE history_cell/mcp_tests.rs:405 code_mode_preserves_text_fields_on_nontext_and_unknown_blocks kind=snap owner=chat-live
#[test]
fn code_mode_preserves_text_fields_on_nontext_and_unknown_blocks() {
    kit::pending(
        Owner::ChatLive,
        "step 6 (left; design in the pack, chunk-cells partial 3) and S27: the oracle asserts the MCP cell's display_lines (code mode: the title with no verb, one row budget over the result blocks), the owned transcript draws compact_mcp_lines (`• Called|Failed <title>` and the last three result rows) and, expanded, the transcript lines; core keeps an MCP result as joined text, the typed result blocks are not built",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(60, 24)
        .script(include_str!(
            "fixtures/cells_1/code_mode_preserves_text_fields_on_nontext_and_unknown_blocks.jsonl"
        ))
        .open();
    s.assert_contains("image-side output");
    s.assert_contains("unknown-side output");
}

// ORACLE history_cell/mcp_tests.rs:534 titled_image_call_keeps_error_and_full_title_when_narrow kind=snap owner=chat-live snap=titled_image_call_keeps_error_and_full_title_when_narrow
#[test]
fn titled_image_call_keeps_error_and_full_title_when_narrow() {
    kit::pending(
        Owner::ChatLive,
        "step 6 (left; design in the pack, chunk-cells partial 3) and S27: the oracle asserts the MCP cell's display_lines (code mode: the title with no verb, one row budget over the result blocks), the owned transcript draws compact_mcp_lines (`• Called|Failed <title>` and the last three result rows) and, expanded, the transcript lines; core keeps an MCP result as joined text, the typed result blocks are not built",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(24, 12)
        .script(include_str!(
            "fixtures/cells_1/titled_image_call_keeps_error_and_full_title_when_narrow.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "titled_image_call_keeps_error_and_full_title_when_narrow",
        0,
        0,
    );
}
