"""An exec_command that needs approval (on-request + read-only + require_escalated)."""
from events import exec_command_call
from scenario import Scenario, Turn, text_response, tool_response

CALL = exec_command_call(
    "call-exec-1",
    "touch approved-by-mock.txt && echo created",
    yield_time_ms=1000,
    escalate_justification="Create the marker file you asked for",
)
SCENARIO = Scenario(
    name="exec-approval",
    description="one exec_command with sandbox_permissions=require_escalated; approval policy on-request",
    turns=[
        Turn(
            "Create a marker file and confirm it exists.",
            [
                tool_response("ea1", CALL, lead_text="I will create the marker file."),
                text_response("ea2", ["The command was handled. ", "END-exec-approval"], pace=0.01),
            ],
        )
    ],
    sentinel="END-exec-approval",
    approval_policy="on-request",
    sandbox_mode="read-only",
    approvals=1,
    notes="Approve or deny: the second response is served either way.",
)
