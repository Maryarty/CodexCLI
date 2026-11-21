//! Shard `cells_3` (screens S11, owner chat-live): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The index of the first screen row that is exactly `text` (trailing blanks
/// ignored), or past the end when there is none: the row checks below then fail.
fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.trim_end() == text)
        .unwrap_or(usize::MAX)
}

// ---- oracle: history_cell/tests.rs (39 screen-level tests here; 3 state tests left to chat-live)
// ORACLE history_cell/tests.rs:40 connected_server_version_notice_snapshot kind=snap owner=chat-live snap=connected_server_version_notice_snapshot
#[test]
fn connected_server_version_notice_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no server-version notice on S11: it comes from a remote app-server target and the show_server_version_notice setting, neither seedable through the kit; the cells screen draws no warning cell",
    );
    let s = Screen::build(Owner::ChatLive).size(100, 6).live().open();
    s.assert_snap_region("connected_server_version_notice_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:67 local_daemon_version_notice_snapshot kind=snap owner=chat-live snap=local_daemon_alpha_mismatch snap=local_daemon_source_mismatch snap=local_daemon_version_notice_snapshot
#[test]
fn local_daemon_version_notice_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no local-daemon version notice on S11: the client/server version pair and the local daemon target are not seedable through the kit; the cells screen draws no warning cell",
    );
    // One screen per (client, server) pair of the oracle loop.
    let s = Screen::build(Owner::ChatLive).size(100, 6).live().open();
    s.assert_snap_region("local_daemon_version_notice_snapshot", 0, 0);
    let s = Screen::build(Owner::ChatLive).size(100, 6).live().open();
    s.assert_snap_region("local_daemon_alpha_mismatch", 0, 0);
    let s = Screen::build(Owner::ChatLive).size(100, 6).live().open();
    s.assert_snap_region("local_daemon_source_mismatch", 0, 0);
}

// ORACLE history_cell/tests.rs:119 streaming_agent_tail_blank_line_uses_one_viewport_row kind=snap owner=chat-live
#[test]
fn streaming_agent_tail_blank_line_uses_one_viewport_row() {
    kit::pending(
        Owner::ChatLive,
        "no streamed-answer tail on S11: the cells screen draws no streaming agent cell yet; the source spans and desired_height are core state, left to the chat-live state test",
    );
    let s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_3/streaming_agent_tail_blank_line.jsonl"
        ))
        .open();
    // Visible half: the two text rows of the tail, no bullet prefix. The blank row
    // between them is one viewport row (oracle inline snapshot "  first\n\n  second").
    s.assert_in_order(&["  first", "  second"]);
}

// ORACLE history_cell/tests.rs:270 raw_lines_from_source_preserves_explicit_blank_lines kind=ui owner=chat-live
#[test]
fn raw_lines_from_source_preserves_explicit_blank_lines() {
    kit::pending(
        Owner::ChatLive,
        "no raw scrollback mode (alt+r) on S11: the cells screen draws no raw-source rows yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!("fixtures/cells_3/raw_lines_blank_lines.jsonl"))
        .open();
    s.keys("alt+r");
    // Visible half: the raw rows, the blank row kept between alpha and beta. Exact rows,
    // so the rich rendering (bulleted paragraphs) does not pass for the raw one.
    let a = row_of(&s, "alpha");
    s.assert_row(a, "alpha");
    s.assert_row(a.saturating_add(1), "");
    s.assert_row(a.saturating_add(2), "beta");
}

// ORACLE history_cell/tests.rs:281 raw_lines_from_source_preserves_trailing_blank_but_not_trailing_newline kind=ui owner=chat-live
#[test]
fn raw_lines_from_source_preserves_trailing_blank_but_not_trailing_newline() {
    kit::pending(
        Owner::ChatLive,
        "no raw scrollback mode (alt+r) on S11: the cells screen draws no raw-source rows yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_3/raw_lines_trailing_blank.jsonl"
        ))
        .open();
    s.keys("alt+r");
    // Visible half: the text row, then the trailing blank row. The empty-source case
    // (`raw_lines_from_source("")` is no lines) draws nothing and is not asserted here.
    let a = row_of(&s, "alpha");
    s.assert_row(a, "alpha");
    s.assert_row(a.saturating_add(1), "");
}

