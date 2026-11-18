"""A long-running command left in the background after the turn ends (/ps lists it)."""
from events import exec_command_call
from scenario import Scenario, Turn, text_response, tool_response

SCENARIO = Scenario(
    name="background-terminal",
    description="exec_command `sleep 3000` yielding at 250 ms: the turn ends with a background terminal still running",
    turns=[
        Turn(
            "Start a long-running command and leave it.",
            [
                tool_response("bt1", exec_command_call("call-bt-1", "exec sleep 3000", yield_time_ms=250), lead_text="Starting it."),
                text_response("bt2", ["It keeps running in the background. ", "END-background-terminal"], pace=0.01),
            ],
        )
    ],
    sentinel="END-background-terminal",
    helpers="ev_exec_command_call_with_args",
    suite="unified_exec.rs:2910 unified_exec_keeps_long_running_session_after_turn_end",
    notes="The sleep ends with the Codex process (Codex terminates its sessions on exit).",
)
