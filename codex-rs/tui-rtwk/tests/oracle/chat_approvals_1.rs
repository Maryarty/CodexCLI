//! Shard `chat_approvals_1` (screens S13, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

/// The turn of the unified-exec fixtures ends.
const TURN_COMPLETED: &str = r#"{"step":"notify","message":{"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","rootTurnId":null,"items":[],"itemsView":"full","status":"completed","error":null,"startedAt":null,"completedAt":null,"durationMs":null}}}}"#;

/// A running turn `turn-1` of the live seed's thread `thread-1`.
const RUNNING_TURN: &str = include_str!("fixtures/chat_status_1/running_turn.jsonl");

/// `item/completed` of `item` in the turn `turn-1`.
fn completed_item(item: &serde_json::Value) -> String {
    serde_json::json!({"step": "notify", "message": {"method": "item/completed", "params": {
        "threadId": "thread-1", "turnId": "turn-1", "item": item, "completedAtMs": 0}}})
    .to_string()
}

/// The screen row a cell starts at (the oracle's snapshot is the bare cell).
fn cell_at(s: &Screen, head: &str) -> usize {
    s.rows()
        .iter()
        .position(|row| row.starts_with(head))
        .unwrap_or_else(|| panic!("no `{head}` row:\n{}", s.text()))
}

/// The screen row of the status line (`• Working`, `• Waiting for background terminal`): the
/// oracle's snapshots of the bottom pane start there.
fn status_row(s: &Screen) -> usize {
    s.rows()
        .iter()
        .position(|row| row.starts_with("• Working") || row.starts_with("• Waiting"))
        .unwrap_or_else(|| panic!("no status row on the screen:\n{}", s.text()))
}

// ---- oracle: chatwidget/tests/approval_requests.rs (8 screen-level tests here; 2 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/approval_requests.rs:8 exec_approval_emits_proposed_command_and_decision_history kind=snap owner=dialogs-pane
#[test]
fn exec_approval_emits_proposed_command_and_decision_history() {
    kit::delta(
        Owner::DialogsPane,
        "command row syntax colours: ours are rtwk-rich's bash token colours (Blue, Reset), the oracle's are the theme's Rgb(137,180,250) and Rgb(205,214,244); the rows are the oracle's",
    );
    // oracle: the proposed exec request (echo hello world, reason) renders as the modal, no history cell;
    // `y` answers it and the history gets the decision cell.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 16)
        .syntax_theme("catppuccin-mocha")
        .script(include_str!("fixtures/chat_approvals_1/exec_short.jsonl"))
        .open();
    s.assert_snap("exec_approval_modal_exec");
    s.keys("y");
    s.assert_in_order(&["✔ You approved codex to run echo hello world this time"]);
}

// ORACLE chatwidget/tests/approval_requests.rs:53 app_server_exec_approval_request_splits_shell_wrapped_command kind=snap owner=dialogs-pane
#[test]
fn app_server_exec_approval_request_splits_shell_wrapped_command() {
    kit::na(
        Owner::DialogsPane,
        "asserts the parsed request.command vector and the pending desktop notification text 'Approval requested: ...'; neither is drawn on the screen",
    );
}

// ORACLE chatwidget/tests/approval_requests.rs:98 app_server_write_stdin_approval_renders_terminal_input kind=snap owner=dialogs-pane
#[test]
fn app_server_write_stdin_approval_renders_terminal_input() {
    kit::delta(
        Owner::DialogsPane,
        "hint row colour: ours keeps the footer cell's fg Indexed(245) under the dialog where the oracle's overlay is Reset, one cell bold where the oracle's is dim; the words are the oracle's",
    );
    // oracle: the write_stdin request shows the modal at 80x11; the desktop notification text is not drawn and is not asserted.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 11)
        .script(include_str!("fixtures/chat_approvals_1/write_stdin.jsonl"))
        .open();
    s.assert_snap("write_stdin_approval_modal");
}

// ORACLE chatwidget/tests/approval_requests.rs:204 network_exec_approval_history_describes_session_host_allowance kind=snap owner=dialogs-pane
#[test]
fn network_exec_approval_history_describes_session_host_allowance() {
    // oracle: `a` (accept for the session) answers the network request; the decision cell names the host and the session.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/network_session.jsonl"
        ))
        .open();
    s.keys("a");
    s.assert_in_order(&[
        "✔ You approved codex network access to https://example.com:8443 every time this",
        "session",
    ]);
}

// ORACLE chatwidget/tests/approval_requests.rs:251 network_exec_approval_history_describes_one_time_host_allowance kind=snap owner=dialogs-pane
#[test]
fn network_exec_approval_history_describes_one_time_host_allowance() {
    // oracle: `y` (accept once) answers the HTTP network request; the decision cell names the host.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_approvals_1/network_once.jsonl"))
        .open();
    s.keys("y");
    s.assert_in_order(&["✔ You approved codex network access to http://example.com this time"]);
}

// ORACLE chatwidget/tests/approval_requests.rs:294 network_exec_approval_history_describes_canceled_host_request kind=snap owner=dialogs-pane
#[test]
fn network_exec_approval_history_describes_canceled_host_request() {
    kit::pending(
        Owner::DialogsPane,
        "G72: `n` cancels and the decision cell is drawn, but `TextWrap::Word` breaks the target `socks5-tcp://example.com:1080` after the hyphen (`socks5-` / `tcp://example.com:1080`), the oracle's URL-aware wrap keeps it whole on the next row",
    );
    // oracle: `n` cancels the SOCKS5 network request; the decision cell names the request and the URL.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/network_cancel.jsonl"
        ))
        .open();
    s.keys("n");
    s.assert_in_order(&[
        "✗ You canceled the request for codex network access to",
        "socks5-tcp://example.com:1080",
    ]);
}

// ORACLE chatwidget/tests/approval_requests.rs:386 exec_approval_uses_approval_id_when_present kind=ui owner=dialogs-pane
#[test]
fn exec_approval_uses_approval_id_when_present() {
    kit::na(
        Owner::DialogsPane,
        "asserts the ExecApproval op sent to the backend (id approval-subcommand, decision Accept); the op is not drawn",
    );
}

