"""The stream closes before response.completed: the client retries, the retry answers."""
import harness
from scenario import Resp, Scenario, Turn, text_response

CUT = Resp(
    [
        harness.ev_response_created("sc1"),
        harness.ev_message_item_added("msg-sc1"),
        harness.ev_output_text_delta("This answer ends without a completed event. "),
    ],
    0.05,
)
SCENARIO = Scenario(
    name="error-stream-closed",
    description="SSE ends with no response.completed (stream closed before completion); stream_max_retries=2; the retry answers",
    turns=[Turn("Answer after the stream closes early.", [CUT, text_response("sc2", ["Retry worked. ", "END-error-stream-closed"], pace=0.02)])],
    sentinel="END-error-stream-closed",
    stream_max_retries=2,
    helpers="sse (events without ev_completed)",
    suite="stream_no_completed.rs:27 retries_on_early_close",
)
