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
  Interrupt scenarios also require the interrupted stream to disconnect and the final
  sentinel to be displayed.
- `--pty` runs the oracle in the pty driver too; a scenario marked `pty_only` (keys, slash
  commands, a question to answer) always does.
- `--size WIDTHxHEIGHT` sets the PTY dimensions. `--capture-dir DIR` runs in PTY
  mode and saves the terminal stream at matching checkpoints for both binaries.
- `--capture-resume` also opens the same fixture saved by the oracle in both
  binaries and saves the resumed frames.
- `--capture-details` records the transcript after Ctrl+T.
- `--json FILE` writes the results (coverage.py reads it); `--dump` prints, per model
  request, the tool names it offered and the output items it carried (to see what the
  binary accepted).
- `--jobs N` runs N scenarios at once, each in a forked worker with its own mock server,
  CODEX_HOME and pty (nothing is shared); the long ones start first, results print as they
  finish. The default is 1 (the order below); land-check.sh passes 8. `--capture-resume` runs
  one at a time whatever N says. With N > 1 a scenario that names `cuts` (router.py: the
  router, 34 turns) runs as pieces `router#1..#4`, each a worker of its own that types its
  slice of the turns (the sentinel is checked by the last piece): a long scenario no longer
  sets the floor of the whole run. `MOCK_TURN_TIMES=1` prints the seconds of each turn.
- `--short-budget NAME[,NAME]` gives the named scenarios 20 s of slack per turn instead of
  60 s: a known red only waits its budget out, a green one never does. land-check.sh passes
  the `scenarios:` entries of known-red.txt.
- RTWK_BIN / ORACLE_BIN override the search of bins.py. A binary that is not
  built is reported as such and skipped; nothing is built here.
Never talks to a real model: every run has its own CODEX_HOME whose provider is
the local mock.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import dataclasses
import fcntl
import multiprocessing
import json
import os
import re
import select
import shutil
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
from server import ScenarioServer, StreamStatus  # noqa: E402

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

@dataclass(frozen=True)
class Terminal:
    size: tuple[int, int] = (120, 40)
    light: bool = False
    details: bool = False


READY_MARK, READY_WAIT = "AskCodextodoanything", 30.0  # the composer's placeholder; seconds to wait for it
SLOW_BASE, SHORT_BASE = 60.0, 20.0
SHORT: set[str] = set()  # scenarios run with the short slack (--short-budget): known reds that only wait it out


def budget(sc: scenarios.Scenario) -> float:
    """Seconds one turn may take: the paced stream time, tripled, plus slack."""
    longest = max((len(r.events) * r.pace for r in sc.responses), default=0.0)
    return (SHORT_BASE if sc.name in SHORT else SLOW_BASE) + 3 * longest + 0.15 * len(sc.responses)


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


