#!/usr/bin/env python3
"""Cuts a recorded fixture where the oracle's test looks at the screen: the file `<test>.jsonl` becomes
`<test>_1.jsonl` ... `<test>_N.jsonl` (the first keeps the leading `#` comment and the seed; a test opens the
first and `feed`s the rest). The cuts are line numbers of the original, each the last line of its part.

    python3 -I tools/rtwk-port/w2-cells/split.py chat_approvals_1::exec_history_extends_previous_when_consecutive 3 4 5 6 8 10
"""
import os
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
FIXTURES = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle', 'fixtures')


def split(spec, cuts):
    shard, test = spec.split('::')
    path = os.path.join(FIXTURES, shard, test + '.jsonl')
    lines = open(path, encoding='utf-8').read().splitlines(keepends=True)
    if cuts[-1] != len(lines):
        sys.exit('the last cut must be the last line, %d (the file has %d)' % (cuts[-1], len(lines)))
    start = 0
    for number, cut in enumerate(cuts, 1):
        out = os.path.join(FIXTURES, shard, '%s_%d.jsonl' % (test, number))
        open(out, 'w', encoding='utf-8').write(''.join(lines[start:cut]))
        print('wrote', os.path.relpath(out, ROOT))
        start = cut
    os.remove(path)


if __name__ == '__main__':
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    split(sys.argv[1], [int(cut) for cut in sys.argv[2:]])
