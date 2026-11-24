//! Shard `cells_4` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

const URL_LIKE: &str =
    "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";

const DISPLAY_MCP: &str = "S27 (inline mode, wave W4): the oracle test draws the MCP cell's `display_lines` (the inline scrollback form: a hyperlinked `Called server.tool(args)` invocation and the result blocks under `  └ `); the owned transcript of S11 draws `compact_hyperlink_lines` (`• Called <tool>` and the last three result rows), which the port's tool cell matches (tests/pages/cells.rs tool_cells_in_transcript, the pack's cells.md chunk-cells-tool-cells)";
const DISPLAY_EXEC: &str = "S27 (inline mode, wave W4): the oracle test draws the agent exec cell's `display_lines` (command wrapped under `  │ `, output under `  └ `); the owned transcript of S11 draws `compact_hyperlink_lines` (`• Ran <first line> …` and the last three output rows), which the port's exec cell matches (tests/pages/cells.rs tool_cells_in_transcript, the pack's cells.md chunk-cells-tool-cells)";
const NO_SEED_MCP_RPC: &str = "no seed: the /mcp inventory is an RPC answer the script cannot carry, and S11 draws no MCP inventory cell";
const HALFWIDTH_HEADER: &str = "G44: rtwk's text engine clusters a halfwidth katakana and its sound mark (`ｶﾞ`) into one cell (TextMode::Graphemes), the oracle counts the mark as its own cell, so the header's directory (`ｶﾞﾊﾟ`x8 and `-project`, 40 oracle cells, 24 in rtwk) fits the 37 columns beside its five-column indent and is not cut to `…` and its tail";
const NO_SEED_LOCAL: &str = "no seed: this cell comes from a local flow (an approval answer, a stdin interaction, a recap or a loading state) the script cannot drive, and S11 draws no such cell";

// ---- oracle: history_cell/tests.rs (39 screen-level tests here; 3 state tests left to chat-live)
// ORACLE history_cell/tests.rs:1162 mcp_tools_output_from_statuses_renders_status_only_servers kind=snap owner=chat-live snap=mcp_tools_output_from_statuses_renders_status_only_servers
#[test]
fn mcp_tools_output_from_statuses_renders_status_only_servers() {
    kit::pending(Owner::ChatLive, NO_SEED_MCP_RPC);
    let mut s = Screen::build(Owner::ChatLive).size(120, 24).live().open();
    s.type_text("/mcp");
    s.keys("enter");
    s.assert_snap_region(
        "mcp_tools_output_from_statuses_renders_status_only_servers",
        0,
        0,
    );
}

// ORACLE history_cell/tests.rs:1198 mcp_tools_output_from_statuses_renders_verbose_inventory kind=snap owner=chat-live snap=mcp_tools_output_from_statuses_renders_verbose_inventory
#[test]
fn mcp_tools_output_from_statuses_renders_verbose_inventory() {
    kit::pending(Owner::ChatLive, NO_SEED_MCP_RPC);
    let mut s = Screen::build(Owner::ChatLive).size(120, 24).live().open();
    s.type_text("/mcp verbose");
    s.keys("enter");
    s.assert_snap_region(
        "mcp_tools_output_from_statuses_renders_verbose_inventory",
        0,
        0,
    );
}

// ORACLE history_cell/tests.rs:1256 prefixed_wrapped_history_cell_indents_wrapped_lines kind=ui owner=chat-live
#[test]
fn prefixed_wrapped_history_cell_indents_wrapped_lines() {
    kit::pending(Owner::ChatLive, NO_SEED_LOCAL);
    let s = Screen::build(Owner::ChatLive).size(24, 8).live().open();
    s.assert_has_row("✔ You approved codex to");
    s.assert_has_row("  run echo something");
    s.assert_has_row("  really long to ensure");
    s.assert_has_row("  wrapping happens this");
    s.assert_has_row("  time");
}

// ORACLE history_cell/tests.rs:1279 prefixed_wrapped_history_cell_does_not_split_url_like_token kind=ui owner=chat-live
#[test]
fn prefixed_wrapped_history_cell_does_not_split_url_like_token() {
    // The oracle renders at width 24 and counts logical lines; the URL is 77 columns,
    // so the screen shows it whole on one row only at a width that holds it with the prefix.
    kit::pending(Owner::ChatLive, NO_SEED_LOCAL);
    let s = Screen::build(Owner::ChatLive).size(80, 8).live().open();
    s.assert_has_row(&format!("✔ {URL_LIKE}"));
}

