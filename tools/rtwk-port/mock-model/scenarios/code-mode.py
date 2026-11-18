"""The code-mode `exec` custom tool (a JS snippet), then an answer."""
from events import ev_custom_tool_call
from scenario import Scenario, Turn, text_response, tool_response

SCENARIO = Scenario(
    name="code-mode",
    description="custom_tool_call `exec` with a JavaScript snippet (code mode), then an answer",
    turns=[
        Turn(
            "Run a snippet.",
            [
                tool_response("cmd1", ev_custom_tool_call("call-code-1", "exec", "text('hello from code mode');"), lead_text="Running the snippet."),
                text_response("cmd2", ["The snippet ran. ", "END-code-mode"], pace=0.01),
            ],
        )
    ],
    sentinel="END-code-mode",
    config_tables="[features]\ncode_mode = true\n",
    helpers="ev_custom_tool_call (responses.rs:1058)",
    suite="code_mode.rs:294 (custom_tool_call exec), app_tool_exposure.rs:155",
)
