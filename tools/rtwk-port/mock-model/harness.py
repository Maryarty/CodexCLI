"""Loads Codex's own mock Responses server by path; nothing is copied.

`sdk/python/tests/app_server_harness.py` is imported as it stands. It has one
import this tool does not have, `from openai_codex import CodexConfig` (a type
used by `AppServerHarness`, which we do not use), so a stub of that module is
registered before the import. Nothing in the harness file is edited or copied.
"""

from __future__ import annotations

import importlib.util
import sys
import types
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]  # tools/rtwk-port/mock-model -> repo root
HARNESS_PATH = REPO_ROOT / "sdk" / "python" / "tests" / "app_server_harness.py"


def _load() -> types.ModuleType:
    if "openai_codex" not in sys.modules:
        stub = types.ModuleType("openai_codex")
        stub.CodexConfig = object  # type: ignore[attr-defined]
        sys.modules["openai_codex"] = stub
    spec = importlib.util.spec_from_file_location("app_server_harness", HARNESS_PATH)
    assert spec is not None and spec.loader is not None, HARNESS_PATH
    module = importlib.util.module_from_spec(spec)
    sys.modules["app_server_harness"] = module
    spec.loader.exec_module(module)
    return module


H = _load()


class HarnessChanged(RuntimeError):
    """The harness no longer has a name this tool relies on."""


# What server.py subclasses or reads from the harness's private internals, and what
# harness.py re-exports. An upstream change to app_server_harness.py breaks these
# silently (an AttributeError deep in a request thread); verify() names them up front.
_MODULE_NAMES = (
    "_ResponsesHandler", "MockResponsesServer", "MockSseResponse", "sse", "ev_response_created", "ev_completed",
    "ev_completed_with_usage", "ev_assistant_message", "ev_message_item_added", "ev_output_text_delta",
    "ev_function_call", "ev_failed",
)
_SERVER_METHODS = ("_record_request", "_next_response", "requests", "close")  # overridden or called by server.py
_SERVER_ATTRS = ("_server", "_responses")  # set by MockResponsesServer.__init__, used by server.py


def verify(module: types.ModuleType | None = None) -> None:
    """Raise HarnessChanged, naming every missing private name, unless the harness has them all."""
    h = module if module is not None else H
    missing = [f"{h.__name__}.{n}" for n in _MODULE_NAMES if not hasattr(h, n)]
    server = getattr(h, "MockResponsesServer", None)
    if server is not None:
        missing += [f"MockResponsesServer.{n}" for n in _SERVER_METHODS if not callable(getattr(server, n, None))]
        if not hasattr(server, "url"):
            missing.append("MockResponsesServer.url")
        if not missing:  # the attributes exist only on an instance: build one (bound, never started) and drop it
            probe = server()
            try:
                missing += [f"MockResponsesServer.{n} (instance attribute)" for n in _SERVER_ATTRS if not hasattr(probe, n)]
                if hasattr(probe, "_server") and not hasattr(probe._server, "RequestHandlerClass"):
                    missing.append("MockResponsesServer._server.RequestHandlerClass")
                if hasattr(probe, "_responses"):
                    missing += [f"MockResponsesServer._responses.{n}" for n in ("put", "qsize", "empty") if not hasattr(probe._responses, n)]
            finally:
                if hasattr(probe, "_server"):
                    probe._server.server_close()
    if missing:
        raise HarnessChanged(
            "the mock tool relies on private names of Codex's test harness (sdk/python/tests/app_server_harness.py) "
            "that are missing: " + ", ".join(missing) + ". The harness changed upstream; adapt harness.py/server.py."
        )


verify()

# The helpers of the harness, re-exported under their own names.
MockResponsesServer = H.MockResponsesServer
MockSseResponse = H.MockSseResponse
sse = H.sse
ev_response_created = H.ev_response_created
ev_completed = H.ev_completed
ev_completed_with_usage = H.ev_completed_with_usage
ev_assistant_message = H.ev_assistant_message
ev_message_item_added = H.ev_message_item_added
ev_output_text_delta = H.ev_output_text_delta
ev_function_call = H.ev_function_call
ev_failed = H.ev_failed
