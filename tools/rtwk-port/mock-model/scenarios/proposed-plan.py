"""A <proposed_plan> block in the answer (rendered as a plan in Plan mode)."""
import harness
from scenario import Resp, Scenario, Turn, chunks

PLAN = "Here is the plan.\n<proposed_plan>\n# Plan\n\n1. Read the code\n2. Change it\n3. Test it\n</proposed_plan>\nEND-proposed-plan"
events = [
    harness.ev_response_created("pp1"),
    harness.ev_message_item_added("msg-pp1"),
    *[harness.ev_output_text_delta(d) for d in chunks(PLAN, 8)],
    harness.ev_assistant_message("msg-pp1", PLAN),
    harness.ev_completed("pp1"),
]
SCENARIO = Scenario(
    name="proposed-plan",
    description="a streamed answer with a <proposed_plan> block (a plan cell in Plan mode, plain text in Default mode)",
    turns=[Turn("Propose a plan.", [Resp(events, 0.05)])],
    sentinel="END-proposed-plan",
    helpers="ev_message_item_added, ev_output_text_delta, ev_assistant_message",
    suite="items.rs:557 (a <proposed_plan> streamed in Plan mode)",
    notes="Switch to Plan mode (Shift+Tab or /plan) to see the plan cell.",
)
