"""A long transcript: one turn of 40 tool rounds (a note, an echo command), then a long answer."""
import os

from events import exec_command_call
from scenario import Scenario, Turn, chunks, text_response, tool_response

ROUNDS = int(os.environ.get("MOCK_LONG_ROUNDS", "40"))
responses = [
    tool_response(
        f"lt{i}",
        exec_command_call(f"call-long-{i}", f"echo step {i}", yield_time_ms=500),
        lead_text=f"Step {i}: running the check number {i} of {ROUNDS}.",
    )
    for i in range(ROUNDS)
]
SUMMARY = "".join(f"- step {i} finished with exit code 0\n" for i in range(ROUNDS)) + "\nEND-long-transcript\n"
responses.append(text_response("lt-final", chunks("All steps done.\n\n" + SUMMARY, 24), pace=0.002))
SCENARIO = Scenario(
    name="long-transcript",
    description=f"one turn, {ROUNDS} rounds of (message + exec_command echo), then a long list as the answer (MOCK_LONG_ROUNDS changes the count)",
    turns=[Turn("Run all the checks.", responses)],
    sentinel="END-long-transcript",
    approval_policy="never",
    sandbox_mode="read-only",
    helpers='ev_assistant_message, ev_exec_command_call x40',
    suite='unified_exec.rs:1822 unified_exec_emits_one_begin_and_one_end_event (many commands); no suite twin for 40 rounds',
)
