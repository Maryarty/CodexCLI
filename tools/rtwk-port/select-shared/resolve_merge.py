#!/usr/bin/env python3 -I
"""Resolves the conflict hunks of the merge of land/batch-9 (features-trait, session) into
fleet/adr013-select-shared, hunk by hunk: `ours` = this branch (the generic list left core,
Model.kept gone), `theirs` = batch-9 (Menu::Feature/Picked::Feature gone, features get a UI half).
Run from codex-rs/tui-rtwk. Rerunnable on a fresh merge; a file whose hunk count differs from the
table is left untouched and named."""
import re
import sys
from pathlib import Path

# file -> one choice per hunk, in order: ours | theirs | empty | text:<literal>
TABLE = {
    "core/src/app/select.rs": ["text:        )\n", "empty"],
    "core/src/screens/select.rs": ["empty"],
    "src/features/export/mod.rs": ["theirs"],
    "src/features/export/strings.rs": ["text:use codex_tui_core::features::export::Note;\nuse crate::widgets::picker::list::{Columns, Item, List};\n"],
    "src/features/goal/mod.rs": ["theirs"],
    "src/features/mod.rs": ["theirs"],
    "src/features/plugins/mod.rs": ["empty"],
    "src/features/rename/mod.rs": ["theirs"],
    "src/pages/session/banners.rs": ["theirs"],
    "tests/rules/adr013_ratchets/select.txt": ["ours"],
}

HUNK = re.compile(
    r"<<<<<<< [^\n]*\n(?P<ours>.*?)=======\n(?P<theirs>.*?)>>>>>>> [^\n]*\n", re.S
)


def resolve(path: str, choices: list[str]) -> bool:
    p = Path(path)
    text = p.read_text()
    hunks = list(HUNK.finditer(text))
    if len(hunks) != len(choices):
        print(f"SKIP {path}: {len(hunks)} hunks, table has {len(choices)}")
        return False
    it = iter(choices)

    def pick(m: re.Match) -> str:
        c = next(it)
        if c == "ours":
            return m.group("ours")
        if c == "theirs":
            return m.group("theirs")
        if c == "empty":
            return ""
        if c.startswith("text:"):
            return c[5:]
        raise SystemExit(f"bad choice {c}")

    p.write_text(HUNK.sub(pick, text))
    print(f"ok   {path}")
    return True


if __name__ == "__main__":
    ok = all([resolve(f, c) for f, c in TABLE.items()])
    sys.exit(0 if ok else 1)
