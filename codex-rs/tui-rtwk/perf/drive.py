#!/usr/bin/env python3
"""Pty A/B driver for the Codex TUI perf comparison (stdlib only).

Forks a binary in a real pty at a fixed size, answers the terminal's queries
(vt.py), feeds every output chunk to a text VT model, and records per run:
keystroke-to-visible, frames (DEC 2026 pairs, else read bursts), bytes, CPU and
RSS (proc_pidinfo samples every 100 ms, wait4 rusage at the end), time to first
byte / ready, the final screen, and an asciicast v2 of the run.
Both binaries talk to the SAME kind of local fake model (perf/mock.py, Codex's own
MockResponsesServer through tools/rtwk-port/mock-model), one server and one fresh
CODEX_HOME per run, analytics off, no network, no real model.

    python3 -I drive.py --a oracle=PATH [--b rtwk=PATH] --scenarios S1,S2,S5,S9,S10,S11 \
        --n 15 --out results/<name> [--measure-only] [--wait-quiet SECONDS]

Order: a preflight launch of each binary (discarded, reported in meta.json), then rounds;
in each round every scenario runs once per binary, the binary order flips each round
(A B, B A, ...). Exit 75 and no data when the 1-minute load is above 1.5 x CPUs
(rtwk's gate rule) unless --measure-only; load before/after is kept for every run.
"""

from __future__ import annotations

import argparse
import ctypes
import datetime as dt
import fcntl
import gzip
import hashlib
import json
import os
import platform
import pty
import random
import re
import resource
import select
import shutil
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
import warnings
from pathlib import Path

warnings.filterwarnings("ignore", category=DeprecationWarning)
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from vt import Term  # noqa: E402

SYNC_HOLD = 0.150  # a DEC 2026 update held open longer than this counts as visible (rtwk's gate rule)
FRAME_BURST_GAP = 0.002  # without DEC 2026: chunks closer than this are one frame
EX_LOAD = 75


# ---------------------------------------------------------------- process info (macOS)
class _TaskInfo(ctypes.Structure):
    _fields_ = [
        ("virtual_size", ctypes.c_uint64),
        ("resident_size", ctypes.c_uint64),
        ("total_user", ctypes.c_uint64),
        ("total_system", ctypes.c_uint64),
        ("threads_user", ctypes.c_uint64),
        ("threads_system", ctypes.c_uint64),
        ("policy", ctypes.c_int32),
        ("faults", ctypes.c_int32),
        ("pageins", ctypes.c_int32),
        ("cow_faults", ctypes.c_int32),
        ("messages_sent", ctypes.c_int32),
        ("messages_received", ctypes.c_int32),
        ("syscalls_mach", ctypes.c_int32),
        ("syscalls_unix", ctypes.c_int32),
        ("csw", ctypes.c_int32),
        ("threadnum", ctypes.c_int32),
        ("numrunning", ctypes.c_int32),
        ("priority", ctypes.c_int32),
    ]


class ProcInfo:
    """CPU seconds and RSS bytes of a pid. libproc on macOS (ns scaled by the mach timebase), `ps` elsewhere."""

    def __init__(self) -> None:
        self.lib = None
        self.scale = 1.0
        if sys.platform == "darwin":
            try:
                self.lib = ctypes.CDLL("/usr/lib/libproc.dylib")
                class TB(ctypes.Structure):
                    _fields_ = [("numer", ctypes.c_uint32), ("denom", ctypes.c_uint32)]
                tb = TB()
                ctypes.CDLL("/usr/lib/libSystem.dylib").mach_timebase_info(ctypes.byref(tb))
                self.scale = tb.numer / tb.denom / 1e9
            except OSError:
                self.lib = None

    def sample(self, pid: int) -> tuple[float, int] | None:
        if self.lib is not None:
            info = _TaskInfo()
            n = self.lib.proc_pidinfo(pid, 4, 0, ctypes.byref(info), ctypes.sizeof(info))
            if n != ctypes.sizeof(info):
                return None
            return (info.total_user + info.total_system) * self.scale, int(info.resident_size)
        try:
            out = subprocess.run(["ps", "-o", "cputime=,rss=", "-p", str(pid)], capture_output=True, text=True).stdout.split()
            h = out[0].replace("-", ":").split(":")
            secs = 0.0
            for part in h:
                secs = secs * 60 + float(part)
            return secs, int(out[1]) * 1024
        except (IndexError, ValueError):
            return None


PROC = ProcInfo()


