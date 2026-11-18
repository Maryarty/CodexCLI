"""A stream that fails with a retryable error, then the retry succeeds."""
import harness
from scenario import Resp, Scenario, Turn, text_response

FAIL = Resp(
    [
        harness.ev_response_created("er1"),
        harness.ev_message_item_added("msg-er1"),
        harness.ev_output_text_delta("This answer is cut off by "),
        harness.ev_failed("er1", "mock server error"),
    ],
    0.05,
)
SCENARIO = Scenario(
    name="error-retry",
    description="response.failed (server_error, retryable) once, the client reconnects (stream_max_retries=2), the retry answers",
    turns=[Turn("Answer after one failure.", [FAIL, text_response("er2", ["Second try worked. ", "END-error-retry"], pace=0.02)])],
    sentinel="END-error-retry",
    stream_max_retries=2,
)
