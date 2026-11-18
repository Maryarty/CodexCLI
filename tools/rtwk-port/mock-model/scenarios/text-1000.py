"""Plain text streamed at 1000 events/s: 6000 deltas, about 6 s. The perf stressor."""
from scenario import Scenario, Turn, text_response

WORDS = 6000
deltas = [f"w{i:04d} " for i in range(WORDS)] + ["END-text-1000"]
SCENARIO = Scenario(
    name="text-1000",
    description="6000 one-word text deltas at 1000 events/s (nominal; sleep(0.001) gives about 600-900/s)",
    turns=[Turn("Stream 1000 events per second.", [text_response("t1000", deltas, pace=0.001)])],
    sentinel="END-text-1000",
)
