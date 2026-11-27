"""context_length_exceeded: a fatal error, the turn ends; the next prompt answers."""
import harness
from events import ev_failed_with_code
from scenario import Resp, Scenario, Turn, text_response

FAIL = Resp(
    [
        harness.ev_response_created("cw1"),
        ev_failed_with_code("cw1", "context_length_exceeded", "Your input exceeds the context window of this model. Please adjust your input and try again."),
    ]
)
SCENARIO = Scenario(
    name="error-context-window",
    description="response.failed context_length_exceeded: the first turn fails; the next prompt compacts the context and answers",
    turns=[
        Turn("Overflow the context.", [FAIL], outcome="failed"),
        Turn(
            "Try again.",
            [
                text_response("cw-sum", ["Summary: the user asked to overflow the context."]),
                text_response("cw2", ["Fine now. ", "END-error-context-window"], pace=0.02),
            ],
        ),
    ],
    sentinel="END-error-context-window",
    helpers="sse_failed (responses.rs:1120)",
    suite="codex-api/src/sse/responses.rs:1216 (context_length_exceeded as a fatal error), compact.rs:4782 manual_compact_retries_after_context_window_error",
)
