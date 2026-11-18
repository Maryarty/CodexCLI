"""A fatal error (shown to the user), then a second prompt that works."""
import harness
from events import ev_failed_with_code
from scenario import Resp, Scenario, Turn, text_response

FAIL = Resp(
    [
        harness.ev_response_created("ef1"),
        ev_failed_with_code("ef1", "insufficient_quota", "You exceeded your current quota (mock)."),
    ]
)
SCENARIO = Scenario(
    name="error-fatal",
    description="insufficient_quota (not retried): the turn ends with an error; the second prompt answers",
    turns=[
        Turn("Fail with a fatal error.", [FAIL], outcome="failed"),
        Turn("Try again.", [text_response("ef2", ["It works now. ", "END-error-fatal"], pace=0.02)]),
    ],
    sentinel="END-error-fatal",
    notes="The first turn ends in an error line; check.py expects the turn to fail.",
)
