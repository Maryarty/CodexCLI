"""An image_generation_call item (done, with base64 image data), then an answer."""
import base64

import harness
from events import ev_image_generation_call, png_bytes
from scenario import Resp, Scenario, Turn, chunks

PNG = base64.b64encode(png_bytes(64, 64, (30, 90, 220))).decode()
ANSWER = "Here is the picture. END-image-generation"
events = [
    harness.ev_response_created("ig1"),
    ev_image_generation_call("ig_123", "completed", "a blue square, flat colour", PNG),
    harness.ev_message_item_added("msg-ig1"),
    *[harness.ev_output_text_delta(d) for d in chunks(ANSWER, 8)],
    harness.ev_assistant_message("msg-ig1", ANSWER),
    harness.ev_completed("ig1"),
]
SCENARIO = Scenario(
    name="image-generation",
    description="image_generation_call completed (revised prompt + a 64x64 PNG), then a streamed answer",
    turns=[Turn("Draw a blue square.", [Resp(events, 0.05)])],
    sentinel="END-image-generation",
    helpers="ev_image_generation_call (responses.rs:998)",
    suite="model_switching.rs:1075 (the item is only replayed there; no suite test renders it)",
)
