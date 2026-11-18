"""view_image on a PNG of the workdir."""
import json

import harness
from events import png_bytes
from scenario import Scenario, Turn, text_response, tool_response

SCENARIO = Scenario(
    name="view-image",
    description="view_image of assets/example.png (a 320x120 red PNG, seeded), then an answer",
    turns=[
        Turn(
            "Look at assets/example.png.",
            [
                tool_response(
                    "vi1",
                    harness.ev_function_call("call-vi-1", "view_image", json.dumps({"path": "assets/example.png"})),
                    lead_text="Opening the image.",
                ),
                text_response("vi2", ["It is a red rectangle. ", "END-view-image"], pace=0.01),
            ],
        )
    ],
    sentinel="END-view-image",
    seed_files={"assets/example.png": png_bytes(320, 120, (220, 30, 30))},
    helpers="ev_function_call (name view_image)",
    suite="view_image.rs:420 view_image_tool_attaches_local_image",
)
