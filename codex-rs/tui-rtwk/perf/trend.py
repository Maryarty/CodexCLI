#!/usr/bin/env python3
"""Milestone trend: one summary line per milestone in history.jsonl, small multiples out.

    python3 -I trend.py add results/<run> --milestone NAME [--history history.jsonl]
    python3 -I trend.py [--history history.jsonl] [--out trend.html]

`add` reads <run>/summary.json (written by report.py) and appends one line:
{milestone, date, commit, run, summary: {scenario: {kind: {metric: {p50,min,max,n}}}}}
where kind is oracle or rtwk. The original's baseline is the fixed reference: the
LAST history line that has an oracle entry for the scenario and metric is drawn as a
horizontal line with its min-max band; the port's p50 per milestone is the polyline
with min-max whiskers. A milestone without port numbers (the baseline itself) only
sets the reference.
"""

from __future__ import annotations

import argparse
import datetime as dt
import html
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import stats as S  # noqa: E402

DEFAULT_HISTORY = HERE / "history.jsonl"


def add(run: Path, milestone: str, history: Path) -> None:
    summ = json.loads((run / "summary.json").read_text())
    by_kind: dict = {}
    for sid, bins in summ.items():
        for label, mets in bins.items():
            kind = label.split(":", 1)[1] if ":" in label else label
            by_kind.setdefault(sid, {})[kind] = mets
    try:
        commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], capture_output=True, text=True, cwd=HERE).stdout.strip()
    except OSError:
        commit = ""
    line = {"milestone": milestone, "date": dt.date.today().isoformat(), "commit": commit, "run": run.name, "summary": by_kind}
    with open(history, "a") as f:
        f.write(json.dumps(line) + "\n")
    print(f"appended {milestone} ({', '.join(sorted(by_kind))}) to {history}")


def read(history: Path) -> list[dict]:
    if not history.exists():
        return []
    return [json.loads(x) for x in history.read_text().splitlines() if x.strip()]


def section(history: Path) -> str:
    hist = read(history)
    if not hist:
        return "<p class=muted>no milestones yet</p>"
    mt = {sid: {m[0]: m for m in S.METRICS[sid]} for sid in S.METRICS}
    o = ["<p class=muted>Port milestones on the x axis; the original's baseline (latest oracle numbers in the history) is the fixed reference: dashed line with its min-max band. Lower is better except frame rate.</p>"]
    for sid in S.HEADLINE:
        mids = [m for m in S.HEADLINE[sid] if any(m in h["summary"].get(sid, {}).get("oracle", {}) or m in h["summary"].get(sid, {}).get("rtwk", {}) for h in hist)]
        if not mids:
            continue
        o.append(f"<h3>{html.escape(sid)}</h3><div style='display:flex;flex-wrap:wrap;gap:8px'>")
        for mid in mids:
            _, label, unit, _, _ = mt[sid][mid]
            ref = next((h["summary"][sid]["oracle"][mid] for h in reversed(hist) if mid in h["summary"].get(sid, {}).get("oracle", {})), None)
            pts = [(i, h["milestone"], h["summary"][sid]["rtwk"][mid]) for i, h in enumerate(hist) if mid in h["summary"].get(sid, {}).get("rtwk", {})]
            vals = [p[2]["p50"] for p in pts] + ([ref["p50"], ref["min"], ref["max"]] if ref else []) + [p[2]["max"] for p in pts]
            if not vals:
                continue
            w, h_, left, bottom = 300, 150, 44, 34
            ya = S_axis(0, max(vals) * 1.1 or 1, h_ - bottom, 18)
            n = max(len(hist), 2)
            xa = lambda i: left + (i + 0.5) * (w - left - 8) / n
            s = [f'<svg viewBox="0 0 {w} {h_}" width="{w}" height="{h_}" font-family="system-ui,sans-serif" font-size="10"><text x="4" y="11" font-weight="600">{html.escape(label)} ({html.escape(unit)})</text>']
            for t in (0, max(vals) * 1.1 / 2, max(vals) * 1.1):
                s.append(f'<line x1="{left}" x2="{w - 8}" y1="{ya(t):.1f}" y2="{ya(t):.1f}" stroke="#e2e8f0"/><text x="{left - 3}" y="{ya(t) + 3:.1f}" text-anchor="end" fill="#718096">{t:.3g}</text>')
            if ref:
                s.append(f'<rect x="{left}" y="{ya(ref["max"]):.1f}" width="{w - left - 8}" height="{max(ya(ref["min"]) - ya(ref["max"]), 1):.1f}" fill="#2b6cb0" fill-opacity=".12"/><line x1="{left}" x2="{w - 8}" y1="{ya(ref["p50"]):.1f}" y2="{ya(ref["p50"]):.1f}" stroke="#2b6cb0" stroke-dasharray="4 3"/>')
            for i, hh in enumerate(hist):
                s.append(f'<text x="{xa(i):.1f}" y="{h_ - 6}" text-anchor="middle" fill="#718096" transform="rotate(-20 {xa(i):.1f} {h_ - 6})">{html.escape(hh["milestone"][:14])}</text>')
            if pts:
                d = " ".join(f"{xa(i):.1f},{ya(m['p50']):.1f}" for i, _, m in pts)
                s.append(f'<polyline fill="none" stroke="#dd6b20" stroke-width="1.5" points="{d}"/>')
                for i, _, m in pts:
                    s.append(f'<line x1="{xa(i):.1f}" x2="{xa(i):.1f}" y1="{ya(m["min"]):.1f}" y2="{ya(m["max"]):.1f}" stroke="#dd6b20"/><circle cx="{xa(i):.1f}" cy="{ya(m["p50"]):.1f}" r="3" fill="#dd6b20"/>')
            s.append("</svg>")
            o.append("".join(s))
        o.append("</div>")
    return "".join(o)


def S_axis(lo, hi, a, b):
    return lambda v: a + (v - lo) / ((hi - lo) or 1) * (b - a)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", nargs="?", choices=["add"])
    ap.add_argument("run", nargs="?")
    ap.add_argument("--milestone")
    ap.add_argument("--history", default=str(DEFAULT_HISTORY))
    ap.add_argument("--out", default=str(HERE / "trend.html"))
    args = ap.parse_args()
    hist = Path(args.history)
    if args.cmd == "add":
        if not args.run or not args.milestone:
            raise SystemExit("usage: trend.py add <results dir> --milestone NAME")
        add(Path(args.run), args.milestone, hist)
    page = (
        "<!doctype html><meta charset=utf-8><title>Codex TUI perf trend</title><style>body{font:14px system-ui,sans-serif;max-width:1000px;margin:20px auto;padding:0 20px}.muted{color:#718096}</style>"
        "<h1>Milestone trend</h1>" + section(hist)
    )
    Path(args.out).write_text(page)
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
