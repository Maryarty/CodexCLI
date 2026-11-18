"""Unified exec: a process kept alive (cat on a tty) and fed through write_stdin."""
import json

import harness
from events import exec_command_call
from scenario import Scenario, Turn, text_response, tool_response

# core's tests see process id 1000 (unified_exec.rs:3128); a release build picks a random one,
# which the server reads from the first output and puts where the placeholder is.
STDIN = {"chars": "hello unified exec\n", "session_id": 0, "yield_time_ms": 500}
SCENARIO = Scenario(
    name="unified-exec",
    description="exec_command /bin/cat (tty, yields at 200 ms, session stays), write_stdin sends a line to it, then an answer",
    turns=[
        Turn(
            "Start cat and talk to it.",
            [
                tool_response("ux1", exec_command_call("call-ux-1", "/bin/cat", yield_time_ms=200, tty=True), lead_text="Starting cat."),
                tool_response("ux2", harness.ev_function_call("call-ux-2", "write_stdin", json.dumps(STDIN).replace('"session_id": 0', '"session_id": @@SESSION_ID@@')), lead_text="Sending a line."),
                text_response("ux3", ["The process echoed it back. ", "END-unified-exec"], pace=0.01),
            ],
        )
    ],
    sentinel="END-unified-exec",
    helpers="ev_exec_command_call_with_args, ev_function_call (name write_stdin)",
    suite="unified_exec.rs:3100 unified_exec_reuses_session_via_stdin",
)
