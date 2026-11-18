"""A client-executed tool_search_call, then an answer."""
import harness
from events import ev_tool_search_call
from scenario import Resp, Scenario, Turn, text_response

search = Resp(
    [
        harness.ev_response_created("ts1"),
        ev_tool_search_call("call-ts-1", {"query": "create calendar event", "limit": 8}),
        harness.ev_completed("ts1"),
    ]
)
SCENARIO = Scenario(
    name="tool-search",
    description="tool_search_call (execution client, query + limit), then an answer",
    turns=[Turn("Find a tool for calendar events.", [search, text_response("ts2", ["Searched. ", "END-tool-search"], pace=0.01)])],
    sentinel="END-tool-search",
    helpers="ev_tool_search_call (responses.rs:1046)",
    suite="search_tool.rs:256 (tool_search_call with query and limit 8)",
    notes="Codex answers a tool_search_call with a tool_search_output item; whether the installed build accepts it without deferred tools is in the coverage table.",
)
