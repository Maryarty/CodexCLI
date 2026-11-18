//! Shard `cells_1` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const RT_SEED: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}
{"step":"notify","message":{"method":"turn/started","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"inProgress","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}
"#;

// ---- oracle: diff_render.rs (23 screen-level tests here; 28 state tests left to chat-live)
// ORACLE diff_render.rs:1379 syntax_colors_follow_the_actual_diff_background kind=snap owner=chat-live snap=syntax_colors_follow_the_actual_diff_background
#[test]
fn syntax_colors_follow_the_actual_diff_background() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the terminal default colours and the light theme are not seedable through the kit",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 12)
        .script(include_str!(
            "fixtures/cells_1/syntax_colors_follow_the_actual_diff_background.jsonl"
        ))
        .open();
    s.assert_snap_region("syntax_colors_follow_the_actual_diff_background", 0, 0);
}

// ORACLE diff_render.rs:1594 display_path_prefers_cwd_without_git_repo kind=ui owner=chat-live
#[test]
fn display_path_prefers_cwd_without_git_repo() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the oracle shows the path relative to the process cwd, which the kit cannot set",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 6)
        .script(include_str!(
            "fixtures/cells_1/display_path_prefers_cwd_without_git_repo.jsonl"
        ))
        .open();
    s.assert_contains("tui/example.png");
}

// ORACLE diff_render.rs:1614 ui_snapshot_wrap_behavior_insert kind=snap owner=chat-live snap=wrap_behavior_insert
#[test]
fn ui_snapshot_wrap_behavior_insert() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(90, 8)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_wrap_behavior_insert.jsonl"
        ))
        .open();
    s.assert_snap_region("wrap_behavior_insert", 0, 0);
}

// ORACLE diff_render.rs:1638 ui_snapshot_apply_update_block kind=snap owner=chat-live snap=apply_update_block
#[test]
fn ui_snapshot_apply_update_block() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_update_block", 0, 0);
}

// ORACLE diff_render.rs:1663 ui_snapshot_apply_update_with_rename_block kind=snap owner=chat-live snap=apply_update_with_rename_block
#[test]
fn ui_snapshot_apply_update_with_rename_block() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_with_rename_block.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_update_with_rename_block", 0, 0);
}

// ORACLE diff_render.rs:1688 ui_snapshot_apply_multiple_files_block kind=snap owner=chat-live snap=apply_multiple_files_block
#[test]
fn ui_snapshot_apply_multiple_files_block() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 14)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_multiple_files_block.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_multiple_files_block", 0, 0);
}

// ORACLE diff_render.rs:1721 ui_snapshot_apply_add_block kind=snap owner=chat-live snap=apply_add_block
#[test]
fn ui_snapshot_apply_add_block() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the narrow frame needs a 3-column screen",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 10)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_add_block.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_add_block", 0, 0);
    let n = Screen::build(Owner::ChatLive)
        .size(3, 8)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_add_block.jsonl"
        ))
        .open();
    n.assert_has_row("1 +a");
    n.assert_has_row("   l");
}

// ORACLE diff_render.rs:1747 ui_snapshot_apply_delete_block kind=snap owner=chat-live snap=apply_delete_block
#[test]
fn ui_snapshot_apply_delete_block() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_delete_block.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_delete_block", 0, 0);
}

// ORACLE diff_render.rs:1766 ui_snapshot_apply_update_block_wraps_long_lines kind=snap owner=chat-live snap=apply_update_block_wraps_long_lines
#[test]
fn ui_snapshot_apply_update_block_wraps_long_lines() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the oracle wraps at 72 columns, the screen wraps at its own width",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_wraps_long_lines.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_update_block_wraps_long_lines", 0, 0);
}

// ORACLE diff_render.rs:1793 ui_snapshot_apply_update_block_wraps_long_lines_text kind=snap owner=chat-live snap=apply_update_block_wraps_long_lines_text
#[test]
fn ui_snapshot_apply_update_block_wraps_long_lines_text() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the oracle wraps at 28 columns, so the screen is 28 wide",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(28, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_wraps_long_lines_text.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_update_block_wraps_long_lines_text", 0, 0);
}

