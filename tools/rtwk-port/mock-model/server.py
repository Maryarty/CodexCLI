"""The harness's MockResponsesServer, loaded with a scenario.

The server is Codex's own (harness.MockResponsesServer). The one thing added
is a hook on two of its private methods, because an interactive Codex also
sends a title request (a thread_source other than "user") next to the user's turn, and
a FIFO queue would hand it the turn's response. codex-tui's resize_reflow.rs
solves the same race by matching the body (mount_resize_response); the hook
does the same: a title request gets a title and does not consume the queue.
"""

from __future__ import annotations

import re
import threading

import harness
import scenario as scenarios

# A request whose thread_source is not "user" is an auxiliary one (the thread title:
# "system" in resize_reflow.rs's tree, "thread_title" in the
# installed 0.167). The turn of the user says "user".
TITLE_RE = re.compile(rb'thread_source\\?"\s*:\s*\\?"(?!user\\?")')


class ScenarioServer(harness.MockResponsesServer):
    def __init__(self) -> None:
        super().__init__()
        self._seen = threading.local()
        self.title_requests = 0
        # Interactive runs (try.sh --loop) answer every later turn by playing the
        # scenario again; check.py keeps the harness's strict queue (500 when empty).
        self.loop = False
        self._scenario: scenarios.Scenario | None = None

    def _record_request(self, handler, body: bytes) -> None:  # type: ignore[override]
        self._seen.is_title = bool(TITLE_RE.search(body))
        super()._record_request(handler, body)

    def _next_response(self):  # type: ignore[override]
        if getattr(self._seen, "is_title", False):
            self.title_requests += 1
            return harness.MockSseResponse(
                body=harness.sse(
                    [
                        harness.ev_assistant_message("mock-title", '{"title":"Mock scenario"}'),
                        harness.ev_completed("mock-title"),
                    ]
                )
            )
        if self.loop and self._scenario is not None and self._responses.empty():
            self.load(self._scenario)
        return super()._next_response()

    def queued(self) -> int:
        return self._responses.qsize()

    def turn_requests(self) -> int:
        """Requests that were model turns (not title requests)."""
        return sum(1 for r in self.requests() if not TITLE_RE.search(r.body))

    def load(self, scenario: scenarios.Scenario) -> None:
        self._scenario = scenario
        for resp in scenario.responses:
            self.enqueue_sse(resp.body(), delay_between_events_s=resp.pace)