// ORACLE chatwidget/tests/approval_requests.rs:433 exec_approval_decision_truncates_multiline_and_long_commands kind=snap owner=dialogs-pane
#[test]
fn exec_approval_decision_truncates_multiline_and_long_commands() {
    // oracle: the multi-line request's first line shows in the modal; `n` aborts it into the history cell;
    // then the 200-character request (no reason) is aborted the same way and its cell is truncated.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/exec_multiline_lines.jsonl"
        ))
        .open();
    s.assert_contains("echo line1");
    s.keys("n");
    s.assert_in_order(&["✗ You canceled the request to run echo line1 ..."]);
    // The oracle feeds the second request into the same widget; a screen on a script takes its feed
    // from the file, so the long request is its own screen (the cell does not depend on the first).
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_approvals_1/exec_long.jsonl"))
        .open();
    s.keys("n");
    s.assert_in_order(&[
        "✗ You canceled the request to run echo",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...",
    ]);
}

// ---- oracle: chatwidget/tests/exec_flow.rs (46 screen-level tests here; 8 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/exec_flow.rs:6 external_writer_snapshot_freezes_active_command_and_mcp_rows kind=snap owner=dialogs-pane
#[test]
fn external_writer_snapshot_freezes_active_command_and_mcp_rows() {
    // oracle: with a running command (sleep 5) or a running MCP call, showing the external writer
    // thread freezes the active row (no animation, the turn no longer running); the rendered rows
    // are the snapshot, the two renders joined by `---`. The thread is shown read-only (the server
    // refuses the resume) with the in-flight call of its recorded history.
    let name = "external_writer_snapshot_freezes_active_command_and_mcp_rows";
    let exec = serde_json::json!({
        "type": "commandExecution", "id": "call-running", "pluginId": null, "scriptPath": null,
        "command": "bash -lc 'sleep 5'", "cwd": "/tmp/project", "processId": null,
        "source": "agent", "status": "inProgress",
        "commandActions": [{"type": "unknown", "command": "sleep 5"}],
        "aggregatedOutput": null, "exitCode": null, "durationMs": null,
        "startedAtMs": 0, "completedAtMs": null
    });
    let mcp = serde_json::json!({
        "type": "mcpToolCall", "id": "mcp-running", "server": "server", "tool": "tool",
        "status": "inProgress", "arguments": null, "appContext": null,
        "mcpAppUiResourceUri": null, "mcpAppUi": null, "pluginId": null, "readOnlyHint": null,
        "result": null, "error": null, "durationMs": null
    });
    for (at, item) in [(0, exec), (1, mcp)] {
        let mut turn = kit::replay_turn("turn-1", serde_json::json!([item]));
        turn["status"] = serde_json::json!("inProgress");
        turn["completedAt"] = serde_json::Value::Null;
        turn["durationMs"] = serde_json::Value::Null;
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .inline(&[])
            .foreign_writer("thread-foreign")
            .live()
            .open();
        s.history_page(vec![turn], None);
        s.assert_contains("This conversation is open in another app");
        s.assert_absent("Working");
        if at == 0 {
            s.snap(name)
                .cut(at)
                .at(cell_at(&s, "• Running sleep 5"), 0)
                .assert();
        } else {
            // the oracle's call has `arguments: None` (`server.tool()`), which the server's item
            // cannot say: its `arguments` is a JSON value, and the null the item carries reads
            // `(null)` in the oracle's own app path too (`tool_lifecycle.rs` `Some(arguments)`)
            s.assert_has_row("• Calling server.tool(null)");
        }
    }
}

// ORACLE chatwidget/tests/exec_flow.rs:51 replayed_background_terminals_belong_to_the_displayed_thread kind=snap owner=dialogs-pane
#[test]
fn replayed_background_terminals_belong_to_the_displayed_thread() {
    // The fork replays the inherited command (`sleep 100`, a terminal of the parent) as history
    // and restores its own: the server lists `sleep 200` (live) and `sleep 300`, which ends while
    // the list is in flight.
    let unified_exec = |id: &str, process: &str, command: &str, status: &str| {
        serde_json::json!({
            "type": "commandExecution", "id": id, "pluginId": null, "scriptPath": null,
            "command": command, "cwd": "/tmp/project", "processId": process,
            "source": "unifiedExecStartup", "status": status, "commandActions": [],
            "aggregatedOutput": null, "exitCode": null, "durationMs": null,
            "startedAtMs": 0, "completedAtMs": null
        })
    };
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .replay(vec![kit::replay_turn(
            "turn-1",
            serde_json::json!([unified_exec(
                "parent-call",
                "42",
                "bash -lc 'sleep 100'",
                "inProgress"
            )]),
        )])
        .open();
    s.wait_for("Ask Codex to do anything");
    // the end of `sleep 300` is seen before the list arrives
    let ended = unified_exec("exited-call", "44", "sleep 300", "completed");
    s.feed(
        &serde_json::json!({"step": "notify", "message": {"method": "item/completed", "params": {
            "threadId": kit::REPLAYED, "turnId": "turn-1", "item": ended, "completedAtMs": 0}}})
        .to_string(),
    );
    s.terminals_listed(&[
        ("child-call", "43", "sleep 200"),
        ("exited-call", "44", "sleep 300"),
    ]);
    s.type_text("/ps");
    s.keys("enter");
    s.assert_in_order(&["/ps", "Background terminals", "• sleep 200"]);
    s.assert_absent("sleep 300");
    s.assert_absent("sleep 100");
    s.assert_contains("1 background terminal running");
}

// ORACLE chatwidget/tests/exec_flow.rs:95 replayed_command_completion_preserves_tracking_without_duplicate_starts kind=ui owner=dialogs-pane
#[test]
fn replayed_command_completion_preserves_tracking_without_duplicate_starts() {
    // oracle: a replayed completion of a unified-exec startup writes one history cell and no
    // duplicate start; no command and no terminal stays tracked.
    let item = serde_json::json!({
        "type": "commandExecution", "id": "call-replay", "pluginId": null, "scriptPath": null,
        "command": "cat replay", "cwd": "/tmp/project", "processId": "process-replay",
        "source": "unifiedExecStartup", "status": "completed",
        "commandActions": [{"type": "unknown", "command": "cat replay"}],
        "aggregatedOutput": null, "exitCode": null, "durationMs": null,
        "startedAtMs": 0, "completedAtMs": 0
    });
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .replay(vec![kit::replay_turn("turn-1", serde_json::json!([item]))])
        .open();
    s.assert_in_order(&["• Ran cat replay", "└ (no output)"]);
    s.assert_text_count("• Ran cat replay", 1);
    s.assert_absent("• Running");
    s.assert_absent("background terminal");
}

