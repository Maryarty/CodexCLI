#!/usr/bin/env python3
"""Pass / n/a / pending / todo / red per shard of the oracle suite.

    cd codex-rs/tui-rtwk
    ~/.cache/rtwk/acargo nextest run --profile oracle --no-fail-fast      # fails by design
    python3 -I tests/oracle/tools/summary.py                              # the table
    python3 -I tests/oracle/tools/summary.py --list red                   # the red tests, with the first line of why
    python3 -I tests/oracle/tools/summary.py --list pending               # pending tests: owner and reason
    python3 -I tests/oracle/tools/summary.py --list stale                 # pending tests that now pass (see --probe)

Probe for stale markers: run the suite with KIT_PROBE_PENDING=1 (the marker lets
the test body run instead of failing); a pending test that passes then is stale
(the thing exists now: remove its kit::pending line). Probe runs are for
`--list stale` only: their numbers are not the suite's.

Verdicts (tests/kit/verdict.rs):
  pass      the test passed
  n/a       passed through kit::na(owner, reason)
  pending   failed with `PENDING[slice]: reason`  (kit::pending / a seed that does not exist)
  todo      failed with `TODO[port]`                (a stub nobody ported yet)
  red       any other failure: the screen differs from the oracle, or the test is wrong
  stale     passed while a kit::pending marker was probing
"""
import collections
import os
import re
import sys
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
JUNIT = os.path.join(CRATE, 'target', 'nextest', 'oracle', 'junit.xml')
KINDS = ['pass', 'n/a', 'pending', 'todo', 'red', 'stale']


def first_line(text):
    for line in (text or '').splitlines():
        line = line.strip()
        if line and not line.startswith(('thread ', 'note:', 'stack backtrace')):
            return line
    return ''


def verdicts(path):
    root = ET.parse(path).getroot()
    out = []
    for case in root.iter('testcase'):
        name = case.get('name', '')
        shard, _, test = name.partition('::')
        if not test or shard == 'guards':
            continue
        failure = case.find('failure')
        if failure is None:
            failure = case.find('error')
        stdout = (case.findtext('system-out') or '')
        stderr = (case.findtext('system-err') or '')
        ftext = (failure.text or '') if failure is not None else ''
        text = '\n'.join([ftext, stdout, stderr])
        if failure is None:
            na = re.search(r'N/A\[([\w-]+)\]: (.*)', text)
            probe = re.search(r'PROBE\[([\w-]+)\]: (.*)', text)
            if probe:
                out.append((shard, test, 'stale', probe.group(1), probe.group(2)))
            elif na:
                out.append((shard, test, 'n/a', na.group(1), na.group(2)))
            else:
                out.append((shard, test, 'pass', '', ''))
            continue
        pend = re.search(r'PENDING\[([\w-]+)\]: (.*)', text)
        if pend:
            out.append((shard, test, 'pending', pend.group(1), pend.group(2)))
        elif 'TODO[port]' in text:
            out.append((shard, test, 'todo', '', ''))
        else:
            body = failure.text or ''
            out.append((shard, test, 'red', '', first_line(body) or failure.get('message', '')))
    return out


def main(argv):
    path = JUNIT
    want = None
    args = argv[1:]
    while args:
        a = args.pop(0)
        if a == '--junit':
            path = args.pop(0)
        elif a == '--list':
            want = args.pop(0)
        else:
            sys.exit(__doc__)
    if not os.path.exists(path):
        sys.exit('no %s: run `acargo nextest run --profile oracle --no-fail-fast` first' % path)
    rows = verdicts(path)
    if want:
        for shard, test, kind, owner, why in rows:
            if kind == want:
                print('%s::%s  %s%s' % (shard, test, ('[%s] ' % owner) if owner else '', why))
        return
    per = collections.OrderedDict()
    for shard, test, kind, owner, why in rows:
        per.setdefault(shard, collections.Counter())[kind] += 1
    total = collections.Counter()
    print('%-24s %5s %5s %7s %5s %5s %6s %6s' % ('shard', 'pass', 'n/a', 'pending', 'todo', 'red', 'stale', 'total'))
    for shard, c in sorted(per.items()):
        total.update(c)
        print('%-24s %5d %5d %7d %5d %5d %6d %6d' % ((shard,) + tuple(c[k] for k in KINDS) + (sum(c.values()),)))
    print('%-24s %5d %5d %7d %5d %5d %6d %6d' % (('TOTAL',) + tuple(total[k] for k in KINDS) + (sum(total.values()),)))
    owners = collections.Counter(owner for _s, _t, kind, owner, _w in rows if kind == 'pending')
    if owners:
        print('\npending by slice: ' + ', '.join('%s %d' % kv for kv in sorted(owners.items())))


if __name__ == '__main__':
    main(sys.argv)
