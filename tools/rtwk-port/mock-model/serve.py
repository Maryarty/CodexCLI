"""Serves one scenario on a local port with Codex's own MockResponsesServer.

    python3 -I serve.py <scenario> [--url-file FILE] [--list] [--prompts]

Prints `MOCK_URL=http://127.0.0.1:PORT` and serves until stopped, one queued
response per model request, in the scenario's order. Requests are logged to
stderr (method, path, kind). A model request beyond the queue gets HTTP 500.
"""

from __future__ import annotations

import argparse
import signal
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import scenario as scenarios  # noqa: E402
from server import ScenarioServer  # noqa: E402


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario", nargs="?")
    ap.add_argument("--url-file", help="write the URL here once the server listens")
    ap.add_argument("--list", action="store_true", help="list the scenarios and exit")
    ap.add_argument("--loop", action="store_true", help="replay the scenario when it runs out")
    ap.add_argument("--prompts", action="store_true", help="print the prompts and exit")
    args = ap.parse_args()
    if args.list or not args.scenario:
        for name in scenarios.names():
            sc = scenarios.load(name)
            print(f"{name:16} {len(sc.turns)} turn(s), {len(sc.responses):3} response(s)  {sc.description}")
        return 0
    sc = scenarios.load(args.scenario)
    if args.prompts:
        for i, turn in enumerate(sc.turns, 1):
            print(f"{i}. {turn.prompt}")
        return 0
    with ScenarioServer() as server:
        server.loop = args.loop
        server.load(sc)
        print(f"MOCK_URL={server.url}", flush=True)
        if args.url_file:
            Path(args.url_file).write_text(server.url)
        stop = False

        def _stop(*_: object) -> None:
            nonlocal stop
            stop = True

        signal.signal(signal.SIGTERM, _stop)
        signal.signal(signal.SIGINT, _stop)
        seen = 0
        while not stop:
            reqs = server.requests()
            for r in reqs[seen:]:
                print(f"[mock] {r.method} {r.path} ({server.queued()} queued)", file=sys.stderr, flush=True)
            seen = len(reqs)
            time.sleep(0.2)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
