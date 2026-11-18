#!/usr/bin/env python3
"""Writes ONE self-contained report.html (and summary.json) from a results directory.

    python3 -I report.py results/<run> [--history ../history.jsonl]

Reads runs.jsonl and meta.json (and casts/ for the side-by-side replay). The page has
no external file: inline SVG charts written here, a small inline script for the log
toggle and the replayer. Sections: header (machine, load, commits, binaries), verdict
table (p50 per binary, ratio B/A with a bootstrap 95% interval over runs, a coloured
cell), per scenario cards (distributions, key latency, frame-interval and
bytes-per-frame histograms, CPU and RSS over time), the side-by-side replayer, the
trend (when a history file is given), and the raw run table with the commands.
"""

from __future__ import annotations

import argparse
import base64
import gzip
import html
import json
import math
import re
import sys
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import stats as S  # noqa: E402
from vt import Term  # noqa: E402

COL = {0: "#2b6cb0", 1: "#dd6b20"}  # A blue, B orange
TITLES = {
    "S1": "S1 stream 100 deltas/s (20 s nominal)", "S2": "S2 stream 1000 deltas/s (10 s nominal)",
    "S5": "S5 typing 10 keys/s and 30 keys/s while streaming", "S9": "S9 startup",
    "S10": "S10 idle after a finished turn (30 s)", "S10w": "S10w idle while a turn is open, model silent (20 s)",
    "S11": "S11 Agent command center",
}
VOCAB = set(
    "the of and to in is that for it as with be on at by this from or an are not have but they which one you all can her was "
    "their there been if more when will who would make like time just know take into year your some could them see other than "
    "then now look only come its over think also back after use two how our work first well way even new want because any these "
    "give day most us render frame buffer stream widget layout style paint cell terminal screen input event queue state model async".split()
)


def esc(s) -> str:
    return html.escape(str(s))


def fmt(v, unit="") -> str:
    if v is None or (isinstance(v, float) and math.isnan(v)):
        return "-"
    a = abs(v)
    s = f"{v:.0f}" if a >= 1000 else f"{v:.1f}" if a >= 100 else f"{v:.2f}" if a >= 1 else f"{v:.3f}"
    return s


# ------------------------------------------------------------------ SVG helpers
def nice_ticks(lo, hi, n=5):
    if hi <= lo:
        hi = lo + 1
    raw = (hi - lo) / n
    mag = 10 ** math.floor(math.log10(raw))
    step = min((m * mag for m in (1, 2, 2.5, 5, 10) if m * mag >= raw), default=mag)
    t = math.ceil(lo / step) * step
    out = []
    while t <= hi + 1e-9:
        out.append(t)
        t += step
    return out


def log_ticks(lo, hi):
    out = []
    e = math.floor(math.log10(lo))
    while 10**e <= hi * 1.001:
        for m in (1, 2, 5):
            v = m * 10**e
            if lo * 0.999 <= v <= hi * 1.001:
                out.append(v)
        e += 1
    return out


class Axis:
    def __init__(self, lo, hi, a, b, log=False):
        self.log = log and lo > 0
        if self.log:
            lo = max(lo, 1e-9)
        if hi <= lo:
            hi = lo * 1.01 if lo > 0 else lo + 1
        self.lo, self.hi, self.a, self.b = lo, hi, a, b

    def __call__(self, v):
        if self.log:
            v = max(v, self.lo)
            f = (math.log10(v) - math.log10(self.lo)) / ((math.log10(self.hi) - math.log10(self.lo)) or 1)
        else:
            f = (v - self.lo) / ((self.hi - self.lo) or 1)
        return self.a + f * (self.b - self.a)

    def ticks(self):
        return log_ticks(self.lo, self.hi) if self.log else nice_ticks(self.lo, self.hi)


def svg_open(w, h):
    return f'<svg viewBox="0 0 {w} {h}" width="{w}" height="{h}" xmlns="http://www.w3.org/2000/svg" font-family="system-ui,sans-serif" font-size="11">'


