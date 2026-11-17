#!/usr/bin/env python3
"""Build tui-bug-classes.html from the bug-history CSVs.

Usage: build_report.py DATA_DIR OUT_HTML
Python stdlib only. All charts are inline SVG generated here; the page has no
scripts, no external fonts, no CDN. Light and dark via prefers-color-scheme.
"""
import csv
import html
import math
import re
import sys
import textwrap
from collections import Counter, OrderedDict, defaultdict
from html.parser import HTMLParser
from pathlib import Path

E = html.escape
REPO = "https://github.com/openai/codex"
WARN = []  # data inconsistencies found while building


def warn(msg):
    WARN.append(msg)
    print("WARN:", msg, file=sys.stderr)


# --------------------------------------------------------------------------- data
def read_csv(path):
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def one_line(s):
    return re.sub(r"\s+", " ", s or "").strip()


def load(d):
    d = Path(d)
    return dict(
        fix=read_csv(d / "fix-set.csv"),
        commits=read_csv(d / "commits.csv"),
        issues=read_csv(d / "issues.csv"),
        counts=read_csv(d / "subclass-counts.csv"),
    )


# ------------------------------------------------------------------ static texts
SHORT = {
    "SC1": "Stale async reply applied",
    "SC2a": "Thread-switch replay",
    "SC2b": "Per-thread widget state on switch",
    "SC3": "Wrong-thread routing",
    "SC4a": "Item rendered twice or split",
    "SC4b": "Distinct items, same content",
    "SC5a": "Transient state kept after turn end",
    "SC5b": "Data dropped at turn end",
    "SC6a": "Widget-held copies of state",
    "SC6b1": "Second source of truth",
    "SC6b2": "Refresh policy",
    "SC6c": "Hand-cached derived value",
    "SC7a": "Display flag set by hand",
    "SC7b": "Wrong gating predicate",
    "SC7c": "One bool for two outcomes",
    "SC8a": "Notification after response",
    "SC8b": "Local state ahead of backend",
    "SC9": "Input vs backend transition",
    "SC10a": "View state stale vs data",
    "SC10b": "Selection and order semantics",
    "SC11": "Blocking on the event path",
    "SC13": "Wrong or unscoped id",
}

MECH = {
    "SC1": "A reply is matched by a per-widget counter or not at all. When the chat widget is replaced (resume, thread switch) the counter restarts, and an aborted task's reply can already be queued, so a stale reply is applied to a newer request.",
    "SC2a": "One ChatWidget holds a thread's state. A switch rebuilds it and replays a bounded buffer, then the queued live events. Replay can handle an event twice, miss one the buffer evicted, or let the previous thread's queued events leak in.",
    "SC2b": "Draft, queued input, steer retries and the kill buffer live in the composer or widget, which is replaced on a switch. They are captured and restored by hand, and a path that forgets one loses it.",
    "SC3": "Handlers use the active thread id (self.thread_id.unwrap_or_default() is still at chatwidget/tool_requests.rs:298 and :322) instead of the id of the thread a request came from. Approval and request ids are unique only per thread or connection but are keyed alone.",
    "SC4a": "History is an append-only list of cells. When one item arrives as started plus completed, live plus replayed, or legacy plus v2 notifications, each path pushes a cell. Nothing keys a cell by (turn, item).",
    "SC4b": "The backend emits two distinct items with identical text (a commentary item and the final answer). The TUI renders both. The cause is in what the backend sends.",
    "SC5a": "Transient rows and prompts (hook row, Guardian reviews, MCP startup, pending questions) are separate fields cleared by explicit calls on the end paths. A path that is missed (success, interrupt, failure, finalize) leaves the row on screen.",
    "SC5b": "finalize_turn clears streaming state before the buffered answer or plan source was flushed, so the text is dropped.",
    "SC6a": "Child components (bottom pane, footer, popups, overlays) keep copies of model values, synced by set_* setters and refresh calls placed in event handlers. A constructor or code path that does not call the setter leaves the copy stale.",
    "SC6b1": "A value is read from a second source (auth.json on disk, a config default) instead of from the in-process owner, and the two diverge.",
    "SC6b2": "Remote data (rate limits, git branch, model catalogs) is cached with no rule for when to fetch it again. A config change or a resume leaves it stale or missing.",
    "SC6c": "A derived value (the /copy source, backtrack positions, the last final response) is cached by hand in state and updated by only some events.",
    "SC7a": "A display flag (status indicator visible) is set imperatively by event handlers and shares meaning with logic (task_running). Hiding the indicator also changed whether input was queued.",
    "SC7b": "The predicate that blocks the input queue is computed from a shared running indicator that other states (MCP startup, review mode) also set.",
    "SC7c": "A bottom-pane view reports one complete bool for both accepted and cancelled, so the parent menu re-opens in a loop.",
    "SC8a": "Code needs an entity from a thread/started notification, which can arrive after the response to the request that created it.",
    "SC8b": "Local state is changed ahead of backend confirmation: the transcript is trimmed before a rollback confirms, or a steer is sent with a cached turn id the server has already replaced.",
    "SC9": "A user action (Enter, submit) is processed while the backend is mid-transition (final answer streaming, session not yet configured), because input handlers run outside the ordered event stream.",
    "SC10a": "View state is tied to position or to the clock: a selection by index survives a filter change, 'No sessions yet' is shown while loading, relative timestamps are recomputed on every redraw.",
    "SC10b": "After data changes, selection or order follows the wrong rule: removing the selected row jumps the selection, re-sorting by changing timestamps moves rows on every update.",
    "SC11": "Blocking IO or a blocking session start runs on the event-handling path, so progress events and input are not consumed meanwhile.",
    "SC13": "An id of the wrong kind or scope is used (call_id for approval_id, turn-level for request-level, an item id unique only per turn) because all of them are plain strings.",
}

# rules: id, name, what, enforced by, sub-classes
RULES = [
    ("R1", "Ticketed replies", "Replies to app-issued requests are opaque values resolved only through a pending-request ticket that checks the id. Task spawning is allowed only in the shell module, and its continuation only forwards to apply. Transport events carry a per-thread attach epoch, and one ingress router drops events from earlier epochs.", "module privacy; clippy disallowed_methods on spawn outside the shell", "SC1"),
    ("R2", "Per-thread data in one place", "Per-thread data (draft, queue, status, permissions, pending requests) lives only in a ThreadModel. Per-thread views are built through one thread_scope(thread_id) helper.", "mandatory switch round-trip property test (state in A, then A to B to A: A unchanged, B shows only B)", "SC2a, SC2b"),
    ("R3", "Thread-scoped apply", "Thread events are applied by apply(&mut ThreadModel, ev, &mut ThreadFx). Thread-scoped requests can only be built through ThreadFx, which stamps the thread's own id. Pending requests are keyed by (ThreadId, RequestId). User intents carry the target thread captured at dispatch.", "types (no active-thread access inside thread apply)", "SC3"),
    ("R4", "Keyed transcript", "The transcript is an ordered map keyed by (TurnId, ItemId) with upsert: started, delta and completed update one entry. One projection from protocol events drops legacy duplicates.", "type API (there is no push)", "SC4a"),
    ("R5", "Transient state inside the lifecycle", "enum TurnState { Idle, Running(RunningTurn{hooks, reviews, prompts, mcp_startup, streams, ...}) }. The end transition takes RunningTurn by value.", "types; end-path matrix test (complete, interrupt, fail)", "SC5a, SC5b"),
    ("R6", "Remote<T>", "Remote data is Loading, Loaded{value, fetched_at, generation} or Failed. Age becomes visible; the refresh policy stays product logic.", "types", "SC6b2, SC10a"),
    ("R7", "Derived values are functions", "Derived values are functions of the model and never stored. A cache is a memo on a versioned value or is checked by Model::assert_consistent(), run by the test harness after every event.", "test harness", "SC6c, SC7a"),
    ("R8", "Lifecycles and outcomes as enums", "Lifecycles are enums, predicates (can_submit, queue_blocked) are functions of them, outcomes are enums and not bools.", "lifecycle x intent table test", "SC7a, SC7b, SC7c"),
    ("R9", "Upsert and confirm", "An entity is created or upserted from whichever of response or notification arrives first. Local mutations wait for confirmation (a pending operation is a model state).", "event-permutation property test per feature", "SC8a, SC8b"),
    ("R10", "Intents in the ordered stream", "Intents that touch backend-shared or cross-thread state (submit, steer, interrupt, approve, switch thread) are values in the same ordered stream as backend events. Local typing, scrolling and popup navigation stay local.", "permutation test interleaving intents and events", "SC9"),
    ("R11", "Keyed lists, clock as input", "Lists use a keyed selection type, not an index. The clock enters as a model input.", "lint disallowed_methods for Instant::now / SystemTime::now in view modules", "SC10a"),
    ("R12", "No IO in apply or views", "IO happens only in effects run by the shell.", "lint disallowed_methods (std::fs, blocking calls) in apply and view modules; responsiveness gate", "SC6b1, SC11"),
    ("R13", "One newtype per id kind", "ThreadId, TurnId, ItemId, CallId, ApprovalId, ReviewId, RequestId; composite keys where an id is unique only in a scope.", "types", "SC13"),
    ("R14", "App data never in widget-local state", "Application data never enters the UI framework's own local state (observable cells, hook state); those are allowed only in an allowlist of UI-only modules.", "clippy disallowed_types", "SC6a"),
]

