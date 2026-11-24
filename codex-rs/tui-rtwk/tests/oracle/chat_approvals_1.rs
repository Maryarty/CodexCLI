//! Shard `chat_approvals_1` (screens S13, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// ---- oracle: chatwidget/tests/approval_requests.rs (8 screen-level tests here; 2 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/approval_requests.rs:8 exec_approval_emits_proposed_command_and_decision_history kind=snap owner=dialogs-pane
#[test]
fn exec_approval_emits_proposed_command_and_decision_history() {
    kit::pending(
        Owner::DialogsPane,
        "the approval's decision cell is not drawn in the chat history after y: model `approvals::decided::Decided` and widget `DecisionCell` are done; NEED threads.rs `Item::Decision`, app.rs recording on resolve, transcript_pane.rs arm",
    );
    // oracle: the proposed exec request (echo hello world, reason) renders as the modal, no history cell;
    // `y` answers it and the history gets the decision cell.
    let mut s = Screen::build(Owner::DialogsPane)
        .size(80, 16)
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
    kit::pending(
        Owner::DialogsPane,
        "no approval entry for a writeStdin request (send terminal input): the feed is fixtures/chat_approvals_1/write_stdin.jsonl, the slice registers it",
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
        "no network-access approval on the chat screen and the history holds no decision cell yet (model `approvals::decided::Decided` and widget `DecisionCell` are done; NEED threads.rs `Item::Decision`, app.rs recording on resolve, transcript_pane.rs arm) (feed: fixtures/chat_approvals_1/network_cancel.jsonl)",
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
    kit::pending(
        Owner::DialogsPane,
        "no approval entry for a multi-line command with a reason, and the history holds no decision cell yet (NEED threads.rs `Item::Decision`, app.rs recording on resolve, transcript_pane.rs arm; feeds: exec_multiline_lines.jsonl, exec_long.jsonl)",
    );
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
    // needs feed: fixtures/chat_approvals_1/exec_long.jsonl (s.feed), then `n` again
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
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-live, not approvals): no external-writer thread view on the chat screen: the active exec row and the MCP row are not frozen on show",
    );
    // oracle: with a running command (sleep 5) or a running MCP call, showing the external writer thread
    // freezes the active row (no animation); the rendered rows are the snapshot.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: a running exec `sleep 5` (call-running), then the external writer thread shown; and the MCP variant
    s.assert_snap("external_writer_snapshot_freezes_active_command_and_mcp_rows");
}

// ORACLE chatwidget/tests/exec_flow.rs:51 replayed_background_terminals_belong_to_the_displayed_thread kind=snap owner=dialogs-pane
#[test]
fn replayed_background_terminals_belong_to_the_displayed_thread() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-chat-commands, not approvals): no /ps background-terminal list on the chat screen (the replayed inherited terminal and the fork's own terminals)",
    );
    // oracle: the fork's /ps lists only its own live terminal; the exited one is not listed.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: a replayed inherited terminal (sleep 100), restore of the fork's terminals (sleep 200 live, sleep 300 exited), then /ps
    s.assert_in_order(&["/ps", "Background terminals", "• sleep 200"]);
    s.assert_absent("sleep 300");
    s.assert_absent("sleep 100");
}

// ORACLE chatwidget/tests/exec_flow.rs:95 replayed_command_completion_preserves_tracking_without_duplicate_starts kind=ui owner=dialogs-pane
#[test]
fn replayed_command_completion_preserves_tracking_without_duplicate_starts() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no exec history cell for a replayed command completion on the chat screen (the completed cat replay is not drawn)",
    );
    // oracle: a replayed completion of a unified-exec startup writes one history cell and no duplicate start.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: a unified exec startup `cat replay` (process-replay), then its completed item as ThreadSnapshot
    s.assert_in_order(&["• Ran cat replay", "└ (no output)"]);
}

