"""Token usage the status line and /status show: cached input, reasoning tokens, a growing total."""
import harness
from scenario import Resp, Scenario, Turn


def reply(rid: str, text: str, **usage: int) -> Resp:
    return Resp(
        [
            harness.ev_response_created(rid),
            harness.ev_assistant_message(f"msg-{rid}", text),
            harness.ev_completed_with_usage(rid, **usage),
        ]
    )


SCENARIO = Scenario(
    name="token-usage",
    description="three turns whose response.completed carries 12k, 48k and 96k tokens (cached and reasoning parts included)",
    turns=[
        Turn("one", [reply("tu1", "First answer.", input_tokens=11_000, cached_input_tokens=2_000, output_tokens=1_000, reasoning_output_tokens=300, total_tokens=12_000)]),
        Turn("two", [reply("tu2", "Second answer.", input_tokens=45_000, cached_input_tokens=30_000, output_tokens=3_000, reasoning_output_tokens=900, total_tokens=48_000)]),
        Turn("three", [reply("tu3", "Third answer. END-token-usage", input_tokens=90_000, cached_input_tokens=60_000, output_tokens=6_000, reasoning_output_tokens=2_000, total_tokens=96_000)]),
    ],
    sentinel="END-token-usage",
    helpers="ev_completed_with_tokens (responses.rs:849); the harness's ev_completed_with_usage for the cached/reasoning split",
    suite="token_usage_rollout.rs, client.rs:3750 token_count_includes_rate_limits_snapshot",
)
