"""Plain text streamed at 100 events/s: 600 deltas, about 6 s. For perf and streaming."""
from scenario import Scenario, Turn, text_response

WORDS = 600
deltas = [f"w{i:04d} " for i in range(WORDS)] + ["END-text-100"]
SCENARIO = Scenario(
    name="text-100",
    description="600 one-word text deltas at 100 events/s (nominal, sleep-paced)",
    turns=[Turn("Stream 100 events per second.", [text_response("t100", deltas, pace=0.01)])],
    sentinel="END-text-100",
    helpers='ev_response_created, ev_message_item_added, ev_output_text_delta, ev_assistant_message, ev_completed',
    suite='client.rs (streamed message deltas); perf stream, no suite twin',
)