// ORACLE chatwidget/tests/exec_flow.rs:168 failed_exploration_keeps_overlapping_commands_active_until_all_finish kind=snap owner=dialogs-pane
#[test]
fn failed_exploration_keeps_overlapping_commands_active_until_all_finish() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no active exploring cell for overlapping exec commands on the chat screen (the failed ls and the running cat stay grouped)",
    );
    // oracle: a failed `ls missing` and a running `cat foo.txt` stay one active Explored cell until all finish.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin ls missing, begin cat foo.txt, end ls missing (exit 1), begin cat bar.txt, output for the running one
    s.assert_contains("missing");
    s.assert_contains("streamed output");
    // needs feed: end cat foo.txt, end cat bar.txt
    s.assert_in_order(&[
        "• Explored",
        "└ List missing (exit 1)",
        "Read foo.txt, bar.txt",
    ]);
    // needs feed: begin and end cat later.txt
    s.assert_in_order(&[
        "• Explored",
        "└ List missing (exit 1)",
        "Read foo.txt, bar.txt, later.txt",
    ]);
}

// ORACLE chatwidget/tests/exec_flow.rs:207 exploration_nonzero_exits_remain_visible_beside_successful_reads kind=snap owner=dialogs-pane
#[test]
fn exploration_nonzero_exits_remain_visible_beside_successful_reads() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no exploring cell with failed and successful reads on the chat screen (exit statuses, their colours)",
    );
    // oracle: six exploration commands, the non-zero exits stay visible beside the successful reads.
    // The compact rendering at width 24 (`compact_exploration_with_failures`) is a second screen, same feed.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin+end of cat first.txt (0), cat missing.txt (1), cat later.txt (0), rg absent . (1), rg text missing.txt (2), cat existing.txt && rg absent . (1)
    s.assert_snap("exploration_nonzero_exits_remain_visible_beside_successful_reads");
    // the first status is the red " (exit 1)" of cat missing.txt; the others are checked where they draw
    s.assert_text_style(" (exit 1)", "fg=Red");
}

// ORACLE chatwidget/tests/exec_flow.rs:258 exploration_reasoning_during_followup_command_stays_ordered kind=snap owner=dialogs-pane
#[test]
fn exploration_reasoning_during_followup_command_stays_ordered() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no reasoning rows interleaved with an exploring cell on the chat screen (reasoning between two exec commands)",
    );
    // oracle: `ls` then reasoning "Read the file next.", `cat file.txt` with reasoning "Waiting for the file.";
    // the active transcript keeps that order.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin+end ls (file.txt), reasoning delta+final, begin cat file.txt, reasoning delta+final, end cat file.txt
    s.assert_snap("exploration_reasoning_during_followup_command_stays_ordered");
}

// ORACLE chatwidget/tests/exec_flow.rs:274 adjacent_exploration_groups_across_reasoning_live_and_replayed kind=snap owner=dialogs-pane
#[test]
fn adjacent_exploration_groups_across_reasoning_live_and_replayed() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no exploring cell grouping across reasoning, live and replayed, on the chat screen (five exec commands, Compact/Expanded/Raw)",
    );
    // oracle: the live and the replayed screens render the same Compact, Expanded and Raw text.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: the five commands (ls missing 1, cat index.html, echo boundary, cat later.txt, cat final.txt) with reasoning between them, live and replayed
    s.assert_snap("adjacent_exploration_groups_across_reasoning_live_and_replayed");
}

// ORACLE chatwidget/tests/exec_flow.rs:378 replayed_commands_preserve_individual_output_and_failure_status kind=snap owner=dialogs-pane
#[test]
fn replayed_commands_preserve_individual_output_and_failure_status() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no replayed command cells with their own output and status marks on the chat screen (✓ and ✗ (7), ✗ (1))",
    );
    // oracle: four replayed command cells keep their own output; the failed and declined ones keep their status.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: replay_thread_turns of the four command items (first, second, failure exit 7, declined)
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
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-status, not approvals): no working status row restored after the preamble of a unified exec begin on the chat screen",
    );
    // oracle: a preamble line, a commit tick, then a unified exec begin: the working status shows again (80 wide).
    let s = Screen::build(Owner::DialogsPane).size(80, 8).live().open();
    // needs feed: task started, agent message delta "Preamble line", commit tick, begin unified exec `sleep 2` (call-1, proc-1)
    s.assert_snap("unified_exec_begin_restores_working_status");
}

// ORACLE chatwidget/tests/exec_flow.rs:517 exec_history_cell_shows_working_then_completed kind=ui owner=dialogs-pane
#[test]
fn exec_history_cell_shows_working_then_completed() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no finalized exec history cell drawn on the chat screen after the end of `echo done`",
    );
    // oracle: nothing is flushed on begin; the end flushes one cell with "• Ran" and the command text.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin `echo done` (call-1), then end it with exit 0
    s.assert_contains("• Ran");
    s.assert_contains("echo done");
}

