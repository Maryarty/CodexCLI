#!/usr/bin/env python3
"""Drops the rows of map-full/pending-tests.tsv of tests whose body no longer holds a kit::pending marker.

    python3 -I tools/rtwk-port/unpend-stale/drop_rows.py [--dry-run] [NAME ...]   # NAME: shard::test, none = every such row

For a test a porter un-pended by hand (the body now passes, or is `kit::na`): `unpend.py` removes the marker and the row of
a test it finds stale under a probe run; this removes only the row, after the marker went by hand. Then rerun
`python3 -I tools/rtwk-port/map-full/build.py` and `check.py`.
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
ORACLE = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle')
TSV = os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full', 'pending-tests.tsv')


def match(src, start, open_, close):
    depth = 0
    i = start
    while i < len(src):
        c = src[i]
        if c == '"':
            i += 1
            while i < len(src) and src[i] != '"':
                i += 2 if src[i] == '\\' else 1
        elif src.startswith('//', i):
            i = src.find('\n', i)
            if i < 0:
                break
        elif c == open_:
            depth += 1
        elif c == close:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return len(src) - 1


def pending(shard, test):
    """True while `fn test` of the shard holds a kit::pending statement; None when the fn is not there."""
    path = os.path.join(ORACLE, shard + '.rs')
    if not os.path.exists(path):
        return None
    src = open(path, encoding='utf-8').read()
    m = re.search(r'\bfn %s\s*\(' % re.escape(test), src)
    if not m:
        return None
    start = src.find('{', m.end())
    body = src[start + 1:match(src, start, '{', '}')]
    return re.search(r'\bkit::pending\s*\(', body) is not None


def main():
    args = [a for a in sys.argv[1:] if not a.startswith('--')]
    dry = '--dry-run' in sys.argv
    keep, dropped = [], []
    for line in open(TSV, encoding='utf-8'):
        name = line.split('\t', 1)[0]
        shard, _, test = name.partition('::')
        if (not args or name in args) and pending(shard, test) is False:
            dropped.append(name)
        else:
            keep.append(line)
    for name in dropped:
        print('drop', name)
    if not dry and dropped:
        open(TSV, 'w', encoding='utf-8').writelines(keep)
    print('%d row(s) %s' % (len(dropped), 'to drop' if dry else 'dropped'))


if __name__ == '__main__':
    main()
