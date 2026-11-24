"""Adds `type Event = Never<Self>;` after `type Interest = ...;` in every feature slice of
codex-rs/tui-rtwk/core/src/features that implements `Feature` and has no `type Event` yet
(Rust has no defaults for associated types, so each slice names the one it does not use).
Idempotent; a slice that follows the stream writes its own Event and is left alone.

    python3 -I tools/rtwk-port/feature-event/add_event.py
"""
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[3] / "codex-rs/tui-rtwk/core/src/features"
PATTERN = re.compile(r"^([ \t]*)type Interest = ([^;]+);\n", re.M)


def never(match):
    spelled = "super::Never<Self>" if match.group(2) == "super::Never<Self>" else "Never<Self>"
    return match.group(0) + f"{match.group(1)}type Event = {spelled};\n"


changed = []
for path in sorted(ROOT.rglob("*.rs")):
    if path.name == "framework.rs" or (path.parent == ROOT and path.name == "mod.rs"):
        continue
    text = path.read_text()
    if "impl Feature for" not in text or re.search(r"^\s*type Event\b", text, re.M):
        continue
    new, count = PATTERN.subn(never, text)
    if count:
        path.write_text(new)
        changed.append(f"{path.relative_to(ROOT)} ({count})")
print("\n".join(changed) or "nothing to do")