// ORACLE chatwidget/tests/exec_flow.rs:547 exec_history_cell_shows_working_then_failed kind=ui owner=dialogs-pane
#[test]
fn exec_history_cell_shows_working_then_failed() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no finalized failed exec history cell drawn on the chat screen (`false` with exit 2 and error text)",
    );
    // oracle: the failed end flushes one cell with "• Ran false" and its error text.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin `false` (call-2), end it with stderr "Bloop" and exit 2
    s.assert_contains("• Ran false");
    s.assert_contains("Bloop");
}

// ORACLE chatwidget/tests/exec_flow.rs:571 exec_end_without_begin_uses_event_command kind=ui owner=dialogs-pane
#[test]
fn exec_end_without_begin_uses_event_command() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no history cell for an exec end without its begin on the chat screen (`• Ran echo orphaned`)",
    );
    // oracle: an end with no begin renders from the event's command; the call id is not shown.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: exec end of call-orphan with command `echo orphaned`, exit 0, no begin
    s.assert_contains("• Ran echo orphaned");
    s.assert_absent("call-orphan");
}

// ORACLE chatwidget/tests/exec_flow.rs:617 exec_end_without_begin_does_not_flush_unrelated_running_exploring_cell kind=ui owner=dialogs-pane
#[test]
fn exec_end_without_begin_does_not_flush_unrelated_running_exploring_cell() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no active exploring cell kept beside an orphan exec end on the chat screen (`• Ran echo repro-marker` above `• Exploring`)",
    );
    // oracle: the orphan end renders its own entry; the running `cat /dev/null` stays the active exploring cell.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: task started, begin cat /dev/null (call-exploring), begin unified exec echo repro-marker (call-orphan), end it
    s.assert_contains("• Ran echo repro-marker");
    s.assert_contains("• Exploring");
    s.assert_contains("Read null");
    s.assert_absent("echo repro-marker\n• Exploring");
}

// ORACLE chatwidget/tests/exec_flow.rs:660 exec_end_without_begin_flushes_completed_unrelated_exploring_cell kind=ui owner=dialogs-pane
#[test]
fn exec_end_without_begin_flushes_completed_unrelated_exploring_cell() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no flush of the completed exploring cell before the orphan entry on the chat screen",
    );
    // oracle: the completed `ls -la` flushes as Explored, then the orphan `echo after` entry follows it.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin+end ls -la (call-ls, exit 0), begin unified exec `echo after` (call-after), end it
    s.assert_in_order(&["• Explored", "List ls -la", "• Ran echo after"]);
}

// ORACLE chatwidget/tests/exec_flow.rs:699 overlapping_exploring_exec_end_is_not_misclassified_as_orphan kind=ui owner=dialogs-pane
#[test]
fn overlapping_exploring_exec_end_is_not_misclassified_as_orphan() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no grouped exploring cell with two running reads on the chat screen (List ls -la and Read foo.txt under one header)",
    );
    // oracle: the end of `ls -la` inside the grouped cell draws no history cell; both reads stay in the active cell.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin ls -la (call-ls), begin cat foo.txt (call-cat), end ls -la (exit 0)
    s.assert_contains("List ls -la");
    s.assert_contains("Read foo.txt");
    s.assert_contains("• Exploring");
    s.assert_absent("• Ran");
}

// ORACLE chatwidget/tests/exec_flow.rs:731 exec_history_shows_unified_exec_startup_commands kind=ui owner=dialogs-pane
#[test]
fn exec_history_shows_unified_exec_startup_commands() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no history cell for a unified exec startup command on the chat screen (`• Ran echo unified exec startup`)",
    );
    // oracle: the begin draws nothing; the end flushes one cell with the startup command.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: task started, begin unified exec startup `echo unified exec startup` (call-startup), end it with exit 0
    s.assert_contains("• Ran echo unified exec startup");
}

// ORACLE chatwidget/tests/exec_flow.rs:781 unified_exec_unknown_end_with_active_exploring_cell_snapshot kind=snap owner=dialogs-pane snap=unified_exec_unknown_end_with_active_exploring_cell
#[test]
fn unified_exec_unknown_end_with_active_exploring_cell_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no active exploring cell with an orphan unified exec end beside it on the chat screen (History and Active rows)",
    );
    // oracle: History (the orphan's entry) and Active (the exploring cell `cat /dev/null`) in one snapshot.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: task started, begin cat /dev/null (call-exploring), begin unified exec echo repro-marker (call-orphan), end it
    s.assert_snap("unified_exec_unknown_end_with_active_exploring_cell");
}

