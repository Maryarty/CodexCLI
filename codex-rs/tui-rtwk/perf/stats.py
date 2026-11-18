"""Small statistics helpers shared by report.py and trend.py (stdlib only)."""

from __future__ import annotations

import json
import math
import random
import statistics
from pathlib import Path


def pct(xs, q: float) -> float:
    s = sorted(x for x in xs if x is not None and not math.isnan(x))
    if not s:
        return float("nan")
    k = (len(s) - 1) * q
    f = int(k)
    c = min(f + 1, len(s) - 1)
    return s[f] + (s[c] - s[f]) * (k - f)


def med(xs) -> float:
    xs = [x for x in xs if x is not None and not math.isnan(x)]
    return statistics.median(xs) if xs else float("nan")


def boot_ratio(a, b, stat=med, n: int = 2000, seed: int = 7):
    """B/A of `stat` with a 95% bootstrap interval over runs (each side resampled with replacement)."""
    a = [x for x in a if x is not None]
    b = [x for x in b if x is not None]
    if not a or not b:
        return None
    sa = stat(a)
    sb = stat(b)
    if not sa:
        return None
    rng = random.Random(seed)
    rs = []
    for _ in range(n):
        ra = stat([rng.choice(a) for _ in a])
        rb = stat([rng.choice(b) for _ in b])
        if ra:
            rs.append(rb / ra)
    rs.sort()
    if not rs:
        return None
    return sb / sa, rs[int(0.025 * len(rs))], rs[min(int(0.975 * len(rs)), len(rs) - 1)]


def noise_band(a) -> float:
    """Relative spread of the baseline run values: (max-min)/2/median, floored at 1%."""
    a = [x for x in a if x is not None]
    m = med(a)
    if not a or not m:
        return 0.01
    return max(0.01, (max(a) - min(a)) / 2 / m)


def load_runs(d: Path) -> list[dict]:
    return [json.loads(line) for line in (d / "runs.jsonl").read_text().splitlines() if line.strip()]


def get(rec: dict, path: str):
    cur = rec
    for part in path.split("."):
        if cur is None:
            return None
        cur = cur.get(part) if isinstance(cur, dict) else None
    return cur


def per_run_pcts(rec: dict, key: str, q: float, phase: str | None = None):
    xs = [k[key] for k in rec.get("keys", []) if k.get(key) is not None and (phase is None or k["phase"] == phase)]
    return pct(xs, q) if xs else None


def frame_bytes_p50(rec: dict, win: str):
    fl = get(rec, f"{win}.frame_list")
    return pct([b for _, b in fl], 0.5) if fl else None


def interval_pct(rec: dict, win: str, q: float):
    iv = get(rec, f"{win}.frame_interval_ms")
    return pct(iv, q) if iv else None


# (scenario, metric id, label, unit, extractor(rec) -> number|None, lower_is_better|None)
def _w(win, field, scale=1.0):
    return lambda r: (get(r, f"{win}.{field}") * scale) if get(r, f"{win}.{field}") is not None else None


def _rss_growth(win):
    def f(r):
        a, b = get(r, f"{win}.rss_start"), get(r, f"{win}.rss_end")
        return (b - a) / 1e6 if a is not None and b is not None else None
    return f


def _mb(path):
    return lambda r: (get(r, path) / 1e6) if get(r, path) is not None else None