// ORACLE history_cell/tests.rs:1295 unified_exec_interaction_details_do_not_split_url_like_stdin_token kind=ui owner=chat-live
#[test]
fn unified_exec_interaction_details_do_not_split_url_like_stdin_token() {
    kit::pending(
        Owner::ChatLive,
        "the cell is built (cells::interaction); the row would hold the token whole only at a width past 81, and at the oracle's 24 TextWrap::Word breaks it at the hyphens and slashes (pending(G-urltoken), tests/pages/cells.rs a_url_like_stdin_token_stays_whole); need: Notice::TerminalInteraction appended by the host",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 8).live().open();
    s.assert_contains(URL_LIKE);
}

// ORACLE history_cell/tests.rs:1311 prefixed_wrapped_history_cell_height_matches_wrapped_rendering kind=ui owner=chat-live
#[test]
fn prefixed_wrapped_history_cell_height_matches_wrapped_rendering() {
    // The oracle asserts the wrapped height exceeds the logical lines and the first row keeps
    // the prefix; on the screen the URL wraps over the rows below the prefix.
    kit::pending(Owner::ChatLive, NO_SEED_LOCAL);
    let s = Screen::build(Owner::ChatLive).size(24, 12).live().open();
    s.assert_contains("✔");
    s.assert_contains("long/path");
}

// ORACLE history_cell/tests.rs:1348 unified_exec_interaction_details_wrap_long_input kind=ui owner=chat-live
#[test]
fn unified_exec_interaction_details_wrap_long_input() {
    kit::pending(
        Owner::ChatLive,
        "the cell is built (cells::interaction, the wrapped input under its branch is tests/pages/cells.rs a_long_stdin_line_wraps_under_its_branch); need: Notice::TerminalInteraction appended by the host, which the kit cannot seed",
    );
    let s = Screen::build(Owner::ChatLive).size(24, 12).live().open();
    s.assert_contains("Interacted with");
}

