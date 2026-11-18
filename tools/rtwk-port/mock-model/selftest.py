"""Quick checks of the server's own bookkeeping (no binary, no network):

    python3 -I selftest.py

1. --loop: queued() stays constant over three reloads of a scenario (matched
   responses are reset by load(), not appended again).
2. harness.verify() passes on the real harness and fails, naming each missing
   private name, when a module-level name, a method or an instance attribute is gone.
"""

from __future__ import annotations

import sys
import types
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import harness  # noqa: E402
import scenario as scenarios  # noqa: E402
from server import ScenarioServer  # noqa: E402


def loop_reload_is_constant() -> None:
    sc = scenarios.load("multi-agent")  # three matched responses
    assert sum(r.matched for r in sc.responses) == 3
    with ScenarioServer() as srv:
        srv.load(sc)
        seen = [srv.queued()]
        for _ in range(3):
            srv.load(sc)
            seen.append(srv.queued())
    assert len(set(seen)) == 1 and seen[0] == len(sc.responses), seen
    print("loop reload: queued() over load + 3 reloads =", seen)


def fake_harness(drop_module=(), drop_methods=()) -> types.ModuleType:
    real = harness.H
    fake = types.ModuleType("fake_harness")
    for n in dir(real):
        if not n.startswith("__") and n not in drop_module:
            setattr(fake, n, getattr(real, n))
    if drop_methods:
        fake.MockResponsesServer = type("M", (real.MockResponsesServer,), {n: None for n in drop_methods})
    return fake


def verify_fails_clearly() -> None:
    harness.verify()  # the real harness passes
    cases = [
        (dict(drop_module=("_ResponsesHandler",)), "fake_harness._ResponsesHandler"),
        (dict(drop_module=("ev_failed",)), "fake_harness.ev_failed"),
        (dict(drop_methods=("_next_response",)), "MockResponsesServer._next_response"),
    ]
    for kwargs, expect in cases:
        try:
            harness.verify(fake_harness(**kwargs))
        except harness.HarnessChanged as exc:
            assert expect in str(exc), (expect, str(exc))
            print("verify fails:", expect)
        else:
            raise AssertionError(f"verify passed with {expect} missing")
    orig = harness.H.MockResponsesServer.__init__

    def init(self):  # type: ignore[no-untyped-def]
        orig(self)
        del self._responses

    harness.H.MockResponsesServer.__init__ = init
    try:
        harness.verify()
    except harness.HarnessChanged as exc:
        assert "_responses (instance attribute)" in str(exc), str(exc)
        print("verify fails: MockResponsesServer._responses (instance attribute)")
    else:
        raise AssertionError("verify passed with _responses missing")
    finally:
        harness.H.MockResponsesServer.__init__ = orig


if __name__ == "__main__":
    loop_reload_is_constant()
    verify_fails_clearly()
    print("ok")