RULE_OF = {  # sub-class -> rule label shown on its card
    "SC1": "R1", "SC2a": "R2", "SC2b": "R2", "SC3": "R3", "SC4a": "R4", "SC4b": "none (backend semantics)",
    "SC5a": "R5", "SC5b": "R5", "SC6a": "R14", "SC6b1": "R12", "SC6b2": "R6", "SC6c": "R7",
    "SC7a": "R7, R8", "SC7b": "R8", "SC7c": "R8 (convention)", "SC8a": "R9", "SC8b": "R9", "SC9": "R10",
    "SC10a": "R6, R11", "SC10b": "none (product rule; list tests)", "SC11": "R12", "SC13": "R13",
}

# primary class (subject keyword) groups for the monthly chart
GROUPS = [
    ("other", ["other"], "other (backend, config, CI, product text)"),
    ("state_ui", ["state_ui"], "state_ui"),
    ("race", ["race"], "race"),
    ("keys", ["keys"], "keys"),
    ("compat", ["compat"], "compat (terminals)"),
    ("wrap_rect", ["wrap_rect"], "wrap_rect"),
    ("render", ["render"], "render"),
    ("paste", ["paste"], "paste"),
    ("small", ["scrollback", "link", "sched_perf", "unicode", "suspend", "resize", "image", "cursor"],
     "8 small classes"),
]
GROUP_CLASS = {k: f"s{i}" for i, (k, _, _) in enumerate(GROUPS)}
GROUP_OF = {c: k for k, cs, _ in GROUPS for c in cs}

AREA_LABEL = OrderedDict([
    ("controller_state", "App and chat controller (app/, chatwidget, app_server_*, app_event)"),
    ("other_ui", "Other UI modules (onboarding, pickers, status, ...)"),
    ("bottompane_state", "Bottom pane (composer, popups, footer)"),
    ("transcript", "Transcript and history cells"),
    ("terminal_layer", "Terminal layer (custom_terminal, insert_history, tui.rs, clipboard)"),
    ("layout_text", "Layout, text, wrap, render"),
    ("input_keys", "Input, keys, paste"),
])

VERDICT_KEYS = ["IMPOSSIBLE", "CAUGHT", "STILL POSSIBLE*", "STILL POSSIBLE"]
VERDICT_CLASS = {"IMPOSSIBLE": "v-imp", "CAUGHT": "v-cau", "STILL POSSIBLE*": "v-psx", "STILL POSSIBLE": "v-pos"}
VERDICT_SHORT = {"IMPOSSIBLE": "impossible", "CAUGHT": "caught", "STILL POSSIBLE*": "possible*", "STILL POSSIBLE": "possible"}
VERDICT_NAME = {
    "IMPOSSIBLE": "impossible (type or lint rejects it)",
    "CAUGHT": "caught (a mandatory test fails)",
    "STILL POSSIBLE*": "still possible, mechanism absent (*)",
    "STILL POSSIBLE": "still possible",
}


# ---------------------------------------------------------------------- SVG kit
def svg_open(w, h, label, extra=""):
    return (f'<svg class="chart" viewBox="0 0 {w} {h}" role="img" aria-label="{E(label)}" '
            f'preserveAspectRatio="xMidYMid meet"{extra}>')


def legend(items):
    """items: [(css class, text)] -> HTML legend."""
    return '<div class="legend">' + "".join(
        f'<span class="lg"><i class="sw {c}"></i>{E(t)}</span>' for c, t in items) + "</div>"


def hbars(rows, series, title, width=460, mode="above", label_w=56, row_h=None, bar_h=16,
          max_total=None, normalize=False, total_fmt=None, inside_min=17):
    """Horizontal (stacked) bars.

    rows: list of dict(label, vals, tip=None, href=None); series: [(css, name)]
    mode 'left': label at left of bar; 'above': label line above each bar.
    """
    pad_r = 34
    if mode == "left":
        x0 = label_w
        row_h = row_h or 22
        top = 4
    else:
        x0 = 4
        row_h = row_h or 40
        top = 4
    bw = width - x0 - pad_r
    totals = [sum(r["vals"]) for r in rows]
    mx = max_total or max(totals or [1])
    if normalize:
        mx = 1
    h = top + row_h * len(rows) + 4
    out = [svg_open(width, h, title)]
    for i, r in enumerate(rows):
        y = top + i * row_h
        tot = totals[i]
        if mode == "left":
            by = y + (row_h - bar_h) / 2
            ly = by + bar_h / 2 + 4
            out.append(f'<text class="t" x="{x0 - 6}" y="{ly:.1f}" text-anchor="end">{E(r["label"])}</text>')
        else:
            by = y + 20
            lab = E(r["label"])
            if r.get("href"):
                lab = f'<a href="{E(r["href"])}">{lab}</a>'
            out.append(f'<text class="t" x="{x0}" y="{y + 13}">{lab}</text>')
        x = x0
        scale = (bw / (tot if tot else 1)) if normalize else bw / mx
        for (css, name), v in zip(series, r["vals"]):
            if v <= 0:
                continue
            wseg = v * scale
            out.append(f'<rect class="{css}" x="{x:.1f}" y="{by:.1f}" width="{max(wseg - 0.8, 0.5):.1f}" '
                       f'height="{bar_h}" rx="2"><title>{E(r["label"])}: {E(name)} {v}</title></rect>')
            if wseg >= inside_min:
                out.append(f'<text class="segt" x="{x + wseg / 2:.1f}" y="{by + bar_h / 2 + 4:.1f}" '
                           f'text-anchor="middle">{v}</text>')
            x += wseg
        tl = total_fmt(tot, r) if total_fmt else str(tot)
        out.append(f'<text class="tm" x="{x + 5:.1f}" y="{by + bar_h / 2 + 4:.1f}">{E(tl)}</text>')
    out.append("</svg>")
    return "".join(out)


def simple_bars(rows, title, css="s1", **kw):
    kw.setdefault("inside_min", 9999)
    return hbars([dict(label=l, vals=[v]) for l, v in rows], [(css, "commits")], title, **kw)


