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

/// The row of the notice (`⚠ ...`): the oracle's snapshot is the bare cell, the chat draws a
/// bottom pane under the transcript.
fn warning_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .position(|row| row.starts_with("⚠ "))
        .unwrap_or_else(|| panic!("no notice row:\n{}", s.text()))
}

/// The row of a refusal notice (`ⓘ ...`): the oracle's snapshot is the bare cell, the chat draws
/// a bottom pane under the transcript.
fn info_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .position(|row| row.starts_with("ⓘ "))
        .unwrap_or_else(|| panic!("no notice row:\n{}", s.text()))
}

/// The server's error info of a refused cybersecurity request, and the message of a
/// biology-policy refusal (`codexErrorInfo` null: the text says it).
const CYBER: (&str, &str) = ("server fallback message", "\"cyberPolicy\"");
const BIOLOGY: (&str, &str) = (
    "This content was flagged for possible biological risk.",
    "null",
);

/// `error` of `turn` (the thread `thread-1` of the live seed): a refused request.
fn refusal(turn: &str, (message, info): (&str, &str)) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"error","params":{{"threadId":"thread-1","turnId":"{turn}","willRetry":false,"error":{{"message":"{message}","codexErrorInfo":{info},"additionalDetails":null,"misalignment":null}}}}}}}}"#
    )
}

/// `thread/started` of `thread-1` again: the server's thread now runs `model`, Daybreak on or off.
fn thread_started(model: &str, daybreak: bool) -> String {
    format!(
        r#"{{"step":"notify","message":{{"method":"thread/started","params":{{"thread":{{"id":"thread-1","environments":null,"extra":null,"sessionId":"thread-1","forkedFromId":null,"attachedToThreadId":null,"parentThreadId":null,"preview":"","ephemeral":false,"section":null,"sectionEnteredAt":null,"projectId":null,"historyMode":"legacy","modelProvider":"openai","model":"{model}","reasoningEffort":null,"createdAt":0,"updatedAt":0,"recencyAt":0,"status":{{"type":"idle"}},"path":null,"cwd":"/tmp/project","cliVersion":"0.0.0","originator":null,"source":"cli","canAcceptDirectInput":true,"threadSource":null,"agentNickname":null,"agentRole":null,"gitInfo":null,"name":null,"daybreakEnabled":{daybreak},"turns":[]}}}}}}}}"#
    )
}

