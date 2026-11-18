"""Rate limits: x-codex-* response headers (HTTP transport) on a normal answer."""
import harness
from events import ev_completed_with_tokens, rate_limit_headers
from scenario import Resp, Scenario, Turn


def reply(rid: str, text: str, primary: float, secondary: float) -> Resp:
    return Resp(
        [
            harness.ev_response_created(rid),
            harness.ev_assistant_message(f"msg-{rid}", text),
            ev_completed_with_tokens(rid, 123),
        ],
        headers=rate_limit_headers(primary, secondary),
    )


SCENARIO = Scenario(
    name="rate-limits",
    description="two answers with x-codex-primary/secondary-used-percent headers (12.5/40.0, then 81.0/92.5)",
    turns=[
        Turn("one", [reply("rl1", "Answer with limits at 12.5% / 40%.", 12.5, 40.0)]),
        Turn("two", [reply("rl2", "Answer with limits at 81% / 92.5%. END-rate-limits", 81.0, 92.5)]),
    ],
    sentinel="END-rate-limits",
    helpers="ev_completed_with_tokens; headers as client.rs sets them (no ev_* builder: the codex.rate_limits event, request_user_input.rs:218, is read on WebSocket only)",
    suite="client.rs:3750 token_count_includes_rate_limits_snapshot",
)