// ORACLE chatwidget/tests/exec_flow.rs:168 failed_exploration_keeps_overlapping_commands_active_until_all_finish kind=snap owner=dialogs-pane
#[test]
fn failed_exploration_keeps_overlapping_commands_active_until_all_finish() {
    // oracle: a failed `ls missing` and a running `cat foo.txt` stay one active Explored cell until all finish.
    // (The `streamed output` the oracle reads from the detailed transcript lines is the Ctrl+T pager's, not the
    // compact cell's; the compact one counts the exits behind its header.)
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!("fixtures/chat_approvals_1/failed_exploration_keeps_overlapping_commands_active_until_all_finish_1.jsonl"))
        .open();
    s.assert_screen_row_then(
        "• Exploring · 1 nonzero exit",
        &[
            "  └ List missing (exit 1)",
            "    Read foo.txt, bar.txt",
            "    + Show details",
        ],
    );
    s.feed(include_str!("fixtures/chat_approvals_1/failed_exploration_keeps_overlapping_commands_active_until_all_finish_2.jsonl"));
    s.assert_screen_row_then(
        "• Explored · 1 nonzero exit",
        &[
            "  └ List missing (exit 1)",
            "    Read foo.txt, bar.txt",
            "    + Show details",
        ],
    );
    s.feed(include_str!("fixtures/chat_approvals_1/failed_exploration_keeps_overlapping_commands_active_until_all_finish_3.jsonl"));
    s.assert_screen_row_then(
        "• Explored · 1 nonzero exit",
        &[
            "  └ List missing (exit 1)",
            "    Read foo.txt, bar.txt, later.txt",
            "    + Show details",
        ],
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:207 exploration_nonzero_exits_remain_visible_beside_successful_reads kind=snap owner=dialogs-pane
#[test]
fn exploration_nonzero_exits_remain_visible_beside_successful_reads() {
    // oracle: six exploration commands, the non-zero exits stay visible beside the successful reads.
    // The owned transcript draws the compact form (`compact_exploration_with_failures`: the header
    // says how many exits were not 0, three rows keep the first lines); the inline `active_blob`
    // (all seven lines) is the scrollback form (S27) and is not drawn here.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!(
            "fixtures/chat_approvals_1/exploration_nonzero_exits_remain_visible_beside_successful_reads.jsonl"
        ))
        .open();
    s.assert_in_order(&[
        "• Explored · 4 nonzero exits",
        "└ Read first.txt",
        "Read missing.txt (exit 1)",
        "Read later.txt",
    ]);
    // the red exit of cat missing.txt (the blank before it keeps no colour of its own)
    s.assert_text_style("(exit 1)", "fg=Red");
}

// ORACLE chatwidget/tests/exec_flow.rs:258 exploration_reasoning_during_followup_command_stays_ordered kind=snap owner=dialogs-pane
#[test]
fn exploration_reasoning_during_followup_command_stays_ordered() {
    // oracle: `ls` then reasoning "Read the file next.", `cat file.txt` with reasoning "Waiting for the file.";
    // the active transcript keeps that order. The snapshot is `active_cell_transcript_lines`: the
    // detailed transcript (Ctrl+T).
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!(
            "fixtures/chat_approvals_1/exploration_reasoning_during_followup_command_stays_ordered.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_snap_region(
        "exploration_reasoning_during_followup_command_stays_ordered",
        cell_at(&s, "$ ls"),
        0,
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:274 adjacent_exploration_groups_across_reasoning_live_and_replayed kind=snap owner=dialogs-pane
#[test]
fn adjacent_exploration_groups_across_reasoning_live_and_replayed() {
    // oracle: the live and the replayed screens render the same Compact (the inline stream), Expanded
    // (the detailed transcript) and Raw text.
    let script = include_str!(
        "fixtures/chat_approvals_1/adjacent_exploration_groups_across_reasoning_live_and_replayed.jsonl"
    );
    let labels = ["Compact:", "Expanded:", "Raw:"];
    let name = "adjacent_exploration_groups_across_reasoning_live_and_replayed";
    let compact = Screen::build(Owner::DialogsPane)
        .size(80, 60)
        .inline(&[])
        .live()
        .script(script)
        .open();
    compact
        .snap(name)
        .labelled(&labels)
        .frame("Compact:")
        .at(cell_at(&compact, "• Explored"), 0)
        .assert();
    let mut expanded = Screen::build(Owner::DialogsPane)
        .size(80, 60)
        .live()
        .script(script)
        .open();
    expanded.keys("ctrl+t");
    expanded
        .snap(name)
        .labelled(&labels)
        .frame("Expanded:")
        .at(cell_at(&expanded, "$ ls missing"), 0)
        .assert();
    let mut raw = Screen::build(Owner::DialogsPane)
        .size(80, 60)
        .live()
        .script(script)
        .open();
    raw.keys("alt+r");
    raw.snap(name)
        .labelled(&labels)
        .frame("Raw:")
        .at(cell_at(&raw, "$ ls missing"), 0)
        .assert();
}

// ORACLE chatwidget/tests/exec_flow.rs:378 replayed_commands_preserve_individual_output_and_failure_status kind=snap owner=dialogs-pane
#[test]
fn replayed_commands_preserve_individual_output_and_failure_status() {
    // oracle: four replayed command cells keep their own output; the failed and declined ones keep
    // their status. The snapshot is the cells' detailed transcript (Ctrl+T), one per command.
    let command = |id: &str, output: &str, source: &str, status: &str, exit: serde_json::Value| {
        serde_json::json!({
            "type": "commandExecution", "id": id, "pluginId": null, "scriptPath": null,
            "command": format!("printf {output}"), "cwd": "/tmp/project", "processId": null,
            "source": source, "status": status, "commandActions": [],
            "aggregatedOutput": format!("{output}\n"), "exitCode": exit, "durationMs": 5,
            "startedAtMs": null, "completedAtMs": null
        })
    };
    let items = serde_json::json!([
        command("call-first", "first", "agent", "completed", 0.into()),
        command(
            "call-second",
            "second",
            "unifiedExecStartup",
            "completed",
            0.into()
        ),
        command(
            "call-failed",
            "failure",
            "unifiedExecStartup",
            "failed",
            7.into()
        ),
        command(
            "call-declined",
            "declined",
            "agent",
            "declined",
            serde_json::Value::Null
        ),
    ]);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 40)
        .replay(vec![kit::replay_turn("turn-1", items)])
        .open();
    s.keys("ctrl+t");
    s.assert_in_order(&[
        "$ printf first",
        "first",
        "✓ • 5ms",
        "$ printf second",
        "second",
        "✓ • 5ms",
        "$ printf failure",
        "failure",
        "✗ (7) • 5ms",
        "$ printf declined",
        "declined",
        "✗ (1) • 5ms",
    ]);
}

// ORACLE chatwidget/tests/exec_flow.rs:493 unified_exec_begin_restores_working_status_snapshot kind=snap owner=dialogs-pane snap=unified_exec_begin_restores_working_status
#[test]
fn unified_exec_begin_restores_working_status_snapshot() {
    // oracle: a preamble line, then a unified exec begin: the working status shows again, with the
    // background terminal in it and no row of the command (the snapshot is the bottom pane, 80 wide).
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/unified_exec_begin_restores_working_status_snapshot.jsonl"
        ))
        .open();
    s.assert_absent("Running sleep 2");
    s.assert_snap_region(
        "unified_exec_begin_restores_working_status",
        status_row(&s).saturating_sub(1),
        0,
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:517 exec_history_cell_shows_working_then_completed kind=ui owner=dialogs-pane
#[test]
fn exec_history_cell_shows_working_then_completed() {
    // oracle: nothing is flushed on begin; the end flushes one cell with "• Ran" and the command text.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!(
            "fixtures/chat_approvals_1/exec_history_cell_shows_working_then_completed.jsonl"
        ))
        .open();
    // needs feed: begin `echo done` (call-1), then end it with exit 0
    s.assert_contains("• Ran");
    s.assert_contains("echo done");
}