// ORACLE history_cell/tests.rs:290 source_backed_cells_render_raw_source_without_prefix_or_style kind=ui owner=chat-live
#[test]
fn source_backed_cells_render_raw_source_without_prefix_or_style() {
    kit::pending(
        Owner::ChatLive,
        "no raw scrollback mode (alt+r) on S11: the cells screen draws no source-backed rows (user, assistant table, reasoning, plan) yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 40)
        .script(include_str!("fixtures/cells_3/source_backed_cells.jsonl"))
        .open();
    s.keys("alt+r");
    // Visible half: the raw source of the four cells, in order, without the
    // prompt chevron, bullets or style of the rich view.
    s.assert_in_order(&[
        "hello",
        "world",
        "- item",
        "| A | B |",
        "| - | - |",
        "| x | y |",
        "first thought",
        "second thought",
        "1. Inspect",
        "```sh",
        "cargo test",
    ]);
}

// ORACLE history_cell/tests.rs:358 proposed_plan_cell_renders_markdown_table kind=snap owner=chat-live snap=proposed_plan_cell_renders_markdown_table
#[test]
fn proposed_plan_cell_renders_markdown_table() {
    kit::pending(
        Owner::ChatLive,
        "no proposed-plan cell on S11: the cells screen draws no plan cell; the oracle's snapshot is a Line-level Debug dump with span styles (dim/bold) and the test's default colours, which the kit cannot seed or compare",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!("fixtures/cells_3/proposed_plan_table.jsonl"))
        .open();
    // Visible half: the plan heading and the table rows of the rich view.
    s.assert_in_order(&["Proposed Plan", "Step", "Owner", "Verify", "Codex"]);
    // The oracle's raw check: the table source survives in raw mode.
    s.keys("alt+r");
    s.assert_has_row("| Step | Owner |");
}

// ORACLE history_cell/tests.rs:424 empty_mcp_output_preserves_docs_hyperlink kind=snap owner=chat-live
#[test]
fn empty_mcp_output_preserves_docs_hyperlink() {
    kit::pending(
        Owner::ChatLive,
        "no /mcp output cell on S11: the slash command draws no MCP tools cell yet, and the docs hyperlink is not carried",
    );
    let mut s = Screen::build(Owner::ChatLive).size(80, 8).live().open();
    s.type_text("/mcp");
    s.keys("enter");
    s.assert_in_order(&[
        "/mcp",
        "🔌  MCP Tools",
        "• No MCP servers configured.",
        "See the MCP docs to configure them.",
    ]);
    s.assert_link("MCP docs", "https://developers.openai.com/codex/mcp");
}

// ORACLE history_cell/tests.rs:475 proposed_plan_cell_unwraps_markdown_fenced_table kind=ui owner=chat-live
#[test]
fn proposed_plan_cell_unwraps_markdown_fenced_table() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_3/proposed_plan_fenced_table.jsonl"
        ))
        .open();
    // The oracle asserts a separator rule (`━`) on a table row, and that no row is
    // the markdown fence line.
    s.assert_contains("━");
    s.assert_absent("```markdown");
}