# ------------------------------------------------------------------ box diagrams
class Diagram:
    """Flow-layout box diagram. Boxes stack top to bottom; text wraps by width."""

    def __init__(self, width, label, marker):
        self.w = width
        self.label = label
        self.marker = marker
        self.parts = []
        self.y = 8
        self.fs = 12.5
        self.lh = 16

    def _wrap(self, text, w):
        n = max(8, int((w - 16) / (self.fs * 0.55)))
        return textwrap.wrap(text, n) or [""]

    def box(self, x, w, title, body="", tags="", cls="box", y=None, minh=0):
        y = self.y if y is None else y
        lines = [("b", t) for t in self._wrap(title, w)]
        if body:
            lines += [("n", t) for t in self._wrap(body, w)]
        if tags:
            lines += [("g", t) for t in self._wrap(tags, w)]
        h = max(minh, 12 + len(lines) * self.lh)
        self.parts.append(f'<rect class="{cls}" x="{x}" y="{y}" width="{w}" height="{h}" rx="6"/>')
        for i, (k, t) in enumerate(lines):
            tcls = {"b": "tb", "n": "t", "g": "tg"}[k]
            self.parts.append(f'<text class="{tcls}" x="{x + 8}" y="{y + 18 + i * self.lh}">{E(t)}</text>')
        return y, h

    def height(self, w, title, body="", tags=""):
        n = len(self._wrap(title, w)) + (len(self._wrap(body, w)) if body else 0) + (len(self._wrap(tags, w)) if tags else 0)
        return 12 + n * self.lh

    def arrow(self, x1, y1, x2, y2, dashed=False):
        d = ' stroke-dasharray="5 4"' if dashed else ""
        self.parts.append(f'<line class="ar" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}"{d} '
                          f'marker-end="url(#{self.marker})"/>')

    def path(self, d, dashed=True):
        da = ' stroke-dasharray="5 4"' if dashed else ""
        self.parts.append(f'<path class="ar" d="{d}" fill="none"{da} marker-end="url(#{self.marker})"/>')

    def render(self):
        h = self.y + 8
        defs = (f'<defs><marker id="{self.marker}" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" '
                f'markerHeight="7" orient="auto-start-reverse"><path class="ah" d="M0,0 L10,5 L0,10 z"/></marker></defs>')
        return svg_open(self.w, h, self.label) + defs + "".join(self.parts) + "</svg>"


def diagram_today():
    d = Diagram(460, "How the main state-bug mechanisms arise in today's code", "arT")
    sw = 140
    xs = [10, 160, 310]
    ys = []
    hs = []
    for x, t in zip(xs, ["App-server notifications", "Keys and slash commands", "Async task replies"]):
        y, h = d.box(x, sw, t)
        ys.append(y)
        hs.append(h)
    d.y += max(hs) + 26
    top = d.y
    for x in xs:
        d.arrow(x + sw / 2, top - 24, 230, top, False)
    y, h = d.box(10, 440, "One AppEvent bus", "335 AppEvent variants dispatched by App::handle_event. Handlers also mutate widget fields directly from key events.")
    d.y += h + 26
    ctrl = d.y
    d.arrow(230, ctrl - 24, 230, ctrl)
    y, h = d.box(10, 440, "ChatWidget and BottomPane", "Controller and widgets in one place: handlers write fields and call set_* setters. One widget is rebuilt on a thread switch.")
    d.y += h + 30
    colt = d.y
    specs = [
        ("Widget-held copies of state", "Footer, popups, overlays keep their own copy; synced by setters and refresh calls.", "SC6a SC6c SC7a SC5a"),
        ("Async replies without ids", "Counter per widget; a reply is applied if it still looks current.", "SC1 SC8 SC9 SC11"),
        ("Per-thread replay", "Widget rebuilt from a bounded buffer; the active thread id used for routing.", "SC2 SC3 SC4a"),
    ]
    hh = []
    mh = max(d.height(sw, t, b, g) for t, b, g in specs)
    for x, (t, b, g) in zip(xs, specs):
        d.arrow(x + sw / 2, colt - 28, x + sw / 2, colt)
        y, h = d.box(x, sw, t, b, g, cls="box bad", minh=mh)
        hh.append(h)
    d.y = colt + max(hh) + 30
    # equalise heights is not needed; arrows to the screen box
    scr = d.y
    for x in xs:
        d.arrow(x + sw / 2, colt + max(hh), 230, scr, True)
    d.box(10, 440, "Screen painted from the copies", "Stale row, duplicated cell, wrong thread's data on screen.", "", cls="box bad")
    d.y += 12 + 16 * 3 + 6
    return d.render()


def diagram_target():
    d = Diagram(460, "Target: events and intents applied per thread to one model, views derived", "arG")
    x, w = 30, 390
    d.y = 8
    y1, h1 = d.box(x, w, "Events and intents (values)", "Backend notifications, user intents that touch shared state, effect results. One ordered stream; each item tagged with ThreadId and attach epoch.", "R3 R10", cls="box good")
    d.y += h1 + 26
    d.arrow(x + w / 2, d.y - 26, x + w / 2, d.y)
    y2, h2 = d.box(x, w, "Ingress router", "Drops events from earlier attach epochs.", "R1", cls="box good")
    d.y += h2 + 26
    d.arrow(x + w / 2, d.y - 26, x + w / 2, d.y)
    y3, h3 = d.box(x, w, "apply(&mut ThreadModel, event), per thread", "Reply ids checked by ticket; upsert by (TurnId, ItemId); lifecycle enum; typed ids; effects stamped with the thread's own id.", "R1 R3 R4 R5 R8 R9 R13", cls="box good")
    d.y += h3 + 26
    d.arrow(x + w / 2, d.y - 26, x + w / 2, d.y)
    y4, h4 = d.box(x, w, "One model", "ThreadModel per thread, Remote<T> for fetched data, derived values as functions of the model.", "R2 R6 R7", cls="box good")
    d.y += h4 + 26
    d.arrow(x + w / 2, d.y - 26, x + w / 2, d.y)
    y5, h5 = d.box(x, w, "Views derived from the model", "No copies in widgets, keyed selection, no clock or IO in views.", "R11 R12 R14", cls="box good")
    d.y += h5 + 40
    y6, h6 = d.box(x, w, "Effects (IO, spawn) run by the shell", "A request returns with a ticket; its reply re-enters the stream as an event. apply and views do no IO.", "R1 R12", cls="box")
    # side loops: apply -> effects (right), effects -> events (left)
    ymid = y3 + h3 / 2
    ye = y6 + h6 / 2
    d.path(f"M {x + w} {ymid} L 445 {ymid} L 445 {ye} L {x + w} {ye}")
    d.path(f"M {x} {ye} L 14 {ye} L 14 {y1 + h1 / 2} L {x} {y1 + h1 / 2}")
    d.y = y6 + h6 + 6
    return d.render()


# ----------------------------------------------------------------- build parts
def pct(a, b):
    return f"{100 * a / b:.1f}%"


def wilson(k, n, z=1.96):
    p = k / n
    den = 1 + z * z / n
    c = (p + z * z / (2 * n)) / den
    hw = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / den
    return c - hw, c + hw


def is_state(sc):
    return sc.startswith("SC") and sc != "SC14"


def commit_link(r):
    return f'{REPO}/commit/{r["full_sha"]}'