// ORACLE diff_render.rs:1814 ui_snapshot_apply_update_block_line_numbers_three_digits_text kind=snap owner=chat-live snap=apply_update_block_line_numbers_three_digits_text
#[test]
fn ui_snapshot_apply_update_block_line_numbers_three_digits_text() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the snapshot is taller than the screen, the cell's rows need the transcript's scroll",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_line_numbers_three_digits_text.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_update_block_line_numbers_three_digits_text", 0, 0);
}

// ORACLE diff_render.rs:1841 ui_snapshot_apply_update_block_relativizes_path kind=snap owner=chat-live snap=apply_update_block_relativizes_path
#[test]
fn ui_snapshot_apply_update_block_relativizes_path() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the oracle relativizes to current_dir, the fixture to the item's cwd",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 10)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_apply_update_block_relativizes_path.jsonl"
        ))
        .open();
    s.assert_snap_region("apply_update_block_relativizes_path", 0, 0);
}

// ORACLE diff_render.rs:1870 ui_snapshot_syntax_highlighted_insert_wraps kind=snap owner=chat-live snap=syntax_highlighted_insert_wraps
#[test]
fn ui_snapshot_syntax_highlighted_insert_wraps() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the oracle's syntax spans are checked on each row, the screen shows the rows",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_syntax_highlighted_insert_wraps.jsonl"
        ))
        .open();
    s.assert_in_order(&["fn very_long_function_name", "Ok(arg_one) }"]);
}

// ORACLE diff_render.rs:1921 ui_snapshot_diff_gallery_80x24 kind=snap owner=chat-live snap=diff_gallery_80x24
#[test]
fn ui_snapshot_diff_gallery_80x24() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/cells_1/diff_gallery_80x24.jsonl"))
        .open();
    s.assert_snap_region("diff_gallery_80x24", 0, 0);
}

// ORACLE diff_render.rs:1926 ui_snapshot_diff_gallery_94x35 kind=snap owner=chat-live snap=diff_gallery_94x35
#[test]
fn ui_snapshot_diff_gallery_94x35() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(94, 35)
        .script(include_str!("fixtures/cells_1/diff_gallery_94x35.jsonl"))
        .open();
    s.assert_snap_region("diff_gallery_94x35", 0, 0);
}

// ORACLE diff_render.rs:1931 ui_snapshot_diff_gallery_120x40 kind=snap owner=chat-live snap=diff_gallery_120x40
#[test]
fn ui_snapshot_diff_gallery_120x40() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(120, 40)
        .script(include_str!("fixtures/cells_1/diff_gallery_120x40.jsonl"))
        .open();
    s.assert_snap_region("diff_gallery_120x40", 0, 0);
}

// ORACLE diff_render.rs:1940 ui_snapshot_ansi16_insert_delete_no_background kind=snap owner=chat-live snap=ansi16_insert_delete_no_background
#[test]
fn ui_snapshot_ansi16_insert_delete_no_background() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the colour level ansi16 is not seedable through the kit",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 4)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_ansi16_insert_delete_no_background.jsonl"
        ))
        .open();
    s.assert_snap_region("ansi16_insert_delete_no_background", 0, 0);
}

// ORACLE diff_render.rs:2115 ui_snapshot_theme_scope_background_resolution kind=snap owner=chat-live snap=theme_scope_background_resolution
#[test]
fn ui_snapshot_theme_scope_background_resolution() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the snapshot is the resolved scope backgrounds (insert=Some(Rgb(12, 34, 56)), delete=Some(Rgb(74, 34, 29))), which show only as cell backgrounds; no theme seed",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(40, 6)
        .script(include_str!(
            "fixtures/cells_1/ui_snapshot_theme_scope_background_resolution.jsonl"
        ))
        .open();
    s.assert_contains("scope");
    // once the theme seed exists: assert_text_style("scope", "bg=Rgb(12,34,56)") for the insert row.
}

// ORACLE diff_render.rs:2381 cpp_module_extensions_use_cpp_highlighting kind=snap owner=chat-live snap=cpp_module_extension_highlighting
#[test]
fn cpp_module_extensions_use_cpp_highlighting() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the theme defaults (fg 220,220,220 bg 20,20,20) are not seedable through the kit",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/cpp_module_extensions_use_cpp_highlighting.jsonl"
        ))
        .open();
    s.assert_snap_region("cpp_module_extension_highlighting", 0, 0);
}

