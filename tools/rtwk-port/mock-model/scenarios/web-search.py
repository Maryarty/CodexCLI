"""A web_search_call item (added, then done with the query), then an answer."""
import harness
from events import ev_web_search_call_added_partial, ev_web_search_call_done
from scenario import Resp, Scenario, Turn, chunks

ANSWER = "Search done: the mock found nothing real. END-web-search"
events = [
    harness.ev_response_created("ws1"),
    ev_web_search_call_added_partial("ws-call-1", "in_progress"),
    ev_web_search_call_done("ws-call-1", "completed", "rtwk terminal ui framework"),
    harness.ev_message_item_added("msg-ws1"),
    *[harness.ev_output_text_delta(d) for d in chunks(ANSWER, 8)],
    harness.ev_assistant_message("msg-ws1", ANSWER),
    harness.ev_completed("ws1"),
]
SCENARIO = Scenario(
    name="web-search",
    description="web_search_call in progress, completed with a query, then a streamed answer",
    turns=[Turn("Search the web for rtwk.", [Resp(events, 0.15)])],
    sentinel="END-web-search",
)
