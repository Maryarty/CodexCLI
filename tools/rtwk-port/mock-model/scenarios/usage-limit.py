"""HTTP 429 usage_limit_reached with rate-limit headers: the turn ends with the usage-limit message."""
import json

import harness
from events import rate_limit_headers
from scenario import Resp, Scenario, Turn, text_response

BODY = json.dumps(
    {"error": {"type": "usage_limit_reached", "message": "limit reached", "resets_at": 1704067242, "plan_type": "pro"}}
)
limit = Resp([], status=429, headers=rate_limit_headers(100.0, 87.5, 15, 60), raw=BODY)
SCENARIO = Scenario(
    name="usage-limit",
    description="429 usage_limit_reached (plan pro, primary 100%): the turn fails; the next prompt answers",
    turns=[
        Turn("Do something.", [limit], outcome="failed"),
        Turn("Try again.", [text_response("ul2", ["Back. ", "END-usage-limit"], pace=0.01)]),
    ],
    sentinel="END-usage-limit",
    helpers="no ev_* builder (a ResponseTemplate::new(429) with x-codex-* headers and a JSON error body)",
    suite="client.rs:3887 usage_limit_error_emits_rate_limit_event, quota_exceeded.rs",
)