// ORACLE chatwidget/tests/exec_flow.rs:547 exec_history_cell_shows_working_then_failed kind=ui owner=dialogs-pane
#[test]
fn exec_history_cell_shows_working_then_failed() {
    // oracle: the failed end flushes one cell with the command and its error text. The owned transcript draws the
    // compact form (`compact_command_lines`: `Exited (status 2)` for a command that did not exit 0); the
    // `• Ran false` is the scrollback form (S27).
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!(
            "fixtures/chat_approvals_1/exec_history_cell_shows_working_then_failed.jsonl"
        ))
        .open();
    s.assert_screen_row_then("• Exited (status 2) false", &["  └ Bloop"]);
}

// ORACLE chatwidget/tests/exec_flow.rs:571 exec_end_without_begin_uses_event_command kind=ui owner=dialogs-pane
#[test]
fn exec_end_without_begin_uses_event_command() {
    // oracle: an end with no begin renders from the event's command; the call id is not shown.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!(
            "fixtures/chat_approvals_1/exec_end_without_begin_uses_event_command.jsonl"
        ))
        .open();
    // needs feed: exec end of call-orphan with command `echo orphaned`, exit 0, no begin
    s.assert_contains("• Ran echo orphaned");
    s.assert_absent("call-orphan");
}

// ORACLE chatwidget/tests/exec_flow.rs:617 exec_end_without_begin_does_not_flush_unrelated_running_exploring_cell kind=ui owner=dialogs-pane
#[test]
fn exec_end_without_begin_does_not_flush_unrelated_running_exploring_cell() {
    // oracle: the orphan end renders its own entry; the running `cat /dev/null` stays the active exploring cell.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().script(include_str!("fixtures/chat_approvals_1/exec_end_without_begin_does_not_flush_unrelated_running_exploring_cell.jsonl")).open();
    // needs feed: task started, begin cat /dev/null (call-exploring), begin unified exec echo repro-marker (call-orphan), end it
    s.assert_contains("• Ran echo repro-marker");
    s.assert_contains("• Exploring");
    s.assert_contains("Read null");
    s.assert_absent("echo repro-marker\n• Exploring");
}

// ORACLE chatwidget/tests/exec_flow.rs:660 exec_end_without_begin_flushes_completed_unrelated_exploring_cell kind=ui owner=dialogs-pane
#[test]
fn exec_end_without_begin_flushes_completed_unrelated_exploring_cell() {
    // oracle: the completed `ls -la` flushes as Explored, then the orphan `echo after` entry follows it.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().script(include_str!("fixtures/chat_approvals_1/exec_end_without_begin_flushes_completed_unrelated_exploring_cell.jsonl")).open();
    // needs feed: begin+end ls -la (call-ls, exit 0), begin unified exec `echo after` (call-after), end it
    s.assert_in_order(&["• Explored", "List ls -la", "• Ran echo after"]);
}

// ORACLE chatwidget/tests/exec_flow.rs:699 overlapping_exploring_exec_end_is_not_misclassified_as_orphan kind=ui owner=dialogs-pane
#[test]
fn overlapping_exploring_exec_end_is_not_misclassified_as_orphan() {
    // oracle: the end of `ls -la` inside the grouped cell draws no history cell; both reads stay in the active cell.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!("fixtures/chat_approvals_1/overlapping_exploring_exec_end_is_not_misclassified_as_orphan_1.jsonl"))
        .open();
    s.assert_screen_row_then(
        "• Exploring",
        &["  └ List ls -la", "    Read foo.txt", "    + Show details"],
    );
    s.assert_absent("• Ran");
    s.feed(include_str!("fixtures/chat_approvals_1/overlapping_exploring_exec_end_is_not_misclassified_as_orphan_2.jsonl"));
    s.assert_screen_row_then(
        "• Explored",
        &["  └ List ls -la", "    Read foo.txt", "    + Show details"],
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:731 exec_history_shows_unified_exec_startup_commands kind=ui owner=dialogs-pane
#[test]
fn exec_history_shows_unified_exec_startup_commands() {
    // oracle: the begin draws nothing; the end flushes one cell with the startup command.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!(
            "fixtures/chat_approvals_1/exec_history_shows_unified_exec_startup_commands.jsonl"
        ))
        .open();
    // needs feed: task started, begin unified exec startup `echo unified exec startup` (call-startup), end it with exit 0
    s.assert_contains("• Ran echo unified exec startup");
}

