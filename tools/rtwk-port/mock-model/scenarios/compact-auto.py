"""Auto-compaction: token usage over the limit makes the next turn compact first."""
import harness
from configs import COMPACT_TOP
from events import ev_completed_with_tokens
from scenario import Resp, Scenario, Turn


def reply(rid: str, text: str, tokens: int) -> Resp:
    return Resp(
        [
            harness.ev_response_created(rid),
            harness.ev_assistant_message(f"msg-{rid}", text),
            ev_completed_with_tokens(rid, tokens),
        ]
    )


SCENARIO = Scenario(
    name="compact-auto",
    description="usage 70k, then 330k (over the 200k limit), then the next prompt first compacts (summary request), then answers",
    turns=[
        Turn("first message", [reply("ca1", "FIRST_REPLY", 70_000)]),
        Turn("second message", [reply("ca2", "SECOND_REPLY", 330_000)]),
        Turn(
            "third message",
            [
                reply("ca3", "AUTO_SUMMARY: the conversation so far was two short exchanges.", 200),
                reply("ca4", "Compacted, then answered. END-compact-auto", 120),
            ],
        ),
    ],
    sentinel="END-compact-auto",
    config_top=COMPACT_TOP,
    helpers="ev_assistant_message, ev_completed_with_tokens (responses.rs:849)",
    suite="compact.rs:2543 auto_compact_runs_after_token_limit_hit",
)
