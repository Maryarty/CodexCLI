#!/usr/bin/env python3
"""Names the gap row in the pending reason of the markdown tests it blocks (rtwk-gaps.md).

The pending reasons of tests/oracle/markdown_streaming_*.rs that say what the chat lacks
(`the chat has no math renderer ...`, `the chat draws no table ...`) start with the gap row
(`G61: ...`, `G73: ...`), as the other pending markdown tests do: a gap that closes is found
with `rg 'G61' tests/oracle`. The pending-tests.tsv row of each is changed the same way.

    cd <worktree> && python3 -I tools/rtwk-port/w2-live/gap_reasons.py

Rerunnable: a reason that already starts with its row is left alone.
"""
import glob
import re

# reason start (as the source has it after the opening quote) -> the gap row it belongs to
ROWS = [
    ("the chat has no math renderer", "G61"),
    ("the chat draws no table", "G73"),
    ("the chat draws a table as its plain text", "G73"),
    ("the chat draws no pipe-table fallback", "G73"),
    ("the chat has no list spacing choice", "G73"),
]
TESTS = "codex-rs/tui-rtwk/tests/oracle/markdown_streaming_*.rs"
TSV = "tools/rtwk-port/map-full/pending-tests.tsv"


def prefixed(text):
    for start, row in ROWS:
        if text.startswith(start):
            return f"{row}: {text}"
    return text


def main():
    changed = 0
    for path in sorted(glob.glob(TESTS)):
        src = open(path).read()
        out = re.sub(
            r'("(?:%s))' % "|".join(re.escape(start) for start, _ in ROWS),
            lambda m: '"' + prefixed(m.group(1)[1:]),
            src,
        )
        if out != src:
            open(path, "w").write(out)
            changed += 1
    rows = []
    for line in open(TSV).read().split("\n"):
        cols = line.split("\t")
        if len(cols) >= 3 and cols[0].startswith("markdown_streaming_"):
            cols[2] = prefixed(cols[2])
        rows.append("\t".join(cols))
    open(TSV, "w").write("\n".join(rows))
    print(f"{changed} test files, tsv synced")


main()
