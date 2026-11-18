"""Finds the two binaries. Never builds one."""

from __future__ import annotations

import glob
import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness import REPO_ROOT  # noqa: E402


def _main_checkout() -> Path:
    parts = REPO_ROOT.parts
    if ".claude" in parts:  # a worktree: <main>/.claude/worktrees/<name>
        return Path(*parts[: parts.index(".claude")])
    return REPO_ROOT


def _newest(paths: list[str]) -> str | None:
    found = [p for p in paths if os.path.isfile(p) and os.access(p, os.X_OK)]
    return max(found, key=os.path.getmtime) if found else None


def rtwk_bin() -> str | None:
    """RTWK_BIN, else a built codex-rtwk: this tree, the main checkout, any worktree (newest)."""
    if os.environ.get("RTWK_BIN"):
        return os.environ["RTWK_BIN"]
    main = _main_checkout()
    cands: list[str] = []
    for base in (REPO_ROOT, main):
        for prof in ("debug", "release"):
            cands.append(str(base / "codex-rs/tui-rtwk/target" / prof / "codex-rtwk"))
    for prof in ("debug", "release"):
        cands += glob.glob(str(main / ".claude/worktrees/*/codex-rs/tui-rtwk/target" / prof / "codex-rtwk"))
    return _newest(cands)


def oracle_bin() -> str | None:
    """ORACLE_BIN, else a built oracle `codex`: ~/.cache/codexcli/oracle-bin, codex-rs/target."""
    if os.environ.get("ORACLE_BIN"):
        return os.environ["ORACLE_BIN"]
    main = _main_checkout()
    cands = [os.path.expanduser("~/.cache/codexcli/oracle-bin/codex")]
    for base in (REPO_ROOT, main):
        for prof in ("debug", "release"):
            cands.append(str(base / "codex-rs/target" / prof / "codex"))
    return _newest(cands)


if __name__ == "__main__":
    which = sys.argv[1] if len(sys.argv) > 1 else ""
    path = {"rtwk": rtwk_bin, "oracle": oracle_bin}.get(which, lambda: None)()
    if not path:
        print(f"no {which} binary found (set {'RTWK_BIN' if which == 'rtwk' else 'ORACLE_BIN'})", file=sys.stderr)
        raise SystemExit(1)
    print(path)