// ORACLE chatwidget/tests/exec_flow.rs:781 unified_exec_unknown_end_with_active_exploring_cell_snapshot kind=snap owner=dialogs-pane snap=unified_exec_unknown_end_with_active_exploring_cell
#[test]
fn unified_exec_unknown_end_with_active_exploring_cell_snapshot() {
    // oracle: History (the orphan's entry) and Active (the exploring cell `cat /dev/null`) in one
    // snapshot: the scrollback form splits them, the owned transcript holds the rows in order.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_approvals_1/unified_exec_unknown_end_with_active_exploring_cell_snapshot.jsonl"))
        .open();
    s.assert_in_order(&["• Ran echo repro-marker", "└ repro-marker"]);
    s.assert_in_order(&["• Exploring", "└ Read null"]);
}

// ORACLE chatwidget/tests/exec_flow.rs:852 unified_exec_wait_after_final_agent_message_snapshot kind=snap owner=dialogs-pane snap=unified_exec_wait_after_final_agent_message
#[test]
fn unified_exec_wait_after_final_agent_message_snapshot() {
    // oracle: a unified exec wait, then the final message and the turn completion: the wait is
    // written before the message (the detailed transcript; the compact one does not list it).
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/unified_exec_wait_after_final_agent_message_snapshot.jsonl"
        ))
        .open();
    s.assert_absent("Waited for background terminal");
    s.keys("ctrl+t");
    s.assert_in_order(&[
        "• Waited for background terminal · cargo test -p codex-core",
        "• Final response.",
        "Worked for",
    ]);
}

// ORACLE chatwidget/tests/exec_flow.rs:871 unified_exec_wait_before_streamed_agent_message_snapshot kind=snap owner=dialogs-pane snap=unified_exec_wait_before_streamed_agent_message
#[test]
fn unified_exec_wait_before_streamed_agent_message_snapshot() {
    // oracle: a unified exec wait, then a streamed message "Streaming response." and the turn
    // completion: the wait is written before the message.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_approvals_1/unified_exec_wait_before_streamed_agent_message_snapshot.jsonl"))
        .open();
    s.assert_absent("Waited for background terminal");
    s.keys("ctrl+t");
    s.assert_in_order(&[
        "• Waited for background terminal · cargo test -p codex-core",
        "• Streaming response.",
        "Worked for",
    ]);
}

// ORACLE chatwidget/tests/exec_flow.rs:895 final_worked_for_uses_cumulative_turn_duration_snapshot kind=snap owner=dialogs-pane snap=final_worked_for_uses_cumulative_turn_duration
#[test]
fn final_worked_for_uses_cumulative_turn_duration_snapshot() {
    // oracle: the final separator says "Worked for 2m 5s" from the cumulative turn time (the
    // server's duration, or the status timer's 125 s when it timed none); both duration cases
    // share one snapshot (`[duration] • [completion time]`: the clock part is the test's own).
    let notify = |method: &str, params: serde_json::Value| {
        serde_json::json!({"step": "notify", "message": {"method": method, "params": params}})
            .to_string()
    };
    let now = kit::unix_now();
    for duration in [serde_json::json!(125_000), serde_json::Value::Null] {
        let mut s = Screen::build(Owner::DialogsPane)
            .size(80, 24)
            .inline(&[])
            .live()
            .open();
        s.feed(&format!("{{\"step\":\"clock\",\"secs\":{now}}}"));
        s.feed(&notify(
            "turn/started",
            serde_json::json!({"threadId": "thread-1", "turn": {
                "id": "turn-1", "rootTurnId": null, "items": [], "itemsView": "full",
                "status": "inProgress", "error": null, "startedAt": now, "completedAt": null,
                "durationMs": null}}),
        ));
        let exec = |status: &str, output: serde_json::Value, exit: serde_json::Value| {
            serde_json::json!({
                "type": "commandExecution", "id": "call-1", "pluginId": null, "scriptPath": null,
                "command": "bash -lc 'echo preparing'", "cwd": "/tmp/project", "processId": null,
                "source": "agent", "status": status,
                "commandActions": [{"type": "unknown", "command": "echo preparing"}],
                "aggregatedOutput": output, "exitCode": exit, "durationMs": null,
                "startedAtMs": null, "completedAtMs": null
            })
        };
        s.feed(&notify(
            "item/started",
            serde_json::json!({"item": exec("inProgress", serde_json::Value::Null, serde_json::Value::Null),
                "threadId": "thread-1", "turnId": "turn-1", "startedAtMs": 0}),
        ));
        s.feed(&completed_item(&exec(
            "completed",
            "preparing\n".into(),
            0.into(),
        )));
        s.feed(&format!("{{\"step\":\"clock\",\"secs\":{}}}", now + 125));
        s.feed(&notify(
            "item/agentMessage/delta",
            serde_json::json!({"threadId": "thread-1", "turnId": "turn-1", "itemId": "msg-final",
                "delta": "Final response.\n"}),
        ));
        s.feed(&completed_item(&serde_json::json!({
            "type": "agentMessage", "moderation": null, "id": "msg-final",
            "text": "Final response.", "phase": "final_answer", "memoryCitation": null,
            "delivery": null, "questions": null, "startedAtMs": null, "completedAtMs": null
        })));
        s.feed(&notify(
            "turn/completed",
            serde_json::json!({"threadId": "thread-1", "turn": {
                "id": "turn-1", "rootTurnId": null, "items": [], "itemsView": "full",
                "status": "completed", "error": null, "startedAt": null, "completedAt": null,
                "durationMs": duration}}),
        ));
        s.wait(1200);
        s.assert_in_order(&[
            "• Ran echo preparing",
            "└ preparing",
            "• Final response.",
            "Worked for 2m 5s",
        ]);
    }
}

