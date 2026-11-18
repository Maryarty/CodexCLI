"""A custom tool call in a namespace Codex does not know: it answers 'unsupported custom tool call'."""
from events import ev_custom_tool_call_with_namespace
from scenario import Scenario, Turn, text_response, tool_response

SCENARIO = Scenario(
    name="custom-tool-unknown",
    description="custom_tool_call functions.custom_echo (unknown): the client returns an error output, the model answers",
    turns=[
        Turn(
            "Call a custom tool.",
            [
                tool_response("cu1", ev_custom_tool_call_with_namespace("custom-1", "functions", "custom_echo", "hello"), lead_text="Calling a custom tool."),
                text_response("cu2", ["It was refused. ", "END-custom-tool-unknown"], pace=0.01),
            ],
        )
    ],
    sentinel="END-custom-tool-unknown",
    allow_unsupported=True,
    helpers="ev_custom_tool_call_with_namespace (responses.rs:1070)",
    suite="search_tool.rs:1016 (custom_tool_call in namespace functions)",
    notes="The expected tool output is the error 'unsupported custom tool call'; check.py allows it for this scenario.",
)