// ---- oracle: history_cell/tests.rs (39 screen-level tests here; 3 state tests left to chat-live)
// ORACLE history_cell/tests.rs:40 connected_server_version_notice_snapshot kind=snap owner=chat-live snap=connected_server_version_notice_snapshot
#[test]
fn connected_server_version_notice_snapshot() {
    // The host words it (host/src/update.rs `server_version_message`, tested there) and announces
    // it to the first conversation; the remote service is not the local daemon: no `/daemon` lines.
    let s = Screen::build(Owner::ChatLive)
        .size(100, 12)
        .live()
        .server_notice(
            "A background Codex service is running v0.152.1, older than your Codex CLI v0.153.0.",
            false,
        )
        .open();
    s.assert_snap_region(
        "connected_server_version_notice_snapshot",
        warning_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:67 local_daemon_version_notice_snapshot kind=snap owner=chat-live snap=local_daemon_alpha_mismatch snap=local_daemon_source_mismatch snap=local_daemon_version_notice_snapshot
#[test]
fn local_daemon_version_notice_snapshot() {
    // One screen per (client, server) pair of the oracle loop.
    for (message, snapshot) in [
        (
            "A background Codex service is running v0.152.1, older than your Codex CLI v0.153.0.",
            "local_daemon_version_notice_snapshot",
        ),
        (
            "A background Codex service is running v0.156.0, different from your Codex CLI v0.155.0-alpha.12.",
            "local_daemon_alpha_mismatch",
        ),
        (
            "A background Codex service is running v0.156.0, different from your Codex CLI v0.0.0.",
            "local_daemon_source_mismatch",
        ),
    ] {
        let s = Screen::build(Owner::ChatLive)
            .size(100, 12)
            .live()
            .server_notice(message, true)
            .open();
        s.assert_snap_region(snapshot, warning_row(&s), 0);
    }
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
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 40)
        .script(include_str!("fixtures/cells_3/source_backed_cells.jsonl"))
        .open();
    s.keys("alt+r");
    // Visible half: the raw source of the cells, in order, without the prompt
    // chevron, bullets or style of the rich view. The reasoning cell is
    // transcript-only in the live chat (oracle `ReasoningSummaryCell::transcript_only`,
    // the unit test builds it with false): it has no row on this screen; its raw
    // lines are checked in tests/pages/cells.rs `raw_mode`.
    s.assert_in_order(&[
        "hello",
        "world",
        "- item",
        "| A | B |",
        "| - | - |",
        "| x | y |",
        "1. Inspect",
        "```sh",
        "cargo test",
    ]);
}

// ORACLE history_cell/tests.rs:358 proposed_plan_cell_renders_markdown_table kind=snap owner=chat-live snap=proposed_plan_cell_renders_markdown_table
#[test]
fn proposed_plan_cell_renders_markdown_table() {
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
    kit::pending(
        Owner::ChatLive,
        "G73: item 3, the oracle draws a table as columns two spaces apart with a `━` rule under the header and `─` rules between logical rows (tui markdown_render.rs:112 TABLE_HEADER_SEPARATOR_CHAR, :1851 render_table_separator); rtwk's MarkdownView draws a `│` grid with one `─┼─` header rule",
    );
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
    // The snapshot's three frames are the oracle's cell-level renderings (`display_lines`,
    // `raw_lines`, `display_lines`) joined without the chat's blank rows between cells, the
    // toggle note or the bottom pane; the rich frames also carry the oracle's `• • first item`
    // list bullet. The chat is compared on what the raw frame says: its rows, in order.
    let mut s = Screen::build(Owner::ChatLive)
        .size(60, 60)
        .script(include_str!("fixtures/cells_3/raw_mode_toggle.jsonl"))
        .open();
    s.assert_in_order(&["› Please format this", "for copying"]);
    s.keys("alt+r");
    s.assert_in_order(&[
        "Please format this",
        "for copying",
        "- first item",
        "- second item",
        "| Col | Value |",
        "| --- | --- |",
        "| code | `x = 1` |",
        "```text",
        "copy me",
        "```",
        "Called workspace.inspect({\"path\":\"README.md\"})",
        "structured output",
        "second line",
    ]);
    s.assert_absent("› Please format this");
    s.keys("alt+r");
    s.assert_in_order(&["› Please format this", "for copying"]);
    s.assert_absent("| --- | --- |");
    s.assert_absent("```text");
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
        "the cell is built (cells::interaction, rows equal in tests/pages/cells.rs notice_cells); need: threads.rs Item::is_detailed_only asks Notice::detailed_only, and the unified-exec flow appends Notice::TerminalInteraction (ThreadEv::Appended), which the kit cannot seed",
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
        "the cell is built (cells::interaction, rows equal in tests/pages/cells.rs notice_cells); need: threads.rs Item::is_detailed_only asks Notice::detailed_only, and the unified-exec flow appends Notice::TerminalInteraction (ThreadEv::Appended), which the kit cannot seed",
    );
    let s = Screen::build(Owner::ChatLive).size(80, 6).live().open();
    s.assert_contains("• Waited for background terminal");
}

// ORACLE history_cell/tests.rs:683 final_message_separator_preserves_runtime_metrics_for_short_turns kind=ui owner=chat-live
#[test]
fn final_message_separator_preserves_runtime_metrics_for_short_turns() {
    // The oracle renders at width 600 and expects exactly one row.
    let s = Screen::build(Owner::ChatLive)
        .size(600, 12)
        .script(include_str!(
            "fixtures/cells_3/final_message_separator_preserves_runtime_metrics_for_short_turns.jsonl"
        ))
        .open();
    let Some(y) = s.rows().iter().position(|row| row.contains("Worked for")) else {
        panic!("no separator row:\n{}", s.text());
    };
    // One row holds the whole label: it starts and ends on the row's first and last parts.
    s.assert_row_starts_with(y, "  Worked for 12s • Local tools:");
    s.assert_row_ends_with(y, "TBT: 1.2s (iapi) 1.2s (service)");
    for part in [
        "Local tools: 3 calls (2.5s)",
        "Inference: 2 calls (1.2s)",
        "WebSocket: 1 events send (700ms)",
        "Streams: 6 events (900ms)",
        "4 events received (1.2s)",
        "Responses API overhead: 650ms",
        "Responses API inference: 1.9s",
        "TTFT: 410ms (iapi) 460ms (service)",
    ] {
        s.assert_contains(part);
    }
}

