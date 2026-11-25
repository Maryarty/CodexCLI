"""Syncs map-full/pending-tests.tsv with the tests of task f-approval-decisions.

For each test below: a row is dropped when the test no longer calls `kit::pending`, and its reason
column is the test's current `kit::pending` string otherwise. Rerunnable; then run map-full/build.py
and map-full/check.py.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
TUI = ROOT / "codex-rs" / "tui-rtwk"
TSV = ROOT / "tools" / "rtwk-port" / "map-full" / "pending-tests.tsv"
TESTS = """approvals_1::exec_history_cell_wraps_with_two_space_indent approvals_1::network_exec_prompt_title_includes_host
approvals_2::render_multiple_threads_snapshot approvals_2::render_single_thread_snapshot
cells_4::prefixed_wrapped_history_cell_does_not_split_url_like_token
cells_4::prefixed_wrapped_history_cell_height_matches_wrapped_rendering
cells_4::prefixed_wrapped_history_cell_indents_wrapped_lines
chat_approvals_1::app_server_write_stdin_approval_renders_terminal_input
chat_approvals_1::apply_patch_events_emit_history_cells chat_approvals_1::apply_patch_manual_approval_adjusts_header
chat_approvals_1::apply_patch_manual_flow_snapshot
chat_approvals_1::exec_approval_emits_proposed_command_and_decision_history
chat_approvals_1::network_exec_approval_history_describes_canceled_host_request
chat_status_1::status_widget_and_approval_modal_snapshot
pane_footer_2::approval_request_is_delayed_after_recent_typing
pane_footer_2::composer_shown_after_denied_while_task_running
pane_footer_2::delayed_approval_shortcut_works_after_idle_deadline
pane_footer_2::overlay_not_shown_above_approval_modal""".split()


def pending_reason(shard: str, name: str):
    text = (TUI / "tests" / "oracle" / f"{shard}.rs").read_text()
    start = text.index(f"fn {name}()")
    end = text.find("\n}\n", start)
    body = text[start:end]
    m = re.search(r'kit::pending\(\s*Owner::\w+,\s*"((?:[^"\\]|\\.)*)"', body)
    if not m:
        return None
    return re.sub(r"\\(.)", r"\1", m.group(1))


def main() -> None:
    wanted = {}
    for test in TESTS:
        shard, name = test.split("::")
        wanted[test] = pending_reason(shard, name)
    out = []
    for line in TSV.read_text().splitlines():
        cols = line.split("\t")
        if cols[0] in wanted:
            reason = wanted.pop(cols[0])
            if reason is None:
                continue
            cols[2] = reason
            line = "\t".join(cols)
        out.append(line)
    TSV.write_text("\n".join(out) + "\n")
    for test, reason in wanted.items():
        if reason is not None:
            print("pending but no row:", test)


main()
