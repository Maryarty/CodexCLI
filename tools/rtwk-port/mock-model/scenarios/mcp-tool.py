"""MCP tool calls (namespace mcp__rmcp): echo with structured content, then image_scenario with text and an image."""
import json

from configs import MCP_TABLES
from events import ev_function_call_with_namespace
from scenario import Scenario, Turn, text_response, tool_response

NS = "mcp__rmcp"
SCENARIO = Scenario(
    name="mcp-tool",
    description="MCP echo (structured result), MCP image_scenario text_then_image, then an answer; server = mcp_server.py",
    turns=[
        Turn(
            "Call the echo tool, then the image tool.",
            [
                tool_response(
                    "mc1",
                    ev_function_call_with_namespace("call-mcp-1", NS, "echo", json.dumps({"message": "hello from the mock model"})),
                    lead_text="Calling the echo tool.",
                ),
                tool_response(
                    "mc2",
                    ev_function_call_with_namespace(
                        "call-mcp-2", NS, "image_scenario", json.dumps({"scenario": "text_then_image", "caption": "Here is the image:"})
                    ),
                    lead_text="Now the image tool.",
                ),
                text_response("mc3", ["Both MCP calls returned. ", "END-mcp-tool"], pace=0.01),
            ],
        )
    ],
    sentinel="END-mcp-tool",
    config_tables=MCP_TABLES,
    helpers="ev_function_call_with_namespace (responses.rs:1028)",
    suite="rmcp_client.rs:711 text_only_mcp_content_uses_content_items (namespace mcp__rmcp, tool image_scenario); echo: rmcp_client.rs:1222",
    notes="The server is tools/rtwk-port/mock-model/mcp_server.py, a stdlib twin of test_stdio_server (which needs cargo).",
)