// ORACLE history_cell/tests.rs:731 runtime_metrics_label_rounds_fractional_tbt_milliseconds kind=snap owner=chat-live
#[test]
fn runtime_metrics_label_rounds_fractional_tbt_milliseconds() {
    let s = Screen::build(Owner::ChatLive)
        .size(100, 12)
        .script(include_str!(
            "fixtures/cells_3/runtime_metrics_label_rounds_fractional_tbt_milliseconds.jsonl"
        ))
        .open();
    // The oracle prints the label alone; here it follows the turn's elapsed time.
    s.assert_has_row("  Worked for 12s • TBT: 2ms (iapi) 5ms (service)");
}

// ORACLE history_cell/tests.rs:742 final_message_separator_includes_worked_label_after_one_minute kind=ui owner=chat-live
#[test]
fn final_message_separator_includes_worked_label_after_one_minute() {
    let s = Screen::build(Owner::ChatLive)
        .size(200, 12)
        .script(include_str!(
            "fixtures/cells_3/final_message_separator_includes_worked_label_after_one_minute.jsonl"
        ))
        .open();
    s.assert_has_row("  Worked for 1m 1s");
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
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .startup(false, Some("Model just became available"))
        .open();
    s.assert_contains("Model just became available");
}

// ORACLE history_cell/tests.rs:780 session_info_availability_nux_tooltip_snapshot kind=snap owner=chat-live snap=session_info_availability_nux_tooltip_snapshot
#[test]
fn session_info_availability_nux_tooltip_snapshot() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .startup(false, Some("Model just became available"))
        .cwd("/tmp/project")
        .open();
    // The oracle's snapshot, row by row: its header reads `(v0.0.0)` where ours reads the
    // running build's version (`transcript_header.rs` drops the source build's 0.0.0).
    let rows = s.rows();
    let Some(y) = rows.iter().position(|row| row.contains(">_ OpenAI Codex")) else {
        panic!("no session header:\n{}", s.text());
    };
    let at = |row: usize| rows.get(row).map_or("", |row| row.trim_end());
    assert_eq!(at(y.saturating_sub(1)), "", "a blank row above the header");
    s.assert_row_starts_with(y, "  >_ OpenAI Codex (v");
    assert!(at(y).ends_with(')'), "{}", at(y));
    assert_eq!(at(y + 1), "     /tmp/project");
    assert_eq!(at(y + 2), "");
    assert_eq!(at(y + 3), "  Tip: Model just became available");
}

// ORACLE history_cell/tests.rs:799 session_info_preserves_styled_tooltip_links kind=snap owner=chat-live snap=session_info_preserves_styled_tooltip_links
#[test]
fn session_info_preserves_styled_tooltip_links() {
    let s = Screen::build(Owner::ChatLive)
        .size(30, 24)
        .live()
        .startup(
            false,
            Some(
                "Use **/copy** or `ctrl+y`; visit the [Codex community forum](https://example.com).",
            ),
        )
        .open();
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
    // The first run: the help shows (tooltips on) and the availability notice is taken unseen.
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .startup(true, Some("Model just became available"))
        .open();
    s.assert_contains("To get started");
    s.assert_absent("Model just became available");
    s.assert_absent("model changed:");
    // show_tooltips = false: the header alone.
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .tooltips_off()
        .startup(true, Some("Model just became available"))
        .open();
    s.assert_absent("To get started");
    s.assert_absent("Model just became available");
    s.assert_absent("model changed:");
}

