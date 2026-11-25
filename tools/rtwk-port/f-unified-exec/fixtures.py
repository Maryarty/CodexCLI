"""Writes the jsonl fixtures of the unified-exec oracle tests (exec_flow.rs, history_cell/tests.rs):
a turn, a unified-exec startup command, empty and non-empty polls of its process, agent messages,
the end of the turn.  Rerun: python3 -I tools/rtwk-port/f-unified-exec/fixtures.py (from the repo
root); the files under tests/oracle/fixtures/{chat_approvals_1,cells_3,cells_4} are its output."""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[3] / "codex-rs/tui-rtwk/tests/oracle/fixtures"
THREAD = "thread-1"
TURN = "turn-1"
URL_LIKE = (
    "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/"
    "reports/performance/summary/detail/with/a/very/long/path"
)


def note(method, params):
    return json.dumps({"step": "notify", "message": {"method": method, "params": params}})


def turn(status):
    return {
        "id": TURN, "rootTurnId": None, "items": [], "itemsView": "full", "status": status,
        "error": None, "startedAt": None, "completedAt": None, "durationMs": None,
    }


def started():
    return note("turn/started", {"threadId": THREAD, "turn": turn("inProgress")})


def completed(status="completed"):
    return note("turn/completed", {"threadId": THREAD, "turn": turn(status)})


def command(call, process, cmd, status="inProgress", actions=(), source="unifiedExecStartup"):
    done = status != "inProgress"
    return {
        "type": "commandExecution", "id": call, "pluginId": None, "scriptPath": None,
        "command": f"bash -lc '{cmd}'", "cwd": "/tmp/project", "processId": process,
        "source": source, "status": status, "commandActions": list(actions),
        "aggregatedOutput": None, "exitCode": 0 if done else None,
        "durationMs": 5 if done else None, "startedAtMs": 0, "completedAtMs": None,
    }


def begin(call, process, cmd, **kw):
    item = command(call, process, cmd, **kw)
    return note("item/started", {"item": item, "threadId": THREAD, "turnId": TURN, "startedAtMs": 0})


def end(call, process, cmd, status="completed", **kw):
    item = command(call, process, cmd, status=status, **kw)
    return note("item/completed", {"item": item, "threadId": THREAD, "turnId": TURN,
                                   "completedAtMs": 0})


def poll(call, process, stdin):
    return note("item/commandExecution/terminalInteraction", {
        "threadId": THREAD, "turnId": TURN, "itemId": call, "processId": process, "stdin": stdin})


def delta(item, text):
    return note("item/agentMessage/delta",
                {"threadId": THREAD, "turnId": TURN, "itemId": item, "delta": text})


def message(item, text, phase=None):
    return note("item/completed", {"item": {
        "type": "agentMessage", "moderation": None, "id": item, "text": text, "phase": phase,
        "memoryCitation": None, "delivery": None, "questions": None},
        "threadId": THREAD, "turnId": TURN, "completedAtMs": 0})


# The oracle snapshots draw a status line and no agents hint: agents
# navigation off, and the status line with the model and the directory.
FOOTER = [
    json.dumps({"step": "screen", "dir": "chat", "ev": {"facts": {"agents_navigation": False}}}),
    json.dumps({"step": "screen", "dir": "chat", "ev": {"status": {
        "model": "GPT-5.6-Sol", "cwd": "/tmp/project",
        "items": ["model-with-reasoning", "current-dir"]}}}),
]


def write(shard, name, origin, lines, footer=False):
    path = ROOT / shard / f"{name}.jsonl"
    seed = json.dumps({"step": "seed", "seed": "session", "thread": THREAD})
    head = [f"# oracle {origin} {name}", seed, *(FOOTER if footer else [])]
    path.write_text("\n".join([*head, *lines]) + "\n")


EXEC = "chatwidget/tests/exec_flow.rs"
CELLS = "history_cell/tests.rs"
CA = "chat_approvals_1"

stdin = "".join(f"line {n}\n" for n in range(1, 17))
write("cells_3", "unified_exec_interaction_cell_retains_detailed_input", f"{CELLS}:650", [
    started(), begin("call-1", "p1", "cat"), poll("call-1", "p1", stdin)])
write("cells_3", "unified_exec_interaction_cell_retains_detailed_wait", f"{CELLS}:676", [
    started(), begin("call-1", "p1", ""), poll("call-1", "p1", ""), completed()])
write("cells_4", "unified_exec_interaction_details_wrap_long_input", f"{CELLS}:1348", [
    started(), begin("call-1", "p1", "true"), poll("call-1", "p1", URL_LIKE)])

write(CA, "unified_exec_begin_restores_working_status_snapshot", f"{EXEC}:493", [
    started(), delta("msg-1", "Preamble line\n"), begin("call-1", "proc-1", "sleep 2")],
      footer=True)
write(CA, "unified_exec_wait_status_renders_command_in_single_details_row_snapshot",
      f"{EXEC}:1032", [
    started(),
    begin("call-wait-ui", "proc-ui",
          "cargo test -p codex-core -- --exact some::very::long::test::name"),
    poll("call-wait-ui-stdin", "proc-ui", "")], footer=True)
write(CA, "unified_exec_waiting_multiple_empty_snapshots", f"{EXEC}:983", [
    started(), begin("call-wait-1", "proc-1", "just fix"),
    poll("call-wait-1a", "proc-1", ""), poll("call-wait-1b", "proc-1", "")])
write(CA, "unified_exec_non_empty_then_empty_snapshots", f"{EXEC}:1069", [
    started(), begin("call-wait-3", "proc-3", "just fix"),
    poll("call-wait-3a", "proc-3", "pwd\n"), poll("call-wait-3b", "proc-3", "")])
write(CA, "unified_exec_empty_then_non_empty_snapshot", f"{EXEC}:1052", [
    started(), begin("call-wait-2", "proc-2", "just fix"),
    poll("call-wait-2a", "proc-2", ""), poll("call-wait-2b", "proc-2", "ls\n")])
write(CA, "unified_exec_wait_after_final_agent_message_snapshot", f"{EXEC}:852", [
    started(), begin("call-wait", "proc-1", "cargo test -p codex-core"),
    poll("call-wait-stdin", "proc-1", ""),
    message("msg-1", "Final response."), completed()])
write(CA, "unified_exec_wait_before_streamed_agent_message_snapshot", f"{EXEC}:871", [
    started(), begin("call-wait-stream", "proc-1", "cargo test -p codex-core"),
    poll("call-wait-stream-stdin", "proc-1", ""),
    delta("msg-1", "Streaming response."), completed()])
write(CA, "interrupt_preserves_unified_exec_wait_streak_snapshot", f"{EXEC}:1604", [
    started(), begin("call-1", "process-1", "just fix"), poll("call-1a", "process-1", ""),
    completed("interrupted"), end("call-1", "process-1", "just fix")])
