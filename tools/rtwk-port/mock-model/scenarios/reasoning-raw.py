"""Raw reasoning text (response.reasoning_text.delta) next to the summary; shown with show_raw_agent_reasoning."""
import harness
from events import (
    ev_reasoning_item,
    ev_reasoning_item_added,
    ev_reasoning_summary_text_delta,
    ev_reasoning_text_delta,
)
from scenario import Resp, Scenario, Turn, chunks

SUMMARY = "**Checking the raw reasoning**"
RAW = "First I note the question. Then I weigh the two readings and pick the simpler one."
ANSWER = "Done thinking. END-reasoning-raw"
events = [
    harness.ev_response_created("rr"),
    ev_reasoning_item_added("rr-think", [""]),
    *[ev_reasoning_summary_text_delta(d) for d in chunks(SUMMARY, 10)],
    *[ev_reasoning_text_delta(d) for d in chunks(RAW, 12)],
    ev_reasoning_item("rr-think", [SUMMARY], [RAW]),
    harness.ev_message_item_added("msg-rr"),
    *[harness.ev_output_text_delta(d) for d in chunks(ANSWER, 6)],
    harness.ev_assistant_message("msg-rr", ANSWER),
    harness.ev_completed("rr"),
]
SCENARIO = Scenario(
    name="reasoning-raw",
    description="reasoning summary deltas plus raw reasoning_text deltas and the item with raw content, then the answer",
    turns=[Turn("Think out loud.", [Resp(events, 0.02)])],
    sentinel="END-reasoning-raw",
    config_top="show_raw_agent_reasoning = true\n",
    helpers="ev_reasoning_text_delta (responses.rs:967), ev_reasoning_item with raw_content (:911), ev_reasoning_item_added, ev_reasoning_summary_text_delta",
    suite="items.rs:1172-1300 (summary and raw reasoning text deltas, show_raw_agent_reasoning = true)",
)