// ORACLE history_cell/tests.rs:495 structured_tool_cell_renders_raw_plain_text_without_prefix_or_style kind=ui owner=chat-live
#[test]
fn structured_tool_cell_renders_raw_plain_text_without_prefix_or_style() {
    kit::pending(
        Owner::ChatLive,
        "no raw scrollback mode (alt+r) on S11: the cells screen draws no MCP tool call rows in raw mode yet",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .script(include_str!(
            "fixtures/cells_3/structured_tool_call_raw.jsonl"
        ))
        .open();
    s.keys("alt+r");
    s.assert_in_order(&["Called search.find_docs(", "alpha", "beta"]);
}

// ORACLE history_cell/tests.rs:522 delegated_message_list_spacing_in_scrollback_and_transcript kind=snap owner=chat-live snap=delegated_message_list_spacing_in_scrollback_and_transcript
#[test]
fn delegated_message_list_spacing_in_scrollback_and_transcript() {
    let s = Screen::build(Owner::ChatLive)
        .size(32, 20)
        .script(include_str!("fixtures/cells_3/delegated_message.jsonl"))
        .open();
    // The chat shows the redrawable transcript's half: the header, the three
    // list items with their wrapped line (the scrollback half joins the same
    // rows, minus the blank rows between the items).
    s.assert_in_order(&[
        "Sent by Codex from task other",
        "• One",
        "• This item has enough words",
        "to wrap",
        "• Three",
    ]);
    // The transcript state (ctrl+t overlay) adds a blank row between list items;
    // it is compared when the overlay is on S11.
}

// ORACLE history_cell/tests.rs:543 raw_mode_toggle_transcript_snapshot kind=snap owner=chat-live snap=raw_mode_toggle_transcript
#[test]
fn raw_mode_toggle_transcript_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no raw scrollback mode (alt+r) on S11: the rich and raw renderings of the three cells are not drawn yet; the kit's snapshot parser also has no frame header for the `label:` sections of this snapshot (rich before:, raw on:, rich after:), so assert_snap_section cannot select them",
    );
    let mut s = Screen::build(Owner::ChatLive)
        .size(40, 60)
        .script(include_str!("fixtures/cells_3/raw_mode_toggle.jsonl"))
        .open();
    s.assert_snap_section("raw_mode_toggle_transcript", "rich before");
    s.keys("alt+r");
    s.assert_snap_section("raw_mode_toggle_transcript", "raw on");
    s.keys("alt+r");
    s.assert_snap_section("raw_mode_toggle_transcript", "rich after");
}

// ORACLE history_cell/tests.rs:600 image_generation_call_renders_saved_path kind=ui owner=chat-live
#[test]
fn image_generation_call_renders_saved_path() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .script(include_str!(
            "fixtures/cells_3/image_generation_saved_path.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "• Generated Image:",
        "  └ A tiny blue square",
        "  └ Saved to: file:///tmp/generated-image.png",
    ]);
}