// ORACLE diff_render.rs:2487 wrap_styled_spans_single_line kind=snap owner=chat-live snap=short
#[test]
fn wrap_styled_spans_single_line() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the oracle's test asserts one chunk and names the snapshot 'short' that no file holds",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 6)
        .script(include_str!(
            "fixtures/cells_1/wrap_styled_spans_single_line.jsonl"
        ))
        .open();
    s.assert_contains("short");
}

// ORACLE diff_render.rs:2592 fallback_wrapping_uses_display_width_for_tabs_and_wide_chars kind=ui owner=chat-live
#[test]
fn fallback_wrapping_uses_display_width_for_tabs_and_wide_chars() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the kit has no display-width assertion: the width check of every row is not ported",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(8, 8)
        .script(include_str!(
            "fixtures/cells_1/fallback_wrapping_uses_display_width_for_tabs_and_wide_chars.jsonl"
        ))
        .open();
    s.assert_contains("界");
}

// ORACLE diff_render.rs:2613 large_update_diff_skips_highlighting kind=ui owner=chat-live
#[test]
fn large_update_diff_skips_highlighting() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the 10001-line patch is built at run time, the performance half is not a screen fact",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    let lines = 10_001;
    let patch = format!(
        "--- a/huge.rs\\n+++ b/huge.rs\\n@@ -0,0 +1,{lines} @@\\n{}",
        "+let value = 1;\\n".repeat(lines)
    );
    let item = format!(
        r#"{{"step":"notify","message":{{"method":"item/completed","params":{{"item":{{"type":"fileChange","id":"p1","changes":[{{"path":"huge.rs","kind":{{"type":"update","movePath":null}},"diff":"{patch}"}}],"status":"completed"}},"threadId":"thread-1","turnId":"turn-1","completedAtMs":0}}}}}}"#
    );
    s.feed(&format!("{RT_SEED}{item}"));
    s.assert_contains("huge.rs");
    s.assert_contains("let value = 1;");
}

// ORACLE diff_render.rs:2699 update_diff_preserves_multiline_highlight_state_within_hunk kind=ui owner=chat-live
#[test]
fn update_diff_preserves_multiline_highlight_state_within_hunk() {
    kit::pending(
        Owner::ChatLive,
        "no patch cell in the S11 transcript: core threads::project ignores fileChange items and the cells screen is an empty form; the expected style is the oracle highlighter's output for that line, not a literal, so the style compare is not ported",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(120, 24)
        .script(include_str!(
            "fixtures/cells_1/update_diff_preserves_multiline_highlight_state_within_hunk.jsonl"
        ))
        .open();
    s.assert_contains("world\";");
}

// ---- oracle: exec_cell/live_output_tests.rs (1 screen-level tests here; 4 state tests left to chat-live)
// ORACLE exec_cell/live_output_tests.rs:70 truncated_ansi_sequence_does_not_hide_the_retained_tail kind=ui owner=chat-live
#[test]
fn truncated_ansi_sequence_does_not_hide_the_retained_tail() {
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the retained raw line of the oracle's transcript_lines is not on the screen",
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
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the oracle's limit arithmetic (101 rows of user shell output) is not transcribed yet",
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
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the oracle's max_rows and the omitted count are helper arguments, the screen's own limit must give the same hint",
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
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the oracle's line_limit is 2, the screen's output limit is its own",
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
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the 100000-newline output is built at run time; the no-materialize half is not a screen fact",
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
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the 160 streamed lines are the fixture's one delta",
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
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the oracle snapshot is a Debug dump of (preview, compact, transcript), not a screen; the 300000-escape output is built at run time",
    );
    let mut s = Screen::build(Owner::ChatLive).size(60, 24).live().open();
    let hidden = "\\u001b[2m".repeat(300_000);
    let item = r#"{"step":"notify","message":{"method":"item/started","params":{"item":{"type":"commandExecution","id":"call-id","command":"bash -lc echo output","cwd":"/workspace","processId":null,"source":"agent","status":"inProgress","commandActions":[],"aggregatedOutput":null,"exitCode":null,"durationMs":null},"threadId":"thread-1","turnId":"turn-1","startedAtMs":0}}}"#;
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
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 6)
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
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the oracle's keep-2 limit is a helper argument",
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
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(36, 12)
        .script(include_str!(
            "fixtures/cells_1/command_display_does_not_split_long_url_token.jsonl"
        ))
        .open();
    s.assert_contains("http://example.com/long-url-with-dashes-wider-than-terminal-window/blah-blah-blah-text/more-gibberish-text");
}

