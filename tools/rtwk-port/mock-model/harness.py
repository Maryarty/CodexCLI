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
