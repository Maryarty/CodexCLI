"""The harness's MockResponsesServer, loaded with a scenario.

The server is Codex's own (harness.MockResponsesServer). The one thing added
is a hook on two of its private methods, because an interactive Codex also
sends a title request (a thread_source other than "user") next to the user's turn, and
a FIFO queue would hand it the turn's response. codex-tui's resize_reflow.rs
solves the same race by matching the body (mount_resize_response); the hook
does the same: a title request gets a title and does not consume the queue.
"""

from __future__ import annotations

import dataclasses
import json
import re
import threading
from dataclasses import dataclass

import harness
import scenario as scenarios

# A request whose thread_source is not "user" is an auxiliary one (the thread title:
# "system" in resize_reflow.rs's tree, "thread_title" in the installed 0.167). The turn of
# the user says "user"; a spawned agent's turn says "subagent" and is a real turn.
TITLE_RE = re.compile(rb'thread_source\\?"\s*:\s*\\?"(?!(?:user|subagent)\\?")')


# A scenario writes `@@SESSION_ID@@` where a write_stdin call names the process (an integer).
SESSION_PLACEHOLDER = "@@SESSION_ID@@"
SESSION_RE = re.compile(r"session ID (\d+)")


@dataclass(frozen=True)
class ScenarioSse(harness.MockSseResponse):
    """The harness's queued response plus an HTTP status and extra headers."""

    status: int = 200
    headers: tuple[tuple[str, str], ...] = ()


class _Handler(harness.H._ResponsesHandler):  # type: ignore[misc]
    """The harness's handler, with the status and headers of the response being served.
    Both are read from the serving thread's `_seen.resp`; nothing else is changed."""

    def do_POST(self):  # type: ignore[override]
        try:
            super().do_POST()
        except (BrokenPipeError, ConnectionResetError):
            pass  # the client cut the stream (an interrupted turn); nothing to report

    def send_response(self, code, message=None):  # type: ignore[override]
        resp = getattr(self.server.mock._seen, "resp", None)
        if code == 200 and resp is not None and getattr(resp, "status", 200) != 200:
            code = resp.status
        super().send_response(code, message)

    def end_headers(self):  # type: ignore[override]
        resp = getattr(self.server.mock._seen, "resp", None)
        for name, value in getattr(resp, "headers", ()):
            self.send_header(name, value)
        super().end_headers()


class ScenarioServer(harness.MockResponsesServer):
    def __init__(self) -> None:
        super().__init__()
        self._server.RequestHandlerClass = _Handler
        self._seen = threading.local()
        self.title_requests = 0
        # Responses with `when`/`unless` are served by body match, once each.
        self._matched: list[scenarios.Resp] = []
        self._matched_lock = threading.Lock()
        # Router mode: the user's message picks the response (router.py); no queue.
        self.router = None
        # Interactive runs (try.sh --loop) answer every later turn by playing the
        # scenario again; check.py keeps the harness's strict queue (500 when empty).
        self.loop = False
        self._scenario: scenarios.Scenario | None = None

    def _record_request(self, handler, body: bytes) -> None:  # type: ignore[override]
        self._seen.is_title = bool(TITLE_RE.search(body))
        self._seen.body = body
        super()._record_request(handler, body)

    def _next_response(self):  # type: ignore[override]
        resp = self._pick_response()
        if SESSION_PLACEHOLDER in resp.body:
            # write_stdin needs the process id unified exec chose (random in a release build;
            # core's tests see 1000): it is in the output of the exec_command that started it.
            found = SESSION_RE.findall(getattr(self._seen, "body", b"").decode("utf-8", "replace"))
            resp = dataclasses.replace(resp, body=resp.body.replace(SESSION_PLACEHOLDER, found[-1] if found else "1000"))
            self._seen.resp = resp
        return resp

    def _pick_response(self):
        if getattr(self._seen, "is_title", False):
            self.title_requests += 1
            return self._remember(
                harness.MockSseResponse(
                    body=harness.sse(
                        [
                            harness.ev_assistant_message("mock-title", '{"title":"Mock scenario"}'),
                            harness.ev_completed("mock-title"),
                        ]
                    )
                )
            )
        body = getattr(self._seen, "body", b"")
        if self.router is not None:
            return self._serve(self.router.respond(body))
        text = body.decode("utf-8", "replace")
        try:
            request = json.loads(text)
        except ValueError:
            request = {}
        with self._matched_lock:
            for resp in self._matched:
                if resp.pick(request, text):
                    self._matched.remove(resp)
                    return self._serve(resp)
        if self.loop and self._scenario is not None and self._responses.empty():
            self.load(self._scenario)
        return self._remember(super()._next_response())

    def _serve(self, resp: scenarios.Resp) -> harness.MockSseResponse:
        return self._remember(
            ScenarioSse(body=resp.body(), delay_between_events_s=resp.pace, status=resp.status, headers=resp.headers)
        )

    def _remember(self, resp: harness.MockSseResponse) -> harness.MockSseResponse:
        self._seen.resp = resp
        return resp

    def queued(self) -> int:
        return self._responses.qsize() + len(self._matched)

    def turn_requests(self) -> int:
        """Requests that were model turns (not title requests)."""
        return sum(1 for r in self.requests() if not TITLE_RE.search(r.body))

    def load(self, scenario: scenarios.Scenario) -> None:
        self._scenario = scenario
        # A reload (--loop plays the scenario again) starts the matched responses afresh: the
        # ones the earlier play did not use must not stay next to their new copies.
        with self._matched_lock:
            self._matched = [resp for resp in scenario.responses if resp.matched]
        for resp in scenario.responses:
            if not resp.matched:
                self._responses.put(
                    ScenarioSse(body=resp.body(), delay_between_events_s=resp.pace, status=resp.status, headers=resp.headers)
                )

    def use_router(self, router) -> None:
        self.router = router