def run_pty(binary: str, sc: scenarios.Scenario, root: Path, srv: ScenarioServer, *, oracle: bool, terminal: Terminal = Terminal(), capture: Path | None = None, resume: str | None = None) -> tuple[bool, str, list[str]]:
    codex_home, work = (root / "codex-home", (root / "work").resolve()) if resume else home.write_home(sc, srv.url, root)
    width, height = terminal.size
    fg, bg = (b"2020/2424/2727", b"fafa/fafa/fafa") if terminal.light else (b"d8d8/e4e4/f0f0", b"0404/1919/2727")
    env = {**os.environ, "CODEX_HOME": str(codex_home), "OPENAI_API_KEY": "dummy", "TERM": "xterm-256color", "COLUMNS": str(width), "LINES": str(height)}
    argv = [binary]
    if resume:
        argv.append("resume")
        if oracle:
            argv.append(resume)
    # rtwk's chat starts the shared background server by default (as the oracle's does):
    # a harness run with a temporary CODEX_HOME stays embedded.
    if capture is not None or not oracle:
        argv.append("--no-daemon")
    if oracle:
        argv += ["-C", str(work)]
    child, fd = spawn_pty(argv, work, env, width, height)
    out = bytearray()
    state = {"last": time.time(), "scan": 0, "answered": 0}
    notes: list[str] = []

    def checkpoint(name: str) -> None:
        if capture is not None:
            capture.mkdir(parents=True, exist_ok=True)
            (capture / f"{name}.ansi").write_bytes(out)

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
            if b"\x1b]10;?" in data:
                send(b"\x1b]10;rgb:" + fg + b"\x07")
            if b"\x1b]11;?" in data:
                send(b"\x1b]11;rgb:" + bg + b"\x07")

    def send(data: bytes) -> None:
        try:
            os.write(fd, data)
        except OSError:  # the child is gone; alive() says so next
            state["dead"] = True

    def alive() -> bool:
        if state.get("dead"):
            return False
        done = child.poll() is not None
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

    def interrupt(request_number: int, delay: float) -> str | None:
        deadline = time.monotonic() + budget(sc)
        while srv.turn_stream_status(request_number) is None:
            if not alive() or time.monotonic() >= deadline:
                return "the model request did not start before Escape"
            pump(0.1)
        pump(delay)
        if srv.turn_stream_status(request_number) is not StreamStatus.STARTED:
            return "the model stream already ended before Escape"

        sent_at = time.monotonic()
        send(b"\x1b")
        notes.append(f"sent a single Escape byte {delay:g}s after the model request")
        deadline = sent_at + budget(sc)
        while (status := srv.turn_stream_status(request_number)) is StreamStatus.STARTED:
            if not alive() or time.monotonic() >= deadline:
                return "no model stream disconnect after Escape"
            pump(0.1)
        if status is not StreamStatus.DISCONNECTED:
            return "Escape did not interrupt the model stream; the complete response was sent"
        notes.append(f"Escape disconnected the model stream in {time.monotonic() - sent_at:.2f}s")
        return None

    try:
        quiet_for(1.5, 30)
        if not alive():
            return False, "exited at startup: " + screen_text()[-200:].replace("\n", " "), notes
        if resume:
            if not oracle:
                deadline = time.monotonic() + 20
                while alive() and time.monotonic() < deadline:
                    pump(0.2)
                    if "Mock scenario" in screen_text():
                        break
                if "Mock scenario" not in screen_text():
                    checkpoint("picker")
                    return False, "resume picker did not load the saved row", notes
                quiet_for(0.5, 2)
                checkpoint("picker")
                # The saved scenario is the oldest row whether or not the
                # app's fresh thread has appeared in the picker.
                send(b"\x1b[F")
                pump(0.3)
                send(b"\r")
            deadline = time.monotonic() + 15.0
            while alive() and time.monotonic() < deadline:
                pump(0.2)
                if sc.sentinel in re.sub(r"\s+", "", screen_text()):
                    quiet_for(0.5, 2)
                    break
            checkpoint("resumed")
            if sc.sentinel not in re.sub(r"\s+", "", screen_text()):
                return False, "resumed thread did not show the scenario answer", notes
        else:
            # Quiet is not ready: under load the app is silent for more than 1.5 s between its
            # terminal queries and the first paint, and a prompt typed before the composer is
            # painted never reaches it (the mock then serves 0 requests, "starting…" on screen).
            # Wait for the composer's placeholder; a build with another opening goes on after
            # the wait.
            deadline = time.monotonic() + READY_WAIT
            while alive() and time.monotonic() < deadline:
                if READY_MARK in re.sub(r"\s+", "", screen_text()):
                    quiet_for(0.5, 2)
                    break
                pump(0.2)
            checkpoint("ready")
        expected = 0
        for i, turn in enumerate(() if resume else sc.turns):
            expected += len(turn.responses)
            turn_started = time.time()
            pump(1.0)
            if NUDGE.search(screen_text(max(0, len(out) - 3000))):
                send(b"2")
                notes.append("answered the rate-limit nudge (keep the model)")
                pump(1.0)
            mark = len(out)
            send(turn.prompt.encode())
            pump(0.4)
            send(b"\r")
            if turn.esc_after is not None and (error := interrupt(expected, turn.esc_after)):
                return False, f"turn {i + 1}: {error}", notes
            end = time.time() + budget(sc)
            working = False
            while time.time() < end:
                pump(0.25)
                if not alive():
                    return False, f"exited during turn {i + 1}", notes
                tail = screen_text(max(mark, len(out) - 4000))
                if not working and "esc to interrupt" in tail:
                    checkpoint(f"turn-{i + 1}-working")
                    working = True
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
                checkpoint(f"timeout-{i + 1}")
                last = " | ".join(x.strip() for x in screen_text(max(0, len(out) - 3000)).splitlines() if x.strip())[-int(os.environ.get("MOCK_SCREEN_CHARS", "360")):]
                return False, f"turn {i + 1}: mock served {srv.turn_requests()} of {expected} model request(s) in {budget(sc):.0f}s; screen ends: {last}", notes
            checkpoint(f"turn-{i + 1}")
            if os.environ.get("MOCK_TURN_TIMES"):  # per-turn seconds, to cut a long scenario into balanced parts
                notes.append(f"turn {i + 1} ({turn.prompt}) {time.time() - turn_started:.1f}s")
        seen = sc.sentinel in re.sub(r"\s+", "", screen_text())
        notes.append(f"sentinel on screen: {'yes' if seen else 'no'}")
        if capture is not None and terminal.details:
            send(b"\x14")
            pump(1.0)
            checkpoint("resumed-detailed" if resume else "detailed")
            send(b"\x14")
            pump(0.3)
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
        if not seen and any(turn.esc_after is not None for turn in sc.turns):
            return False, "the answer after Escape did not appear", notes
        ok, detail, notes = drained(srv, sc, notes)
        return ok, detail, notes
    finally:
        try:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=3)
        finally:
            os.close(fd)