def build(data):
    fix, commits, issues, counts = data["fix"], data["commits"], data["issues"], data["counts"]
    fix_by = {r["hash"]: r for r in fix}
    N = len(fix)
    dates = sorted(r["date"] for r in fix)
    d0, d1 = dates[0], dates[-1]

    # ---- consistency checks -------------------------------------------------
    if len({r["hash"] for r in fix}) != N:
        warn("duplicate hashes in fix-set.csv")
    cm_fix = [r for r in commits if r["set"] == "fix-set"]
    cm_out = [r for r in commits if r["set"] == "outside-fix-set"]
    for r in cm_fix:
        f = fix_by.get(r["hash"])
        if f is None:
            warn(f"commit {r['hash']} marked fix-set but absent from fix-set.csv")
        elif f["state_subclass"] != r["subclass"]:
            warn(f"sub-class mismatch for {r['hash']}")
    for r in cm_out:
        if r["hash"] in fix_by:
            warn(f"commit {r['hash']} marked outside-fix-set but present in fix-set.csv")
    if any(r["verdict_M"] != r["verdict_mix_ADR003"] for r in commits + issues):
        warn("verdict_M and verdict_mix differ on some rows (the report merges them)")
    cnt = {r["subclass"]: r for r in counts}
    rc = Counter(r["subclass"] for r in cm_fix)
    ro = Counter(r["subclass"] for r in cm_out)
    ri = Counter(r["subclass"] for r in issues)
    for sc, r in cnt.items():
        if (int(r["fix_set"]), int(r["outside_fix_set"]), int(r["issues"])) != (rc[sc], ro[sc], ri[sc]):
            warn(f"subclass-counts.csv disagrees with row data for {sc}")
    ids = [sc for sc in cnt if is_state(sc)]
    nums = sorted(int(re.sub(r"\D", "", s.split("SC")[1][:2])) for s in ids)
    missing = [n for n in range(1, max(nums) + 1) if n not in nums]
    if missing:
        warn("sub-class numbering has gaps: " + ", ".join(f"SC{n}" for n in missing))

    st_fix = sum(rc[s] for s in ids)
    st_out = sum(ro[s] for s in ids)
    st_iss = sum(ri[s] for s in ids)

    def tally(rows, key):
        c = Counter(r[key] for r in rows if is_state(r["subclass"]))
        return [c.get(k, 0) for k in VERDICT_KEYS]

    after_fix = tally(cm_fix, "verdict_after_rules")
    after_out = tally(cm_out, "verdict_after_rules")
    after_iss = tally(issues, "verdict_after_rules")
    mix_fix = tally(cm_fix, "verdict_mix_ADR003")
    mix_out = tally(cm_out, "verdict_mix_ADR003")
    mix_iss = tally(issues, "verdict_mix_ADR003")
    r_fix = tally(cm_fix, "verdict_R")
    r_out = tally(cm_out, "verdict_R")
    r_iss = tally(issues, "verdict_R")
    imp, cau = after_fix[0], after_fix[1]
    still = after_fix[2] + after_fix[3]
    sp_fix = Counter(r["subclass"] for r in cm_fix if is_state(r["subclass"]))
    top3 = sum(rc[s] for s in ("SC2a", "SC6a", "SC7a"))
    top3i = sum(ri[s] for s in ("SC2a", "SC6a", "SC7a"))

    if cnt.get("SC3", {}).get("F5_actors") == "IMPOSSIBLE":
        warn("SC3 under actors: data says IMPOSSIBLE; research text section 5 says STILL POSSIBLE (address picked from the active thread)")
    if cnt.get("SC7a", {}).get("F2_store") == "STILL POSSIBLE*":
        warn("SC7a under store/reducer library: data says STILL POSSIBLE*; research text section 5 says STILL POSSIBLE")
    if cnt.get("SC11", {}).get("F5_actors") == "STILL POSSIBLE":
        warn("SC11 under actors: data says STILL POSSIBLE; research text section 5 notes two-writer races are gone inside an actor")
    # state_ui + race decomposition (joined on hash)
    dec = Counter()
    for r in cm_fix:
        f = fix_by.get(r["hash"])
        if f and f["primary_class_by_subject"] in ("state_ui", "race"):
            sc = r["subclass"]
            dec["state" if is_state(sc) else ("feature" if sc == "SC14" else sc)] += 1
    n_sr = sum(1 for f in fix if f["primary_class_by_subject"] in ("state_ui", "race"))
    if sum(dec.values()) != n_sr:
        warn(f"state_ui+race: {n_sr} commits but {sum(dec.values())} classified")

    # monthly
    months = sorted({r["date"][:7] for r in fix})
    allm = []
    y, m = int(months[0][:4]), int(months[0][5:])
    while f"{y:04d}-{m:02d}" <= months[-1]:
        allm.append(f"{y:04d}-{m:02d}")
        m += 1
        if m > 12:
            y, m = y + 1, 1
    mc = defaultdict(Counter)
    for r in fix:
        mc[r["date"][:7]][GROUP_OF.get(r["primary_class_by_subject"], "small")] += 1
    mtot = {k: sum(v.values()) for k, v in mc.items()}
    peak = max(mtot, key=mtot.get)
    low = [k for k in allm if mtot.get(k, 0) <= 3]
    for k in [k for k in low if k != allm[0]]:
        warn(f"month {k} has only {mtot.get(k, 0)} fix commits next to months with 13+ (history gap or real lull)")
    pclass = Counter(r["primary_class_by_subject"] for r in fix)
    if sum(pclass.values()) != N:
        warn("class counts do not sum to fix-set size")

    # areas and modules
    area_c = Counter(a for r in fix for a in r["areas"].split(";") if a)
    no_area = sum(1 for r in fix if not r["areas"])
    mod_c = Counter()
    for r in fix:
        mods = {re.sub(r"^tui2:", "", t) for t in r["files_top"].split(";") if t}
        for t in mods:
            if t in ("snapshots", "tui/Cargo.toml", "tui/tests"):
                continue
            mod_c[t] += 1

    # ------------------------------------------------------------ HTML pieces
    H = []
    add = H.append

    # summary
    shares = [("GONE", 53), ("SHRINKS", 139), ("STAYS", 316)]
    summary = [
        f"<strong>{N} bug-fix commits</strong> touched codex-rs/tui and tui2 between {d0} and {d1} (of 3,612 non-merge commits since 2025-04-24). The busiest month is {peak} with {mtot[peak]}.",
        f"I read <strong>{len(cm_fix)} of them, {len(cm_out)} more outside the subject filter, and 42 public issues</strong>. {st_fix} fix-set commits, {st_out} outside commits and {st_iss} issues are state-mechanism bugs. The rest are layout, terminal, text, backend and non-runtime changes.",
        f"Three mechanisms cause {top3} of the {st_fix} commits and {top3i} of the {st_iss} issues: a widget rebuilt and replayed on thread switch (SC2a), model state copied into child widgets (SC6a), and display flags set by hand (SC7a).",
        f"With 14 design rules (R1 to R14), <strong>{imp} of {st_fix} commits become impossible</strong> by type or lint, <strong>{cau} are caught</strong> by a test the rules require, and {still} stay possible. For issues the split is {after_iss[0]} / {after_iss[1]} / {after_iss[2] + after_iss[3]}.",
        f"As a share of all {N} fix commits that is {pct(imp, N)} removed and {pct(cau, N)} caught. A retained-mode widget framework alone removes about 10% of the {N} as a class (terminal and layout mechanics), shrinks about 27%, and leaves about 62%, with an error of about +-5 points.",
    ]
    add('<header class="hero"><p class="eyebrow">openai/codex, codex-rs/tui</p>'
        "<h1>Codex TUI bug history and the state-design rules that remove bug classes</h1>"
        f'<p class="sub">508 bug-fix commits, {d0} to {d1}. Which bugs came from where state lives, and which design rules would make each class impossible, caught by a test, or leave it open.</p>'
        '<ol class="summary">' + "".join(f"<li>{s}</li>" for s in summary) + "</ol>"
        '<nav class="toc"><a href="#history">1 Fix history</a><a href="#state">2 State sub-classes</a>'
        '<a href="#mechanisms">3 How they arise</a><a href="#rules">4 Rules</a><a href="#effect">5 Before and after</a>'
        '<a href="#approaches">6 Approaches</a><a href="#method">7 Method and limits</a></nav></header>')

    # --- 1 history
    add('<section id="history"><h2>1. The whole fix history</h2>')
    add(f"<p>A fix commit here is a non-merge commit touching <code>codex-rs/tui</code> or <code>codex-rs/tui2</code> whose subject contains a bug keyword (fix, revert, stale, duplicate, race, panic, crash, hang, flicker, garbled and similar). Each commit has one primary class, set by an ordered regex on the subject.</p>")
    add("<h3>Fix commits per month, by primary class</h3>")
    add(legend([(GROUP_CLASS[k], lab) for k, _, lab in GROUPS]))
    rows = []
    for k in allm:
        rows.append(dict(label=k, vals=[mc[k].get(g, 0) for g, _, _ in GROUPS]))
    add('<div class="fig">' + hbars(rows, [(GROUP_CLASS[g], lab) for g, _, lab in GROUPS],
                                    "Bug-fix commits per month by primary class", mode="left", label_w=64,
                                    row_h=21, bar_h=15, inside_min=15) + "</div>")
    last = allm[-1]
    add(f'<p class="note">{last} is partial (data ends {d1}). The 8 small classes are: scrollback, link, sched_perf, unicode, suspend, resize, image, cursor. Counts for all 16 classes:</p>')
    add('<details><summary>All 16 primary classes</summary><div class="tw"><table class="data"><thead><tr><th>class</th><th>commits</th><th>share</th></tr></thead><tbody>'
        + "".join(f"<tr><td>{E(c)}</td><td>{n}</td><td>{pct(n, N)}</td></tr>" for c, n in pclass.most_common())
        + "</tbody></table></div></details>")

    add("<h3>Fix commits by area</h3>")
    add("<p>Areas group the files a commit touches. One commit can touch several areas, so the bars add up to more than the commit count.</p>")
    arows = [(AREA_LABEL.get(a, a), area_c[a]) for a in AREA_LABEL if a in area_c]
    arows.sort(key=lambda t: -t[1])
    add('<div class="fig">' + simple_bars(arows, "Fix commits by area", css="s3", row_h=38) + "</div>")
    add(f'<p class="note">{no_area} commits have no area attribution.</p>')
    add("<h3>Fix commits by module (first path component under tui/src)</h3>")
    mrows = mod_c.most_common(12)
    add('<div class="fig">' + simple_bars([(m, n) for m, n in mrows], "Fix commits by module", css="s3", mode="left", label_w=150, row_h=22, bar_h=15) + "</div>")
    add("<p class=\"note\">tui2 files are folded into the same module name. Snapshots, tests and Cargo.toml are left out.</p>")

    add("<h3>What a retained widget framework does to each fix class</h3>")
    add(f"<p>Share of the {N} commits by verdict. The numbers are estimates from the class table in <code>bug-classes.md</code> (about +-5 points), not counts of reviewed commits.</p>")
    add(legend([("v-imp", "gone with a retained widget framework"), ("v-psx", "shrinks"), ("v-pos", "stays")]))
    vrows = [dict(label=f"All {N} fix commits (estimate)", vals=[a for _, a in shares])]
    add('<div class="fig">' + hbars(vrows, [("v-imp", "gone"), ("v-psx", "shrinks"), ("v-pos", "stays")],
                                    "Share of fix commits by verdict", normalize=True, row_h=44, bar_h=24, inside_min=26,
                                    total_fmt=lambda t, r: "") + "</div>")
    add('<div class="tw"><table class="data"><thead><tr><th>verdict</th><th>share</th><th>what is in it</th></tr></thead><tbody>'
        "<tr><td><span class='pill v-imp'>gone</span></td><td>about 10% (about 53)</td><td>cursor placement, suspend and restore, manual Rect, height and clip arithmetic and zero-area panics, partial or doubled rows and stale cells, clipboard and notification plumbing, key-sequence repair</td></tr>"
        "<tr><td><span class='pill v-psx'>shrinks</span></td><td>about 27% (about 139)</td><td>terminal quirks (tmux, zellij, kitty, Windows), Unicode width and graphemes, text wrap, inline scrollback and resize, paste, images, parts of rendering</td></tr>"
        "<tr><td><span class='pill v-pos'>stays</span></td><td>about 62% (about 316)</td><td>state desync after events, event-order races, key routing, composer logic, markdown and diff semantics, backend and product changes</td></tr>"
        "</tbody></table></div>")
    add(f"<p>The 'stays' group contains the {n_sr} commits whose subject class is state_ui or race ({pct(n_sr, N)}). Reading them shows that only <strong>{dec['state']}</strong> are state-mechanism bugs. The others are {dec['N']} non-runtime changes (tests, fixtures, build, copy, reverts), {dec['B']} backend, persistence or protocol, {dec['L']} layout, style or input, and {dec['feature']} feature-logic errors.</p>")
    drows = [dict(label="state_ui + race commits (%d)" % n_sr,
                  vals=[dec["state"], dec["feature"], dec["B"], dec["L"], dec["N"]])]
    add(legend([("s1", "state mechanism"), ("s6", "feature logic"), ("s3", "backend / protocol"), ("s5", "layout / style / input"), ("s0", "not a runtime bug")]))
    add('<div class="fig">' + hbars(drows, [("s1", "state mechanism"), ("s6", "feature logic"), ("s3", "backend / protocol"), ("s5", "layout / style / input"), ("s0", "not a runtime bug")],
                                    "Composition of the state_ui and race commits", row_h=44, bar_h=24, inside_min=14,
                                    total_fmt=lambda t, r: "") + "</div>")
    add("</section>")

    # --- 2 state sub-classes
    add('<section id="state"><h2>2. State-mechanism bugs</h2>')
    add(f"<p>The {st_fix} fix-set commits, {st_out} outside-filter commits and {st_iss} issues below share a pattern: two parts of the program disagree about the state of the world. Each sub-class has one root-cause mechanism. A commit counts once, under its primary sub-class. Feature-logic errors in a state transition (SC14, {rc['SC14']} commits) are not a state mechanism and are left out of the cards.</p>")
    add(legend([("s1", "commits in the fix set"), ("s4", "commits outside the fix set"), ("s2", "issues")]))
    order = sorted(ids, key=lambda s: (-(rc[s] + ro[s] + ri[s]), s))
    srows = [dict(label=f"{s}  {SHORT[s]}", vals=[rc[s], ro[s], ri[s]], href=f"#{s}") for s in order]
    add('<div class="fig">' + hbars(srows, [("s1", "fix set"), ("s4", "outside fix set"), ("s2", "issues")],
                                    "State-mechanism sub-classes: commits and issues", row_h=38, bar_h=16, inside_min=14) + "</div>")
    add('<p class="note">Bars are counts of items read, not estimates of the 508. Names link to the cards below.</p>')

    def pick_examples(sc):
        cf = [r for r in cm_fix if r["subclass"] == sc]
        co = [r for r in cm_out if r["subclass"] == sc]
        ii = [r for r in issues if r["subclass"] == sc]
        cs = cf[:2] + co[:1]
        cs += [r for r in cf[2:] + co[1:] if r not in cs]
        chosen_c = cs[:3]
        chosen_i = ii[:2]
        rest_c = [r for r in cs if r not in chosen_c]
        rest_i = [r for r in ii if r not in chosen_i]
        out = [("c", r) for r in chosen_c] + [("i", r) for r in chosen_i]
        for kind, pool in (("c", rest_c), ("i", rest_i)):
            for r in pool:
                if len(out) < 5:
                    out.append((kind, r))
        return out

    full_rule = {r["subclass"]: r["rule"] for r in counts}
    for sc in ids:
        r = cnt[sc]
        before = r["mix_ADR003"]
        after = r["after_rules"]
        red = r["R"]
        ex = pick_examples(sc)
        tot = rc[sc] + ro[sc] + ri[sc]
        add(f'<article class="card" id="{sc}"><header><h3><span class="id">{sc}</span> {E(SHORT[sc])}</h3>'
            f'<p class="cnt">{rc[sc]} fix-set commit{"" if rc[sc] == 1 else "s"}, {ro[sc]} outside the filter, {ri[sc]} issue{"" if ri[sc] == 1 else "s"}</p></header>')
        add(f'<p class="mech">{E(MECH[sc])}</p>')
        add('<ul class="ex">')
        for kind, e in ex:
            if kind == "c":
                tag = "" if e["set"] == "fix-set" else ' <span class="tag">outside filter</span>'
                add(f'<li><a href="{commit_link(e)}"><code>{E(e["hash"])}</code></a> {E(one_line(e["subject"]))[:120]}{tag}'
                    f'<br><span class="why">{E(one_line(e["root_cause"]))[:170]}</span></li>')
            else:
                add(f'<li><a href="{REPO}/issues/{e["number"]}"><code>#{e["number"]}</code></a> {E(one_line(e["title"]))[:110]} '
                    f'<span class="tag">{E(e["state"])}</span><br><span class="why">{E(one_line(e["mechanism"]))[:170]}</span></li>')
        add("</ul>")
        if tot < 3:
            add(f'<p class="note">Only {tot} item{"s" if tot != 1 else ""} in the read sample.</p>')
        add('<div class="rule"><div class="rl">Rule: <strong>' + E(RULE_OF[sc]) + "</strong></div>"
            f'<p class="rt">{rule_text(sc, full_rule[sc])}</p>'
            f'<div class="vd">Plain model without rules: {vpill(before)} &rarr; with rules: {vpill(after)}'
            f'<span class="small"> (hand-written reducer alone: {vpill(red)})</span></div></div></article>')
    add("</section>")

    # --- 3 diagram
    add('<section id="mechanisms"><h2>3. How the main mechanisms arise, and the target shape</h2>')
    add("<p>Left: the shape of today's code. Events from several sources go through one bus into a controller that also owns widgets. Three mechanisms (red) produce most state bugs. Right: the target shape. Every change is a value in one ordered stream, applied per thread with ids checked, into one model; views are derived from it. Rule numbers (R1 to R14) refer to section 4.</p>")
    add('<div class="duo"><figure>' + diagram_today() + "<figcaption>Today</figcaption></figure>"
        "<figure>" + diagram_target() + "<figcaption>Target</figcaption></figure></div>")
    add("<p class=\"note\">A single dispatcher is not what removes the classes: the code already sends 335 AppEvent variants through App::handle_event and has every sub-class. Where state lives, and the types used for ids, threads, lifecycles and transcripts, decide.</p></section>")

    # --- 4 rules
    add('<section id="rules"><h2>4. The rules</h2>')
    add("<p>Each rule is named after what makes it hold: a type, a lint, or a test the design makes mandatory. <em>Impossible</em> means the compiler or a CI lint rejects the program; <em>caught</em> means a required test fails.</p>")
    add('<div class="tw"><table class="data rules"><thead><tr><th>rule</th><th>what</th><th>held by</th><th>removes</th></tr></thead><tbody>')
    for rid, name, what, how, scs in RULES:
        links = ", ".join(f'<a href="#{s.strip()}">{s.strip()}</a>' for s in scs.split(","))
        add(f"<tr><td><strong>{rid}</strong><br>{E(name)}</td><td>{E(what)}</td><td>{E(how)}</td><td>{links}</td></tr>")
    add("</tbody></table></div>")
    add('<p class="note">R11 and R14 name types of the retained-mode framework used in the port study (a keyed selection list; observable cells and hook state). Any equivalent in another framework works the same way.</p></section>')

    # --- 5 before / after
    add('<section id="effect"><h2>5. Before and after the rules</h2>')
    add("<p>Share of state-mechanism items by verdict, for a plain model without rules and with R1 to R14. Starred: the oracle's mechanism is absent, but the same bug can still be written.</p>")
    add(legend([(VERDICT_CLASS[k], VERDICT_NAME[k]) for k in VERDICT_KEYS]))
    ba = []
    for lab, n, mix, aft in (("Fix-set commits", st_fix, mix_fix, after_fix),
                             ("Commits outside the filter", st_out, mix_out, after_out),
                             ("Issues", st_iss, mix_iss, after_iss)):
        ba.append(dict(label=f"{lab} ({n}): no rules", vals=mix))
        ba.append(dict(label=f"{lab} ({n}): with R1 to R14", vals=aft))
    add('<div class="fig">' + hbars(ba, [(VERDICT_CLASS[k], VERDICT_NAME[k]) for k in VERDICT_KEYS],
                                    "State-mechanism items by verdict, without and with rules", normalize=True,
                                    row_h=40, bar_h=18, inside_min=15, total_fmt=lambda t, r: "") + "</div>")
    add('<div class="tw"><table class="data"><thead><tr><th></th><th>impossible</th><th>caught</th><th>possible*</th><th>possible</th><th>total</th></tr></thead><tbody>')
    for lab, vals in (("Fix set, no rules", mix_fix), ("Fix set, hand-written reducer", r_fix), ("Fix set, R1 to R14", after_fix),
                      ("Outside filter, no rules", mix_out), ("Outside filter, R1 to R14", after_out),
                      ("Issues, no rules", mix_iss), ("Issues, hand-written reducer", r_iss), ("Issues, R1 to R14", after_iss)):
        add(f"<tr><td>{lab}</td>" + "".join(f"<td>{v}</td>" for v in vals) + f"<td>{sum(vals)}</td></tr>")
    add("</tbody></table></div>")
    add(f"<p>Without rules no sub-class is impossible or caught: the design removes the mechanism of three of them (SC2a, SC6a, SC7a, marked *), but none of the code that writes the same bug is rejected. With the rules {imp} of {st_fix} fix-set commits are impossible and {cau} caught; {still} stay possible (SC6b2 refresh policy, SC7c one bool for two outcomes). Of the {N} fix commits that is {pct(imp, N)} and {pct(cau, N)}. The remaining state-mechanism bugs sit in classes the rules do not claim (SC4b backend semantics, SC6b2 refresh policy).</p></section>")

    # --- 6 approaches
    add('<section id="approaches"><h2>6. Approaches compared on the same sub-classes</h2>')
    add("<p>Verdicts before any of the rules R1 to R14 are added, except the last column. Of the rules, only the ordering tests (SC8b, SC9) depend on how state changes are expressed, and they favour changes as values. The impossible results come from types and lints that any family of approaches could adopt; the families differ mainly in how they fail: untracked reads in signals, forgotten notify in entities, snapshot lag in actors.</p>")
    cols = [("M", "Handler methods (today's shape)"), ("R", "Hand-written single reducer"),
            ("F2_store", "Store / reducer library"), ("F3_signals", "Signals"),
            ("F4_entities", "Entities + notify"), ("F5_actors", "Actors"),
            ("after_rules", "Plain model + R1 to R14")]
    add('<div class="tw"><table class="data cmp"><thead><tr><th>sub-class</th>' + "".join(f"<th>{E(n)}</th>" for _, n in cols) + "</tr></thead><tbody>")
    wsum = {c: Counter() for c, _ in cols}
    for sc in ids:
        r = cnt[sc]
        w = rc[sc] + ro[sc] + ri[sc]
        add(f'<tr><td><a href="#{sc}">{sc}</a> {E(SHORT[sc])} <span class="small">({w})</span></td>')
        for c, _ in cols:
            v = r[c]
            wsum[c][v] += w
            add(f'<td class="vc {VERDICT_CLASS.get(v, "")}">{VERDICT_SHORT.get(v, E(v))}</td>')
        add("</tr>")
    total_items = sum(rc[s] + ro[s] + ri[s] for s in ids)
    for k in VERDICT_KEYS:
        add(f'<tr class="sum"><td>Items {VERDICT_SHORT[k]} (of {total_items})</td>' + "".join(f"<td>{wsum[c][k]}</td>" for c, _ in cols) + "</tr>")
    add("</tbody></table></div>")
    add("<p class=\"note\">The number in brackets is the evidence behind each row: fix-set commits, outside commits and issues. Family columns come from a quick per-family reading and are less firm than the first and last columns. Columns F2 to F5 are verdicts before rules; they are not 'family + rules'. The chart in section 5 and the last column use verdicts for the plain model with rules.</p></section>")

    # --- 7 method
    lo, hi = wilson(23, 24)
    add('<section id="method"><h2>7. Method and limits</h2><ul class="m">')
    add(f"<li><strong>Selection.</strong> <code>git log --no-merges -- codex-rs/tui codex-rs/tui2</code> gives 3,612 commits. A subject regex (fix, bug, regress, revert, panic, crash, hang, flicker, glitch, garbl, corrupt, broken, stale, duplicate, race, leak, deadlock, freez) selects {N}. Each commit gets a primary class from an ordered subject regex. The {pclass['other']} 'other' commits are mostly backend, config and CI text; a 50% sample read found about 80% non-UI.</li>")
    add(f"<li><strong>Reading.</strong> {len(cm_fix)} fix-set commits were read for root cause: all {n_sr} state_ui and race commits, and {len(cm_fix) - n_sr} state-flavoured commits from other classes (subjects with stale, switch, resume, duplicate and similar). For each I read the commit message (Codex PRs carry Why and What sections) and, where it was empty or vague, the TUI diff. {len(cm_out)} commits outside the fix set were read the same way. Total {len(commits)} commits. One reader, one primary sub-class per commit; root causes come from commit text and diff reading, not from reproduction.</li>")
    add(f"<li><strong>Unread part.</strong> {N - len(cm_fix)} fix-set commits (mostly terminal, layout, key and backend classes) were not read for state mechanism. The state-mechanism count ({st_fix}) is therefore a floor for the fix set. The class-level shares in section 1 are estimates with an error of about +-5 points; sub-class counts are counts of reviewed items and carry no sampling error for the {len(commits)} read, but they say little about unread commits.</li>")
    add(f"<li><strong>What the subject filter misses.</strong> Subjects shaped like 'Preserve / Keep / Clear / Scope / Route ...' without a fix keyword give 106 more commits. A sample of 24 held 23 state-mechanism bugs, including the whole SC3 thread-routing series (8843960ba, 6f47fcb0b, 53a1f4c29). Taken at face value that is about {106 * 23 / 24:.0f} of the 106; a 95% Wilson interval for 23 of 24 is {100 * lo:.0f}% to {100 * hi:.0f}% (about {106 * lo:.0f} to {106 * hi:.0f} commits). The way the 24 were picked is not recorded as random, so treat this as a lower bound on how large the class is: the 508 undercount state bugs. The rules cover the 23 in similar proportions ({after_out[0]} impossible, {after_out[1]} caught, {after_out[2] + after_out[3]} possible).</li>")
    add("<li><strong>Issues.</strong> GitHub search on openai/codex, label TUI, keyword in title (stale, desync, out of sync, not updated, wrong thread, after resume, after switch, duplicate, race, doesn't update, not refresh, still shows, switching, disappear, reappear, stuck): 171 hits, 150 unique. 42 state-looking issues were read. The mechanism comes from the reporter's analysis or the linked fix; the cause of #30714 is not confirmed. 37 of the 42 are closed.</li>")
    add("<li><strong>Verdict scale.</strong> Impossible: the type system or a CI lint rejects the program. Caught: a test the design makes mandatory fails. Still possible: anything else; 'careful code avoids it' counts as possible. * marks sub-classes where today's mechanism is absent from the design but the same bug can be written. Verdicts were reasoned on paper; nothing was implemented and run. 'Caught' assumes the required tests are written; 'impossible' assumes the lints are in CI.</li>")
    add("<li><strong>Not in scope.</strong> The layout, terminal, text, style and input classes are covered in <code>bug-classes.md</code>, not here. Backend, persistence and feature-logic commits are counted but not analysed.</li>")
    add("<li><strong>Code claims.</strong> Counts such as 335 AppEvent variants and the unwrap_or_default sites in chatwidget/tool_requests.rs were re-checked against the checkout of 2026-11-17.</li>")
    add("</ul>")
    add('<h3>Data in this report</h3><p class="note">fix-set.csv (508 commits), commits.csv (159 classified commits), issues.csv (42 issues), subclass-counts.csv. Built by build_report.py with the Python standard library; the page has no scripts and loads nothing from the network.</p>')
    if WARN:
        add('<h3>Observations on the data</h3><ul class="m">' + "".join(f"<li>{E(w)}</li>" for w in WARN) + "</ul>")
    add("</section>")
    return "\n".join(H)


