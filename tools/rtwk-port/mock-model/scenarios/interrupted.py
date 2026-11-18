"""A turn the user interrupts (Esc) while the answer streams; the next prompt works."""
import harness
from scenario import Resp, Scenario, Turn, text_response

WORDS = [f"word{i:03d} " for i in range(150)]
slow = Resp(
    [
        harness.ev_response_created("int1"),
        harness.ev_message_item_added("msg-int1"),
        *[harness.ev_output_text_delta(w) for w in WORDS],
        harness.ev_assistant_message("msg-int1", "".join(WORDS)),
        harness.ev_completed("int1"),
    ],
    pace=0.2,
)
SCENARIO = Scenario(
    name="interrupted",
    description="a 30 s paced answer to interrupt with Esc, then a normal turn",
    turns=[
        Turn("Write something long.", [slow], esc_after=4.0),
        Turn("Now answer shortly.", [text_response("int2", ["Back to normal. ", "END-interrupted"], pace=0.01)]),
    ],
    sentinel="END-interrupted",
    pty_only=True,
    helpers="ev_output_text_delta (stream cut by the client)",
    suite="abort_tasks.rs (interrupt of a running turn), abort_lifecycle.rs",
    notes="Press Esc while the words stream; the second prompt then answers. check.py presses Esc 4 s after Enter.",
)
