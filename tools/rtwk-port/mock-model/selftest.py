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
    sc = scenarios.load("multi-agent")
    assert sc.responses and all(response.matched for response in sc.responses)
    with ScenarioServer() as srv:
        srv.load(sc)
        seen = [srv.queued()]
        for _ in range(3):
            srv.load(sc)
            seen.append(srv.queued())
    assert len(set(seen)) == 1 and seen[0] == len(sc.responses), seen
    print("loop reload: queued() over load + 3 reloads =", seen)


def cuts_cover_every_turn_once() -> None:
    """check.py --jobs N runs a scenario's pieces (Scenario.cuts) as separate sessions: together
    they are every turn once, in order, and the router never cuts between `compact` and the
    message that compacts."""
    for name in scenarios.names():
        sc = scenarios.load(name)
        if not sc.cuts:
            continue
        bounds = [0, *sc.cuts, len(sc.turns)]
        assert bounds == sorted(set(bounds)), (name, sc.cuts)
        pieces = [sc.turns[lo:hi] for lo, hi in zip(bounds, bounds[1:])]
        assert [t for piece in pieces for t in piece] == sc.turns, name
        if name == "router":
            for piece in pieces:
                prompts = [t.prompt for t in piece]
                assert ("compact" in prompts) == ("nothing special" in prompts), prompts
                assert prompts.index("compact") + 1 == prompts.index("nothing special") if "compact" in prompts else True
        print(f"cuts: {name} = {len(pieces)} pieces of {[len(p) for p in pieces]} turns")


def short_budget_is_shorter() -> None:
    import check

    sc = scenarios.load("multi-agent")
    long = check.budget(sc)
    check.SHORT.add("multi-agent")
    try:
        assert abs(check.budget(sc) - (long - (check.SLOW_BASE - check.SHORT_BASE))) < 1e-9, (check.budget(sc), long)
    finally:
        check.SHORT.discard("multi-agent")
    print(f"budget: multi-agent {long:.0f} s, short {long - (check.SLOW_BASE - check.SHORT_BASE):.0f} s")


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
    cuts_cover_every_turn_once()
    short_budget_is_shorter()
    verify_fails_clearly()
    print("ok")