def spawn_pty(argv: list[str], cwd: Path, env: dict[str, str], width: int, height: int) -> tuple[subprocess.Popen, int]:
    master, slave = os.openpty()
    try:
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        # The mock HTTP server has worker threads. Acquire the controlling
        # terminal in a fresh process rather than running Python after forkpty.
        bootstrap = (
            "import fcntl, os, sys, termios\n"
            "fcntl.ioctl(0, termios.TIOCSCTTY, 0)\n"
            "os.execve(sys.argv[1], sys.argv[1:], os.environ)\n"
        )
        child = subprocess.Popen(
            [sys.executable, "-I", "-c", bootstrap, *argv],
            cwd=cwd, env=env, stdin=slave, stdout=slave, stderr=slave, start_new_session=True,
        )
    except Exception:
        os.close(master)
        raise
    finally:
        os.close(slave)
    return child, master


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


def run_one(name: str, label: str, binary: str, mode: str, show: bool = False, *, terminal: Terminal = Terminal(), capture_dir: Path | None = None, resume_with: str | None = None, piece: tuple[int, int, int] | None = None) -> Result:
    sc = scenarios.load(name)
    if piece is not None:  # (first turn, end turn, number): one session of a scenario cut at sc.cuts
        lo, hi, number = piece
        sc = dataclasses.replace(sc, turns=sc.turns[lo:hi], sentinel=sc.sentinel if hi >= len(sc.turns) else "")
        name = f"{name}#{number}"
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
                    capture = capture_dir / name / label if capture_dir is not None else None
                    ok, detail, notes = run_pty(binary, sc, root, srv, oracle=(label == "oracle"), terminal=terminal, capture=capture)
                    if ok and resume_with and capture_dir is not None:
                        rollouts = list((root / "codex-home" / "sessions").rglob("rollout-*.jsonl"))
                        if len(rollouts) != 1:
                            raise ValueError(f"expected one saved session, found {len(rollouts)}")
                        with rollouts[0].open() as rollout:
                            thread_id = json.loads(rollout.readline())["payload"]["id"]
                        for replay_label, replay_bin in (("oracle", binary), ("rtwk", resume_with)):
                            ok, detail, _ = run_pty(
                                replay_bin, sc, root, srv, oracle=(replay_label == "oracle"), terminal=terminal,
                                capture=capture_dir / name / replay_label, resume=thread_id,
                            )
                            if not ok:
                                detail = f"{replay_label} resume: {detail}"
                                break
                            notes.append(f"{replay_label} restored the saved turn")
            finally:
                if show:
                    dump(srv)
    except Exception as exc:  # a harness fault is a failure, with its type
        ok, detail, notes = False, f"{type(exc).__name__}: {exc}", []
    finally:
        shutil.rmtree(root, ignore_errors=True)
    return Result(name, label, mode, "PASS" if ok else "FAIL", time.time() - started, detail, notes)


# Scenarios that last longest, started first under --jobs (seconds in a land-check of 2026-11-21:
# router 151; error-context-window 66 and multi-agent 65 wait out their budget; the rest 6 to 19).
SLOW_FIRST = ("router", "error-context-window", "multi-agent")