def strip_plot(lanes, unit, log=False, w=700, lane_h=34, left=150):
    """lanes: [(label, values, colour)]; strip of all values + box (q25-q75) + p50/p95/p99 ticks."""
    if log:
        lanes = [(lb, [v for v in vs if v and v > 0], c) for lb, vs, c in lanes]
    vals = [v for _, vs, _ in lanes for v in vs if v is not None]
    if not vals:
        return "<p class=muted>no data</p>"
    lo, hi = min(vals), max(vals)
    if log:
        lo = max(lo, 1e-3)
        lo, hi = lo / 1.3, hi * 1.3
    else:
        pad = (hi - lo) * 0.05 or abs(hi) * 0.05 or 1
        lo, hi = max(0, lo - pad) if lo >= 0 else lo - pad, hi + pad
    ax = Axis(lo, hi, left, w - 20, log)
    h = len(lanes) * lane_h + 34
    o = [svg_open(w, h)]
    for t in ax.ticks():
        x = ax(t)
        o.append(f'<line x1="{x:.1f}" x2="{x:.1f}" y1="6" y2="{h - 22}" stroke="#e2e8f0"/><text x="{x:.1f}" y="{h - 8}" text-anchor="middle" fill="#718096">{fmt(t)}</text>')
    for i, (label, vs, colour) in enumerate(lanes):
        y = 12 + i * lane_h + lane_h / 2 - 6
        o.append(f'<text x="{left - 8}" y="{y + 4}" text-anchor="end">{esc(label)}</text>')
        vs = [v for v in vs if v is not None]
        if not vs:
            continue
        for k, v in enumerate(vs):
            jy = y + ((k * 37) % 11 - 5) * 0.9
            o.append(f'<circle cx="{ax(v):.1f}" cy="{jy:.1f}" r="2" fill="{colour}" fill-opacity=".35"/>')
        q1, q3 = S.pct(vs, 0.25), S.pct(vs, 0.75)
        o.append(f'<rect x="{ax(q1):.1f}" y="{y - 9:.1f}" width="{max(ax(q3) - ax(q1), 1):.1f}" height="18" fill="none" stroke="{colour}" stroke-width="1.2"/>')
        for q, wdt in ((0.5, 2.5), (0.95, 1.2), (0.99, 1.2)):
            x = ax(S.pct(vs, q))
            o.append(f'<line x1="{x:.1f}" x2="{x:.1f}" y1="{y - 12:.1f}" y2="{y + 12:.1f}" stroke="{colour}" stroke-width="{wdt}"/>')
    o.append("</svg>")
    return "".join(o)


def toggled(make, key):
    """Both linear and log versions; the checkbox at the top of the report flips them."""
    return f'<div class="lin">{make(False)}</div><div class="logv">{make(True)}</div>'


def line_chart(series, xlabel, ylabel, w=700, h=190, marks=(), ymin=0.0):
    """series: [(label, colour, [(x, y)])]."""
    pts = [p for _, _, ps in series for p in ps]
    if not pts:
        return "<p class=muted>no data</p>"
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    xa = Axis(min(xs), max(xs), 50, w - 15)
    ya = Axis(ymin, max(ys) * 1.08 or 1, h - 26, 10)
    o = [svg_open(w, h)]
    for t in ya.ticks():
        o.append(f'<line x1="50" x2="{w - 15}" y1="{ya(t):.1f}" y2="{ya(t):.1f}" stroke="#e2e8f0"/><text x="46" y="{ya(t) + 4:.1f}" text-anchor="end" fill="#718096">{fmt(t)}</text>')
    for t in xa.ticks():
        o.append(f'<text x="{xa(t):.1f}" y="{h - 10}" text-anchor="middle" fill="#718096">{fmt(t)}</text>')
    for x, label in marks:
        if xa.lo <= x <= xa.hi:
            o.append(f'<line x1="{xa(x):.1f}" x2="{xa(x):.1f}" y1="10" y2="{h - 26}" stroke="#a0aec0" stroke-dasharray="3 3"/><text x="{xa(x) + 3:.1f}" y="20" fill="#718096">{esc(label)}</text>')
    for i, (label, colour, ps) in enumerate(series):
        d = " ".join(f"{xa(x):.1f},{ya(y):.1f}" for x, y in ps)
        o.append(f'<polyline fill="none" stroke="{colour}" stroke-width="1.5" points="{d}"/>')
        o.append(f'<text x="{w - 20}" y="{24 + i * 14}" text-anchor="end" fill="{colour}">{esc(label)}</text>')
    o.append(f'<text x="{w / 2}" y="{h - 0}" text-anchor="middle" fill="#718096">{esc(xlabel)}</text><text x="12" y="12" fill="#718096">{esc(ylabel)}</text></svg>')
    return "".join(o)