// ORACLE history_cell/tests.rs:875 session_info_hides_tooltips_when_disabled kind=ui owner=chat-live
#[test]
fn session_info_hides_tooltips_when_disabled() {
    let s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .tooltips_off()
        .startup(false, Some("Model just became available"))
        .cwd("/tmp/project")
        .open();
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
    // No fact of the account: the refusal words as `Apply`.
    let mut s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
    s.feed(&refusal("turn-1", CYBER));
    s.assert_snap_region("cyber_policy_error_event_snapshot", info_row(&s), 0);
}

// ORACLE history_cell/tests.rs:935 cyber_policy_error_event_astra_snapshot kind=snap owner=chat-live snap=cyber_policy_error_event_astra_snapshot
#[test]
fn cyber_policy_error_event_astra_snapshot() {
    // An eligible account on the Astra model, whose catalog lists no Daybreak program.
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 12)
        .live()
        .daybreak(&[("gpt-6-astra", true, false)])
        .open();
    s.feed(&thread_started("gpt-6-astra", false));
    s.feed(&refusal("turn-1", CYBER));
    s.assert_snap_region("cyber_policy_error_event_astra_snapshot", info_row(&s), 0);
}

// ORACLE history_cell/tests.rs:942 cyber_policy_error_event_available_snapshot kind=snap owner=chat-live snap=cyber_policy_error_event_available_snapshot
#[test]
fn cyber_policy_error_event_available_snapshot() {
    // Daybreak is off, then on: two refusals, each worded by the setting of its moment.
    let mut s = Screen::build(Owner::ChatLive)
        .size(80, 24)
        .live()
        .daybreak(&[("gpt-5.6-sol", true, true)])
        .open();
    s.feed(&thread_started("gpt-5.6-sol", false));
    s.feed(&refusal("turn-1", CYBER));
    s.feed(&thread_started("gpt-5.6-sol", true));
    s.feed(&refusal("turn-2", CYBER));
    s.assert_snap_region(
        "cyber_policy_error_event_available_snapshot",
        info_row(&s),
        0,
    );
}

// ORACLE history_cell/tests.rs:957 safety_access_block_event_snapshot kind=snap owner=chat-live snap=safety_access_block_event_snapshot
#[test]
fn safety_access_block_event_snapshot() {
    let mut s = Screen::build(Owner::ChatLive).size(80, 12).live().open();
    s.feed(&refusal("turn-1", BIOLOGY));
    s.assert_snap_region("safety_access_block_event_snapshot", info_row(&s), 0);
    // The oracle's second check (raw_lines()[1] equals display row 1 without its indent)
    // is the raw rendering of the same cell; it is compared once the cell is on S11.
}

// ORACLE history_cell/tests.rs:966 cyber_policy_error_event_narrow_snapshot kind=snap owner=chat-live snap=cyber_policy_error_event_narrow_snapshot
#[test]
fn cyber_policy_error_event_narrow_snapshot() {
    kit::pending(
        Owner::ChatLive,
        "G49 (TextWrap::Word does not break after `/`): the cell is drawn and its prose rows equal; the two links wrap at the width (`https://help.openai.com/en/artic` / `les/20001326`) where the oracle breaks after `https://`, `en/` and `enterprise-`",
    );
    let mut s = Screen::build(Owner::ChatLive).size(36, 20).live().open();
    s.feed(&refusal("turn-1", CYBER));
    s.assert_snap_region("cyber_policy_error_event_narrow_snapshot", info_row(&s), 0);
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
    // The oracle's snapshot is the cell's `display_lines`, one unwrapped line: the terminal is
    // as wide as that line, so the cell reads as the snapshot's single row.
    let (width, _) = kit::snap_size("error_event_bedrock_expired_signature_snapshot");
    let s = Screen::build(Owner::ChatLive)
        .size(u16::try_from(width + 2).unwrap_or(u16::MAX), 24)
        .script(include_str!(
            "fixtures/cells_3/error_event_bedrock_expired_signature.jsonl"
        ))
        .open();
    let row = s
        .rows()
        .iter()
        .position(|row| row.starts_with("■ Amazon Bedrock"))
        .expect("the error row");
    s.assert_snap_region("error_event_bedrock_expired_signature_snapshot", row, 0);
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
