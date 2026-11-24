#!/usr/bin/env python3 -I
"""A `link` field on the picker's `Span` (doors-r1 item 6): every struct literal of the picker's
`Span { text, role }` in tui-rtwk's src gets `link: None`.

    python3 -I tools/rtwk-port/doors-r1/span_link.py

A literal is `Span {` (not `Span::`, not a function's `-> Span {`) whose body names `role` and no `link` and no `..` base;
`link: None,` goes before the closing brace. Rerunnable: a literal that has `link` is left.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3] / "codex-rs" / "tui-rtwk"
LITERAL = re.compile(r"(?<![\w:])(?<!-> )Span \{")


def body_end(text: str, start: int) -> int:
    """The index of the `}` that closes the `{` at `start`."""
    depth = 0
    for at in range(start, len(text)):
        if text[at] == "{":
            depth += 1
        elif text[at] == "}":
            depth -= 1
            if depth == 0:
                return at
    raise ValueError("unclosed")


def patch(text: str) -> tuple[str, int]:
    done = 0
    at = 0
    while True:
        found = LITERAL.search(text, at)
        if not found:
            return text, done
        open_at = found.end() - 1
        close = body_end(text, open_at)
        body = text[open_at + 1 : close]
        at = open_at + 1
        if not re.search(r"\brole\b", body) or re.search(r"\blink\b|\.\.", body):
            continue
        inner = body.rstrip()
        sep = "" if inner.endswith(",") else ","
        if "\n" in body:
            indent = re.search(r"\n(\s*)\S", body)
            pad = indent.group(1) if indent else ""
            tail = body[len(inner):]
            new = f"{inner}{sep}\n{pad}link: None,{tail}"
        else:
            new = f"{inner}{sep} link: None "
        text = text[: open_at + 1] + new + text[close:]
        done += 1
        at = open_at + 1 + len(new)


def main() -> int:
    total = 0
    for path in sorted([*ROOT.joinpath("src").rglob("*.rs"), *ROOT.joinpath("core/src").rglob("*.rs")]):
        old = path.read_text()
        if "Span {" not in old:
            continue
        new, count = patch(old)
        if count:
            path.write_text(new)
            total += count
            print(f"{path.relative_to(ROOT)}: {count}")
    print(f"{total} literals")
    return 0


if __name__ == "__main__":
    sys.exit(main())