// ORACLE chatwidget/tests/exec_flow.rs:983 unified_exec_waiting_multiple_empty_snapshots kind=snap owner=dialogs-pane snap=unified_exec_waiting_multiple_empty_after
#[test]
fn unified_exec_waiting_multiple_empty_snapshots() {
    // oracle: two empty polls of `just fix` show the waiting status with the command as its detail;
    // the turn completion writes ONE wait entry (the compact rendering has only the separator).
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/unified_exec_waiting_multiple_empty_snapshots.jsonl"
        ))
        .open();
    s.assert_contains("Waiting for background terminal");
    s.assert_contains("just fix");
    s.feed(TURN_COMPLETED);
    s.assert_absent("Waiting for background terminal");
    s.assert_absent("Waited for background terminal");
    s.keys("ctrl+t");
    s.assert_in_order(&["• Waited for background terminal · just fix", "Worked for"]);
    s.assert_text_count("Waited for background terminal", 1);
}

// ORACLE chatwidget/tests/exec_flow.rs:1032 unified_exec_wait_status_renders_command_in_single_details_row_snapshot kind=snap owner=dialogs-pane snap=unified_exec_wait_status_renders_command_in_single_details_row
#[test]
fn unified_exec_wait_status_renders_command_in_single_details_row_snapshot() {
    // oracle: a long command (cargo test ... --exact some::very::long::test::name) stays in one
    // details row at 48 columns (the snapshot is the bottom pane: from the status row down).
    let s = Screen::build(Owner::DialogsPane)
        .size(48, 8)
        .script(include_str!("fixtures/chat_approvals_1/unified_exec_wait_status_renders_command_in_single_details_row_snapshot.jsonl"))
        .open();
    s.assert_snap_region(
        "unified_exec_wait_status_renders_command_in_single_details_row",
        status_row(&s),
        0,
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:1052 unified_exec_empty_then_non_empty_snapshot kind=snap owner=dialogs-pane snap=unified_exec_empty_then_non_empty_after
#[test]
fn unified_exec_empty_then_non_empty_snapshot() {
    // oracle: an empty poll then a poll with `ls`: the non-empty write settles the wait first, so
    // the transcript holds the wait, then the interaction.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/unified_exec_empty_then_non_empty_snapshot.jsonl"
        ))
        .open();
    s.keys("ctrl+t");
    s.assert_in_order(&[
        "• Waited for background terminal · just fix",
        "↳ Interacted with background terminal · just fix",
        "└ ls",
    ]);
}

// ORACLE chatwidget/tests/exec_flow.rs:1069 unified_exec_non_empty_then_empty_snapshots kind=snap owner=dialogs-pane snap=unified_exec_non_empty_then_empty_active snap=unified_exec_non_empty_then_empty_after
#[test]
fn unified_exec_non_empty_then_empty_snapshots() {
    // oracle: a non-empty poll (`pwd`) then an empty one: the status waits with the command `just
    // fix` as its detail; after the turn completes the transcript holds both.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/unified_exec_non_empty_then_empty_snapshots.jsonl"
        ))
        .open();
    s.assert_contains("Waiting for background terminal");
    s.assert_contains("just fix");
    s.keys("ctrl+t");
    s.assert_in_order(&["↳ Interacted with background terminal · just fix", "└ pwd"]);
    s.assert_absent("Waited for background terminal");
    s.keys("esc");
    s.feed(TURN_COMPLETED);
    s.keys("ctrl+t");
    s.assert_in_order(&[
        "↳ Interacted with background terminal · just fix",
        "└ pwd",
        "• Waited for background terminal · just fix",
        "Worked for",
    ]);
}

// ORACLE chatwidget/tests/exec_flow.rs:1112 view_image_tool_call_adds_history_cell kind=ui owner=dialogs-pane
#[test]
fn view_image_tool_call_adds_history_cell() {
    // oracle: one history cell, exactly "• Viewed image example.png".
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(RUNNING_TURN)
        .open();
    s.feed(&completed_item(
        &serde_json::json!({"type": "imageView", "id": "call-image", "path": "/work/example.png"}),
    ));
    s.assert_has_row("• Viewed image example.png");
}

// ORACLE chatwidget/tests/exec_flow.rs:1125 view_image_tool_call_preserves_foreign_path kind=ui owner=dialogs-pane
#[test]
fn view_image_tool_call_preserves_foreign_path() {
    // oracle: the path C:\workspace\assets\example.png shows as "example.png".
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(RUNNING_TURN)
        .open();
    s.feed(&completed_item(
        &serde_json::json!({"type": "imageView", "id": "call-image",
        "path": "C:\\workspace\\assets\\example.png"}),
    ));
    s.assert_has_row("• Viewed image example.png");
}

// ORACLE chatwidget/tests/exec_flow.rs:1152 image_generation_call_adds_history_cell kind=snap owner=dialogs-pane snap=failed_image_generation_call_history_snapshot snap=image_generation_call_history_snapshot
#[test]
fn image_generation_call_adds_history_cell() {
    // oracle: a completed generation cell (saved to file:///tmp/ig-1.png), then a failed one.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(RUNNING_TURN)
        .open();
    let generation = |id: &str, status: &str, saved: serde_json::Value| {
        completed_item(
            &serde_json::json!({"type": "imageGeneration", "id": id, "status": status,
            "revisedPrompt": "A tiny blue square", "failure": null, "savedPath": saved,
            "generationId": null, "result": ""}),
        )
    };
    s.feed(&generation(
        "call-image-generation",
        "completed",
        serde_json::json!("/tmp/ig-1.png"),
    ));
    let at = cell_at(&s, "• Generated Image:");
    s.assert_snap_region("image_generation_call_history_snapshot", at, 0);
    s.feed(&generation(
        "call-image-generation-failed",
        "failed",
        serde_json::Value::Null,
    ));
    let at = cell_at(&s, "✗ Image generation failed");
    s.assert_snap_region("failed_image_generation_call_history_snapshot", at, 0);
}

