#!/usr/bin/env python3
"""Wires the recorded fixture of a stub test into its screen: `.live().open()` becomes
`.live().script(include_str!("fixtures/<shard>/<test>.jsonl")).open()` in the first `Screen::build` chain of the test
(run `fixture.py` first; then `rustfmt --edition 2024 tests/oracle/<shard>.rs`).

    python3 -I tools/rtwk-port/w2-cells/wire.py chat_approvals_1::exec_history_cell_shows_working_then_completed [...]
"""
import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
SUITE = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle')


def wire(spec):
    shard, test = spec.split('::')
    path = os.path.join(SUITE, shard + '.rs')
    src = open(path, encoding='utf-8').read()
    head = re.search(r'^fn %s\(\) \{' % re.escape(test), src, re.M)
    if not head:
        sys.exit('no test %s' % spec)
    end = src.find('\n}\n', head.end())
    body = src[head.start():end]
    if '.script(' in body:
        print('already wired', spec)
        return
    new = re.sub(
        r'(Screen::build\([^)]*\)[^;]*?)\.open\(\)',
        lambda m: '%s.script(include_str!("fixtures/%s/%s.jsonl")).open()' % (m.group(1), shard, test),
        body,
        count=1,
    )
    if new == body:
        sys.exit('no Screen::build chain in %s' % spec)
    open(path, 'w', encoding='utf-8').write(src[:head.start()] + new + src[end:])
    print('wired', spec)


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    for arg in sys.argv[1:]:
        wire(arg)