def histogram(series, xlabel, w=700, h=170, log_x=True, bins=40):
    """series: [(label, colour, values)] as step lines of the share per bin."""
    allv = [v for _, _, vs in series for v in vs if v and v > 0]
    if not allv:
        return "<p class=muted>no data</p>"
    lo, hi = min(allv), max(allv)
    if hi <= lo:
        hi = lo * 1.5
    xa = Axis(lo, hi, 50, w - 15, log_x)
    edges = [lo * (hi / lo) ** (i / bins) for i in range(bins + 1)] if log_x else [lo + (hi - lo) * i / bins for i in range(bins + 1)]
    shares = []
    for _, _, vs in series:
        vs = [v for v in vs if v and v > 0]
        c = [0] * bins
        for v in vs:
            k = min(bins - 1, max(0, next((i for i in range(bins) if v < edges[i + 1]), bins - 1)))
            c[k] += 1
        n = len(vs) or 1
        shares.append([x / n for x in c])
    top = max((max(s) for s in shares), default=1) or 1
    ya = Axis(0, top * 1.1, h - 26, 10)
    o = [svg_open(w, h)]
    for t in xa.ticks():
        o.append(f'<text x="{xa(t):.1f}" y="{h - 10}" text-anchor="middle" fill="#718096">{fmt(t)}</text><line x1="{xa(t):.1f}" x2="{xa(t):.1f}" y1="10" y2="{h - 26}" stroke="#edf2f7"/>')
    for (label, colour, _), sh in zip(series, shares):
        pts = []
        for i, s in enumerate(sh):
            pts.append(f"{xa(edges[i]):.1f},{ya(s):.1f} {xa(edges[i + 1]):.1f},{ya(s):.1f}")
        o.append(f'<polyline fill="none" stroke="{colour}" stroke-width="1.5" points="{" ".join(pts)}"/>')
    for i, (label, colour, _) in enumerate(series):
        o.append(f'<text x="{w - 20}" y="{24 + i * 14}" text-anchor="end" fill="{colour}">{esc(label)}</text>')
    o.append(f'<text x="{w / 2}" y="{h}" text-anchor="middle" fill="#718096">{esc(xlabel)} (share of frames per bin)</text></svg>')
    return "".join(o)


# ------------------------------------------------------------------ data
def cpu_curve(rec, t0_ms, smooth_ms=1000):
    """[(seconds since t0, CPU % of one core)] from the 100 ms samples, smoothed over 1 s."""
    s = rec.get("cpu_series") or []
    out = []
    j = 0
    for i in range(len(s)):
        while s[i][0] - s[j][0] > smooth_ms:
            j += 1
        dt_ = s[i][0] - s[j][0]
        if dt_ >= 300:
            out.append(((s[i][0] - t0_ms) / 1000, 100 * (s[i][1] - s[j][1]) / (dt_ / 1000)))
    return out


def rss_curve(rec, t0_ms):
    return [((t - t0_ms) / 1000, r / 1e6) for t, _, r in (rec.get("cpu_series") or [])]