// ORACLE chatwidget/tests/exec_flow.rs:1189 exec_history_extends_previous_when_consecutive kind=snap owner=dialogs-pane snap=exploring_step1_start_ls snap=exploring_step2_finish_ls snap=exploring_step3_start_cat_foo snap=exploring_step4_finish_cat_foo snap=exploring_step6_finish_cat_bar
#[test]
fn exec_history_extends_previous_when_consecutive() {
    // oracle: each step's active cell is one snapshot; step 5 (sed -n on foo.txt) leaves step 4's cell unchanged.
    let part = |n: usize| match n {
        1 => include_str!(
            "fixtures/chat_approvals_1/exec_history_extends_previous_when_consecutive_1.jsonl"
        ),
        2 => include_str!(
            "fixtures/chat_approvals_1/exec_history_extends_previous_when_consecutive_2.jsonl"
        ),
        3 => include_str!(
            "fixtures/chat_approvals_1/exec_history_extends_previous_when_consecutive_3.jsonl"
        ),
        4 => include_str!(
            "fixtures/chat_approvals_1/exec_history_extends_previous_when_consecutive_4.jsonl"
        ),
        5 => include_str!(
            "fixtures/chat_approvals_1/exec_history_extends_previous_when_consecutive_5.jsonl"
        ),
        _ => include_str!(
            "fixtures/chat_approvals_1/exec_history_extends_previous_when_consecutive_6.jsonl"
        ),
    };
    let cell = |s: &Screen| {
        s.rows()
            .iter()
            .rposition(|row| row.starts_with("• Explor"))
            .unwrap_or_else(|| panic!("no exploring cell on the screen:\n{}", s.text()))
    };
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(part(1))
        .open();
    s.assert_snap_rows_at("exploring_step1_start_ls", cell(&s));
    s.feed(part(2));
    s.assert_snap_rows_at("exploring_step2_finish_ls", cell(&s));
    s.feed(part(3));
    s.assert_snap_rows_at("exploring_step3_start_cat_foo", cell(&s));
    s.feed(part(4));
    s.assert_snap_rows_at("exploring_step4_finish_cat_foo", cell(&s));
    s.feed(part(5));
    s.assert_snap_rows_at("exploring_step4_finish_cat_foo", cell(&s));
    s.feed(part(6));
    s.assert_snap_rows_at("exploring_step6_finish_cat_bar", cell(&s));
}

// ORACLE chatwidget/tests/exec_flow.rs:1239 user_shell_command_renders_output_not_exploring kind=snap owner=dialogs-pane snap=user_shell_ls_output
#[test]
fn user_shell_command_renders_output_not_exploring() {
    // oracle: one history cell for the user's `ls` with file1 and file2 as its output.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .live()
        .script(include_str!(
            "fixtures/chat_approvals_1/user_shell_command_renders_output_not_exploring.jsonl"
        ))
        .open();
    let row = s
        .rows()
        .iter()
        .rposition(|row| row.starts_with("• You ran"))
        .unwrap_or_else(|| panic!("no user-shell cell on the screen:\n{}", s.text()));
    s.assert_snap_rows_at("user_shell_ls_output", row);
}

// ORACLE chatwidget/tests/exec_flow.rs:1267 bang_shell_enter_while_task_running_submits_run_user_shell_command kind=ui owner=dialogs-pane
#[test]
fn bang_shell_enter_while_task_running_submits_run_user_shell_command() {
    kit::na(
        Owner::DialogsPane,
        "asserts the RunUserShellCommand op sent, the FollowTranscript and history-append events and the empty queue; none is drawn",
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:1316 user_message_during_user_shell_command_is_queued_not_steered kind=ui owner=dialogs-pane
#[test]
fn user_message_during_user_shell_command_is_queued_not_steered() {
    // oracle: a message sent while `sleep 10` (user shell) runs is queued, not steered; it is sent after the turn completes.
    // The visible half: the queued text shows as a row; the sent op and the queue field are not drawn and not asserted here.
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started, begin unified user shell `sleep 10` (user-shell-sleep)
    s.type_text("hi");
    s.keys("enter");
    s.assert_contains("hi");
    // needs feed: end `sleep 10`, final answer "done", turn completed; then the queued "hi" goes out as a user turn
}

// ORACLE chatwidget/tests/exec_flow.rs:1358 disabled_slash_command_while_task_running_snapshot kind=snap owner=dialogs-pane snap=disabled_slash_command_while_task_running_snapshot
#[test]
fn disabled_slash_command_while_task_running_snapshot() {
    // oracle: the last history cell is the error line for /resume while the turn is active.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!("fixtures/chat_commands_2/running.jsonl"))
        .open();
    s.type_text("/resume");
    s.keys("enter");
    // the snapshot is that one cell: its row is where the transcript draws it
    let row = s
        .rows()
        .iter()
        .position(|row| row.contains("is disabled while a task is in progress"))
        .unwrap_or(usize::MAX);
    s.assert_snap_region("disabled_slash_command_while_task_running_snapshot", row, 0);
}

// ORACLE chatwidget/tests/exec_flow.rs:1382 approval_modal_exec_snapshot kind=snap owner=dialogs-pane snap=approval_modal_exec
#[test]
fn approval_modal_exec_snapshot() {
    // oracle: the exec request (bash -lc 'echo hello world', reason, execpolicy echo hello world) at width 100.
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 14)
        .script(include_str!("fixtures/chat_approvals_1/exec_command.jsonl"))
        .open();
    s.assert_contains("echo hello world");
    s.assert_snap("approval_modal_exec");
}

// ORACLE chatwidget/tests/exec_flow.rs:1443 approval_modal_exec_without_reason_snapshot kind=snap owner=dialogs-pane snap=approval_modal_exec_no_reason
#[test]
fn approval_modal_exec_without_reason_snapshot() {
    // oracle: the same command with no reason: the spacing of the modal without a reason line, width 100.
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 12)
        .script(include_str!(
            "fixtures/chat_approvals_1/exec_no_reason.jsonl"
        ))
        .open();
    s.assert_snap("approval_modal_exec_no_reason");
}

// ORACLE chatwidget/tests/exec_flow.rs:1487 approval_modal_exec_multiline_prefix_hides_execpolicy_option_snapshot kind=snap owner=dialogs-pane snap=approval_modal_exec_multiline_prefix_no_execpolicy
#[test]
fn approval_modal_exec_multiline_prefix_hides_execpolicy_option_snapshot() {
    // oracle: a multi-line prefix offers no "don't ask again" option; the modal is the snapshot, width 100.
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 13)
        .script(include_str!(
            "fixtures/chat_approvals_1/exec_multiline_prefix.jsonl"
        ))
        .open();
    s.assert_absent("don't ask again");
    s.assert_snap("approval_modal_exec_multiline_prefix_no_execpolicy");
}