def rule_text(sc, fallback):
    ids = re.findall(r"R\d+", RULE_OF[sc])
    if not ids:
        return E(fallback)
    byid = {r[0]: r for r in RULES}
    return "<br>".join(f"<strong>{i}</strong> {E(byid[i][1])}. Held by: {E(byid[i][3])}." for i in ids)


def vpill(v):
    return f'<span class="pill {VERDICT_CLASS.get(v, "")}">{VERDICT_SHORT.get(v, E(v))}</span>'


# ------------------------------------------------------------------------ page
CSS = """
:root{color-scheme:light dark;--bg:#fafaf8;--fg:#1b1f24;--mut:#5b6572;--card:#fff;--bd:#d9dde2;--link:#0b5cad;--code:#eef0f3;
--s0:#6b7785;--s1:#c24d00;--s2:#b0507f;--s3:#0072b2;--s4:#1e88c7;--s5:#00806a;--s6:#b87400;--s7:#6f5bb5;--s8:#5f8a1a;
--v-imp:#1b7f4b;--v-cau:#1f6fb2;--v-psx:#b87400;--v-pos:#b3382c;--segt:#fff;
--bad:#fbe9e5;--badbd:#c24d00;--good:#e5f4ec;--goodbd:#1b7f4b;--boxbg:#f1f3f6}
@media (prefers-color-scheme:dark){:root{--bg:#12161b;--fg:#e6e9ed;--mut:#98a2ae;--card:#1a1f26;--bd:#2f3742;--link:#7cb7ff;--code:#252c35;
--s0:#9aa7b4;--s1:#f08a4b;--s2:#e08fb8;--s3:#5aa9e6;--s4:#6cc3ee;--s5:#3cc7a6;--s6:#f0b84a;--s7:#a99bec;--s8:#a8d05a;
--v-imp:#4cc38a;--v-cau:#6bb0ee;--v-psx:#f0b84a;--v-pos:#f08070;--segt:#0b0f14;
--bad:#3a2420;--badbd:#f08a4b;--good:#18312a;--goodbd:#4cc38a;--boxbg:#222831}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:16px/1.55 system-ui,-apple-system,"Segoe UI",Roboto,sans-serif}
main{max-width:980px;margin:0 auto;padding:0 16px 48px}
a{color:var(--link)}
code{background:var(--code);padding:1px 5px;border-radius:4px;font-size:.88em}
h1{font-size:1.9rem;line-height:1.2;margin:.2em 0 .4em}
h2{font-size:1.45rem;margin:2.2em 0 .5em;border-bottom:1px solid var(--bd);padding-bottom:.25em}
h3{font-size:1.1rem;margin:1.6em 0 .4em}
.hero{padding:28px 0 8px}
.eyebrow{color:var(--mut);text-transform:uppercase;letter-spacing:.08em;font-size:.78rem;margin:0}
.sub{color:var(--mut);margin:0 0 1em}
.summary{padding-left:1.3em;margin:0 0 1em}.summary li{margin:.45em 0}
.toc{display:flex;flex-wrap:wrap;gap:6px 14px;font-size:.92rem}
.legend{display:flex;flex-wrap:wrap;gap:4px 14px;margin:.4em 0 .6em;font-size:.88rem;color:var(--mut)}
.lg{display:inline-flex;align-items:center;gap:6px}
.sw{display:inline-block;width:12px;height:12px;border-radius:3px}
.fig{max-width:720px;margin:.3em auto}
svg.chart{width:100%;height:auto;display:block}
svg text{font:12.5px system-ui,-apple-system,"Segoe UI",Roboto,sans-serif;fill:var(--fg)}
svg .tm{fill:var(--mut);font-size:12px}
svg .tb{font-weight:600}
svg .tg{fill:var(--mut);font-size:11.5px;font-weight:600}
svg .segt{fill:var(--segt);font-size:11px;font-weight:600}
svg a text{fill:var(--link);text-decoration:underline}
svg .box{fill:var(--boxbg);stroke:var(--bd);stroke-width:1.2}
svg .box.bad{fill:var(--bad);stroke:var(--badbd)}
svg .box.good{fill:var(--good);stroke:var(--goodbd)}
svg .ar{stroke:var(--mut);stroke-width:1.5}
svg .ah{fill:var(--mut)}
.s0{fill:var(--s0);background:var(--s0)}.s1{fill:var(--s1);background:var(--s1)}.s2{fill:var(--s2);background:var(--s2)}
.s3{fill:var(--s3);background:var(--s3)}.s4{fill:var(--s4);background:var(--s4)}.s5{fill:var(--s5);background:var(--s5)}
.s6{fill:var(--s6);background:var(--s6)}.s7{fill:var(--s7);background:var(--s7)}.s8{fill:var(--s8);background:var(--s8)}
.v-imp{fill:var(--v-imp);background:var(--v-imp)}.v-cau{fill:var(--v-cau);background:var(--v-cau)}
.v-psx{fill:var(--v-psx);background:var(--v-psx)}.v-pos{fill:var(--v-pos);background:var(--v-pos)}
svg .sw{display:none}
.pill{color:var(--segt)}
.pill{display:inline-block;padding:0 8px;border-radius:10px;font-size:.82rem;font-weight:600;white-space:nowrap}
.note{color:var(--mut);font-size:.9rem}
.small{color:var(--mut);font-size:.85rem}
.tw{overflow-x:auto;margin:.6em 0}
table.data{border-collapse:collapse;width:100%;font-size:.9rem}
table.data th,table.data td{border:1px solid var(--bd);padding:6px 8px;text-align:left;vertical-align:top}
table.data th{background:var(--code)}
table.cmp td.vc{text-align:center;font-weight:600;font-size:.82rem;white-space:nowrap;color:var(--fg)}
table.cmp td.v-imp{background:color-mix(in srgb,var(--v-imp) 55%,transparent)}
table.cmp td.v-cau{background:color-mix(in srgb,var(--v-cau) 50%,transparent)}
table.cmp td.v-psx{background:color-mix(in srgb,var(--v-psx) 35%,transparent)}
table.cmp td.v-pos{background:color-mix(in srgb,var(--v-pos) 16%,transparent);font-weight:500}
table.cmp tr.sum td{font-weight:600;background:var(--code);text-align:center}
table.cmp tr.sum td:first-child{text-align:left}
table.rules td:first-child{white-space:nowrap}
.card{overflow-wrap:anywhere;background:var(--card);border:1px solid var(--bd);border-radius:10px;padding:14px 16px;margin:14px 0}
.card h3{margin:0}.card .id{display:inline-block;background:var(--code);border-radius:5px;padding:0 7px;margin-right:6px;font-size:.9rem}
.cnt{margin:.1em 0 .6em;color:var(--mut);font-size:.88rem}
.mech{margin:.3em 0}
.ex{padding-left:1.2em;margin:.5em 0;font-size:.92rem}.ex li{margin:.4em 0}
.why{color:var(--mut);font-size:.86rem}
.tag{border:1px solid var(--bd);border-radius:8px;padding:0 6px;font-size:.75rem;color:var(--mut);white-space:nowrap}
.rule{border-top:1px solid var(--bd);margin-top:.7em;padding-top:.5em}
.rl{font-size:.92rem}.rt{margin:.2em 0;color:var(--mut);font-size:.88rem}
.vd{font-size:.92rem}
.duo{display:grid;grid-template-columns:1fr 1fr;gap:18px;align-items:start}
.duo figure{margin:0}.duo figcaption{text-align:center;font-weight:600;color:var(--mut);margin-top:4px}
@media (max-width:760px){.duo{grid-template-columns:1fr}h1{font-size:1.5rem}.summary{font-size:.95rem}}
details{margin:.4em 0}summary{cursor:pointer;color:var(--link)}
ul.m li{margin:.5em 0}
footer{color:var(--mut);font-size:.85rem;margin-top:3em;border-top:1px solid var(--bd);padding-top:.8em}
"""


