"""Runs every scenario against each binary that is built, headless, and asserts
that the turns end as the scenario says.

    python3 -I check.py [--only NAME[,NAME]] [--bin rtwk|oracle] [--pty] [--json FILE] [--dump] [--list-bins]

- oracle `codex`: `codex exec --json` per turn (`exec resume --last` for the next
  turns); asserts turn.completed / turn.failed per turn, the sentinel in the last
  agent_message, and that the mock served every queued response.
- codex-rtwk has no exec mode (its command line is --debug-screens/--model/--locale),
  so it runs in a pty (120x40, answering the terminal's cursor/device queries),
  gets each prompt typed and Enter, "y" at an approval prompt, then Ctrl+C. It
  passes when the mock served every queued response, the process stayed up until
  the quit and the quit worked; "sentinel on screen" is reported next to it.
- `--pty` runs the oracle in the pty driver too; a scenario marked `pty_only` (keys, slash
  commands, a question to answer) always does.
- `--json FILE` writes the results (coverage.py reads it); `--dump` prints, per model
  request, the tool names it offered and the output items it carried (to see what the
  binary accepted).
- RTWK_BIN / ORACLE_BIN override the search of bins.py. A binary that is not
  built is reported as such and skipped; nothing is built here.
Never talks to a real model: every run has its own CODEX_HOME whose provider is
the local mock.
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import re
import select
import shutil
import signal
import struct
import subprocess
import sys
import termios
import time
from dataclasses import dataclass, field
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

try:
    import harness  # noqa: E402  (verifies on import the private names of Codex's harness that server.py uses)
except RuntimeError as exc:  # harness.HarnessChanged: a missing name, said plainly, not a traceback
    if type(exc).__name__ != "HarnessChanged":
        raise
    print(f"check.py: {exc}", file=sys.stderr)
    raise SystemExit(2)

import bins  # noqa: E402
import home  # noqa: E402
import scenario as scenarios  # noqa: E402
from server import ScenarioServer  # noqa: E402

ANSI = re.compile(rb"\x1b(\[[0-9;?<>=! ]*[ -/]*[@-~]|\][^\x07\x1b]*(\x07|\x1b\\)|[()][0-9A-B]|[=>78M])")
APPROVAL = re.compile(r"\(y\)|Yes, proceed|Would you like to")  # the prompt, not the history line "You approved ..."
QUESTION = re.compile(r"Question \d+/\d+.*enter\s*to\s*submit", re.S)  # request_user_input: Enter submits the highlighted option
NUDGE = re.compile(r"Approaching rate limits.*Keep\s*current\s*model", re.S)  # after a 429: "Switch to gpt-6-luna?"; 2 = keep
DISMISS = re.compile(r"cannot answer it yet")  # the skeleton's request dialog: Enter dismisses (rejects)


@dataclass
class Result:
    scenario: str
    binary: str
    mode: str
    status: str  # PASS / FAIL
    seconds: float
    detail: str = ""
    notes: list[str] = field(default_factory=list)


def budget(sc: scenarios.Scenario) -> float:
    """Seconds one turn may take: the paced stream time, doubled, plus slack."""
    longest = max((len(r.events) * r.pace for r in sc.responses), default=0.0)
    return 60.0 + 3 * longest + 0.15 * len(sc.responses)


# ---------------------------------------------------------------- exec mode


def run_exec(binary: str, sc: scenarios.Scenario, root: Path, srv: ScenarioServer) -> tuple[bool, str, list[str]]:
    codex_home, work = home.write_home(sc, srv.url, root)
    env = {**os.environ, "CODEX_HOME": str(codex_home), "OPENAI_API_KEY": "dummy", "NO_COLOR": "1"}
    notes: list[str] = []
    last_text = ""
    for i, turn in enumerate(sc.turns):
        base = [binary, "exec"] + (["resume", "--last"] if i else [])
        cmd = base + ["--json", "--skip-git-repo-check", "-c", "analytics.enabled=false"]
        if i == 0:
            cmd += ["-C", str(work)]
        cmd.append(turn.prompt)
        try:
            p = subprocess.run(cmd, cwd=work, env=env, stdin=subprocess.DEVNULL, capture_output=True, text=True, timeout=budget(sc))
        except subprocess.TimeoutExpired:
            return False, f"turn {i + 1}: timed out after {budget(sc):.0f}s", notes
        events = []
        for line in p.stdout.splitlines():
            try:
                events.append(json.loads(line))
            except ValueError:
                pass
        types = [e.get("type") for e in events]
        got = "completed" if "turn.completed" in types else "failed" if "turn.failed" in types else "none"
        if got != turn.outcome:
            tail = (p.stderr or "").strip().splitlines()[-3:]
            return False, f"turn {i + 1}: expected {turn.outcome}, got {got}; types={types[-6:]} stderr={tail}", notes
        msgs = [e["item"]["text"] for e in events if e.get("type") == "item.completed" and e["item"].get("type") == "agent_message"]
        if msgs:
            last_text = msgs[-1]
        for e in events:
            if e.get("type") == "item.completed" and e["item"].get("type") == "error":
                notes.append("error item: " + e["item"].get("message", "")[:80])
    if sc.sentinel not in last_text:
        return False, f"sentinel {sc.sentinel!r} not in the last agent message ({last_text[-60:]!r})", notes
    return drained(srv, sc, notes)


def drained(srv: ScenarioServer, sc: scenarios.Scenario, notes: list[str]) -> tuple[bool, str, list[str]]:
    want = len(sc.responses)
    got = srv.turn_requests()
    if srv.queued() != 0 or got != want:
        return False, f"mock served {got} model request(s) of {want}, {srv.queued()} still queued", notes
    problem = tools_answered(srv, sc)
    if problem:
        return False, problem, notes
    notes.append(f"{got} model request(s), {srv.title_requests} title request(s)")
    return True, "", notes


def tools_answered(srv: ScenarioServer, sc: scenarios.Scenario) -> str:
    """Every tool call of the scenario came back as an output item of a later request,
    and none was refused as an unknown tool (a model slug without the tool would do that)."""
    calls = [
        e["item"]["call_id"]
        for r in sc.responses
        for e in r.events
        if e.get("item", {}).get("type") in ("function_call", "custom_tool_call", "tool_search_call")
        and e.get("type") == "response.output_item.done"
    ]
    outputs: dict[str, str] = {}
    for req in srv.requests():
        for item in req.input():
            if str(item.get("type", "")).endswith("_output") and "call_id" in item:
                outputs[item["call_id"]] = json.dumps(item.get("output"))
    for call_id in calls:
        if call_id not in outputs:
            return f"tool call {call_id} was never answered by an output item"
        if "unsupported" in outputs[call_id] and not sc.allow_unsupported:
            return f"tool call {call_id} was refused: {outputs[call_id][:120]}"
    return ""


# ----------------------------------------------------------------- pty mode


def run_pty(binary: str, sc: scenarios.Scenario, root: Path, srv: ScenarioServer, *, oracle: bool) -> tuple[bool, str, list[str]]:
    codex_home, work = home.write_home(sc, srv.url, root)
    env = {**os.environ, "CODEX_HOME": str(codex_home), "OPENAI_API_KEY": "dummy", "TERM": "xterm-256color", "COLUMNS": "120", "LINES": "40"}
    argv = [binary] + (["-c", "analytics.enabled=false", "-C", str(work)] if oracle else [])
    pid, fd = os_forkpty(argv, work, env)
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    out = bytearray()
    state = {"last": time.time(), "scan": 0, "answered": 0}
    notes: list[str] = []

    def pump(seconds: float) -> None:
        end = time.time() + seconds
        while time.time() < end:
            r, _, _ = select.select([fd], [], [], 0.1)
            if not r:
                continue
            try:
                data = os.read(fd, 65536)
            except OSError:
                return
            if not data:
                return
            out.extend(data)
            state["last"] = time.time()
            if b"\x1b[6n" in data:
                send(b"\x1b[1;1R")
            if b"\x1b[c" in data or b"\x1b[0c" in data:
                send(b"\x1b[?62;c")

    def send(data: bytes) -> None:
        try:
            os.write(fd, data)
        except OSError:  # the child is gone; alive() says so next
            state["dead"] = True

    def alive() -> bool:
        if state.get("dead"):
            return False
        done, _ = os.waitpid(pid, os.WNOHANG)
        if done:
            state["dead"] = True
        return not done

    def screen_text(since: int = 0) -> str:
        return ANSI.sub(b"", bytes(out[since:])).decode("utf-8", "replace")

    def quiet_for(seconds: float, limit: float) -> bool:
        end = time.time() + limit
        while time.time() < end:
            pump(0.2)
            if time.time() - state["last"] >= seconds:
                return True
            if not alive():
                return False
        return False

    try:
        quiet_for(1.5, 30)
        if not alive():
            return False, "exited at startup: " + screen_text()[-200:].replace("\n", " "), notes
        expected = 0
        for i, turn in enumerate(sc.turns):
            expected += len(turn.responses)
            pump(1.0)
            if NUDGE.search(screen_text(max(0, len(out) - 3000))):
                send(b"2")
                notes.append("answered the rate-limit nudge (keep the model)")
                pump(1.0)
            mark = len(out)
            send(turn.prompt.encode())
            pump(0.4)
            send(b"\r")
            if turn.esc_after is not None:
                pump(turn.esc_after)
                send(b"\x1b")
                notes.append(f"pressed Esc {turn.esc_after:g}s after Enter")
                pump(1.0)
            end = time.time() + budget(sc)
            while time.time() < end:
                pump(0.25)
                if not alive():
                    return False, f"exited during turn {i + 1}", notes
                tail = screen_text(max(mark, len(out) - 4000))
                if NUDGE.search(tail) and time.time() - state["last"] > 0.5:
                    send(b"2")
                    notes.append("answered the rate-limit nudge (keep the model)")
                    mark = len(out)
                    continue
                if srv.turn_requests() < expected and state["answered"] < 60 and time.time() - state["last"] > 0.5:
                    if QUESTION.search(tail):
                        send(b"\r")
                        notes.append("answered a request_user_input question (Enter)")
                    elif DISMISS.search(tail):
                        send(b"\r")
                        notes.append("pressed Enter on the dismiss dialog (the build cannot answer requests yet)")
                    elif APPROVAL.search(tail):
                        send(b"y")
                        notes.append("sent y at an approval prompt")
                    else:
                        continue
                    state["answered"] += 1
                    mark = len(out)
                if srv.turn_requests() >= expected and time.time() - state["last"] > 2.0:
                    break
            else:
                last = " | ".join(x.strip() for x in screen_text(max(0, len(out) - 3000)).splitlines() if x.strip())[-int(os.environ.get("MOCK_SCREEN_CHARS", "360")):]
                return False, f"turn {i + 1}: mock served {srv.turn_requests()} of {expected} model request(s) in {budget(sc):.0f}s; screen ends: {last}", notes
        seen = sc.sentinel in re.sub(r"\s+", "", screen_text())
        notes.append(f"sentinel on screen: {'yes' if seen else 'no'}")
        if not seen and os.environ.get("MOCK_DUMP_SCREEN"):
            notes.append("screen: " + " | ".join(x.strip() for x in screen_text().splitlines() if x.strip())[-600:])
        send(b"\x03")
        pump(0.8)
        if alive():
            send(b"\x03")
        quiet_for(1.0, 6)
        still = alive()
        if still:
            notes.append("did not quit on Ctrl+C twice; killed")
        ok, detail, notes = drained(srv, sc, notes)
        return ok, detail, notes
    finally:
        if not state.get("dead"):
            try:
                os.kill(pid, signal.SIGKILL)
                os.waitpid(pid, 0)
            except (ProcessLookupError, ChildProcessError):
                pass
        os.close(fd)


def os_forkpty(argv: list[str], cwd: Path, env: dict[str, str]) -> tuple[int, int]:
    import pty

    pid, fd = pty.fork()
    if pid == 0:
        try:
            os.chdir(cwd)
            os.execve(argv[0], argv, env)
        finally:
            os._exit(127)
    return pid, fd


# --------------------------------------------------------------------- main


def dump(srv: ScenarioServer) -> None:
    """What each model request offered and carried (tool names; the items of its input tail)."""
    for n, req in enumerate(srv.requests(), 1):
        body = req.body_json()
        tools = [t.get("name") or t.get("type") for t in body.get("tools", [])]
        items = [str(i.get("type") or i.get("role")) + (":" + i["name"] if "name" in i else "") for i in body.get("input", [])][-6:]
        print(f"    req {n}: tools={tools}")
        print(f"           input tail={items}")
        if os.environ.get("MOCK_DUMP_META"):
            print("           meta:", json.dumps({k: v for k, v in body.get("client_metadata", {}).items() if "source" in k or "parent" in k or "x-codex-turn-metadata" in k})[:900])
        if os.environ.get("MOCK_DUMP_TAIL"):
            for i in body.get("input", [])[-int(os.environ["MOCK_DUMP_TAIL"]):]:
                print("           item:", json.dumps(i)[:300])
        for i in body.get("input", []):
            if str(i.get("type", "")).endswith("_output"):
                print(f"           output {i.get('call_id')}: {json.dumps(i.get('output'))[:200]}")


def run_one(name: str, label: str, binary: str, mode: str, show: bool = False) -> Result:
    sc = scenarios.load(name)
    root = home.temp_root()
    started = time.time()
    try:
        with ScenarioServer() as srv:
            if sc.router:
                from router import Router

                srv.use_router(Router())
            else:
                srv.load(sc)
            try:
                if mode == "exec":
                    ok, detail, notes = run_exec(binary, sc, root, srv)
                else:
                    ok, detail, notes = run_pty(binary, sc, root, srv, oracle=(label == "oracle"))
            finally:
                if show:
                    dump(srv)
    except Exception as exc:  # a harness fault is a failure, with its type
        ok, detail, notes = False, f"{type(exc).__name__}: {exc}", []
    finally:
        shutil.rmtree(root, ignore_errors=True)
    return Result(name, label, mode, "PASS" if ok else "FAIL", time.time() - started, detail, notes)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", help="comma-separated scenario names")
    ap.add_argument("--bin", choices=["rtwk", "oracle"], action="append")
    ap.add_argument("--pty", action="store_true", help="drive the oracle in a pty instead of exec")
    ap.add_argument("--list-bins", action="store_true")
    ap.add_argument("--json", help="write the results to this file")
    ap.add_argument("--dump", action="store_true", help="print the tools and tool outputs of every model request")
    args = ap.parse_args()
    found = {"rtwk": bins.rtwk_bin(), "oracle": bins.oracle_bin()}
    for label, path in found.items():
        print(f"{label:7} {path or 'NOT BUILT (no binary found; nothing is built here)'}")
    if args.list_bins:
        return 0
    names = args.only.split(",") if args.only else scenarios.names()
    if os.environ.get("MOCK_TURNS"):  # debugging: MOCK_TURNS=a:b keeps turns a..b-1 of every scenario
        import dataclasses

        lo, hi = (int(x) for x in os.environ["MOCK_TURNS"].split(":"))
        orig = scenarios.load
        scenarios.load = lambda n: (lambda sc: dataclasses.replace(sc, turns=sc.turns[lo:hi]) if n in names else sc)(orig(n))
    results: list[Result] = []
    for label in args.bin or ["oracle", "rtwk"]:
        path = found[label]
        if not path:
            continue
        for name in names:
            mode = "pty" if (label == "rtwk" or args.pty or scenarios.load(name).pty_only) else "exec"
            r = run_one(name, label, path, mode, args.dump)
            results.append(r)
            extra = f"  {r.detail}" if r.detail else ""
            print(f"{r.status}  {label:6} {mode:4} {name:16} {r.seconds:5.1f}s{extra}  [{'; '.join(dict.fromkeys(r.notes))}]", flush=True)
    if not results:
        print("no binary to run: nothing checked")
        return 2
    if args.json:
        Path(args.json).write_text(json.dumps([r.__dict__ for r in results], indent=1))
    failed = [r for r in results if r.status != "PASS"]
    print(f"{len(results) - len(failed)}/{len(results)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
