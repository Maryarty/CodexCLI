"""A reasoning summary streamed, then the answer."""
import harness
from events import (
    ev_reasoning_item,
    ev_reasoning_item_added,
    ev_reasoning_summary_text_delta,
)
from scenario import Resp, Scenario, Turn, chunks

SUMMARY = "**Weighing the options**\n\nThe question has two readings. I compare them, then pick the simpler one."
ANSWER = "The simpler reading wins. END-reasoning"
events = [
    harness.ev_response_created("rs"),
    ev_reasoning_item_added("rs-think", [""]),
    *[ev_reasoning_summary_text_delta(d) for d in chunks(SUMMARY, 10)],
    ev_reasoning_item("rs-think", [SUMMARY], []),
    harness.ev_message_item_added("msg-rs"),
    *[harness.ev_output_text_delta(d) for d in chunks(ANSWER, 6)],
    harness.ev_assistant_message("msg-rs", ANSWER),
    harness.ev_completed("rs"),
]
SCENARIO = Scenario(
    name="reasoning",
    description="reasoning summary deltas, the reasoning item, then a streamed answer",
    turns=[Turn("Think about it, then answer.", [Resp(events, 0.02)])],
    sentinel="END-reasoning",
    helpers='ev_reasoning_item_added, ev_reasoning_summary_text_delta, ev_reasoning_item (responses.rs:911,943,959)',
    suite='compact.rs:1897 (a reasoning item), items.rs:1172 (summary deltas)',
)