// ORACLE exec_cell/render.rs:1261 active_command_without_animations_is_stable kind=ui owner=chat-live
#[test]
fn active_command_without_animations_is_stable() {
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 6)
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
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(36, 12)
        .script(include_str!(
            "fixtures/cells_1/exploring_display_does_not_split_long_url_like_search_query.jsonl"
        ))
        .open();
    s.assert_contains("example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/with/a/very/long/path");
}

// ORACLE exec_cell/render.rs:1330 output_display_does_not_split_long_url_like_token_without_scheme kind=ui owner=chat-live
#[test]
fn output_display_does_not_split_long_url_like_token_without_scheme() {
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive).size(36, 12).script(include_str!("fixtures/cells_1/output_display_does_not_split_long_url_like_token_without_scheme.jsonl")).open();
    s.assert_contains("example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/session_id=abc123def456ghi789jkl012mno345pqr678");
}

// ORACLE exec_cell/render.rs:1364 transcript_view_renders_wrapped_url_like_rows_without_clipping kind=ui owner=chat-live
#[test]
fn transcript_view_renders_wrapped_url_like_rows_without_clipping() {
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the oracle renders the TranscriptView into a 36x16 buffer, the screen is the transcript overlay",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(36, 16)
        .script(include_str!(
            "fixtures/cells_1/transcript_view_renders_wrapped_url_like_rows_without_clipping.jsonl"
        ))
        .open();
    s.assert_contains("https://example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/with/a/very/long/path/that/keeps/going/for/testing/purposes");
}

// ---- oracle: exec_cell/transcript_tests.rs (1 screen-level tests here; 0 state tests left to chat-live)
// ORACLE exec_cell/transcript_tests.rs:34 raw_grouped_history_retains_terminal_input_and_omits_reasoning kind=snap owner=chat-live snap=raw_grouped_history_retains_terminal_input_and_omits_reasoning
#[test]
fn raw_grouped_history_retains_terminal_input_and_omits_reasoning() {
    kit::pending(
        Owner::ChatLive,
        "no exec cell in the S11 transcript: core threads::project ignores commandExecution items and the cells screen is an empty form; the snapshot joins Rich and Raw blocks, the Raw block is not on the screen; the unified-exec interaction is not in the fixture",
    );
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
    kit::pending(
        Owner::ChatLive,
        "no computer-activity cell (a group of cua_repl mcpToolCall cells) in the S11 transcript: core ignores mcpToolCall items",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_1/computer_activity_active_and_interrupted.jsonl"
        ))
        .open();
    s.assert_snap_region("computer_activity_active", 0, 0);
    s.feed(include_str!(
        "fixtures/cells_1/computer_activity_active_and_interrupted_interrupt.jsonl"
    ));
    s.assert_snap_region("computer_activity_interrupted", 0, 0);
}

