"""/compact: a reasoning item and the summary answer the compaction request."""
import harness
from configs import COMPACT_PROMPT
from events import ev_reasoning_item
from scenario import Resp, Scenario, Turn, text_response

SUMMARY = "Summary of the conversation: the user said hello and the model greeted back."
compaction = Resp(
    [
        harness.ev_response_created("cm2"),
        ev_reasoning_item("compact-reasoning", ["Summarizing the conversation"], []),
        harness.ev_assistant_message("cm2-msg", SUMMARY),
        harness.ev_completed("cm2"),
    ]
)
SCENARIO = Scenario(
    name="compact-manual",
    description="a normal turn, then /compact (the model answers the compaction request with a reasoning item and a summary), then a turn after it",
    turns=[
        Turn("hello", [text_response("cm1", ["Hello! How can I help?"], pace=0.01)]),
        Turn("/compact", [compaction]),
        Turn("and now?", [text_response("cm3", ["After compaction. ", "END-compact-manual"], pace=0.01)]),
    ],
    sentinel="END-compact-manual",
    config_top=f'compact_prompt = "{COMPACT_PROMPT}"\n',
    pty_only=True,
    helpers="ev_reasoning_item (responses.rs:911), ev_assistant_message",
    suite="compact.rs:1894 manual_compact_emits_context_compaction_items",
    notes="The second prompt is the slash command /compact (typed, then Enter); it never reaches the model as text.",
)