def representative(runs):
    """The run with the median CPU (stream, idle) of its group, to plot and replay."""
    def key(r):
        for k in ("stream", "idle"):
            if r.get(k) and r[k].get("cpu_pct") is not None:
                return r[k]["cpu_pct"]
        return r.get("t_ready_ms") or 0
    rs = sorted(runs, key=key)
    return rs[len(rs) // 2] if rs else None


def stream_tail(rows, sentinel, n=60):
    idx = next((i for i, r in enumerate(rows) if sentinel in r), None)
    if idx is None:
        return None
    toks = [w for r in rows[:idx] for w in re.findall(r"[a-z]+", r) if w in VOCAB]
    return toks[-n:]


def replay_snaps(cast_path: Path, t_from, t_to, fps=8):
    """Row-diff snapshots of the cast replayed through vt.Term, [(t_ms, {row: text})], first one full."""
    with gzip.open(cast_path, "rt") as f:
        head = json.loads(f.readline())
        ev = [json.loads(line) for line in f]
    term = Term(head["height"], head["width"])
    snaps, last, nxt, i = [], None, t_from, 0
    step = 1 / fps
    while nxt <= t_to:
        while i < len(ev) and ev[i][0] <= nxt:
            term.feed(ev[i][2].encode(), ev[i][0])
            i += 1
        rows = term.screen_rows()
        if last is None:
            snaps.append([round((nxt - t_from) * 1000), {str(k): r for k, r in enumerate(rows)}])
        else:
            d = {str(k): r for k, r in enumerate(rows) if r != last[k]}
            if d:
                snaps.append([round((nxt - t_from) * 1000), d])
        last = rows
        nxt += step
        if i >= len(ev) and nxt > ev[-1][0] + 1:
            break
    return {"w": head["width"], "h": head["height"], "snaps": snaps}


def pack(obj):
    return base64.b64encode(zlib.compress(json.dumps(obj, separators=(",", ":")).encode(), 9)).decode()


# ------------------------------------------------------------------ page
CSS = """
body{font:14px/1.45 system-ui,sans-serif;margin:0;color:#1a202c;background:#f7fafc}
main{max-width:1000px;margin:0 auto;padding:16px 20px 60px}
h1{font-size:22px;margin:8px 0}h2{font-size:18px;margin:34px 0 8px;border-bottom:2px solid #cbd5e0;padding-bottom:4px}
h3{font-size:14px;margin:16px 0 4px;color:#2d3748}
table{border-collapse:collapse;font-size:12.5px;margin:6px 0}th,td{border:1px solid #e2e8f0;padding:3px 8px;text-align:right}
th{background:#edf2f7}td:first-child,th:first-child{text-align:left}
.muted{color:#718096}.card{background:#fff;border:1px solid #e2e8f0;border-radius:6px;padding:8px 14px;margin:12px 0}
.g{background:#c6f6d5}.r{background:#fed7d7}.n{background:#edf2f7}.w{background:#fefcbf}
.A{color:#2b6cb0}.B{color:#dd6b20}.logv{display:none}body.log .logv{display:block}body.log .lin{display:none}
pre{background:#edf2f7;padding:8px;overflow:auto;font-size:12px}.warn{background:#fefcbf;border:1px solid #ecc94b;padding:6px 10px;border-radius:4px}
.player{display:flex;gap:8px}.player pre{margin:0;flex:1;background:#1a202c;color:#e2e8f0;font-size:9px;line-height:1.1;white-space:pre;overflow:hidden}
.scr{width:100%}
"""

JS = """
document.getElementById('logt').onchange=function(){document.body.classList.toggle('log',this.checked)};
async function inflate(b64){const bin=Uint8Array.from(atob(b64),c=>c.charCodeAt(0));
 const ds=new DecompressionStream('deflate');const s=new Blob([bin]).stream().pipeThrough(ds);
 return JSON.parse(await new Response(s).text());}
function Pane(el,data){this.el=el;this.data=data;this.i=-1;this.rows=Array(data.h).fill('');}
Pane.prototype.seek=function(t){const s=this.data.snaps;let j=this.i;
 if(j<0||s[j][0]>t){j=-1;this.rows=Array(this.data.h).fill('');}
 while(j+1<s.length&&s[j+1][0]<=t){j++;for(const k in s[j][1])this.rows[+k]=s[j][1][k];}
 this.i=j;this.el.textContent=this.rows.join('\\n');};
document.querySelectorAll('.player').forEach(async function(pl){
 try{const a=await inflate(pl.dataset.a);const b=await inflate(pl.dataset.b);
 const pa=new Pane(pl.querySelector('.pa'),a),pb=new Pane(pl.querySelector('.pb'),b);
 const sl=pl.parentNode.querySelector('input[type=range]');const lab=pl.parentNode.querySelector('.tl');
 const end=Math.max(a.snaps.length?a.snaps[a.snaps.length-1][0]:0,b.snaps.length?b.snaps[b.snaps.length-1][0]:0);
 sl.max=end;let playing=false,t=Math.min(1500,end),last=0;
 function show(){pa.seek(t);pb.seek(t);sl.value=t;lab.textContent=(t/1000).toFixed(1)+' s'}
 sl.oninput=function(){t=+sl.value;show()};
 pl.parentNode.querySelector('button').onclick=function(){playing=!playing;this.textContent=playing?'pause':'play';last=performance.now();
  (function tick(n){if(!playing)return;t+=n-last;last=n;if(t>=end){t=end;playing=false}show();if(playing)requestAnimationFrame(tick)})(last)};
 show();}catch(e){pl.textContent='replay needs DecompressionStream (a current browser): '+e}});
"""


def verdict_cell(a_vals, b_vals, lower_better, band):
    r = S.boot_ratio(a_vals, b_vals)
    if r is None:
        return "-", "n"
    ratio, lo, hi = r
    txt = f"{ratio:.2f}x [{lo:.2f}, {hi:.2f}]"
    if lower_better is None:
        return txt, "n"
    within = (lo <= 1 <= hi) or abs(ratio - 1) <= band or min(len(a_vals), len(b_vals)) < 5
    if within:
        return txt, "n"
    good = hi < 1 if lower_better else lo > 1
    return txt, "g" if good else "r"


def build(d: Path, history: Path | None) -> str:
    runs = S.load_runs(d)
    meta = json.loads((d / "meta.json").read_text()) if (d / "meta.json").exists() else {}
    summ = S.summarize(runs)
    (d / "summary.json").write_text(json.dumps({sid: {b: {m: {k: v for k, v in x.items() if k != "vals"} | {"n": len(x["vals"])} for m, x in bm.items() if isinstance(x, dict)} for b, bm in bs.items()} for sid, bs in summ.items()}, indent=1))
    labels = sorted({r["bin"] for r in runs})
    ab = len(labels) == 2
    colour = {lb: COL[i] for i, lb in enumerate(labels)}
    cls = {lb: ("A", "B")[i] for i, lb in enumerate(labels)}
    scs = [s for s in meta.get("scenarios", []) if s in summ] or [s for s in METRIC_ORDER if s in summ]
    ok_runs = [r for r in runs if r["round"] >= 0 and r.get("ok")]
    bad = [r for r in runs if r["round"] >= 0 and not r.get("ok")]
    loads = [r["load_before"] for r in runs if r["round"] >= 0]
    out = [f"<!doctype html><meta charset=utf-8><title>Codex TUI perf report</title><style>{CSS}</style><body><main>"]
    title = " vs ".join(labels)
    out.append(f"<h1>Codex TUI performance: {esc(title)}</h1>")
    out.append(f'<p class=muted>{esc(meta.get("label") or "")} - started {esc(meta.get("started", ""))}, finished {esc(meta.get("finished", ""))}</p>')
    out.append('<label><input type=checkbox id=logt> log scale on the distribution plots</label>')
    # header card
    mach = meta.get("machine", {})
    out.append('<div class=card><h3>What is compared</h3><p>Same Codex core and the same kind of fake model (Codex\'s own MockResponsesServer, one server and one fresh CODEX_HOME per run, analytics off, no network); both run in a pty at the scenario size with TERM=xterm-256color and the driver\'s VT model; only the frontend differs. Everything is black box through the pty: CPU and RSS from proc_pidinfo every 100 ms and wait4 rusage, frames are DEC 2026 pairs (read bursts when absent), a key is "visible" when the VT model shows it in the screen (a DEC 2026 update held longer than 150 ms counts as visible). The VT model is a model, not a GPU terminal: byte volume is the proxy for terminal cost.</p>')
    out.append("<table><tr><th>binary</th><th>kind</th><th>version / sha256</th><th>runs ok</th></tr>")
    for lb in labels:
        bi = meta.get("binaries", {}).get(lb, {})
        n = len([r for r in ok_runs if r["bin"] == lb])
        out.append(f'<tr><td class={cls[lb]}>{esc(lb)}</td><td>{esc(bi.get("kind", ""))}</td><td>{esc(bi.get("version", ""))} {esc(bi.get("sha256", ""))} ({bi.get("size_mb", "")} MB)<br><span class=muted>{esc(" ".join(bi.get("version_file", [])))}</span></td><td>{n}</td></tr>')
    out.append("</table>")
    out.append(f'<p>N = {esc(meta.get("n", "?"))} rounds, interleaved ({"A B, B A, ..." if ab else "single binary"}), a preflight launch per binary discarded. Machine: {esc(mach.get("host_model", ""))}, {esc(mach.get("cpus", ""))} CPUs, {esc(mach.get("mem_gb", ""))} GB, {esc(mach.get("os", ""))}, power: {esc(mach.get("power", ""))}; {esc(mach.get("rustc", ""))}; driver commit {esc(meta.get("driver_commit", ""))}.</p>')
    if loads:
        gate = meta.get("load_gate", 15)
        hot = sum(1 for x in loads if x > gate)
        cls_ = "warn" if hot else "muted"
        out.append(f'<p class="{cls_}">Load (1-min, before each run): median {S.med(loads):.1f}, max {max(loads):.1f}, gate {gate:.1f} (1.5 x CPUs); {hot} of {len(loads)} runs started above the gate{" (taken with --measure-only; the shared machine was busy: absolute numbers carry that noise, the interleaving keeps A and B comparable)" if hot else ""}. Start {meta.get("load_start", 0):.1f}, end {meta.get("load_end", 0):.1f}.</p>')
    modes = {lb: ("alt-screen" if any(r.get("alt_screen") for r in ok_runs if r["bin"] == lb) else "inline") for lb in labels}
    out.append(f'<p>Screen mode: {esc(", ".join(f"{k}: {v}" for k, v in modes.items()))}. Frame source: {esc(", ".join(sorted({(r.get("stream") or r.get("idle") or {}).get("frame_mode", "-") for r in ok_runs} - {"-"})))}.</p>')
    if meta.get("skipped"):
        out.append('<p class=warn>Skipped: ' + esc("; ".join(f'{s["scenario"]} on {s["bin"]} ({s["why"]})' for s in meta["skipped"])) + "</p>")
    if bad:
        out.append(f'<p class=warn>{len(bad)} run(s) failed and are excluded from the statistics (listed in the run table): ' + esc("; ".join(sorted({f'{r["scenario"]} {r["bin"]}: {r["errors"][0]}' for r in bad if r["errors"]}))[:600]) + "</p>")
    out.append("</div>")

    # final screens
    out.append("<h2>Final screens</h2>")
    out.append("<table><tr><th>scenario</th>" + "".join(f"<th>{esc(lb)}</th>" for lb in labels) + ("<th>transcript tail equal</th>" if ab else "") + "</tr>")
    for sid in scs:
        cells, tails = [], []
        sent = (meta.get("scenario_defs", {}).get(sid, {}).get("mock") or {}).get("sentinel")
        for lb in labels:
            rs = [r for r in ok_runs if r["scenario"] == sid and r["bin"] == lb]
            seen = sum(1 for r in rs if r.get("sentinel_seen", True)) if sid in ("S1", "S2", "S5") else None
            hashes = len({r.get("final_hash") for r in rs})
            cells.append(f'{len(rs)} runs, sentinel {seen}/{len(rs)}' if seen is not None else f"{len(rs)} runs")
            rep = representative(rs)
            tails.append(stream_tail(rep["final_rows"], sent) if rep and sent and sid in ("S1", "S2", "S5", "S10") else None)
        extra = ""
        if ab:
            if all(t for t in tails):
                extra = f'<td class={"g" if tails[0] == tails[1] else "r"}>{"yes" if tails[0] == tails[1] else "NO (flagged)"}</td>'
            else:
                extra = "<td class=n>-</td>"
        out.append(f"<tr><td>{esc(sid)}</td>" + "".join(f"<td>{c}</td>" for c in cells) + extra + "</tr>")
    out.append("</table><p class=muted>Transcript tail: the last 60 stream words above the sentinel row of the median run, compared between binaries; a mismatch is flagged, never timed silently.</p>")

    # verdict table
    out.append("<h2>Verdict table</h2>")
    if ab:
        out.append("<p>p50 of the per-run values for each binary; ratio = B/A of the medians with a 95% bootstrap interval over runs (2000 resamples). Green: B lower (better) with the interval clear of 1.0 and of the baseline noise band; red: B higher; grey: within noise, fewer than 5 runs per side, or no direction (frame rate is informational).</p>")
    out.append("<table><tr><th>scenario</th><th>metric</th>" + "".join(f"<th class={cls[lb]}>{esc(lb)} p50 [min, max]</th>" for lb in labels) + ("<th>ratio B/A [95% CI]</th>" if ab else "") + "</tr>")
    mt = {sid: {m[0]: m for m in S.METRICS[sid]} for sid in S.METRICS}
    for sid in scs:
        for mid in S.HEADLINE[sid]:
            if not any(mid in summ[sid].get(lb, {}) for lb in labels):
                continue
            _, label, unit, _, lower = mt[sid][mid]
            row = f"<tr><td>{esc(sid)}</td><td style='text-align:left'>{esc(label)} ({esc(unit)})</td>"
            for lb in labels:
                m = summ[sid].get(lb, {}).get(mid)
                row += f"<td>{fmt(m['p50'])} <span class=muted>[{fmt(m['min'])}, {fmt(m['max'])}]</span></td>" if m else "<td>-</td>"
            if ab:
                a, b = (summ[sid].get(lb, {}).get(mid) for lb in labels)
                if a and b:
                    txt, c = verdict_cell(a["vals"], b["vals"], lower, S.noise_band(a["vals"]))
                    row += f"<td class={c}>{txt}</td>"
                else:
                    row += "<td class=n>-</td>"
            out.append(row + "</tr>")
    out.append("</table>")

    # cards
    for sid in scs:
        out.append(f"<h2>{esc(TITLES.get(sid, sid))}</h2><div class=card>")
        d_ = meta.get("scenario_defs", {}).get(sid, {})
        out.append(f'<p class=muted>{esc(d_.get("title", ""))}; size {esc("x".join(map(str, d_.get("size", []))))}; mock {esc(json.dumps(d_.get("mock", {})))}</p>')
        # achieved delta rate
        m = d_.get("mock") or {}
        if m.get("deltas") and sid in ("S1", "S2", "S5"):
            rates = []
            for lb in labels:
                v = summ[sid].get(lb, {}).get("stream_dur")
                if v:
                    rates.append(f"{esc(lb)}: {m['deltas'] / v['p50']:.0f}/s")
            out.append(f'<p>Achieved delta rate (deltas / stream duration, median): {", ".join(rates)}; nominal {1 / m["pace"]:.0f}/s. The server paces with sleep(), so the achieved rate is lower than nominal, more so on a loaded host.</p>')
        # per metric strips
        rows_out = []
        for mid, label, unit, fn, lower in S.METRICS[sid]:
            lanes = [(f"{lb}", summ[sid][lb][mid]["vals"], colour[lb]) for lb in labels if mid in summ[sid].get(lb, {})]
            if not lanes or mid in ("rusage_cpu",):
                continue
            tbl = "".join(f"<td>{fmt(summ[sid][lb][mid]['p50'])}</td><td>{fmt(summ[sid][lb][mid]['p95'])}</td><td>{fmt(summ[sid][lb][mid]['p99'])}</td>" for lb in labels if mid in summ[sid].get(lb, {}))
            rows_out.append((mid, label, unit, lanes))
        out.append("<h3>Per-run values (one point per run; box = p25-p75 across runs, thick tick = p50, thin ticks = p95 and p99)</h3>")
        for mid, label, unit, lanes in rows_out:
            out.append(f"<div><b>{esc(label)}</b> <span class=muted>({esc(unit)})</span>" + toggled(lambda lg, lanes=lanes, unit=unit: strip_plot(lanes, unit, lg, lane_h=28), mid) + "</div>")
        if sid == "S5":
            for ph in ("10/s", "30/s"):
                lanes = []
                for lb in labels:
                    v = [k["lat_ms"] for r in ok_runs if r["scenario"] == sid and r["bin"] == lb for k in r.get("keys", []) if k["phase"] == ph and k["lat_ms"] is not None]
                    lanes.append((f"{lb} n={len(v)}", v, colour[lb]))
                out.append(f"<div><b>keystroke to visible, all samples pooled, {ph}</b> <span class=muted>(ms)</span>" + toggled(lambda lg, lanes=lanes: strip_plot(lanes, "ms", lg, lane_h=40), ph) + "</div>")
                tb = "<table><tr><th></th><th>p50</th><th>p95</th><th>p99</th><th>max</th><th>unseen keys</th></tr>"
                for lb, v, _ in lanes:
                    unseen = sum(1 for r in ok_runs if r["scenario"] == sid and r["bin"] == lb for k in r.get("keys", []) if k["phase"] == ph and k["lat_ms"] is None)
                    tb += f"<tr><td>{esc(lb)}</td><td>{fmt(S.pct(v, .5))}</td><td>{fmt(S.pct(v, .95))}</td><td>{fmt(S.pct(v, .99))}</td><td>{fmt(max(v) if v else None)}</td><td>{unseen}</td></tr>"
                out.append(tb + "</table>")
        # histograms
        win = "stream" if sid in ("S1", "S2", "S5") else "idle" if sid in ("S10", "S10w", "S11") else None
        if win:
            iv, bp = [], []
            for lb in labels:
                rs = [r for r in ok_runs if r["scenario"] == sid and r["bin"] == lb]
                iv.append((lb, colour[lb], [x for r in rs for x in (S.get(r, f"{win}.frame_interval_ms") or [])]))
                bp.append((lb, colour[lb], [b for r in rs for _, b in (S.get(r, f"{win}.frame_list") or [])]))
            if any(v for _, _, v in iv):
                out.append("<h3>Frame interval</h3>" + histogram(iv, "ms between frame ends"))
                out.append("<h3>Bytes per frame</h3>" + histogram(bp, "bytes"))
        # time series of the median run
        if win in ("stream", "idle"):
            cpu_series, rss_series, marks = [], [], []
            for lb in labels:
                rs = [r for r in ok_runs if r["scenario"] == sid and r["bin"] == lb]
                rep = representative(rs)
                if not rep:
                    continue
                t0 = S.get(rep, f"{win}.t0_ms") or 0
                cpu_series.append((lb, colour[lb], cpu_curve(rep, t0)))
                rss_series.append((lb, colour[lb], rss_curve(rep, t0)))
                if win == "stream":
                    marks = [(0, "stream start"), ((S.get(rep, "stream.t1_ms") - t0) / 1000, f"end ({lb})")] if not marks else marks + [((S.get(rep, "stream.t1_ms") - t0) / 1000, f"end ({lb})")]
            xl = "seconds since " + ("stream start" if win == "stream" else "start of the idle window")
            out.append("<h3>CPU over time (median run per binary, 1 s window)</h3>" + line_chart(cpu_series, xl, "% of one core", marks=marks))
            out.append("<h3>RSS over time (same runs)</h3>" + line_chart(rss_series, xl, "MB", marks=marks, ymin=0))
        # replayer
        casts = {}
        for lb in labels:
            rs = [r for r in ok_runs if r["scenario"] == sid and r["bin"] == lb and r.get("cast")]
            rep = representative(rs)
            if rep and (d / "casts" / rep["cast"]).exists():
                casts[lb] = rep
        if len(casts) == len(labels) and sid in ("S1", "S2", "S5", "S9", "S11", "S10"):
            def span(rep):
                t0 = (rep.get("t_begin_ms") or rep.get("t_ready_ms") or 0) / 1000
                return max(0, t0 - 1.0)
            packed = []
            for lb in labels:
                rep = casts[lb]
                t_from = span(rep)
                t_to = t_from + min(16.0, (rep.get("duration_ms", 0) / 1000) - t_from)
                packed.append(pack(replay_snaps(d / "casts" / rep["cast"], t_from, t_to)))
            if len(packed) == 1:
                packed.append(packed[0])
            out.append("<h3>Side-by-side replay (text of the median run per binary, 8 frames/s, no colours)</h3>")
            out.append(f'<div><div class=player data-a="{packed[0]}" data-b="{packed[1]}"><pre class=pa></pre><pre class=pb></pre></div><div><button>play</button> <input type=range min=0 max=1000 value=0 style="width:70%"> <span class=tl></span> <span class=muted>left {esc(labels[0])}, right {esc(labels[-1])}; aligned at 1 s before the stream starts (startup scenarios: before first frame)</span></div></div>')
        out.append("</div>")

    # trend
    if history and history.exists():
        import trend

        out.append("<h2>Trend over milestones</h2>" + trend.section(history))
    # raw table
    out.append("<h2>Appendix: run table</h2><table><tr><th>scenario</th><th>bin</th><th>round</th><th>load</th><th>ok</th><th>key numbers</th></tr>")
    for r in runs:
        if r["round"] < 0:
            continue
        key = ""
        w = r.get("stream") or r.get("idle")
        if w:
            key = f'cpu {fmt(w.get("cpu_pct"))}%, {fmt(w.get("frames_per_s"))} fps, {fmt(w.get("bytes_per_s", 0) / 1000)} KB/s'
        if r.get("t_ready_ms"):
            key += f' ready {r["t_ready_ms"]:.0f} ms'
        out.append(f'<tr><td>{esc(r["scenario"])}</td><td>{esc(r["bin"])}</td><td>{r["round"]}</td><td>{r["load_before"]:.1f}</td><td>{"ok" if r.get("ok") else esc("; ".join(r["errors"]))}</td><td style="text-align:left">{esc(key)}</td></tr>')
    out.append("</table>")
    cmd = f'python3 -I perf/drive.py --a {labels[0].split(":")[1]}=<A> ' + (f'--b {labels[1].split(":")[1]}=<B> ' if ab else "") + f'--scenarios {",".join(scs)} --n {meta.get("n", "?")} --out <dir>{" --measure-only" if meta.get("measure_only") else ""}\npython3 -I perf/report.py <dir>'
    out.append(f"<h2>Appendix: commands</h2><pre>{esc(cmd)}</pre><p class=muted>Micro-benchmarks: none in this report (they would be labelled as such and never prove a speed claim).</p>")
    out.append(f"<script>{JS}</script></main></body>")
    return "\n".join(out)


METRIC_ORDER = ["S1", "S2", "S5", "S9", "S10", "S10w", "S11"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("dir")
    ap.add_argument("--history", help="history.jsonl for the trend section")
    args = ap.parse_args()
    d = Path(args.dir)
    page = build(d, Path(args.history) if args.history else None)
    (d / "report.html").write_text(page)
    print(f"wrote {d / 'report.html'} ({len(page) // 1024} KB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
