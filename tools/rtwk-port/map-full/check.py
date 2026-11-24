"""Checks of the full port map (python3 -I check.py [worktree]): no port file in two packages, every
production file has an owner, every non-✅ row has a package, every CSV id and oracle module is covered,
every pending test is assigned.  Exit 1 on any failure."""
import fnmatch
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import packages  # noqa: E402

ROOT = sys.argv[1] if len(sys.argv) > 1 else os.path.abspath(os.path.join(HERE, "..", "..", ".."))
PORT = os.path.join(ROOT, "codex-rs", "tui-rtwk")
# One-line doors, appended to by every slice (ADR-012: `feature_list!`, `sheet_list!`, `action_list!`, `cell_list!`,
# `mode_list!`); they own nothing.
DOORS = {"core/src/features/mod.rs", "src/features/mod.rs",
         "src/pages/session/key_actions/mod.rs", "src/pages/session/cells/mod.rs", "src/pages/session/transcript/mod.rs"}
# Test files (tests/oracle shards, tests/pages) are edited by function: a package changes only the
# tests of its own un-pend list (unpend/<package>.txt), several packages in one shard file merge cleanly.
PROD = ("src/", "core/src/", "host/src/", "tests/rules/")


def files():
    out = subprocess.run(["git", "ls-files", "codex-rs/tui-rtwk"], cwd=ROOT, capture_output=True, text=True, check=True)
    n = len("codex-rs/tui-rtwk/")
    fs = [p[n:] for p in out.stdout.split("\n") if p.startswith("codex-rs/tui-rtwk/")]
    return [f for f in fs if f.startswith(PROD) and f not in DOORS
            and not f.startswith(("tests/snapshots", "tests/oracle_snap", "tests/srcscan"))
            and (f.endswith((".rs", ".txt", ".jsonl", ".toml")))]


def explicit(pat, f):
    if pat.endswith("/**"):
        return f.startswith(pat[:-2])
    return f == pat


def owners(fs):
    """-> {file: [package...]}; `rest:dir` takes what no explicit pattern took, longest dir first."""
    own = {f: [] for f in fs}
    for p in packages.P.values():
        for pat in p["own"]:
            if not pat.startswith("rest:"):
                for f in fs:
                    if explicit(pat, f):
                        own[f].append(p["id"])
    rests = sorted(((pat[5:], p["id"]) for p in packages.P.values() for pat in p["own"] if pat.startswith("rest:")),
                   key=lambda t: -len(t[0]))
    for f in fs:
        if not own[f]:
            for d, pid in rests:
                if f.startswith(d.rstrip("/") + "/") or f == d:
                    own[f].append(pid)
                    break
    return own


def pattern_overlaps():
    """Two packages naming the same new path, or one naming a path under another's `dir/**`."""
    named = {}
    bad = []
    for p in packages.P.values():
        for pat in p["own"]:
            if pat.startswith("rest:"):
                continue
            named.setdefault(pat, []).append(p["id"])
    for pat, ids in named.items():
        if len(ids) > 1:
            bad.append((pat, ids))
    globs = [(pat, ids[0]) for pat, ids in named.items() if pat.endswith("/**")]
    for pat, ids in named.items():
        for g, gid in globs:
            if pat != g and gid not in ids and (pat.startswith(g[:-2])):
                bad.append((pat + "  under  " + g, [ids[0], gid]))
    for i, (a, ai) in enumerate(globs):
        for b, bi in globs[i + 1:]:
            if a[:-2].startswith(b[:-2]) or b[:-2].startswith(a[:-2]):
                bad.append((a + "  vs  " + b, [ai, bi]))
    return bad


def main():
    fs = files()
    own = owners(fs)
    fail = 0
    two = {f: o for f, o in own.items() if len(o) > 1}
    none = [f for f, o in own.items() if not o]
    po = pattern_overlaps()
    print("packages: %d, production files: %d, doors: %d" % (len(packages.P), len(fs), len(DOORS)))
    print("files in two packages: %d" % len(two))
    for f, o in list(two.items())[:20]:
        print("  ", f, o)
    print("pattern overlaps: %d" % len(po))
    for x in po[:20]:
        print("  ", x)
    print("files with no owner: %d" % len(none))
    for f in none[:20]:
        print("  ", f)
    fail += bool(two) + bool(po) + bool(none)
    try:
        import build
        fail += build.verify()
    except ImportError:
        pass
    print("RESULT:", "FAIL" if fail else "OK")
    sys.exit(1 if fail else 0)


if __name__ == "__main__":
    main()