// ORACLE history_cell/tests.rs:1386 web_search_history_cell_snapshot kind=snap owner=chat-live snap=web_search_history_cell_snapshot
#[test]
fn web_search_history_cell_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(64, 18)
        .script(include_str!(
            "fixtures/cells_4/web_search_history_cell_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region("web_search_history_cell_snapshot", first_cell_row(&s), 0);
}

// ORACLE history_cell/tests.rs:1402 standalone_unix_update_available_history_cell_snapshot kind=snap owner=chat-live snap=standalone_unix_update_available_history_cell_snapshot
#[test]
fn standalone_unix_update_available_history_cell_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(110, 22)
        .live()
        .update_available("9.9.9", Some(kit::Install::StandaloneUnix))
        .open();
    s.assert_snap_region(
        "standalone_unix_update_available_history_cell_snapshot",
        box_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1411 standalone_windows_update_available_history_cell_snapshot kind=snap owner=chat-live snap=standalone_windows_update_available_history_cell_snapshot
#[test]
fn standalone_windows_update_available_history_cell_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(110, 22)
        .live()
        .update_available("9.9.9", Some(kit::Install::StandaloneWindows))
        .open();
    s.assert_snap_region(
        "standalone_windows_update_available_history_cell_snapshot",
        box_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1420 pnpm_update_available_history_cell_snapshot kind=snap owner=chat-live snap=pnpm_update_available_history_cell_snapshot
#[test]
fn pnpm_update_available_history_cell_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(110, 22)
        .live()
        .update_available("9.9.9", Some(kit::Install::Pnpm))
        .open();
    s.assert_snap_region(
        "pnpm_update_available_history_cell_snapshot",
        box_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1429 web_search_history_cell_without_detail_snapshot kind=snap owner=chat-live snap=web_search_history_cell_without_detail_snapshot
#[test]
fn web_search_history_cell_without_detail_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(64, 18)
        .script(include_str!(
            "fixtures/cells_4/web_search_history_cell_without_detail_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "web_search_history_cell_without_detail_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1437 web_search_history_cell_truncates kind=ui owner=chat-live
#[test]
fn web_search_history_cell_truncates() {
    let s = Screen::build(Owner::ChatLive)
        .size(64, 18)
        .script(include_str!(
            "fixtures/cells_4/web_search_history_cell_truncates.jsonl"
        ))
        .open();
    s.assert_has_row("• Searched the web for example search query with several generi…");
}

// ORACLE history_cell/tests.rs:1456 web_search_history_cell_short_query_does_not_wrap kind=ui owner=chat-live
#[test]
fn web_search_history_cell_short_query_does_not_wrap() {
    let s = Screen::build(Owner::ChatLive)
        .size(64, 18)
        .script(include_str!(
            "fixtures/cells_4/web_search_history_cell_short_query_does_not_wrap.jsonl"
        ))
        .open();
    s.assert_has_row("• Searched the web for short query");
}

// ORACLE history_cell/tests.rs:1475 web_search_history_cell_transcript_snapshot kind=snap owner=chat-live snap=web_search_history_cell_transcript_snapshot
#[test]
fn web_search_history_cell_transcript_snapshot() {
    let mut s = Screen::build(Owner::ChatLive)
        .size(64, 22)
        .script(include_str!(
            "fixtures/cells_4/web_search_history_cell_transcript_snapshot.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_snap_region(
        "web_search_history_cell_transcript_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1491 active_mcp_tool_call_snapshot kind=snap owner=chat-live snap=active_mcp_tool_call_snapshot
#[test]
fn active_mcp_tool_call_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(include_str!(
            "fixtures/cells_4/active_mcp_tool_call_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region("active_mcp_tool_call_snapshot", first_cell_row(&s), 0);
}

// ORACLE history_cell/tests.rs:1512 code_mode_tool_call_uses_title_and_preserves_full_transcript kind=snap owner=chat-live
#[test]
fn code_mode_tool_call_uses_title_and_preserves_full_transcript() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    // Inline snapshot of the oracle: history at 40 columns, then the transcript at 180.
    // The long output rows are asserted by their fragments.
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 22)
        .script(include_str!(
            "fixtures/cells_4/code_mode_tool_call_uses_title_and_preserves_full_transcript.jsonl"
        ))
        .open();
    s.assert_has_row("• Inspect Spotify workspace");
    s.assert_has_row("  └ 012345678901234567890123456789012345");
    s.assert_has_row("    678901234567890123456789012345678901");
    s.assert_has_row("    234567890123456789012345678901234567");
    s.assert_has_row("    +1 line (⌃t to view transcript)");
    s.resize(180, 24);
    s.keys("ctrl+t");
    s.assert_has_row("• Called node_repl.js({\"title\":\"Inspect Spotify workspace\",\"code\":\"await tools.exec_command({ cmd: 'git status' })\"})");
    s.assert_has_row("  └ Script completed");
    s.assert_has_row("    Wall time 0.1 seconds");
    s.assert_has_row("    Output:");
    s.assert_has_row("    {\"chunk_id\":\"chunk-");
    s.assert_contains("transcript tail\",\"exit_code\":0}");
}

// ORACLE history_cell/tests.rs:1563 code_mode_tool_call_preserves_failure_details kind=snap owner=chat-live
#[test]
fn code_mode_tool_call_preserves_failure_details() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    // Inline snapshot of the oracle: history at 80 columns, then the transcript at 120.
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(include_str!(
            "fixtures/cells_4/code_mode_tool_call_preserves_failure_details.jsonl"
        ))
        .open();
    s.assert_has_row("• Inspect workspace");
    s.assert_has_row("  └ Script failed");
    s.assert_has_row("    Output:");
    s.assert_has_row("    permission denied");
    s.resize(120, 12);
    s.keys("ctrl+t");
    s.assert_has_row("• Called node_repl.js({\"title\":\"Inspect workspace\",\"code\":\"throw Error('denied')\"})");
    s.assert_has_row("  └ Script failed");
    s.assert_has_row("    Output:");
    s.assert_has_row("    permission denied");
}

// ORACLE history_cell/tests.rs:1601 mcp_inventory_loading_snapshot kind=snap owner=chat-live snap=mcp_inventory_loading_snapshot
#[test]
fn mcp_inventory_loading_snapshot() {
    kit::pending(Owner::ChatLive, NO_SEED_MCP_RPC);
    let mut s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.type_text("/mcp");
    s.keys("enter");
    s.assert_snap_region("mcp_inventory_loading_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:1609 mcp_inventory_loading_without_animations_is_stable kind=ui owner=chat-live
#[test]
fn mcp_inventory_loading_without_animations_is_stable() {
    kit::pending(Owner::ChatLive, NO_SEED_MCP_RPC);
    let mut s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.type_text("/mcp");
    s.keys("enter");
    s.assert_has_row("• Loading MCP inventory…");
    let first = s.shot();
    s.wait(500);
    s.assert_same_rows(&first);
}

// ORACLE history_cell/tests.rs:1619 thread_recap_loading_without_animations_snapshot kind=snap owner=chat-live
#[test]
fn thread_recap_loading_without_animations_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "the row is built (cells::notice, Notice::RecapLoading, equal in tests/pages/cells.rs notice_cells); need: /recap appends it while the recap generates (the host), which the kit cannot seed",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.type_text("/recap");
    s.keys("enter");
    s.assert_has_row("• Generating conversation recap…");
}

// ORACLE history_cell/tests.rs:1627 completed_mcp_tool_call_success_snapshot kind=snap owner=chat-live snap=completed_mcp_tool_call_success_snapshot
#[test]
fn completed_mcp_tool_call_success_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_success_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_success_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1657 completed_mcp_tool_call_image_after_text_snapshot kind=snap owner=chat-live snap=completed_mcp_tool_call_image_after_text_snapshot
#[test]
fn completed_mcp_tool_call_image_after_text_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(80, 20)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_image_after_text_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_image_after_text_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1688 completed_mcp_tool_call_accepts_data_url_image_blocks kind=snap owner=chat-live snap=completed_mcp_tool_call_accepts_data_url_image_blocks
#[test]
fn completed_mcp_tool_call_accepts_data_url_image_blocks() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(80, 20)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_accepts_data_url_image_blocks.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_accepts_data_url_image_blocks",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1717 completed_mcp_tool_call_multiple_image_blocks_snapshot kind=snap owner=chat-live snap=completed_mcp_tool_call_multiple_image_blocks_snapshot
#[test]
fn completed_mcp_tool_call_multiple_image_blocks_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(80, 20)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_multiple_image_blocks_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_multiple_image_blocks_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1745 completed_mcp_tool_call_error_snapshot kind=snap owner=chat-live snap=completed_mcp_tool_call_error_snapshot
#[test]
fn completed_mcp_tool_call_error_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(80, 18)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_error_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_error_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1768 completed_mcp_tool_call_multiple_outputs_snapshot kind=snap owner=chat-live snap=completed_mcp_tool_call_multiple_outputs_snapshot
#[test]
fn completed_mcp_tool_call_multiple_outputs_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(48, 22)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_multiple_outputs_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_multiple_outputs_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1808 completed_mcp_tool_call_wrapped_outputs_snapshot kind=snap owner=chat-live snap=completed_mcp_tool_call_wrapped_outputs_snapshot
#[test]
fn completed_mcp_tool_call_wrapped_outputs_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(40, 26)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_wrapped_outputs_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_wrapped_outputs_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1840 completed_mcp_tool_call_multiple_outputs_inline_snapshot kind=snap owner=chat-live snap=completed_mcp_tool_call_multiple_outputs_inline_snapshot
#[test]
fn completed_mcp_tool_call_multiple_outputs_inline_snapshot() {
    kit::pending(Owner::ChatLive, DISPLAY_MCP);
    let s = Screen::build(Owner::ChatLive)
        .size(120, 20)
        .script(include_str!(
            "fixtures/cells_4/completed_mcp_tool_call_multiple_outputs_inline_snapshot.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "completed_mcp_tool_call_multiple_outputs_inline_snapshot",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:1873 session_header_clamps_to_narrow_width kind=snap owner=chat-live snap=session_header_clamps_to_narrow_width
#[test]
fn session_header_clamps_to_narrow_width() {
    let s = Screen::build(Owner::ChatLive)
        .size(44, 12)
        .script(include_str!(
            "fixtures/cells_4/session_header_clamps_to_narrow_width.jsonl"
        ))
        .open();
    s.assert_snap_region("session_header_clamps_to_narrow_width", 0, 0);
}

// ORACLE history_cell/tests.rs:1895 session_header_indicates_yolo_mode kind=snap owner=chat-live snap=session_header_indicates_yolo_mode
#[test]
fn session_header_indicates_yolo_mode() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_4/session_header_indicates_yolo_mode.jsonl"
        ))
        .open();
    s.assert_snap_region("session_header_indicates_yolo_mode", 0, 0);
}

// ORACLE history_cell/tests.rs:1909 session_header_truncates_halfwidth_directory kind=snap owner=chat-live snap=session_header_halfwidth_directory
#[test]
fn session_header_truncates_halfwidth_directory() {
    kit::pending(Owner::ChatLive, HALFWIDTH_HEADER);
    let s = Screen::build(Owner::ChatLive)
        .size(42, 24)
        .script(include_str!(
            "fixtures/cells_4/session_header_truncates_halfwidth_directory.jsonl"
        ))
        .open();
    s.assert_snap_region("session_header_halfwidth_directory", 0, 0);
}

// ORACLE history_cell/tests.rs:1976 coalesces_sequential_reads_within_one_call kind=snap owner=chat-live snap=coalesces_sequential_reads_within_one_call
#[test]
fn coalesces_sequential_reads_within_one_call() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 15)
        .script(include_str!(
            "fixtures/cells_4/coalesces_sequential_reads_within_one_call.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "coalesces_sequential_reads_within_one_call",
        explored_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2017 coalesces_reads_across_multiple_calls kind=snap owner=chat-live snap=coalesces_reads_across_multiple_calls
#[test]
fn coalesces_reads_across_multiple_calls() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 15)
        .script(include_str!(
            "fixtures/cells_4/coalesces_reads_across_multiple_calls.jsonl"
        ))
        .open();
    s.assert_snap_region("coalesces_reads_across_multiple_calls", explored_row(&s), 0);
}

// ORACLE history_cell/tests.rs:2070 coalesced_reads_dedupe_names kind=snap owner=chat-live snap=coalesced_reads_dedupe_names
#[test]
fn coalesced_reads_dedupe_names() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 15)
        .script(include_str!(
            "fixtures/cells_4/coalesced_reads_dedupe_names.jsonl"
        ))
        .open();
    s.assert_snap_region("coalesced_reads_dedupe_names", explored_row(&s), 0);
}

/// The row of the update card's box: the oracle's snapshot is the bare cell, the chat draws a
/// bottom pane under the transcript.
fn box_row(screen: &Screen) -> usize {
    screen
        .rows()
        .iter()
        .position(|row| row.starts_with('╭'))
        .unwrap_or_else(|| panic!("no box on the screen:\n{}", screen.text()))
}

/// The row of the transcript's first cell: the oracle's snapshot is the bare cell, the chat
/// draws its bottom pane under the transcript, so the cell is looked up by its bullet.
fn first_cell_row(screen: &Screen) -> usize {
    screen
        .rows()
        .iter()
        .position(|row| row.starts_with("• ") && !row.starts_with("• Working"))
        .unwrap_or_else(|| panic!("missing cell row:\n{}", screen.text()))
}

fn explored_row(screen: &Screen) -> usize {
    screen
        .rows()
        .iter()
        .position(|row| row.contains("• Explored"))
        .unwrap_or_else(|| panic!("missing exploration row:\n{}", screen.text()))
}

// ORACLE history_cell/tests.rs:2107 multiline_command_wraps_with_extra_indent_on_subsequent_lines kind=snap owner=chat-live snap=multiline_command_wraps_with_extra_indent_on_subsequent_lines
#[test]
fn multiline_command_wraps_with_extra_indent_on_subsequent_lines() {
    kit::pending(Owner::ChatLive, DISPLAY_EXEC);
    let s = Screen::build(Owner::ChatLive)
        .size(28, 20)
        .script(include_str!(
            "fixtures/cells_4/multiline_command_wraps_with_extra_indent_on_subsequent_lines.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "multiline_command_wraps_with_extra_indent_on_subsequent_lines",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2135 single_line_command_compact_when_fits kind=snap owner=chat-live snap=single_line_command_compact_when_fits
#[test]
fn single_line_command_compact_when_fits() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 16)
        .script(include_str!(
            "fixtures/cells_4/single_line_command_compact_when_fits.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "single_line_command_compact_when_fits",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2158 single_line_command_wraps_with_four_space_continuation kind=snap owner=chat-live snap=single_line_command_wraps_with_four_space_continuation
#[test]
fn single_line_command_wraps_with_four_space_continuation() {
    kit::pending(Owner::ChatLive, DISPLAY_EXEC);
    let s = Screen::build(Owner::ChatLive)
        .size(24, 20)
        .script(include_str!(
            "fixtures/cells_4/single_line_command_wraps_with_four_space_continuation.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "single_line_command_wraps_with_four_space_continuation",
        first_cell_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:2181 single_line_command_over_highlight_limit_uses_plain_text_fallback kind=snap owner=chat-live snap=single_line_command_over_highlight_limit_uses_plain_text_fallback
#[test]
fn single_line_command_over_highlight_limit_uses_plain_text_fallback() {
    kit::pending(Owner::ChatLive, DISPLAY_EXEC);
    let s = Screen::build(Owner::ChatLive)
        .size(24, 22)
        .script(include_str!(
            "fixtures/cells_4/single_line_command_over_highlight_limit_uses_plain_text_fallback.jsonl"
        ))
        .open();
    s.assert_snap_region(
        "single_line_command_over_highlight_limit_uses_plain_text_fallback",
        first_cell_row(&s),
        0,
    );
}