// ORACLE chatwidget/tests/exec_flow.rs:852 unified_exec_wait_after_final_agent_message_snapshot kind=snap owner=dialogs-pane snap=unified_exec_wait_after_final_agent_message
#[test]
fn unified_exec_wait_after_final_agent_message_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no waiting-for-terminal row in the transcript after the final agent message on the chat screen",
    );
    // oracle: a unified exec wait then the final message and the turn completion: the transcript snapshot.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started, begin `cargo test -p codex-core` (call-wait, proc-1), empty terminal interaction, final message, turn completed
    s.assert_snap("unified_exec_wait_after_final_agent_message");
}

// ORACLE chatwidget/tests/exec_flow.rs:871 unified_exec_wait_before_streamed_agent_message_snapshot kind=snap owner=dialogs-pane snap=unified_exec_wait_before_streamed_agent_message
#[test]
fn unified_exec_wait_before_streamed_agent_message_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no waiting-for-terminal row in the transcript before a streamed agent message on the chat screen",
    );
    // oracle: a unified exec wait, then a streamed message "Streaming response." and the turn completion.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started, begin `cargo test -p codex-core` (call-wait-stream), empty terminal interaction, streamed delta, turn completed
    s.assert_snap("unified_exec_wait_before_streamed_agent_message");
}

// ORACLE chatwidget/tests/exec_flow.rs:895 final_worked_for_uses_cumulative_turn_duration_snapshot kind=snap owner=dialogs-pane snap=final_worked_for_uses_cumulative_turn_duration
#[test]
fn final_worked_for_uses_cumulative_turn_duration_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-live, not approvals): no 'Worked for 2m 5s' final separator drawn on the chat screen (cumulative turn duration, with and without a duration)",
    );
    // oracle: the final separator says "Worked for 2m 5s" from the cumulative turn time; both duration cases share one snapshot.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started, exec `echo preparing` (call-1, exit 0), status timer 125 s, final answer "Final response.", turn completed with 125000 ms (and with none)
    s.assert_snap("final_worked_for_uses_cumulative_turn_duration");
}

// ORACLE chatwidget/tests/exec_flow.rs:983 unified_exec_waiting_multiple_empty_snapshots kind=snap owner=dialogs-pane snap=unified_exec_waiting_multiple_empty_after
#[test]
fn unified_exec_waiting_multiple_empty_snapshots() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-status, not approvals): no 'Waiting for background terminal' status with its command detail on the chat screen after two empty polls",
    );
    // oracle: two empty polls of `just fix` show the waiting status with the command as its detail; the turn completion
    // writes the compact and the transcript renderings.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: task started, begin unified exec `just fix` (call-wait-1, proc-1), two empty terminal interactions, turn completed
    s.assert_contains("Waiting for background terminal");
    s.assert_contains("just fix");
    s.assert_snap("unified_exec_waiting_multiple_empty_after");
}

// ORACLE chatwidget/tests/exec_flow.rs:1032 unified_exec_wait_status_renders_command_in_single_details_row_snapshot kind=snap owner=dialogs-pane snap=unified_exec_wait_status_renders_command_in_single_details_row
#[test]
fn unified_exec_wait_status_renders_command_in_single_details_row_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-status, not approvals): no bottom status popup with a long command in one details row on the chat screen (width 48)",
    );
    // oracle: a long command (cargo test ... --exact some::very::long::test::name) stays in one details row at 48 columns.
    let s = Screen::build(Owner::DialogsPane).size(48, 8).live().open();
    // needs feed: task started, begin unified exec with the long command (call-wait-ui, proc-ui), empty terminal interaction
    s.assert_snap("unified_exec_wait_status_renders_command_in_single_details_row");
}

// ORACLE chatwidget/tests/exec_flow.rs:1052 unified_exec_empty_then_non_empty_snapshot kind=snap owner=dialogs-pane snap=unified_exec_empty_then_non_empty_after
#[test]
fn unified_exec_empty_then_non_empty_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no transcript rendering of an empty then a non-empty terminal poll on the chat screen",
    );
    // oracle: an empty poll then a poll with `ls` output: the transcript snapshot.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: task started, begin unified exec `just fix` (call-wait-2, proc-2), empty poll, poll with `ls\n`
    s.assert_snap("unified_exec_empty_then_non_empty_after");
}

