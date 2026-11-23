#!/usr/bin/env python3 -I
"""The settings fixtures' `screen` steps become `fact` steps of the feature
(`{"step":"screen","dir":"settings","ev":..}` -> `{"step":"fact","feature":"settings","fact":..}`),
in the slice's fixtures and the oracle shard's. Run from the repo root; reruns change nothing."""
import pathlib
import re

ROOT = pathlib.Path("codex-rs/tui-rtwk")
# compact or spaced JSON, as the fixture was written
OLD = re.compile(r'\{"step": ?"screen", ?"dir": ?"settings", ?"ev": ?')
NEW = '{"step":"fact","feature":"settings","fact":'
files = sorted(ROOT.glob("src/features/settings/fixtures/*.jsonl")) + [
    ROOT / "tests/oracle/fixtures/chat_popups_4/memories_enabled.jsonl"
]
changed = 0
for path in files:
    text = path.read_text()
    rewritten = OLD.sub(NEW, text)
    if rewritten != text:
        path.write_text(rewritten)
        changed += 1
print(f"{changed} of {len(files)} fixtures rewritten")