// ORACLE history_cell/computer_activity_tests.rs:87 computer_activity_prioritizes_errors_and_images_without_reordering kind=snap owner=chat-live snap=computer_activity_compact snap=computer_activity_expanded snap=computer_activity_mixed snap=computer_activity_narrow
#[test]
fn computer_activity_prioritizes_errors_and_images_without_reordering() {
    kit::pending(
        Owner::ChatLive,
        "no computer-activity cell (a group of cua_repl mcpToolCall cells) in the S11 transcript: core ignores mcpToolCall items; the compact and expanded frames (their keys) are not transcribed yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(90, 12).script(include_str!("fixtures/cells_1/computer_activity_prioritizes_errors_and_images_without_reordering.jsonl")).open();
    s.assert_snap_region("computer_activity_mixed", 0, 0);
    s.resize(24, 12);
    s.assert_snap_region("computer_activity_narrow", 0, 0);
}

// ORACLE history_cell/computer_activity_tests.rs:146 computer_activity_completed_and_transport_errors_keep_full_details kind=snap owner=chat-live snap=computer_activity_failed_result snap=computer_activity_success
#[test]
fn computer_activity_completed_and_transport_errors_keep_full_details() {
    kit::pending(
        Owner::ChatLive,
        "no computer-activity cell (a group of cua_repl mcpToolCall cells) in the S11 transcript: core ignores mcpToolCall items",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 12).script(include_str!("fixtures/cells_1/computer_activity_completed_and_transport_errors_keep_full_details.jsonl")).open();
    s.assert_snap_region("computer_activity_success", 0, 0);
    s.feed(include_str!("fixtures/cells_1/computer_activity_completed_and_transport_errors_keep_full_details_failure.jsonl"));
    s.resize(90, 12);
    s.assert_snap_region("computer_activity_failed_result", 0, 0);
}

// ORACLE history_cell/computer_activity_tests.rs:179 computer_activity_preview_handles_unicode_and_long_diagnostics kind=snap owner=chat-live snap=computer_activity_unicode
#[test]
fn computer_activity_preview_handles_unicode_and_long_diagnostics() {
    kit::pending(
        Owner::ChatLive,
        "no computer-activity cell (a group of cua_repl mcpToolCall cells) in the S11 transcript: core ignores mcpToolCall items",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(24, 12)
        .script(include_str!(
            "fixtures/cells_1/computer_activity_preview_handles_unicode_and_long_diagnostics.jsonl"
        ))
        .open();
    s.assert_snap_region("computer_activity_unicode", 0, 0);
}

// ---- oracle: history_cell/dynamic_tests.rs (2 screen-level tests here; 0 state tests left to chat-live)
// ORACLE history_cell/dynamic_tests.rs:29 dynamic_status_and_output_match_persisted_presentations kind=snap owner=chat-live snap=dynamic_status_and_output_match_persisted_presentations
#[test]
fn dynamic_status_and_output_match_persisted_presentations() {
    kit::pending(
        Owner::ChatLive,
        "no dynamic tool-call cell in the S11 transcript: core threads::project ignores dynamicToolCall items and the cells screen is an empty form; the snapshot joins four statuses over three modes, the port cuts its frames per status",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/dynamic_status_and_output_match_persisted_presentations.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "dynamic_status_and_output_match_persisted_presentations",
        0,
        0,
    );
}

// ORACLE history_cell/dynamic_tests.rs:80 dynamic_preview_reports_hidden_lines_and_retains_full_output kind=snap owner=chat-live snap=dynamic_preview_reports_hidden_lines_and_retains_full_output
#[test]
fn dynamic_preview_reports_hidden_lines_and_retains_full_output() {
    kit::pending(
        Owner::ChatLive,
        "no dynamic tool-call cell in the S11 transcript: core threads::project ignores dynamicToolCall items and the cells screen is an empty form; the snapshot joins frames labelled 'lines=N, width=W'",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_1/dynamic_preview_reports_hidden_lines_and_retains_full_output.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "dynamic_preview_reports_hidden_lines_and_retains_full_output",
        0,
        0,
    );
}

// ---- oracle: history_cell/hook_cell.rs (6 screen-level tests here; 1 state tests left to chat-live)
// ORACLE history_cell/hook_cell.rs:545 completed_hook_system_message_uses_dim_provenance_and_hides_context kind=ui owner=chat-live
#[test]
fn completed_hook_system_message_uses_dim_provenance_and_hides_context() {
    kit::pending(
        Owner::ChatLive,
        "no hook cell in the S11 transcript: core ignores hook/completed notifications and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 8).script(include_str!("fixtures/cells_1/completed_hook_system_message_uses_dim_provenance_and_hides_context.jsonl")).open();
    s.assert_contains("↳ Hook · Heads up from the hook");
    s.assert_absent("Private model-facing");
    s.assert_text_style("↳ Hook ·", "dim");
    s.assert_text_style("Heads up from the hook", "plain");
}

// ORACLE history_cell/hook_cell.rs:571 completed_hook_system_message_renders_ansi_styles kind=snap owner=chat-live snap=completed_hook_system_message_ansi_styles
#[test]
fn completed_hook_system_message_renders_ansi_styles() {
    kit::pending(
        Owner::ChatLive,
        "no hook cell in the S11 transcript: core ignores hook/completed notifications and the cells screen is an empty form; the snapshot is a Debug dump of the lines, not a screen",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 8)
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
    kit::pending(
        Owner::ChatLive,
        "no hook cell in the S11 transcript: core ignores hook/completed notifications and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 8)
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
    kit::pending(
        Owner::ChatLive,
        "no hook cell in the S11 transcript: core ignores hook/completed notifications and the cells screen is an empty form; the four kinds are four fixtures",
    );
    let s_warning = Screen::build(Owner::ChatLive)
        .size(20, 12)
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
        .size(20, 12)
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
        .size(20, 12)
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
        .size(20, 12)
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
    kit::pending(
        Owner::ChatLive,
        "no hook cell in the S11 transcript: core ignores hook/completed notifications and the cells screen is an empty form",
    );
    let s_failed = Screen::build(Owner::ChatLive).size(80, 8).script(include_str!("fixtures/cells_1/unsuccessful_hooks_use_bold_red_bullets_and_actionable_details_failed.jsonl")).open();
    s_failed.assert_in_order(&["• Hook failed", "└ Policy prevented this action."]);
    s_failed.assert_text_style("•", "fg=Red bold");
    let s_blocked = Screen::build(Owner::ChatLive).size(80, 8).script(include_str!("fixtures/cells_1/unsuccessful_hooks_use_bold_red_bullets_and_actionable_details_blocked.jsonl")).open();
    s_blocked.assert_in_order(&["• Blocked by hook", "└ Policy prevented this action."]);
    s_blocked.assert_text_style("•", "fg=Red bold");
    let s_stopped = Screen::build(Owner::ChatLive).size(80, 8).script(include_str!("fixtures/cells_1/unsuccessful_hooks_use_bold_red_bullets_and_actionable_details_stopped.jsonl")).open();
    s_stopped.assert_in_order(&["• Hook stopped", "└ Policy prevented this action."]);
    s_stopped.assert_text_style("•", "fg=Red bold");
}

// ORACLE history_cell/hook_cell.rs:718 completed_stop_hook_multiline_system_message_prefixes_first_line_only kind=ui owner=chat-live
#[test]
fn completed_stop_hook_multiline_system_message_prefixes_first_line_only() {
    kit::pending(
        Owner::ChatLive,
        "no hook cell in the S11 transcript: core ignores hook/completed notifications and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 8).script(include_str!("fixtures/cells_1/completed_stop_hook_multiline_system_message_prefixes_first_line_only.jsonl")).open();
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
    kit::pending(
        Owner::ChatLive,
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form; the snapshot joins compact, history and transcript blocks",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).script(include_str!("fixtures/cells_1/mcp_preview_shares_one_limit_across_blocks_and_preserves_transcript.jsonl")).open();
    s.assert_contains("Called search.lookup");
}

// ORACLE history_cell/mcp_tests.rs:195 code_mode_output_shares_a_row_budget_across_blocks kind=snap owner=chat-live
#[test]
fn code_mode_output_shares_a_row_budget_across_blocks() {
    kit::pending(
        Owner::ChatLive,
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form",
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
    kit::pending(
        Owner::ChatLive,
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form",
    );
    let s = Screen::build(Owner::ChatLive).size(60, 24).script(include_str!("fixtures/cells_1/code_mode_output_preserves_trailing_failure_diagnostics_in_transcript.jsonl")).open();
    s.assert_in_order(&["Script failed", "Script error:", "permission denied"]);
}

// ORACLE history_cell/mcp_tests.rs:282 code_mode_output_row_budget_applies_after_wrapping_and_to_errors kind=ui owner=chat-live
#[test]
fn code_mode_output_row_budget_applies_after_wrapping_and_to_errors() {
    kit::pending(
        Owner::ChatLive,
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form; the oracle loops eight cases (two servers, four completions); the body covers the first, the rest is not transcribed yet",
    );
    let s = Screen::build(Owner::ChatLive).size(40, 24).script(include_str!("fixtures/cells_1/code_mode_output_row_budget_applies_after_wrapping_and_to_errors.jsonl")).open();
    s.assert_contains("transcript tail");
}

// ORACLE history_cell/mcp_tests.rs:345 projected_content_preserves_full_rendering kind=ui owner=chat-live
#[test]
fn projected_content_preserves_full_rendering() {
    kit::pending(
        Owner::ChatLive,
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form; the oracle compares each projected block's render() string, which is not transcribed yet",
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
    kit::pending(
        Owner::ChatLive,
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form",
    );
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
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form; the oracle's snapshot is unnamed in the manifest, the body asserts the text fields only",
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
        "no mcp tool-call cell in the S11 transcript: core threads::project ignores mcpToolCall items and the cells screen is an empty form",
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