// ORACLE chatwidget/tests/exec_flow.rs:1069 unified_exec_non_empty_then_empty_snapshots kind=snap owner=dialogs-pane snap=unified_exec_non_empty_then_empty_active snap=unified_exec_non_empty_then_empty_after
#[test]
fn unified_exec_non_empty_then_empty_snapshots() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-status, not approvals): no 'Waiting for background terminal' status after a non-empty then an empty poll on the chat screen",
    );
    // oracle: a non-empty poll (`pwd`) then an empty one: the status waits with the command `just fix` as its detail;
    // after the turn completes the transcript holds both.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: task started, begin unified exec `just fix` (call-wait-3, proc-3), poll with `pwd\n`, empty poll
    s.assert_contains("Waiting for background terminal");
    s.assert_contains("just fix");
    s.assert_snap("unified_exec_non_empty_then_empty_active");
    // needs feed: turn completed
    s.assert_snap("unified_exec_non_empty_then_empty_after");
}

// ORACLE chatwidget/tests/exec_flow.rs:1112 view_image_tool_call_adds_history_cell kind=ui owner=dialogs-pane
#[test]
fn view_image_tool_call_adds_history_cell() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no 'Viewed image' history cell on the chat screen for a view-image tool call",
    );
    // oracle: one history cell, exactly "• Viewed image example.png".
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: view-image tool call call-image with the path <cwd>/example.png
    s.assert_has_row("• Viewed image example.png");
}

// ORACLE chatwidget/tests/exec_flow.rs:1125 view_image_tool_call_preserves_foreign_path kind=ui owner=dialogs-pane
#[test]
fn view_image_tool_call_preserves_foreign_path() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no 'Viewed image' history cell on the chat screen for a foreign (Windows) path",
    );
    // oracle: the path C:\workspace\assets\example.png shows as "example.png".
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: view-image tool call call-image with the path C:\workspace\assets\example.png
    s.assert_has_row("• Viewed image example.png");
}

// ORACLE chatwidget/tests/exec_flow.rs:1152 image_generation_call_adds_history_cell kind=snap owner=dialogs-pane snap=failed_image_generation_call_history_snapshot snap=image_generation_call_history_snapshot
#[test]
fn image_generation_call_adds_history_cell() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no image-generation history cell (completed and failed) on the chat screen",
    );
    // oracle: a completed generation cell (saved to file:///tmp/ig-1.png), then a failed one.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: image generation end call-image-generation (completed, saved /tmp/ig-1.png), then call-image-generation-failed (failed)
    s.assert_snap("image_generation_call_history_snapshot");
    s.assert_snap("failed_image_generation_call_history_snapshot");
}

// ORACLE chatwidget/tests/exec_flow.rs:1189 exec_history_extends_previous_when_consecutive kind=snap owner=dialogs-pane snap=exploring_step1_start_ls snap=exploring_step2_finish_ls snap=exploring_step3_start_cat_foo snap=exploring_step4_finish_cat_foo snap=exploring_step6_finish_cat_bar
#[test]
fn exec_history_extends_previous_when_consecutive() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no exploring cell that extends across consecutive exec commands on the chat screen (six steps)",
    );
    // oracle: each step's active cell is one snapshot; step 5 (sed -n on foo.txt) leaves step 4's cell unchanged.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin ls -la (call-ls)
    s.assert_snap("exploring_step1_start_ls");
    // needs feed: end ls -la (exit 0)
    s.assert_snap("exploring_step2_finish_ls");
    // needs feed: begin cat foo.txt (call-cat-foo)
    s.assert_snap("exploring_step3_start_cat_foo");
    // needs feed: end cat foo.txt with output hello from foo
    s.assert_snap("exploring_step4_finish_cat_foo");
    let explored_foo = s.shot();
    // needs feed: begin and end sed -n 100,200p foo.txt (call-sed-range)
    s.assert_same_rows(&explored_foo);
    // needs feed: begin and end cat bar.txt (call-cat-bar)
    s.assert_snap("exploring_step6_finish_cat_bar");
}