// ORACLE history_cell/tests.rs:650 unified_exec_interaction_cell_retains_detailed_input kind=snap owner=chat-live
#[test]
fn unified_exec_interaction_cell_retains_detailed_input() {
    kit::pending(
        Owner::ChatLive,
        "no background-terminal interaction cell on S11: a write to a unified-exec terminal is not fed as a step and the cells screen draws no interaction cell",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_in_order(&[
        "↳ Interacted with background terminal · cat",
        "└ line 1",
        "line 2",
        "line 3",
        "line 4",
        "line 5",
        "line 6",
        "line 7",
        "line 8",
        "line 9",
        "line 10",
        "line 11",
        "line 12",
        "line 13",
        "line 14",
        "line 15",
        "line 16",
    ]);
}

// ORACLE history_cell/tests.rs:676 unified_exec_interaction_cell_retains_detailed_wait kind=ui owner=chat-live
#[test]
fn unified_exec_interaction_cell_retains_detailed_wait() {
    kit::pending(
        Owner::ChatLive,
        "no background-terminal wait cell on S11: an empty write (wait) is not fed as a step and the cells screen draws no interaction cell",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.assert_contains("• Waited for background terminal");
}

// ORACLE history_cell/tests.rs:683 final_message_separator_preserves_runtime_metrics_for_short_turns kind=ui owner=chat-live
#[test]
fn final_message_separator_preserves_runtime_metrics_for_short_turns() {
    kit::pending(
        Owner::ChatLive,
        "no final-message separator with runtime metrics on S11: the separator and its RuntimeMetricsSummary are not drawn, and no fixture step carries the summary",
    );
    // The oracle renders at width 600 and expects exactly one row.
    let s = Screen::build(Owner::ChatLive).size(600, 4).live().open();
    s.assert_in_order(&[
        "  Worked for 12s • Local tools:",
        "Local tools: 3 calls (2.5s)",
        "Inference: 2 calls (1.2s)",
        "WebSocket: 1 events send (700ms)",
        "Streams: 6 events (900ms)",
        "4 events received (1.2s)",
        "Responses API overhead: 650ms",
        "Responses API inference: 1.9s",
        "TTFT: 410ms (iapi) 460ms (service)",
        "TBT: 1.2s (iapi) 1.2s (service)",
    ]);
}

// ORACLE history_cell/tests.rs:731 runtime_metrics_label_rounds_fractional_tbt_milliseconds kind=snap owner=chat-live
#[test]
fn runtime_metrics_label_rounds_fractional_tbt_milliseconds() {
    kit::pending(
        Owner::ChatLive,
        "no final-message separator with a TBT runtime metric on S11: the label is part of the separator, which the cells screen does not draw yet",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 4).live().open();
    s.assert_contains("TBT: 2ms (iapi) 5ms (service)");
}

// ORACLE history_cell/tests.rs:742 final_message_separator_includes_worked_label_after_one_minute kind=ui owner=chat-live
#[test]
fn final_message_separator_includes_worked_label_after_one_minute() {
    kit::pending(
        Owner::ChatLive,
        "no final-message separator on S11: the cells screen draws no 'Worked for' separator yet",
    );
    let s = Screen::build(Owner::ChatLive).size(200, 4).live().open();
    s.assert_contains("Worked for");
}

// ORACLE history_cell/tests.rs:751 ps_output_empty_snapshot kind=snap owner=chat-live snap=ps_output_empty_snapshot
#[test]
fn ps_output_empty_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no /ps background-terminal list cell on S11: the slash command draws no process list yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(60, 8).live().open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_snap_region("ps_output_empty_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:758 session_info_uses_availability_nux_tooltip_override kind=ui owner=chat-live
#[test]
fn session_info_uses_availability_nux_tooltip_override() {
    kit::pending(
        Owner::ChatLive,
        "no session-info cell with the availability NUX on S11: the startup banner and the model-availability tooltip are not drawn, and the model availability is not seedable",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_contains("Model just became available");
}

// ORACLE history_cell/tests.rs:780 session_info_availability_nux_tooltip_snapshot kind=snap owner=chat-live snap=session_info_availability_nux_tooltip_snapshot
#[test]
fn session_info_availability_nux_tooltip_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no session-info cell with the availability NUX on S11: the startup banner and the model-availability tooltip are not drawn, and the model availability is not seedable",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_snap_region("session_info_availability_nux_tooltip_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:799 session_info_preserves_styled_tooltip_links kind=snap owner=chat-live snap=session_info_preserves_styled_tooltip_links
#[test]
fn session_info_preserves_styled_tooltip_links() {
    kit::pending(
        Owner::ChatLive,
        "no session-info cell on S11: the tip with **/copy** bold and the community link is not drawn; the oracle's link-column dump is compared through assert_link and assert_text_style",
    );
    let s = Screen::build(Owner::ChatLive).size(30, 24).live().open();
    s.assert_in_order(&[
        "  Tip: Use /copy or ctrl+y;",
        "  visit the Codex community",
        "  forum (https://example.com).",
    ]);
    s.assert_text_style("/copy", "bold");
    s.assert_link("Codex community", "https://example.com");
}

// ORACLE history_cell/tests.rs:851 session_info_first_event_respects_tooltip_preference kind=ui owner=chat-live
#[test]
fn session_info_first_event_respects_tooltip_preference() {
    kit::pending(
        Owner::ChatLive,
        "no session-info cell on S11, and the tooltip preference (show_tooltips = false) is not seedable through the kit; the oracle's second pass of the loop is not reachable yet",
    );
    // show_tooltips = true: the tip is shown, the availability notice is not.
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_contains("To get started");
    s.assert_absent("Model just became available");
    s.assert_absent("model changed:");
}

// ORACLE history_cell/tests.rs:875 session_info_hides_tooltips_when_disabled kind=ui owner=chat-live
#[test]
fn session_info_hides_tooltips_when_disabled() {
    kit::pending(
        Owner::ChatLive,
        "no session-info cell on S11, and show_tooltips = false is not seedable through the kit",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    // The session banner is drawn without the tip: its header line is the oracle's.
    s.assert_contains("OpenAI Codex");
    s.assert_absent("Model just became available");
}

// ORACLE history_cell/tests.rs:894 ps_output_multiline_snapshot kind=snap owner=chat-live snap=ps_output_multiline_snapshot
#[test]
fn ps_output_multiline_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no /ps background-terminal list cell on S11: the multi-line commands and their recent chunks are not seedable and the cells screen draws no process list yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(40, 24).live().open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_snap_region("ps_output_multiline_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:918 ps_output_multiline_long_command_snapshot kind=snap owner=chat-live snap=ps_output_multiline_long_command_snapshot
#[test]
fn ps_output_multiline_long_command_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no /ps background-terminal list cell on S11: a long multi-line command is not seedable and the cells screen draws no process list yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(100, 12).live().open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_snap_region("ps_output_multiline_long_command_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:928 cyber_policy_error_event_snapshot kind=snap owner=chat-live snap=cyber_policy_error_event_snapshot
#[test]
fn cyber_policy_error_event_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no cyber-policy error cell on S11: the daybreak notice (Apply) is not carried by the app-server events the kit feeds, and the cells screen draws no such cell",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
    s.assert_snap_region("cyber_policy_error_event_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:935 cyber_policy_error_event_astra_snapshot kind=snap owner=chat-live snap=cyber_policy_error_event_astra_snapshot
#[test]
fn cyber_policy_error_event_astra_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no cyber-policy error cell on S11: the daybreak notice (Astra) is not carried by the app-server events the kit feeds, and the cells screen draws no such cell",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
    s.assert_snap_region("cyber_policy_error_event_astra_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:942 cyber_policy_error_event_available_snapshot kind=snap owner=chat-live snap=cyber_policy_error_event_available_snapshot
#[test]
fn cyber_policy_error_event_available_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no cyber-policy error cell on S11: the daybreak notices (Disabled, Enabled) are not carried by the app-server events the kit feeds; the oracle joins both cells in one snapshot",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 24).live().open();
    s.assert_snap_region("cyber_policy_error_event_available_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:957 safety_access_block_event_snapshot kind=snap owner=chat-live snap=safety_access_block_event_snapshot
#[test]
fn safety_access_block_event_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no safety-access block cell on S11: the block notice is not carried by the app-server events the kit feeds, and the cells screen draws no such cell",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
    s.assert_snap_region("safety_access_block_event_snapshot", 0, 0);
    // The oracle's second check (raw_lines()[1] equals display row 1 without its indent)
    // is the raw rendering of the same cell; it is compared once the cell is on S11.
}

// ORACLE history_cell/tests.rs:966 cyber_policy_error_event_narrow_snapshot kind=snap owner=chat-live snap=cyber_policy_error_event_narrow_snapshot
#[test]
fn cyber_policy_error_event_narrow_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no cyber-policy error cell on S11: the daybreak notice (Apply) at width 36 is not carried by the app-server events the kit feeds, and the cells screen draws no such cell",
    );
    let s = Screen::build(Owner::ChatLive).size(36, 12).live().open();
    s.assert_snap_region("cyber_policy_error_event_narrow_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:973 ps_output_long_command_snapshot kind=snap owner=chat-live snap=ps_output_long_command_snapshot
#[test]
fn ps_output_long_command_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no /ps background-terminal list cell on S11: a long rg command with a recent chunk is not seedable and the cells screen draws no process list yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(36, 12).live().open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_snap_region("ps_output_long_command_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:985 ps_output_halfwidth_sound_marks_snapshot kind=snap owner=chat-live snap=ps_output_halfwidth_sound_marks_snapshot
#[test]
fn ps_output_halfwidth_sound_marks_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no /ps background-terminal list cell on S11: half-width katakana commands are not seedable and the cells screen draws no process list yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(24, 12).live().open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_snap_region("ps_output_halfwidth_sound_marks_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:995 ps_output_many_sessions_snapshot kind=snap owner=chat-live snap=ps_output_many_sessions_snapshot
#[test]
fn ps_output_many_sessions_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no /ps background-terminal list cell on S11: twenty sessions are not seedable and the cells screen draws no process list yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(32, 30).live().open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_snap_region("ps_output_many_sessions_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:1009 ps_output_chunk_leading_whitespace_snapshot kind=snap owner=chat-live snap=ps_output_chunk_leading_whitespace_snapshot
#[test]
fn ps_output_chunk_leading_whitespace_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "no /ps background-terminal list cell on S11: recent chunks with leading whitespace are not seedable and the cells screen draws no process list yet",
    );
    let mut s = Screen::build(Owner::ChatLive).size(60, 12).live().open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_snap_region("ps_output_chunk_leading_whitespace_snapshot", 0, 0);
}

// ORACLE history_cell/tests.rs:1022 error_event_oversized_input_snapshot kind=snap owner=chat-live snap=error_event_oversized_input_snapshot
#[test]
fn error_event_oversized_input_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(120, 24)
        .script(include_str!(
            "fixtures/cells_3/error_event_oversized_input.jsonl"
        ))
        .open();
    let row = s
        .rows()
        .iter()
        .position(|row| row.starts_with("■ Message exceeds"))
        .expect("the error row");
    s.assert_snap_region("error_event_oversized_input_snapshot", row, 0);
}

// ORACLE history_cell/tests.rs:1031 error_event_bedrock_expired_signature_snapshot kind=snap owner=chat-live snap=error_event_bedrock_expired_signature_snapshot
#[test]
fn error_event_bedrock_expired_signature_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(100, 24)
        .script(include_str!(
            "fixtures/cells_3/error_event_bedrock_expired_signature.jsonl"
        ))
        .open();
    // The oracle's snapshot is the cell's `display_lines`, one unwrapped line; the chat wraps it
    // by words at the terminal's width, so the rows of the cell read as that line.
    let rows = s.rows();
    let first = rows
        .iter()
        .position(|row| row.starts_with("■ Amazon Bedrock"))
        .expect("the error row");
    let cell: Vec<&str> = rows[first..]
        .iter()
        .take_while(|row| !row.is_empty())
        .map(String::as_str)
        .collect();
    assert_eq!(cell.len(), 3, "{cell:#?}");
    assert!(cell.iter().all(|row| row.chars().count() <= 100));
    assert_eq!(
        vec![cell.join(" ")],
        kit::snap_rows("error_event_bedrock_expired_signature_snapshot", "")
    );
}

// ORACLE history_cell/tests.rs:1056 mcp_tools_output_masks_sensitive_values kind=snap owner=chat-live snap=mcp_tools_output_masks_sensitive_values
#[test]
fn mcp_tools_output_masks_sensitive_values() {
    kit::pending(
        Owner::ChatLive,
        "no /mcp tools cell on S11: the stdio and streamable-HTTP server config (env, headers, bearer env var) is not seedable through the kit, and the cells screen draws no MCP tools list",
    );
    let mut s = Screen::build(Owner::ChatLive).size(120, 24).live().open();
    s.type_text("/mcp");
    s.keys("enter");
    s.assert_snap_region("mcp_tools_output_masks_sensitive_values", 0, 0);
}

// ORACLE history_cell/tests.rs:1122 mcp_tools_output_lists_tools_for_hyphenated_server_names kind=snap owner=chat-live snap=mcp_tools_output_lists_tools_for_hyphenated_server_names
#[test]
fn mcp_tools_output_lists_tools_for_hyphenated_server_names() {
    kit::pending(
        Owner::ChatLive,
        "no /mcp tools cell on S11: a hyphenated server name in the MCP config is not seedable through the kit, and the cells screen draws no MCP tools list",
    );
    let mut s = Screen::build(Owner::ChatLive).size(120, 24).live().open();
    s.type_text("/mcp");
    s.keys("enter");
    s.assert_snap_region(
        "mcp_tools_output_lists_tools_for_hyphenated_server_names",
        0,
        0,
    );
}
