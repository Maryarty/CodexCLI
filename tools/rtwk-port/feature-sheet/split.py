#!/usr/bin/env python3 -I
"""Split `FeatureView` into `FeatureView` + `FeatureSheet` in the feature slices of tui-rtwk.

    python3 -I tools/rtwk-port/feature-sheet/split.py [--check]

For each `codex-rs/tui-rtwk/src/features/<f>/mod.rs` whose `impl FeatureView for X` still has
`type Args` / `fn args` / `fn open`:

  * a slice that declares `Feature::OPENS` (core: `const OPENS: ... = Some(`) gets those three
    items moved into `impl FeatureSheet for X { .. }` right after the `FeatureView` impl, and
    `FeatureSheet` added to its `use super::..` line;
  * any other slice (the `Args = Never<Self>` / `.absurd()` stubs, and the settings dialogs,
    which `Target::Settings` opens through the plain `settings::open`) has the three items
    deleted, and the imports they were the last user of (`Never`, `Cwd`, `ThreadId`, `Cx`)
    dropped.

Rerunnable: an impl without the three items is left alone. `--check` only lists what it would
change. The list of OPENS slices is read from core, not written here.
"""

import re
import sys
from pathlib import Path

CRATE = Path(__file__).resolve().parents[3] / "codex-rs" / "tui-rtwk"
ITEMS = ("Args", "args", "open")
DROPPABLE = ("Never", "Cwd", "ThreadId", "Cx")


def opens(feature: str) -> bool:
    core = CRATE / "core" / "src" / "features" / feature / "mod.rs"
    return re.search(r"const OPENS: [^=]*= Some\(", core.read_text()) is not None


def depth_of(line: str) -> int:
    code = re.sub(r'"(?:[^"\\]|\\.)*"', '""', line.split("//")[0])
    return sum(code.count(c) for c in "{(") - sum(code.count(c) for c in "})")


def impl_range(lines: list[str]) -> tuple[int, int, str] | None:
    for start, line in enumerate(lines):
        found = re.match(r"impl FeatureView for (\w+) \{$", line)
        if found:
            for end in range(start + 1, len(lines)):
                if lines[end] == "}":
                    return start, end, found.group(1)
    return None


def items_of(lines: list[str], start: int, end: int) -> list[tuple[int, int, str]]:
    """(first line, last line, name) of the wanted items of the impl body."""
    found = []
    i = start + 1
    while i < end:
        if not lines[i].strip():
            i += 1
            continue
        first = i
        while lines[i].lstrip().startswith(("///", "#[")):
            i += 1
        header = lines[i]
        depth = depth_of(header)
        last = i
        while depth > 0 or not (lines[last].endswith(";") or lines[last].endswith("}")):
            last += 1
            depth += depth_of(lines[last])
        named = re.match(r"\s+(?:pub )?(?:type|fn|const) (\w+)", header)
        if named and named.group(1) in ITEMS:
            found.append((first, last, named.group(1)))
        i = last + 1
    return found


def drop_names(text: str) -> str:
    body = re.sub(r"^use [^;]*;\n", "", text, flags=re.M | re.S)
    for name in DROPPABLE:
        if re.search(rf"\b{name}\b", body):
            continue

        def fix(m: re.Match) -> str:
            stmt = m.group(0)
            if not re.search(rf"\b{name}\b", stmt):
                return stmt
            brace = re.match(r"use ([\w:]+)::\{([^}]*)\};\n", stmt, re.S)
            if brace:
                names = [n.strip() for n in brace.group(2).split(",") if n.strip()]
                names = [n for n in names if n != name]
                if not names:
                    return ""
                if len(names) == 1:
                    return f"use {brace.group(1)}::{names[0]};\n"
                return f"use {brace.group(1)}::{{{', '.join(names)}}};\n"
            return ""

        text = re.sub(r"^use [^;]*;\n", fix, text, flags=re.M | re.S)
        body = re.sub(r"^use [^;]*;\n", "", text, flags=re.M | re.S)
    return text


def split(path: Path, check: bool) -> bool:
    feature = path.parent.name
    text = path.read_text()
    lines = text.split("\n")
    span = impl_range(lines)
    if span is None:
        return False
    start, end, ty = span
    wanted = items_of(lines, start, end)
    if not wanted:
        return False
    if check:
        print(f"{feature}: {'FeatureSheet' if opens(feature) else 'delete'} ({', '.join(n for *_, n in wanted)})")
        return True
    moved = []
    for first, last, _ in reversed(wanted):
        moved.insert(0, lines[first : last + 1])
        del lines[first : last + 1]
        end -= last - first + 1
        # the blank line that separated the item from the next one
        if first < end and not lines[first].strip():
            del lines[first]
            end -= 1
    # a blank line left at the head of the impl body
    while lines[start + 1].strip() == "":
        del lines[start + 1]
        end -= 1
    if opens(feature):
        sheet = [f"impl FeatureSheet for {ty} {{"]
        for n, item in enumerate(moved):
            if n:
                sheet.append("")
            sheet.extend(item)
        sheet.append("}")
        lines[end + 1 : end + 1] = ["", *sheet]
        out = "\n".join(lines)
        out = re.sub(
            r"^use super::(\{[^}]*\}|FeatureView);$",
            lambda m: "use super::{FeatureSheet, "
            + (m.group(1)[1:-1] if m.group(1).startswith("{") else m.group(1))
            + "};",
            out,
            count=1,
            flags=re.M,
        )
    else:
        out = drop_names("\n".join(lines))
    path.write_text(out)
    return True


def main() -> None:
    check = "--check" in sys.argv[1:]
    changed = [p for p in sorted((CRATE / "src" / "features").glob("*/mod.rs")) if split(p, check)]
    print(f"{len(changed)} file(s) {'to change' if check else 'changed'}")


if __name__ == "__main__":
    main()
