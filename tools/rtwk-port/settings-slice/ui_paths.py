#!/usr/bin/env python3 -I
"""The sheets of the settings dialogs move from `sheets::settings` over `core::screens::settings`
to `features::settings` (ADR-012/013): paths, and the offers read from the feature states. Run
from the repo root; reruns change nothing. Hand edits (the intents the sheets send, `mod.rs`)
are not here."""
import pathlib
import re

ROOT = pathlib.Path("codex-rs/tui-rtwk")
files = sorted((ROOT / "src/features/settings").glob("*.rs"))
RULES = [
    (r"codex_tui_core::screens::settings::", "codex_tui_core::features::settings::"),
    (r"core::screens::settings::", "core::features::settings::"),
    (r"codex_tui_core::screens::\{ScreenIntent, ScreenStates\}", "codex_tui_core::features::FeatureStates"),
    (r"use codex_tui_core::screens::ScreenStates;", "use codex_tui_core::features::FeatureStates;"),
    (r"select::<ScreenStates,", "select::<FeatureStates,"),
]
changed = []
for path in files:
    text = path.read_text()
    new = text
    for old, repl in RULES:
        new = re.sub(old, repl, new)
    if new != text:
        path.write_text(new)
        changed.append(path.name)
print("rewritten:", " ".join(changed))