def page(body, d1):
    return ("<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">"
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">"
            "<meta name=\"color-scheme\" content=\"light dark\">"
            "<title>Codex TUI bug history and state-design rules</title>"
            f"<style>{CSS}</style></head><body><main>{body}"
            f"<footer>Data to {d1}. Self-contained page: inline SVG, no scripts, no network requests.</footer>"
            "</main></body></html>\n")


# -------------------------------------------------------------------- validate
VOID = {"meta", "br", "hr", "img", "input", "link", "path", "line", "rect", "circle", "use"}
SVG_SELF = {"path", "line", "rect"}


class Check(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.stack, self.errors = [], []
        self.svgs = 0
        self.no_viewbox = 0
        self.links, self.bad_links = 0, []
        self.ids = Counter()
        self.script = False

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if "id" in a:
            self.ids[a["id"]] += 1
        if tag == "svg":
            self.svgs += 1
            if "viewbox" not in a:
                self.no_viewbox += 1
        if tag == "script":
            self.script = True
        if tag == "a":
            href = a.get("href", "")
            if href.startswith("#"):
                pass
            else:
                self.links += 1
                if not href.startswith(REPO + "/"):
                    self.bad_links.append(href)
        for k in ("src",):
            if k in a and a[k].startswith("http"):
                self.bad_links.append(a[k])
        if tag in VOID or tag in SVG_SELF:
            return
        self.stack.append(tag)

    def handle_startendtag(self, tag, attrs):
        a = dict(attrs)
        if tag == "svg":
            self.svgs += 1
        # self-closing tags such as <rect/> and <path/>
        return

    def handle_endtag(self, tag):
        if tag in VOID or tag in SVG_SELF:
            return
        if not self.stack or self.stack[-1] != tag:
            self.errors.append(f"unexpected </{tag}> (open: {self.stack[-3:]})")
            if tag in self.stack:
                while self.stack and self.stack.pop() != tag:
                    pass
        else:
            self.stack.pop()


def validate(doc):
    c = Check()
    c.feed(doc)
    c.close()
    problems = list(c.errors)
    if c.stack:
        problems.append(f"unclosed tags: {c.stack}")
    if c.no_viewbox:
        problems.append(f"{c.no_viewbox} svg without viewBox")
    if c.bad_links:
        problems.append(f"non-codex links: {c.bad_links[:5]}")
    dup = [k for k, v in c.ids.items() if v > 1]
    if dup:
        problems.append(f"duplicate ids: {dup}")
    for m in re.finditer(r'href="#([^"]+)"', doc):
        if m.group(1) not in c.ids:
            problems.append(f"dangling anchor #{m.group(1)}")
    if re.search(r"<script|https?://(?!github\.com/openai/codex/)", re.sub(r'xmlns="[^"]*"', "", doc)):
        problems.append("script tag or external URL present")
    return c, problems


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: build_report.py DATA_DIR OUT_HTML")
    data = load(sys.argv[1])
    body = build(data)
    dates = sorted(r["date"] for r in data["fix"])
    doc = page(body, dates[-1])
    Path(sys.argv[2]).write_text(doc, encoding="utf-8")
    c, problems = validate(doc)
    print(f"wrote {sys.argv[2]}: {len(doc):,} bytes, {c.svgs} svg, {c.links} github links, "
          f"{len(WARN)} data warnings, {len(problems)} validation problems")
    for p in problems:
        print("PROBLEM:", p, file=sys.stderr)
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
