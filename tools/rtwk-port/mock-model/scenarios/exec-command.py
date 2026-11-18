"""exec_command that needs no approval (never): a command that prints, one that fails."""
from events import exec_command_call
from scenario import Scenario, Turn, text_response, tool_response

SCENARIO = Scenario(
    name="exec-command",
    description="two exec_command calls without approval (an ls-like listing, a failing command), then an answer",
    turns=[
        Turn(
            "List the files, then run a command that fails.",
            [
                tool_response("ec1", exec_command_call("call-ec-1", "printf 'one\\ntwo\\nthree\\n' && echo done"), lead_text="Listing."),
                tool_response("ec2", exec_command_call("call-ec-2", "echo about to fail >&2; exit 3"), lead_text="Now a failing command."),
                text_response("ec3", ["Both commands ran. ", "END-exec-command"], pace=0.01),
            ],
        )
    ],
    sentinel="END-exec-command",
    helpers="ev_exec_command_call (responses.rs:1103)",
    suite="unified_exec.rs:830 unified_exec_emits_exec_command_begin_event, :1014 ..._end_event, :1941 exec_command_reports_chunk_and_exit_metadata",
)