def terminal_size(value: str) -> tuple[int, int]:
    match = re.fullmatch(r"(\d+)x(\d+)", value)
    if match is None:
        raise argparse.ArgumentTypeError("expected terminal WIDTHxHEIGHT")
    width, height = map(int, match.groups())
    if not 1 <= width <= 1000 or not 1 <= height <= 1000:
        raise argparse.ArgumentTypeError("terminal width and height must be between 1 and 1000")
    return width, height


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", help="comma-separated scenario names")
    ap.add_argument("--bin", choices=["rtwk", "oracle"], action="append")
    ap.add_argument("--pty", action="store_true", help="drive the oracle in a pty instead of exec")
    ap.add_argument("--size", type=terminal_size, default=(120, 40), metavar="WIDTHxHEIGHT")
    ap.add_argument("--palette", choices=["dark", "light"], default="dark", help="default PTY colors")
    ap.add_argument("--capture-dir", type=Path, help="save named PTY checkpoints under DIR/scenario/binary")
    ap.add_argument("--capture-resume", action="store_true", help="add a shared saved-session checkpoint")
    ap.add_argument("--capture-details", action="store_true", help="also capture after Ctrl+T")
    ap.add_argument("--jobs", type=int, default=int(os.environ.get("CHECK_JOBS", "1")), metavar="N", help="scenarios run at once (default 1)")
    ap.add_argument("--short-budget", metavar="NAME[,NAME]", help=f"run these scenarios with {SHORT_BASE:.0f} s of slack per turn instead of {SLOW_BASE:.0f} (known reds: they only wait out the budget, a green one never waits; land-check.sh passes the known-red.txt names)")
    ap.add_argument("--list-bins", action="store_true")
    ap.add_argument("--json", help="write the results to this file")
    ap.add_argument("--dump", action="store_true", help="print the tools and tool outputs of every model request")
    args = ap.parse_args()
    SHORT.update(n for n in (args.short_budget or "").split(",") if n)
    found = {"rtwk": bins.rtwk_bin(), "oracle": bins.oracle_bin()}
    if args.capture_resume and (args.capture_dir is None or args.bin or not all(found.values())):
        ap.error("--capture-resume requires --capture-dir and both binaries")
    if args.capture_details and args.capture_dir is None:
        ap.error("--capture-details requires --capture-dir")
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
    terminal = Terminal(args.size, args.palette == "light", args.capture_details)
    order = ["rtwk", "oracle"] if args.capture_resume else ["oracle", "rtwk"]
    jobs: list[tuple[str, str, str, str, str | None, tuple[int, int, int] | None]] = []  # (name, label, path, mode, resume_with, piece)
    for label in args.bin or order:
        path = found[label]
        if not path:
            continue
        for name in names:
            mode = "pty" if (label == "rtwk" or args.pty or args.capture_dir or scenarios.load(name).pty_only) else "exec"
            resume_with = found["rtwk"] if args.capture_resume and label == "oracle" else None
            cuts = scenarios.load(name).cuts if args.jobs > 1 and not args.capture_dir and not args.capture_resume else ()
            if cuts:  # independent sessions side by side instead of one long one
                bounds = [0, *cuts, len(scenarios.load(name).turns)]
                jobs += [(name, label, path, mode, resume_with, (lo, hi, n)) for n, (lo, hi) in enumerate(zip(bounds, bounds[1:]), 1)]
            else:
                jobs.append((name, label, path, mode, resume_with, None))

    def show(r: Result) -> None:
        results.append(r)
        extra = f"  {r.detail}" if r.detail else ""
        print(f"{r.status}  {r.binary:6} {r.mode:4} {r.scenario:16} {r.seconds:5.1f}s{extra}  [{'; '.join(dict.fromkeys(r.notes))}]", flush=True)

    if args.jobs > 1 and len(jobs) > 1 and not args.capture_resume:
        # The slow ones first (the router is one long session, the two build-dependent reds wait
        # out their 60 s budget): the run then lasts as long as the longest, not the sum.
        slow = {name: i for i, name in enumerate(SLOW_FIRST)}
        jobs.sort(key=lambda j: slow.get(j[0], len(slow)))
        with concurrent.futures.ProcessPoolExecutor(max_workers=args.jobs, mp_context=multiprocessing.get_context("fork")) as pool:
            futures = [
                pool.submit(run_one, name, label, path, mode, args.dump, terminal=terminal, capture_dir=args.capture_dir, resume_with=resume_with, piece=piece)
                for name, label, path, mode, resume_with, piece in jobs
            ]
            for future in concurrent.futures.as_completed(futures):
                show(future.result())
    else:
        for name, label, path, mode, resume_with, piece in jobs:
            show(run_one(name, label, path, mode, args.dump, terminal=terminal, capture_dir=args.capture_dir, resume_with=resume_with, piece=piece))
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
