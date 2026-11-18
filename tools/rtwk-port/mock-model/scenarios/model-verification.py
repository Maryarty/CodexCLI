"""response.metadata with openai_verification_recommendation, then an answer."""
import harness
from events import ev_model_verification_metadata
from scenario import Resp, Scenario, Turn, chunks

ANSWER = "The metadata came before this answer. END-model-verification"
events = [
    harness.ev_response_created("mv1"),
    ev_model_verification_metadata("mv1", ["trusted_access_for_cyber"]),
    harness.ev_message_item_added("msg-mv1"),
    *[harness.ev_output_text_delta(d) for d in chunks(ANSWER, 8)],
    harness.ev_assistant_message("msg-mv1", ANSWER),
    harness.ev_completed("mv1"),
]
SCENARIO = Scenario(
    name="model-verification",
    description="response.metadata carrying openai_verification_recommendation = trusted_access_for_cyber, then a streamed answer",
    turns=[Turn("Say something.", [Resp(events, 0.05)])],
    sentinel="END-model-verification",
    helpers="ev_model_verification_metadata (responses.rs:838)",
    suite="safety_check_downgrade.rs:332 (model verification metadata on a normal turn), :405 (with a tool call)",
)