STREAM = [
    ("cpu_pct", "CPU while streaming", "% of one core", _w("stream", "cpu_pct"), True),
    ("cpu_s", "CPU seconds (stream window)", "s", _w("stream", "cpu_s"), True),
    ("bytes_per_s", "output bytes per second", "KB/s", _w("stream", "bytes_per_s", 1e-3), True),
    ("frames_per_s", "frames per second", "fps", _w("stream", "frames_per_s"), None),
    ("bytes_per_frame_p50", "bytes per frame, p50", "bytes", lambda r: frame_bytes_p50(r, "stream"), True),
    ("frame_interval_p50", "frame interval p50", "ms", lambda r: interval_pct(r, "stream", 0.5), None),
    ("frame_interval_p95", "frame interval p95", "ms", lambda r: interval_pct(r, "stream", 0.95), True),
    ("frame_interval_p99", "frame interval p99", "ms", lambda r: interval_pct(r, "stream", 0.99), True),
    ("stream_dur", "stream duration (begin to last text)", "s", _w("stream", "dur_ms", 1e-3), True),
    ("rss_max", "peak RSS in window", "MB", _mb("stream.rss_max"), True),
    ("rss_growth", "RSS growth over the window", "MB", _rss_growth("stream"), True),
    ("rusage_cpu", "CPU seconds, whole run (wait4)", "s", lambda r: get(r, "rusage.cpu_s"), True),
    ("maxrss", "peak RSS whole run (wait4)", "MB", lambda r: get(r, "rusage.maxrss") / 1e6 if get(r, "rusage.maxrss") else None, True),
]
KEYS = [
    ("key_p50_10", "keystroke to visible p50, 10 keys/s", "ms", lambda r: per_run_pcts(r, "lat_ms", 0.5, "10/s"), True),
    ("key_p95_10", "keystroke to visible p95, 10 keys/s", "ms", lambda r: per_run_pcts(r, "lat_ms", 0.95, "10/s"), True),
    ("key_p99_10", "keystroke to visible p99, 10 keys/s", "ms", lambda r: per_run_pcts(r, "lat_ms", 0.99, "10/s"), True),
    ("key_p50_30", "keystroke to visible p50, 30 keys/s", "ms", lambda r: per_run_pcts(r, "lat_ms", 0.5, "30/s"), True),
    ("key_p95_30", "keystroke to visible p95, 30 keys/s", "ms", lambda r: per_run_pcts(r, "lat_ms", 0.95, "30/s"), True),
    ("key_p99_30", "keystroke to visible p99, 30 keys/s", "ms", lambda r: per_run_pcts(r, "lat_ms", 0.99, "30/s"), True),
    ("key_fb_p50", "keystroke to first byte p50 (secondary)", "ms", lambda r: per_run_pcts(r, "fb_ms", 0.5), True),
]
STARTUP = [
    ("t_first_byte", "start to first byte", "ms", lambda r: r.get("t_first_byte_ms"), True),
    ("t_ready", "start to first frame with the composer", "ms", lambda r: r.get("t_ready_ms"), True),
    ("startup_cpu", "CPU seconds to ready + 1 s", "s", lambda r: r.get("cpu_s_to_ready_hold"), True),
    ("startup_rss", "RSS at ready + 1 s", "MB", _mb("rss_at_ready_hold"), True),
    ("startup_bytes", "bytes output during startup (whole run)", "bytes", lambda r: r.get("total_bytes"), True),
]
IDLE = [
    ("idle_cpu_pct", "idle CPU", "% of one core", _w("idle", "cpu_pct"), True),
    ("idle_frames_per_s", "idle frames per second", "fps", _w("idle", "frames_per_s"), True),
    ("idle_bytes_per_s", "idle output bytes per second", "B/s", _w("idle", "bytes_per_s"), True),
    ("idle_rss", "RSS at end of window", "MB", _mb("idle.rss_end"), True),
    ("idle_rss_growth", "RSS growth over the window", "MB", _rss_growth("idle"), True),
]
AGENTS = STARTUP[:3] + [
    ("inter_bytes", "bytes during 10 Down keys", "bytes", _w("interaction", "bytes"), True),
] + IDLE[:3]

METRICS = {
    "S1": STREAM, "S2": STREAM, "S5": STREAM + KEYS,
    "S9": STARTUP, "S10": STARTUP[:2] + IDLE, "S10w": STARTUP[:2] + IDLE, "S11": AGENTS,
}
HEADLINE = {  # the rows of the verdict table
    "S1": ["cpu_pct", "bytes_per_s", "bytes_per_frame_p50", "frame_interval_p99", "rss_max"],
    "S2": ["cpu_pct", "bytes_per_s", "bytes_per_frame_p50", "frame_interval_p99", "rss_max"],
    "S5": ["key_p50_10", "key_p99_10", "key_p50_30", "key_p99_30", "cpu_pct", "bytes_per_s"],
    "S9": ["t_first_byte", "t_ready", "startup_cpu", "startup_rss"],
    "S10": ["idle_cpu_pct", "idle_frames_per_s", "idle_bytes_per_s", "idle_rss"],
    "S10w": ["idle_cpu_pct", "idle_frames_per_s", "idle_bytes_per_s", "idle_rss"],
    "S11": ["t_ready", "startup_cpu", "idle_cpu_pct", "idle_bytes_per_s"],
}


def summarize(runs: list[dict]) -> dict:
    """{scenario: {bin: {metric: {"vals": [...per run...], "p50":, "p95":, "p99":, "min":, "max":}}}} from ok runs."""
    out: dict = {}
    for sid, mets in METRICS.items():
        for label in sorted({r["bin"] for r in runs}):
            rs = [r for r in runs if r["scenario"] == sid and r["bin"] == label and r["round"] >= 0 and r.get("ok")]
            if not rs:
                continue
            d = out.setdefault(sid, {}).setdefault(label, {"n": len(rs)})
            for mid, _label, _unit, fn, _lower in mets:
                vals = []
                for r in rs:
                    try:
                        v = fn(r)
                    except (TypeError, KeyError):
                        v = None
                    if v is not None and not (isinstance(v, float) and math.isnan(v)):
                        vals.append(v)
                if vals:
                    d[mid] = {"vals": vals, "p50": med(vals), "p95": pct(vals, 0.95), "p99": pct(vals, 0.99), "min": min(vals), "max": max(vals)}
    return out
