#!/usr/bin/env python3
"""Check a behaviour map's citations against the oracle.

Usage: validate.py --root <codex-rs/tui/src> MAP.csv [MAP.csv ...]

Stdlib only. For each row: every `oracle` citation (file:line or
file:line-line, ";"-separated) names an existing file and lines inside it;
every name in `tests` is a fn defined in the cited files' test code or
anywhere under --root; every name in `snapshots` matches a .snap file stem
(suffix match) or an assert_snapshot! literal under --root. Prints one line per
bad row and a summary per map.
"""
import argparse
import csv
import re
from pathlib import Path

CITE_RE = re.compile(r"([\w./-]+\.rs):(\d+)(?:-(\d+))?")


def index(root):
    fns, snaps = set(), set()
    for p in root.rglob("*.rs"):
        text = p.read_text(encoding="utf-8", errors="replace")
        fns.update(re.findall(r"\bfn\s+([a-z_][a-z0-9_]*)", text))
        snaps.update(re.findall(r"assert_snapshot!\(\s*\"([^\"]+)\"", text))
    stems = {p.stem for p in root.rglob("*.snap")}
    return fns, snaps, stems


def split(cell):
    return [x.strip() for x in re.split(r"[;,]", cell or "") if x.strip() and x.strip() != "-"]


def check(map_path, root, fns, snaps, stems):
    lengths = {}
    bad = 0
    rows = list(csv.DictReader(open(map_path, newline="", encoding="utf-8")))
    stats = dict(rows=len(rows), cites=0, bad_cites=0, tests=0, bad_tests=0, snaps=0, bad_snaps=0)
    for r in rows:
        problems = []
        cites = CITE_RE.findall(r.get("oracle", ""))
        if not cites:
            problems.append("no oracle citation")
        for f, a, b in cites:
            stats["cites"] += 1
            p = root / f
            if f not in lengths:
                lengths[f] = len(p.read_text(encoding="utf-8").splitlines()) if p.exists() else -1
            n, a, b = lengths[f], int(a), int(b or a)
            if n < 0 or not (1 <= a <= b <= n):
                stats["bad_cites"] += 1
                problems.append(f"bad cite {f}:{a}-{b} (file has {n} lines)")
        for t in split(r.get("tests")):
            stats["tests"] += 1
            name = t.split("::")[-1].strip("() ")
            if name not in fns:
                stats["bad_tests"] += 1
                problems.append(f"unknown test {t}")
        for s in split(r.get("snapshots")):
            stats["snaps"] += 1
            if s not in snaps and not any(st.endswith(s) for st in stems):
                stats["bad_snaps"] += 1
                problems.append(f"unknown snapshot {s}")
        if problems:
            bad += 1
            print(f"  {r.get('id')}: " + "; ".join(problems))
    print(f"{map_path}: {stats['rows']} rows, {bad} with problems; cites {stats['cites']} "
          f"(bad {stats['bad_cites']}), tests {stats['tests']} (unknown {stats['bad_tests']}), "
          f"snapshots {stats['snaps']} (unknown {stats['bad_snaps']})")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    ap.add_argument("maps", nargs="+")
    a = ap.parse_args()
    root = Path(a.root).resolve()
    fns, snaps, stems = index(root)
    for m in a.maps:
        check(m, root, fns, snaps, stems)


if __name__ == "__main__":
    main()
