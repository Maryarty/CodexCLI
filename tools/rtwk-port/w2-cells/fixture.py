#!/usr/bin/env python3
"""Copies the oracle's own recorded fixture of a test into the oracle suite's fixtures.

    python3 -I tools/rtwk-port/w2-cells/fixture.py chat_approvals_1::interrupt_preserves_unified_exec_processes [...]

Reads ~/.cache/codexcli/oracle-gen/fixtures/<shard>/<test>.jsonl (the exact `ServerNotification` values the
oracle's test fed its widget, recorded by tools/rtwk-port/oracle-seeds) and writes
codex-rs/tui-rtwk/tests/oracle/fixtures/<shard>/<test>.jsonl: the recorder's widget has no thread id, so
`"threadId": ""` becomes `"thread-1"` (the thread the `session` seed opens) and `"thread_id": ""` likewise.
The `screen` / `fact` steps and the test body stay hand work.
"""
import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
GEN = os.path.expanduser('~/.cache/codexcli/oracle-gen/fixtures')
OUT = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle', 'fixtures')
THREAD = 'thread-1'


def copy(name):
    shard, test = name.split('::')
    src = os.path.join(GEN, shard, test + '.jsonl')
    if not os.path.exists(src):
        sys.exit('no recorded fixture %s' % src)
    out = []
    for line in open(src, encoding='utf-8'):
        line = re.sub(r'"(threadId|thread_id)"\s*:\s*""', lambda m: '"%s":"%s"' % (m.group(1), THREAD), line)
        out.append(line)
    dst_dir = os.path.join(OUT, shard)
    os.makedirs(dst_dir, exist_ok=True)
    dst = os.path.join(dst_dir, test + '.jsonl')
    open(dst, 'w', encoding='utf-8').write(''.join(out))
    print('wrote', os.path.relpath(dst, ROOT))


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    for arg in sys.argv[1:]:
        copy(arg)
