"""One perf run's fake model: a temp CODEX_HOME plus Codex's own MockResponsesServer.

Reuses tools/rtwk-port/mock-model (server.py, home.py, scenario.py, harness.py) as they
stand; only the scenario is built here, with parameters, because the perf scenarios
need lengths and rates that scenarios/text-100.py and text-1000.py do not have
(20 s at 100/s, 10 s at 1000/s, deltas of about 12 tokens, paragraphs).

    python3 -I mock.py --root DIR --deltas N --pace SECONDS --sentinel TEXT [--words K] [--seed S]

Prints one JSON line {"url","home","work"} once it listens, then serves until SIGTERM.
The workdir is empty and trusted; analytics are off; the model request count is not capped
(--loop replays the response for later turns).
"""

from __future__ import annotations

import argparse
import json
import random
import signal
import sys
import time
from pathlib import Path

MOCK_DIR = Path(__file__).resolve().parents[3] / "tools" / "rtwk-port" / "mock-model"
sys.path.insert(0, str(MOCK_DIR))

import home as mock_home  # noqa: E402
import scenario as scenarios  # noqa: E402
from server import ScenarioServer  # noqa: E402

WORDS = (
    "the of and to in is that for it as with be on at by this from or an are not have but they which "
    "one you all can her was their there been if more when will who would make like time just know take "
    "into year your some could them see other than then now look only come its over think also back after "
    "use two how our work first well way even new want because any these give day most us render frame "
    "buffer stream widget layout style paint cell terminal screen input event queue state model async"
).split()


def build(deltas: int, pace: float, sentinel: str, words: int, seed: int, begin: str) -> scenarios.Scenario:
    rng = random.Random(seed)
    parts: list[str] = [begin + "\n\n"] if begin else []
    for i in range(deltas):
        s = " ".join(rng.choice(WORDS) for _ in range(words)) + " "
        if i % 8 == 7:
            s = s.rstrip() + "\n\n"
        parts.append(s)
    parts.append("\n\n" + sentinel)
    return scenarios.Scenario(
        name="perf",
        description=f"{deltas} deltas of {words} words at {1 / pace if pace else 0:.0f}/s (nominal)",
        turns=[scenarios.Turn("go", [scenarios.text_response("perf", parts, pace=pace)])],
        sentinel=sentinel,
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    ap.add_argument("--deltas", type=int, default=600)
    ap.add_argument("--pace", type=float, default=0.01)
    ap.add_argument("--sentinel", default="END-perf")
    ap.add_argument("--words", type=int, default=6)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--begin", default="", help="a token the first delta is: when it is visible the stream has started")
    args = ap.parse_args()
    sc = build(args.deltas, args.pace, args.sentinel, args.words, args.seed, args.begin)
    with ScenarioServer() as server:
        server.loop = True
        server.load(sc)
        home, work = mock_home.write_home(sc, server.url, Path(args.root))
        print(json.dumps({"url": server.url, "home": str(home), "work": str(work)}), flush=True)
        stop = False

        def _stop(*_: object) -> None:
            nonlocal stop
            stop = True

        signal.signal(signal.SIGTERM, _stop)
        signal.signal(signal.SIGINT, _stop)
        while not stop:
            time.sleep(0.1)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