// ORACLE chatwidget/tests/exec_flow.rs:1239 user_shell_command_renders_output_not_exploring kind=snap owner=dialogs-pane snap=user_shell_ls_output
#[test]
fn user_shell_command_renders_output_not_exploring() {
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no user-shell command cell on the chat screen: `ls` run by the user draws its output, not an exploring row",
    );
    // oracle: one history cell for the user's `ls` with file1 and file2 as its output.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: begin and end `ls` with source UserShell (user-shell-ls), output file1 and file2, exit 0
    s.assert_snap("user_shell_ls_output");
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
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-chat-commands, not approvals): no error history cell for a slash command disabled while a task runs (/resume during a turn) on the chat screen",
    );
    // oracle: the last history cell is the error line for /resume while the turn is active.
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started (turn-1)
    s.type_text("/resume");
    s.keys("enter");
    s.assert_snap("disabled_slash_command_while_task_running_snapshot");
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
    kit::pending(
        Owner::DialogsPane,
        "no approvals entry for a multi-line proposed prefix (fixtures/chat_approvals_1/exec_multiline_prefix.jsonl, the slice registers it)",
    );
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
    kit::pending(
        Owner::DialogsPane,
        "no approvals entry for the README.md file-change request with its reason and grant root (the request carries no changes)",
    );
    // oracle: the patch modal (README.md added, reason, grant root /tmp/project) at width 80; no "$ apply_patch" line.
    let s = Screen::build(Owner::DialogsPane)
        .size(80, 13)
        .entry("approvals", "patch_edits")
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
    kit::pending(
        Owner::DialogsPane,
        "OTHER CHUNK (chunk-cells, not approvals): no waiting-streak history after an interrupt on the chat screen (cells count and the wait row)",
    );
    // oracle: an interrupted turn with a waiting streak, then the exec end: the cell count and the history.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: turn started, begin unified exec `just fix` (call-1, process-1), empty poll (call-1a), turn interrupted, end the exec
    s.assert_snap("interrupt_preserves_unified_exec_wait_streak");
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
    kit::pending(
        Owner::DialogsPane,
        "no apply block cell for foo.txt (+16 -0) on the chat screen; the approval adds no cell before apply",
    );
    // oracle: the approval adds no history cell; the apply begin draws the block "Added foo.txt (+16 -0)" with lines 1..16;
    // the apply end adds no cell.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: apply_patch approval call-c1 (foo.txt, 16 lines), patch apply begin call-c1, patch apply end call-c1 (completed)
    s.assert_in_order(&["• Added foo.txt (+16 -0)", "1 +line 1", "16 +line 16"]);
}

// ORACLE chatwidget/tests/exec_flow.rs:1736 apply_patch_manual_approval_adjusts_header kind=ui owner=dialogs-pane
#[test]
fn apply_patch_manual_approval_adjusts_header() {
    kit::pending(
        Owner::DialogsPane,
        "no apply block header 'Added foo.txt' on the chat screen after a manually approved patch",
    );
    // oracle: the apply block of the approved manual patch has the header "Added foo.txt" (or "Edited foo.txt").
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: apply_patch approval call-c1 (foo.txt add "hello"), patch apply begin call-c1
    s.assert_contains("Added foo.txt");
}

// ORACLE chatwidget/tests/exec_flow.rs:1778 apply_patch_manual_flow_snapshot kind=snap owner=dialogs-pane snap=apply_patch_manual_flow_history_approved
#[test]
fn apply_patch_manual_flow_snapshot() {
    kit::pending(
        Owner::DialogsPane,
        "no approved apply cell on the chat screen after the manual review approval (the approval modal defers its history)",
    );
    // oracle: the approval modal emits no history; the apply begin emits the approved cell.
    let s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    // needs feed: apply_patch approval call-c1 (foo.txt add, reason "Manual review required"), patch apply begin call-c1
    s.assert_snap("apply_patch_manual_flow_history_approved");
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
    kit::pending(
        Owner::DialogsPane,
        "no file-change approval modal on the chat screen for an untrusted patch (a.rs, no reason)",
    );
    // oracle: the modal shows its title "Would you like to make the following edits?" at width 80, height 12.
    let s = Screen::build(Owner::DialogsPane).size(80, 12).live().open();
    // needs feed: apply_patch approval call-1 (a.rs add, empty content) under OnRequest
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