# ---------------------------------------------------------------- a session
class Session:
    def __init__(self, argv: list[str], env: dict[str, str], cwd: str, rows: int, cols: int):
        self.argv, self.rows, self.cols = argv, rows, cols
        self.term = Term(rows, cols)
        self.term.on_sync_end = self._sync_end
        self.cond = threading.Condition()
        self.chunks: list[tuple[float, int]] = []
        self.frames: list[tuple[float, int]] = []  # (t, bytes) per DEC 2026 pair
        self.frame_bytes = 0
        self.cast: list[tuple[float, str]] = []
        self.cast_dec = __import__("codecs").getincrementaldecoder("utf-8")("replace")
        self.watches: dict[str, object] = {}
        self.hits: dict[str, float] = {}
        self.probes: list[dict] = []
        self.cpu: list[tuple[float, float, int]] = []  # (t, cpu_s, rss)
        self.first_byte: float | None = None
        self.eof = False
        self.stop_sampler = False
        self.t0 = time.monotonic()  # before the fork: the child may run before the parent is scheduled again
        self.pid, self.fd = pty.fork()
        if self.pid == 0:  # child
            try:
                fcntl.ioctl(0, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
                os.chdir(cwd)
                os.execvpe(argv[0], argv, env)
            finally:
                os._exit(127)
        self.reader = threading.Thread(target=self._read_loop, daemon=True)
        self.sampler = threading.Thread(target=self._sample_loop, daemon=True)
        self.reader.start()
        self.sampler.start()

    # -- time base: seconds since fork
    def now(self) -> float:
        return time.monotonic() - self.t0

    def _visible(self, now: float) -> bool:
        return not self.term.sync or (now - (self.term.sync_since)) > SYNC_HOLD

    def _sync_end(self, now: float) -> None:  # called inside feed(), lock held
        self.frames.append((now, self.frame_bytes))
        self.frame_bytes = 0
        self._evaluate(now)

    def _evaluate(self, now: float) -> None:
        for name, pred in list(self.watches.items()):
            if name not in self.hits and pred(self.term):
                self.hits[name] = now
        for p in self.probes:
            if p["visible"] is None and self.term.count(p["needle"]) > p["before"]:
                p["visible"] = now

    def _read_loop(self) -> None:
        fd = self.fd
        while True:
            try:
                r, _, _ = select.select([fd], [], [], 0.25)
                if not r:
                    if self.stop_sampler:
                        break
                    continue
                data = os.read(fd, 1 << 16)
            except OSError:
                data = b""
            if not data:
                with self.cond:
                    self.eof = True
                    self.cond.notify_all()
                return
            t = time.monotonic() - self.t0
            with self.cond:
                if self.first_byte is None:
                    self.first_byte = t
                self.chunks.append((t, len(data)))
                self.frame_bytes += len(data)
                self.cast.append((t, self.cast_dec.decode(data)))
                self.term.feed(data, t)
                if self.term.out:
                    try:
                        os.write(fd, bytes(self.term.out))
                    except OSError:
                        pass
                    self.term.out.clear()
                if self._visible(t):
                    self._evaluate(t)
                for p in self.probes:
                    if p["fb"] is None and t >= p["t"]:
                        p["fb"] = t
                self.cond.notify_all()

    def _sample_loop(self) -> None:
        while not self.stop_sampler:
            s = PROC.sample(self.pid)
            if s:
                self.cpu.append((self.now(), s[0], s[1]))
            time.sleep(0.1)

    # -- driving
    def watch(self, name: str, pred) -> None:
        with self.cond:
            self.watches[name] = pred
            self.hits.pop(name, None)
            if pred(self.term):
                self.hits[name] = self.now()

    def wait_hit(self, name: str, timeout: float) -> float | None:
        end = time.monotonic() + timeout
        with self.cond:
            while name not in self.hits:
                left = end - time.monotonic()
                if left <= 0 or self.eof:
                    return self.hits.get(name)
                self.cond.wait(min(left, 0.05))
            return self.hits[name]

    def send(self, data: bytes) -> float:
        t = self.now()
        os.write(self.fd, data)
        return t

    def resize(self, rows: int, cols: int) -> float:
        """Resize the pty like a terminal window: TIOCSWINSZ + SIGWINCH, and the in-band
        report when the app has set mode 2048 (the VT model queues it in `term.out`)."""
        with self.cond:
            fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
            self.term.resize(rows, cols)
            if self.term.out:
                os.write(self.fd, bytes(self.term.out))
                self.term.out.clear()
        t = self.now()
        try:
            os.kill(self.pid, signal.SIGWINCH)
        except ProcessLookupError:
            pass
        return t

    def type_text(self, text: str, gap: float = 0.03) -> None:
        for ch in text:
            self.send(ch.encode())
            time.sleep(gap)

    def key_probe(self, ch: str, needle: str) -> dict:
        with self.cond:
            before = self.term.count(needle)
            p = {"needle": needle, "before": before, "t": 0.0, "visible": None, "fb": None, "ch": ch}
            p["t"] = self.now()
            self.probes.append(p)
        os.write(self.fd, ch.encode())
        return p

    def sleep_until(self, t: float) -> None:
        d = t - self.now()
        if d > 0:
            time.sleep(d)

    def screen(self) -> list[str]:
        with self.cond:
            return self.term.screen_rows()

    def cpu_at(self, t: float) -> tuple[float, int] | None:
        """CPU seconds and RSS at time t, interpolated between samples."""
        s = self.cpu
        if not s:
            return None
        prev = s[0]
        for cur in s:
            if cur[0] >= t:
                if cur[0] == prev[0]:
                    return cur[1], cur[2]
                f = (t - prev[0]) / (cur[0] - prev[0]) if t > prev[0] else 0.0
                return prev[1] + f * (cur[1] - prev[1]), cur[2]
            prev = cur
        return s[-1][1], s[-1][2]

    def finish(self) -> dict:
        """Ends the child (SIGTERM, then SIGKILL) and returns wait4's rusage."""
        self.stop_sampler = True
        try:
            os.kill(self.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        ru = None
        end = time.monotonic() + 3.0
        while time.monotonic() < end:
            pid, status, ru = os.wait4(self.pid, os.WNOHANG)
            if pid:
                break
            time.sleep(0.02)
        else:
            try:
                os.kill(self.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            pid, status, ru = os.wait4(self.pid, 0)
        self.reader.join(timeout=1.0)
        try:
            os.close(self.fd)
        except OSError:
            pass
        return {"cpu_s": ru.ru_utime + ru.ru_stime, "maxrss": ru.ru_maxrss if sys.platform == "darwin" else ru.ru_maxrss * 1024}


# ---------------------------------------------------------------- binaries and scenarios
def argv_for(kind: str, binary: str, scenario: dict, work: str) -> list[str] | None:
    sub = scenario.get("subcommand")
    if kind == "oracle":
        base = [binary, "-c", "analytics.enabled=false"]
        if sub:
            return base + sub
        return base + ["-C", work]
    if kind == "rtwk":
        if sub:
            if not rtwk_has_subcommand(binary, sub[0]):
                return None
            return [binary] + list(sub)
        # --no-daemon: the embedded server, as the oracle's run (a chat starts the shared
        # background server by default, which needs a packaged CLI)
        return [binary, "--no-daemon"]
    raise SystemExit(f"unknown binary kind {kind!r} (oracle or rtwk)")


_SUB_CACHE: dict[tuple[str, str], bool] = {}


def rtwk_has_subcommand(binary: str, sub: str) -> bool:
    key = (binary, sub)
    if key not in _SUB_CACHE:
        try:
            r = subprocess.run([binary, sub, "--help"], capture_output=True, text=True, timeout=10, stdin=subprocess.DEVNULL)
            _SUB_CACHE[key] = r.returncode == 0 and "unrecognized" not in (r.stderr + r.stdout).lower()
        except (OSError, subprocess.TimeoutExpired):
            _SUB_CACHE[key] = False
    return _SUB_CACHE[key]


def load_scenarios() -> dict[str, dict]:
    out = {}
    for p in sorted((HERE / "scenarios").glob("*.json")):
        sc = json.loads(p.read_text())
        out[sc["id"]] = sc
    return out


ORACLE_BIN = Path.home() / ".cache" / "codexcli" / "oracle-bin" / "codex"


def seed_root(scenario: dict) -> Path:
    # fixed: the state DB and the rollout hold absolute paths of this root
    return Path(os.path.realpath("/tmp")) / f"perf-seed-{scenario['id']}"


def ensure_seed(scenario: dict) -> Path:
    """A CODEX_HOME + work dir with one long session (scenario seed.turns turns), built once through the
    oracle's `codex exec` / `exec resume --last` against the fake model, kept as <root>.tmpl."""
    root = seed_root(scenario)
    tmpl = root.with_name(root.name + ".tmpl")
    if (tmpl / "ok").exists():
        return tmpl
    shutil.rmtree(root, ignore_errors=True)
    shutil.rmtree(tmpl, ignore_errors=True)
    mock = MockProc(scenario, root)
    try:
        env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "LANG", "LC_ALL", "TMPDIR", "USER")}
        env.update({"CODEX_HOME": mock.info["home"], "OPENAI_API_KEY": "dummy"})
        base = [str(ORACLE_BIN), "-c", "analytics.enabled=false", "exec", "--skip-git-repo-check"]
        for i in range(scenario["seed"]["turns"]):
            argv = base + ([] if i == 0 else ["resume", "--last"]) + [f"turn {i}"]
            subprocess.run(argv, cwd=mock.info["work"], env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=60)
    finally:
        mock.stop()
    shutil.copytree(root, tmpl)
    (tmpl / "ok").write_text("1")
    return tmpl


class MockProc:
    def __init__(self, scenario: dict, root: Path):
        m = scenario.get("mock", {})
        cmd = [sys.executable, "-I", str(HERE / "mock.py"), "--root", str(root),
               "--deltas", str(m.get("deltas", 100)), "--pace", str(m.get("pace", 0.01)),
               "--sentinel", m.get("sentinel", "END-perf"), "--words", str(m.get("words", 6)),
               "--begin", m.get("begin", "")]
        self.p = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        line = self.p.stdout.readline()
        if not line:
            raise RuntimeError("the mock model did not start")
        self.info = json.loads(line)

    def stop(self) -> None:
        self.p.terminate()
        try:
            self.p.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.p.kill()


def ready_pred(kind: str, scenario: dict):
    rx = re.compile(scenario.get("ready", {}).get(kind) or scenario.get("ready", {}).get("any") or r"›")

    def pred(term: Term) -> bool:
        return any(rx.search(term.row_text(i)) for i in range(term.rows))

    return pred


def text_pred(s: str):
    def pred(term: Term) -> bool:
        return term.count(s) > 0

    return pred


def percentile(xs: list[float], q: float) -> float:
    s = sorted(xs)
    if not s:
        return float("nan")
    k = (len(s) - 1) * q
    f = int(k)
    c = min(f + 1, len(s) - 1)
    return s[f] + (s[c] - s[f]) * (k - f)


def window_stats(sess: Session, t0: float, t1: float) -> dict:
    """Frames, bytes, CPU and RSS over [t0, t1] seconds since fork."""
    dur = max(t1 - t0, 1e-9)
    nbytes = sum(n for t, n in sess.chunks if t0 <= t <= t1)
    if sess.term.sync_seen:
        mode = "sync"
        frames = [(t, b) for t, b in sess.frames if t0 <= t <= t1]
    else:
        mode = "burst"
        frames, cur_b, last_t, start = [], 0, None, None
        for t, n in sess.chunks:
            if t < t0 or t > t1:
                continue
            if last_t is not None and t - last_t > FRAME_BURST_GAP:
                frames.append((last_t, cur_b))
                cur_b = 0
            cur_b += n
            last_t = t
        if cur_b:
            frames.append((last_t, cur_b))
    a, b = sess.cpu_at(t0), sess.cpu_at(t1)
    cpu_s = (b[0] - a[0]) if a and b else None
    rss = [r for t, c, r in sess.cpu if t0 <= t <= t1]
    ivals = [(frames[i][0] - frames[i - 1][0]) * 1000 for i in range(1, len(frames))]
    return {
        "t0_ms": round(t0 * 1000, 1),
        "t1_ms": round(t1 * 1000, 1),
        "dur_ms": round(dur * 1000, 1),
        "frame_mode": mode,
        "frames": len(frames),
        "frame_list": [[round(t * 1000, 2), b] for t, b in frames],
        "frame_interval_ms": [round(x, 3) for x in ivals],
        "bytes": nbytes,
        "bytes_per_s": nbytes / dur,
        "frames_per_s": len(frames) / dur,
        "cpu_s": cpu_s,
        "cpu_pct": 100 * cpu_s / dur if cpu_s is not None else None,
        "rss_start": a[1] if a else None,
        "rss_end": b[1] if b else None,
        "rss_max": max(rss) if rss else None,
    }


# ---------------------------------------------------------------- scenario runners
KEY_ALPHABET = "JQXZKVWY"


def run_startup(sess: Session, scenario: dict, kind: str, rec: dict) -> None:
    sess.watch("ready", ready_pred(kind, scenario))
    t = sess.wait_hit("ready", scenario.get("timeout", 30))
    rec["t_first_byte_ms"] = round(sess.first_byte * 1000, 1) if sess.first_byte is not None else None
    rec["t_ready_ms"] = round(t * 1000, 1) if t is not None else None
    if t is None:
        rec["errors"].append("ready text never seen")
        return
    hold = scenario.get("hold", 1.0)
    sess.sleep_until(t + hold)
    rec["after_ready"] = window_stats(sess, t, t + hold)
    a = sess.cpu_at(t + hold)
    rec["cpu_s_to_ready_hold"] = a[0] if a else None
    rec["rss_at_ready_hold"] = a[1] if a else None


def submit_prompt(sess: Session, scenario: dict, rec: dict) -> bool:
    """Types the prompt and Enter, returns True when the stream's begin token showed."""
    sess.type_text(scenario.get("prompt", "go"), 0.04)
    time.sleep(0.4)  # outside any paste burst before Enter
    rec["t_submit_ms"] = round(sess.send(b"\r") * 1000, 1)
    m = scenario.get("mock", {})
    if scenario.get("working"):  # the turn stays open: no begin token, give the status indicator a moment
        time.sleep(1.0)
        return True
    if m.get("begin"):
        sess.watch("begin", text_pred(m["begin"]))
        t = sess.wait_hit("begin", scenario.get("timeout", 30))
        if t is None:
            rec["errors"].append("stream begin token never seen")
            return False
        rec["t_begin_ms"] = round(t * 1000, 1)
    return True


def run_stream(sess: Session, scenario: dict, kind: str, rec: dict) -> None:
    run_startup(sess, scenario, kind, rec)
    if rec["errors"]:
        return
    if not submit_prompt(sess, scenario, rec):
        return
    sentinel = scenario["mock"]["sentinel"]
    sess.watch("end", text_pred(sentinel))
    t_begin = sess.hits["begin"]
    typing = scenario.get("typing")
    if typing:
        run_typing(sess, scenario, rec, t_begin)
    t_end = sess.wait_hit("end", scenario.get("stream_timeout", 90))
    if t_end is None:
        rec["errors"].append("sentinel never seen")
        t_end = sess.now()
    rec["sentinel_seen"] = "end" in sess.hits
    rec["stream"] = window_stats(sess, t_begin, t_end)
    time.sleep(scenario.get("settle", 1.0))
    rec["settle_screen_ms"] = None


def run_typing(sess: Session, scenario: dict, rec: dict, t_begin: float) -> None:
    typing = scenario["typing"]
    sess.sleep_until(t_begin + typing.get("lead", 2.0))
    keys = []
    k = 0
    for phase in typing["phases"]:
        period = 1.0 / phase["rate"]
        start = sess.now() + 0.2
        typed = ""
        for i in range(phase["keys"]):
            sess.sleep_until(start + i * period)
            ch = KEY_ALPHABET[(k * 5 + k // 3 + i) % len(KEY_ALPHABET)]
            typed += ch
            needle = typed[-6:]
            p = sess.key_probe(ch, needle)
            p["phase"] = phase["name"]
            keys.append(p)
            k += 1
        time.sleep(0.5)
        sess.send(b"\x15")  # Ctrl+U clears the composer line
        time.sleep(0.3)
    time.sleep(0.2)
    out = []
    for p in keys:
        out.append({
            "phase": p["phase"],
            "lat_ms": round((p["visible"] - p["t"]) * 1000, 3) if p["visible"] is not None else None,
            "fb_ms": round((p["fb"] - p["t"]) * 1000, 3) if p["fb"] is not None else None,
        })
    rec["keys"] = out
    rec["keys_unseen"] = sum(1 for x in out if x["lat_ms"] is None)


def run_idle(sess: Session, scenario: dict, kind: str, rec: dict) -> None:
    run_startup(sess, scenario, kind, rec)
    if rec["errors"]:
        return
    if scenario.get("mock"):
        if not submit_prompt(sess, scenario, rec):
            return
        sentinel = scenario["mock"].get("sentinel")
        if sentinel and not scenario.get("working"):
            sess.watch("end", text_pred(sentinel))
            if sess.wait_hit("end", scenario.get("stream_timeout", 60)) is None:
                rec["errors"].append("sentinel never seen")
                return
    time.sleep(scenario.get("settle", 3.0))
    t0 = sess.now()
    sess.sleep_until(t0 + scenario.get("window", 30.0))
    t1 = sess.now()
    rec["idle"] = window_stats(sess, t0, t1)
    rec["idle_screen_changes"] = len([1 for t, n in sess.chunks if t0 <= t <= t1])


def run_agents(sess: Session, scenario: dict, kind: str, rec: dict) -> None:
    run_startup(sess, scenario, kind, rec)
    if rec["errors"]:
        return
    t0 = sess.now()
    keys = []
    for i in range(scenario.get("downs", 10)):
        sess.sleep_until(t0 + 0.5 + i * 0.2)
        p = sess.key_probe("\x1b[B", "\x00never")  # latency is measured by screen change below
        keys.append(p)
    time.sleep(1.0)
    t1 = sess.now()
    rec["interaction"] = window_stats(sess, t0, t1)
    sess.sleep_until(t1 + scenario.get("window", 10.0))
    rec["idle"] = window_stats(sess, t1 + 0.5, sess.now())


def run_open(sess: Session, scenario: dict, kind: str, rec: dict) -> None:
    """Command center up (the seeded session listed) -> Enter -> the chat shows the transcript's last text."""
    run_startup(sess, scenario, kind, rec)
    if rec["errors"]:
        return
    sess.watch("list", text_pred(scenario["row"]))
    if sess.wait_hit("list", scenario.get("timeout", 30)) is None:
        rec["errors"].append("the seeded session never showed in the list")
        return
    time.sleep(scenario.get("settle", 1.0))
    sess.watch("chat", text_pred(scenario["open_ready"]))
    sess.watch("transcript", text_pred(scenario["transcript"]))
    # First frame of the screen change: the command center is gone from the screen (the session page,
    # loading or ready, is what shows).
    seen: list[list[str]] = []

    def first_pred(term: Term) -> bool:
        hit = term.count(scenario["ready"]["any"]) == 0 and any(r.strip() for r in term.screen_rows())
        if hit and not seen:
            seen.append(term.screen_rows())
        return hit

    sess.watch("first", first_pred)
    t0 = sess.send(b"\r")
    t = sess.wait_hit("chat", scenario.get("timeout", 30))
    tf = sess.hits.get("first")
    rec["t_first_ms"] = round((tf - t0) * 1000, 1) if tf is not None else None
    rec["first_rows"] = [r.rstrip() for r in seen[0]] if seen else None
    rec["t_open_ms"] = round((t - t0) * 1000, 1) if t is not None else None
    if t is None:
        rec["errors"].append("the chat page never showed (composer)")
        return
    # the transcript's last text: the resumed session's history (thread/turns/list into the transcript)
    tt = sess.wait_hit("transcript", scenario.get("transcript_wait", 2.0))
    rec["t_transcript_ms"] = round((tt - t0) * 1000, 1) if tt is not None else None
    hold = scenario.get("hold", 1.0)
    sess.sleep_until(t + hold)
    rec["after_open"] = window_stats(sess, t, t + hold)
    a = sess.cpu_at(t + hold)
    rec["rss_after_open"] = a[1] if a else None
    rec["cpu_s_to_open"] = a[0] if a else None


def run_resize(sess: Session, sc: dict, kind: str, rec: dict) -> None:
    """Stream an answer, then a resize storm (40 random sizes, `gap` apart), then a rest."""
    run_startup(sess, sc, kind, rec)
    if rec["errors"] or not submit_prompt(sess, sc, rec):
        return
    sess.watch("end", text_pred(sc["mock"]["sentinel"]))
    if sess.wait_hit("end", sc.get("stream_timeout", 60)) is None:
        rec["errors"].append("sentinel never seen")
        return
    time.sleep(1.0)
    rng = random.Random(7)
    sizes = [(rng.randint(80, 200), rng.randint(24, 60)) for _ in range(39)] + [tuple(sc["size"])]
    rec["inband_resize"] = sess.term.modes[2048]
    t0 = sess.now()
    last = t0
    for k, (c, r) in enumerate(sizes):
        sess.sleep_until(t0 + k * sc.get("gap", 0.05))
        last = sess.resize(r, c)
    sess.sleep_until(last + sc.get("rest", 3.0))
    rec["storm"] = window_stats(sess, t0, sess.now())
    after = [t for t, _ in sess.chunks if t > last]
    rec["settle_ms"] = round((after[-1] - last) * 1000, 1) if after else 0.0
    rec["storm_screen_tail"] = sess.screen()[-3:]


RUNNERS = {"resize": run_resize, "open": run_open, "stream": run_stream, "startup": run_startup, "idle": run_idle, "agents": run_agents}


def run_once(label: str, kind: str, binary: str, scenario: dict, rnd: int, out: Path, cast_dir: Path | None, load_gate: float) -> dict | None:
    if scenario.get("seed"):
        tmpl = ensure_seed(scenario)
        root = seed_root(scenario)
        shutil.rmtree(root, ignore_errors=True)
        shutil.copytree(tmpl, root)
        (root / "ok").unlink()
    else:
        root = Path(tempfile.mkdtemp(prefix="perf-run-"))
    rec: dict = {
        "scenario": scenario["id"], "bin": label, "kind": kind, "round": rnd,
        "size": scenario["size"], "started": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "load_before": os.getloadavg()[0], "errors": [],
    }
    mock = None
    try:
        mock = MockProc(scenario, root)
        work = mock.info["work"]
        argv = argv_for(kind, binary, scenario, work)
        if argv is None:
            return None
        rec["argv"] = [a if a != work else "<work>" for a in argv]
        env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "LANG", "LC_ALL", "TMPDIR", "USER", "SHELL")}
        env.update({"CODEX_HOME": mock.info["home"], "OPENAI_API_KEY": "dummy", "TERM": "xterm-256color",
                    "COLORTERM": "truecolor", "LINES": str(scenario["size"][1]), "COLUMNS": str(scenario["size"][0])})
        env.pop("NO_COLOR", None)
        cols, rows = scenario["size"]
        if scenario.get("warm"):
            # Warm home: one discarded launch first creates the state DB and the rest of CODEX_HOME,
            # so the measured launch is the second start of the same home (S9 fresh is the first).
            prime = Session(argv, env, work, rows, cols)
            try:
                prime.watch("ready", ready_pred(kind, scenario))
                prime.wait_hit("ready", scenario.get("timeout", 30))
                time.sleep(scenario.get("prime_hold", 1.0))
            finally:
                prime.finish()
        sess = Session(argv, env, work, rows, cols)
        try:
            RUNNERS[scenario["kind"]](sess, scenario, kind, rec)
        except Exception as e:  # keep the run, flagged
            rec["errors"].append(f"{type(e).__name__}: {e}")
        rec["final_rows"] = sess.screen()
        rec["final_hash"] = hashlib.sha1("\n".join(rec["final_rows"]).encode()).hexdigest()[:12]
        rec["alt_screen"] = sess.term.alt_entries > 0
        rec["sync_2026"] = sess.term.sync_seen
        rec["full_repaints"] = sess.term.repaints
        cpu_series = [[round(t * 1000), round(c, 4), r] for t, c, r in sess.cpu]
        rec["cpu_series"] = cpu_series
        ru = sess.finish()
        rec["rusage"] = ru
        rec["total_bytes"] = sum(n for _, n in sess.chunks)
        rec["duration_ms"] = round(sess.now() * 1000)
        if cast_dir is not None:
            cast_dir.mkdir(parents=True, exist_ok=True)
            name = f"{scenario['id']}-{label}-r{rnd:02d}.cast.gz"
            with gzip.open(cast_dir / name, "wt") as f:
                f.write(json.dumps({"version": 2, "width": cols, "height": rows, "timestamp": int(time.time()),
                                    "env": {"TERM": "xterm-256color"}, "title": f"{scenario['id']} {label} round {rnd}"}) + "\n")
                for t, s in sess.cast:
                    f.write(json.dumps([round(t, 4), "o", s]) + "\n")
            rec["cast"] = name
    except Exception as e:
        rec["errors"].append(f"{type(e).__name__}: {e}")
    finally:
        if mock:
            mock.stop()
        shutil.rmtree(root, ignore_errors=True)
    rec["load_after"] = os.getloadavg()[0]
    rec["ok"] = not rec["errors"]
    return rec


# ---------------------------------------------------------------- main
def machine_info() -> dict:
    def sh(*a: str) -> str:
        try:
            return subprocess.run(list(a), capture_output=True, text=True, timeout=10).stdout.strip()
        except (OSError, subprocess.TimeoutExpired):
            return ""
    return {
        "host_model": sh("sysctl", "-n", "hw.model"), "cpus": os.cpu_count(), "os": platform.platform(),
        "mem_gb": round(int(sh("sysctl", "-n", "hw.memsize") or 0) / 2**30, 1),
        "power": sh("pmset", "-g", "batt").splitlines()[0] if sh("pmset", "-g", "batt") else "",
        "python": platform.python_version(), "rustc": sh("rustc", "-V"),
    }


def bin_info(kind: str, path: str) -> dict:
    p = Path(path)
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for blk in iter(lambda: f.read(1 << 20), b""):
            h.update(blk)
    info = {"path": str(p), "sha256": h.hexdigest()[:16], "size_mb": round(p.stat().st_size / 2**20, 1)}
    if kind == "oracle":
        v = p.parent / "VERSION"
        if v.exists():
            info["version_file"] = v.read_text().strip().splitlines()[:3]
    try:
        info["version"] = subprocess.run([path, "--version"], capture_output=True, text=True, timeout=10).stdout.strip()
    except (OSError, subprocess.TimeoutExpired):
        pass
    return info


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--a", required=True, help="kind=PATH, kind is oracle or rtwk; the first binary (label A)")
    ap.add_argument("--b", help="kind=PATH, the second binary (label B)")
    ap.add_argument("--scenarios", default="S1,S2,S5,S9,S10,S11")
    ap.add_argument("--n", type=int, default=15)
    ap.add_argument("--out", required=True)
    ap.add_argument("--measure-only", action="store_true", help="take data under load too (load is recorded per run)")
    ap.add_argument("--wait-quiet", type=float, default=0, help="seconds to wait for the load gate before giving up")
    ap.add_argument("--no-casts", action="store_true")
    ap.add_argument("--label", help="free label of this run set (kept in meta.json)")
    args = ap.parse_args()

    scen = load_scenarios()
    ids = [s for s in args.scenarios.split(",") if s]
    for s in ids:
        if s not in scen:
            raise SystemExit(f"unknown scenario {s}; have {', '.join(scen)}")
    bins = []
    for spec, tag in ((args.a, "A"), (args.b, "B")):
        if not spec:
            continue
        kind, _, path = spec.partition("=")
        if not path or not Path(path).is_file():
            raise SystemExit(f"--{tag.lower()}: {spec!r} is not kind=PATH of a file")
        bins.append({"tag": tag, "kind": kind, "path": path, "label": f"{tag}:{kind}"})
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    gate = 1.5 * (os.cpu_count() or 1)
    end = time.monotonic() + args.wait_quiet
    while os.getloadavg()[0] > gate and time.monotonic() < end:
        time.sleep(10)
    load0 = os.getloadavg()[0]
    if load0 > gate and not args.measure_only:
        print(f"load {load0:.1f} is above the gate {gate:.1f} (1.5 x CPUs): no data; --measure-only takes it anyway", file=sys.stderr)
        return EX_LOAD
    print(f"load {load0:.1f} (gate {gate:.1f}){' - measuring under load' if load0 > gate else ''}", file=sys.stderr)

    meta = {
        "label": args.label, "started": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
        "n": args.n, "scenarios": ids, "machine": machine_info(), "load_start": load0, "load_gate": gate,
        "measure_only": args.measure_only,
        "binaries": {b["label"]: {**bin_info(b["kind"], b["path"]), "kind": b["kind"]} for b in bins},
        "scenario_defs": {s: scen[s] for s in ids}, "preflight": {}, "skipped": [],
        "driver_commit": subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, cwd=HERE).stdout.strip(),
        "same_core": "both frontends run the same Codex core and a fake model of the same shape; only the frontend differs",
    }
    cast_dir = None if args.no_casts else out / "casts"
    runs_path = out / "runs.jsonl"
    done: set[tuple[str, str, int]] = set()
    if runs_path.exists():  # resume
        for line in runs_path.read_text().splitlines():
            r = json.loads(line)
            done.add((r["scenario"], r["bin"], r["round"]))

    # preflight: one launch per binary, discarded (cold-cache first launch)
    for b in bins:
        t = time.monotonic()
        r = run_once(b["label"], b["kind"], b["path"], scen["S9"] if "S9" in scen else next(iter(scen.values())), -1, out, None, gate)
        meta["preflight"][b["label"]] = {"ok": bool(r and r["ok"]), "t_ready_ms": r.get("t_ready_ms") if r else None,
                                         "wall_s": round(time.monotonic() - t, 1)}
    skipped = set()
    with open(runs_path, "a") as runs:
        for rnd in range(args.n):
            order = bins if rnd % 2 == 0 else list(reversed(bins))
            for sid in ids:
                for b in order:
                    if (sid, b["label"], rnd) in done:
                        continue
                    r = run_once(b["label"], b["kind"], b["path"], scen[sid], rnd, out, cast_dir, gate)
                    if r is None:
                        if (sid, b["label"]) not in skipped:
                            skipped.add((sid, b["label"]))
                            meta["skipped"].append({"scenario": sid, "bin": b["label"], "why": "subcommand not available in this binary"})
                            print(f"skip {sid} on {b['label']}: subcommand not available", file=sys.stderr)
                        continue
                    r["order_pos"] = [x["label"] for x in order].index(b["label"])
                    runs.write(json.dumps(r) + "\n")
                    runs.flush()
                    flag = "ok" if r["ok"] else "ERR " + "; ".join(r["errors"])
                    print(f"round {rnd:2d} {sid:4} {b['label']:12} load {r['load_before']:.1f} {flag}", file=sys.stderr, flush=True)
    meta["finished"] = dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")
    meta["load_end"] = os.getloadavg()[0]
    (out / "meta.json").write_text(json.dumps(meta, indent=1))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
