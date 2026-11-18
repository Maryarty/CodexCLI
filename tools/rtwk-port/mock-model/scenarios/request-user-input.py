"""The model asks the user a question with request_user_input and gets the answer."""
import json

import harness
from scenario import Scenario, Turn, text_response, tool_response

ARGS = {
    "questions": [
        {
            "id": "confirm_path",
            "header": "Confirm",
            "question": "Proceed with the plan?",
            "options": [
                {"label": "Yes (Recommended)", "description": "Continue the current plan."},
                {"label": "No", "description": "Stop and revisit the approach."},
            ],
        }
    ]
}
SCENARIO = Scenario(
    name="request-user-input",
    description="request_user_input with one multiple-choice question, then an answer that ends the turn",
    turns=[
        Turn(
            "Ask me before you continue.",
            [
                tool_response("ru1", harness.ev_function_call("call-ru-1", "request_user_input", json.dumps(ARGS)), lead_text="I need to ask you something."),
                text_response("ru2", ["Thanks for answering. ", "END-request-user-input"], pace=0.01),
            ],
        )
    ],
    sentinel="END-request-user-input",
    config_tables="[features]\ndefault_mode_request_user_input = true\n",
    pty_only=True,
    helpers="ev_function_call (name request_user_input)",
    suite="request_user_input.rs:72 request_user_input_round_trip_resolves_pending (Plan mode), :428 ..._in_default_mode_with_feature",
    notes="Needs a person (or the pty driver) to answer the question; `codex exec` has no one to ask.",
)
