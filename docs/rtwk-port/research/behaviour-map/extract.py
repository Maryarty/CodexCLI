#!/usr/bin/env python3
"""List candidate behaviours of codex-tui Rust files, deterministically.

Usage: extract.py --screen S14 --root <codex-rs/tui/src> FILE... > candidates.csv

Stdlib only. The script finds *where* behaviour is decided; describing *what*
it does is the describer's job (behaviour-map-pilot, process.md Part 1 §3
step 1). Kinds:
  key-arm     an arm of a `match` inside a fn whose name starts with handle_key;
              symbol `<fn>#arm<k>/<n>`: earlier arms of the same match win
              (precedence is behaviour too)
  field       a field of a non-test struct (a parameter or row attribute
              changes what the user sees: selected_description, category_tag)
  view-hook   a method of an `impl BottomPaneView for` / `impl Renderable for` block
  mutator     a non-test fn taking `&mut self` (state transitions: move, accept, filter)
  send        a line that calls an action or sends on the app-event sender
  test        a #[test] fn
  snapshot    an assert_snapshot! call (name when it is a literal)
Rows are cited as file:line-end_line relative to --root.
"""
import argparse
import csv
import re
import sys
from pathlib import Path

FN_RE = re.compile(r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?fn\s+([a-z_][a-z0-9_]*)\s*(?:<[^>]*>)?\s*\(")
IMPL_RE = re.compile(r"^impl(?:<[^>]*>)?\s+(\w+)\s+for\s+(\w+)")
TEST_ATTR_RE = re.compile(r"^\s*#\[(?:tokio::)?test")
CFG_TEST_RE = re.compile(r"^\s*#\[cfg\(test\)\]")
SEND_RE = re.compile(r"app_event_tx\.send|\btx\.send\(|\(\s*act\s*\)|\bact\(&|\bcb\(&|\baction\(&|\(\w+\.action\)\(")
SNAP_RE = re.compile(r"assert_snapshot!\(\s*(?:\"([^\"]+)\")?")
STRUCT_RE = re.compile(r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?struct\s+(\w+)(?:<[^>]*>)?\s*\{\s*$")
FIELD_RE = re.compile(r"^\s*pub(?:\([^)]*\))?\s+([a-z_][a-z0-9_]*)\s*:\s*(.+?),?\s*$")
ARM_SKIP = ("}", "{", ")", ".", "&&", "||", "code:", "modifiers:", "kind:", "state:", "..", "if ", "=>", "//", "|")


def strip_code(line):
    """Drop string literals and // comments so braces in them do not count."""
    line = re.sub(r'"(?:\\.|[^"\\])*"', '""', line)
    line = re.sub(r"'(?:\\.|[^'\\])'", "''", line)
    i = line.find("//")
    return line if i < 0 else line[:i]


def fn_end(lines, start):
    """Index of the line closing the fn whose signature starts at `start`."""
    depth, seen = 0, False
    for j in range(start, len(lines)):
        code = strip_code(lines[j])
        depth += code.count("{") - code.count("}")
        if "{" in code:
            seen = True
        if seen and depth <= 0:
            return j
        if not seen and code.rstrip().endswith(";"):
            return j  # trait method declaration without a body
    return len(lines) - 1


def test_ranges(lines):
    """Line ranges of #[cfg(test)] modules (so mutators there are skipped)."""
    out = []
    for i, line in enumerate(lines):
        if CFG_TEST_RE.match(line):
            for j in range(i + 1, min(i + 4, len(lines))):
                if re.match(r"^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", lines[j]):
                    out.append((j, fn_end(lines, j)))
                    break
    return out


def in_ranges(i, ranges):
    return any(a <= i <= b for a, b in ranges)


def key_arms(lines, fn_start, fn_stop):
    """Arms of `match ... {` blocks inside a key-handling fn."""
    rows = []
    i = fn_start
    while i <= fn_stop:
        m = re.match(r"^(\s*)match\s+.*\{\s*$", strip_code(lines[i]))
        if not m:
            i += 1
            continue
        arm_indent = len(m.group(1)) + 4
        end = fn_end(lines, i)
        starts = []
        for j in range(i + 1, end):
            raw = lines[j]
            if not raw.strip():
                continue
            indent = len(raw) - len(raw.lstrip())
            if indent == arm_indent and not raw.strip().startswith(ARM_SKIP):
                starts.append(j)
        for k, s in enumerate(starts):
            e = (starts[k + 1] - 1) if k + 1 < len(starts) else end - 1
            rows.append((s, e))
        i = end + 1
    return rows


def excerpt(lines, a, b, n=3):
    text = " ".join(l.strip() for l in lines[a : min(b, a + n - 1) + 1])
    return re.sub(r"\s+", " ", text)[:220]


def scan(path, rel, screen):
    lines = path.read_text(encoding="utf-8").splitlines()
    tests = test_ranges(lines)
    rows = []
    impl_of, impl_end = None, -1
    i = 0
    while i < len(lines):
        line = lines[i]
        im = IMPL_RE.match(line)
        if im:
            impl_of, impl_end = im.group(1), fn_end(lines, i)
        if i > impl_end:
            impl_of = None
        fm = FN_RE.match(line)
        if fm:
            name = fm.group(2)
            stop = fn_end(lines, i)
            is_test_fn = any(TEST_ATTR_RE.match(lines[k]) for k in range(max(0, i - 3), i))
            sig = " ".join(lines[k].strip() for k in range(i, min(stop, i + 3) + 1))
            if is_test_fn:
                rows.append(("test", i, stop, name, excerpt(lines, i, stop, 1)))
            elif impl_of in ("BottomPaneView", "Renderable") and not in_ranges(i, tests):
                rows.append(("view-hook", i, stop, f"{impl_of}::{name}", excerpt(lines, i, stop, 2)))
            elif "&mut self" in sig.split("{")[0] and not in_ranges(i, tests):
                rows.append(("mutator", i, stop, name, excerpt(lines, i, stop, 2)))
            if name.startswith("handle_key") and not in_ranges(i, tests):
                arms = key_arms(lines, i, stop)
                for k, (a, b) in enumerate(arms, 1):
                    rows.append(("key-arm", a, b, f"{name}#arm{k}/{len(arms)}", excerpt(lines, a, b, 4)))
        st = STRUCT_RE.match(line)
        if st and not in_ranges(i, tests):
            send = fn_end(lines, i)
            for j in range(i + 1, send):
                fm2 = FIELD_RE.match(lines[j])
                if fm2 and len(lines[j]) - len(lines[j].lstrip()) == len(st.group(1)) + 4:
                    doc = j
                    while doc > i + 1 and lines[doc - 1].strip().startswith("///"):
                        doc -= 1
                    rows.append(("field", doc, j, f"{st.group(2)}.{fm2.group(1)}", excerpt(lines, doc, j, 3)))
        if not in_ranges(i, tests) and SEND_RE.search(strip_code(line)):
            rows.append(("send", i, i, "", line.strip()[:220]))
        sm = SNAP_RE.search(line)
        if sm:
            rows.append(("snapshot", i, i, sm.group(1) or "", line.strip()[:220]))
        i += 1
    out = []
    for kind, a, b, sym, ex in rows:
        out.append(dict(screen=screen, kind=kind, file=rel, line=a + 1, end_line=b + 1, symbol=sym, excerpt=ex))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--screen", required=True)
    ap.add_argument("--root", required=True)
    ap.add_argument("files", nargs="+")
    a = ap.parse_args()
    root = Path(a.root).resolve()
    rows = []
    for f in a.files:
        p = Path(f).resolve()
        rows += scan(p, str(p.relative_to(root)), a.screen)
    order = {"key-arm": 0, "view-hook": 1, "mutator": 2, "send": 3, "field": 4, "test": 5, "snapshot": 6}
    rows.sort(key=lambda r: (order[r["kind"]], r["file"], r["line"]))
    w = csv.DictWriter(sys.stdout, fieldnames=["id", "screen", "kind", "file", "line", "end_line", "symbol", "excerpt"])
    w.writeheader()
    for n, r in enumerate(rows, 1):
        w.writerow({"id": f"{a.screen}-C{n:03d}", **r})
    print(f"{len(rows)} candidates", file=sys.stderr)


if __name__ == "__main__":
    main()