// ORACLE chatwidget/tests/exec_flow.rs:1533 approval_modal_patch_snapshot kind=snap owner=dialogs-pane snap=approval_modal_patch
#[test]
fn approval_modal_patch_snapshot() {
    // oracle: the patch modal (README.md added, reason, destination /tmp/project/README.md) at the
    // chat's desired height (13 rows) at width 80; no "$ apply_patch" line.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 13)
        .script(include_str!("fixtures/chat_approvals_1/patch_reason.jsonl"))
        .open();
    s.assert_absent("$ apply_patch");
    s.assert_snap("approval_modal_patch");
}

// ORACLE chatwidget/tests/exec_flow.rs:1573 interrupt_preserves_unified_exec_processes kind=ui owner=dialogs-pane
#[test]
fn interrupt_preserves_unified_exec_processes() {
    // oracle: after the turn is interrupted, /ps still lists both running processes.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/interrupt_preserves_unified_exec_processes.jsonl"
        ))
        .open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_contains("Background terminals");
    s.assert_contains("sleep 5");
    s.assert_contains("sleep 6");
}

// ORACLE chatwidget/tests/exec_flow.rs:1604 interrupt_preserves_unified_exec_wait_streak_snapshot kind=snap owner=dialogs-pane snap=interrupt_preserves_unified_exec_wait_streak
#[test]
fn interrupt_preserves_unified_exec_wait_streak_snapshot() {
    // oracle: an interrupted turn with a waiting streak, then the exec end: one cell, the
    // interrupted notice; the streak and the late end draw nothing.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/interrupt_preserves_unified_exec_wait_streak_snapshot.jsonl"
        ))
        .open();
    s.assert_text_count(
        "■ Conversation interrupted - use /feedback if something went wrong",
        1,
    );
    s.keys("ctrl+t");
    s.assert_contains("■ Conversation interrupted - use /feedback if something went wrong");
    s.assert_absent("Waited for background terminal");
    s.assert_absent("just fix");
}

// ORACLE chatwidget/tests/exec_flow.rs:1626 turn_complete_keeps_unified_exec_processes kind=ui owner=dialogs-pane
#[test]
fn turn_complete_keeps_unified_exec_processes() {
    // oracle: after the turn completes, /ps still lists both running processes.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/turn_complete_keeps_unified_exec_processes.jsonl"
        ))
        .open();
    s.type_text("/ps");
    s.keys("enter");
    s.assert_contains("Background terminals");
    s.assert_contains("sleep 5");
    s.assert_contains("sleep 6");
}

// ORACLE chatwidget/tests/exec_flow.rs:1657 apply_patch_events_emit_history_cells kind=snap owner=dialogs-pane
#[test]
fn apply_patch_events_emit_history_cells() {
    // oracle: the approval adds no history cell; the apply begin draws the block "Added foo.txt (+16 -0)" with lines 1..16;
    // the apply end adds no cell.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 30)
        .script(include_str!(
            "fixtures/chat_approvals_1/apply_patch_16.jsonl"
        ))
        .open();
    s.keys("y");
    // The oracle draws the whole block (`create_diff_summary`): the detailed transcript.
    s.keys("ctrl+t");
    s.assert_in_order(&["• Added foo.txt (+16 -0)", "1 +line 1", "16 +line 16"]);
}

// ORACLE chatwidget/tests/exec_flow.rs:1736 apply_patch_manual_approval_adjusts_header kind=ui owner=dialogs-pane
#[test]
fn apply_patch_manual_approval_adjusts_header() {
    // oracle: the apply block of the approved manual patch has the header "Added foo.txt" (or "Edited foo.txt").
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/apply_patch_manual.jsonl"
        ))
        .open();
    s.keys("y");
    s.assert_contains("Added foo.txt");
}

// ORACLE chatwidget/tests/exec_flow.rs:1778 apply_patch_manual_flow_snapshot kind=snap owner=dialogs-pane snap=apply_patch_manual_flow_history_approved
#[test]
fn apply_patch_manual_flow_snapshot() {
    // oracle: the approval modal emits no history; the apply begin emits the approved cell.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/apply_patch_manual.jsonl"
        ))
        .open();
    s.keys("y");
    let at = s
        .rows()
        .iter()
        .position(|row| row.starts_with("• Added foo.txt"))
        .expect("the approved patch cell");
    s.assert_snap_region("apply_patch_manual_flow_history_approved", at, 0);
}

// ORACLE chatwidget/tests/exec_flow.rs:1824 apply_patch_approval_sends_op_with_call_id kind=ui owner=dialogs-pane
#[test]
fn apply_patch_approval_sends_op_with_call_id() {
    kit::na(
        Owner::DialogsPane,
        "asserts the PatchApproval op sent with the call id (call-999) and decision Accept after y; the op is not drawn",
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:1867 apply_patch_full_flow_integration_like kind=ui owner=dialogs-pane
#[test]
fn apply_patch_full_flow_integration_like() {
    kit::na(
        Owner::DialogsPane,
        "asserts the PatchApproval op forwarded to the codex channel (call-1, Accept); the forwarded op is not drawn, the patch events assert nothing",
    );
}

// ORACLE chatwidget/tests/exec_flow.rs:1937 apply_patch_untrusted_shows_approval_modal kind=ui owner=dialogs-pane
#[test]
fn apply_patch_untrusted_shows_approval_modal() {
    // oracle: the modal shows its title "Would you like to make the following edits?" at width 80, height 12.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 12)
        .script(include_str!(
            "fixtures/chat_approvals_1/patch_omits_diff.jsonl"
        ))
        .open();
    s.assert_contains("Would you like to make the following edits?");
}

// ORACLE chatwidget/tests/exec_flow.rs:1988 apply_patch_request_omits_diff_summary_from_modal kind=ui owner=dialogs-pane
#[test]
fn apply_patch_request_omits_diff_summary_from_modal() {
    // oracle: the request's modal shows no "README.md (+2 -0)" summary and no diff lines. The modal
    // is the sheet the request opens on the chat (the first assert), so the absence has bite.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 24)
        .script(include_str!(
            "fixtures/chat_approvals_1/patch_omits_diff.jsonl"
        ))
        .open();
    s.assert_contains("Would you like to make the following edits?");
    s.assert_absent("README.md (+2 -0)");
    s.assert_absent("+line one");
    s.assert_absent("+line two");
}
