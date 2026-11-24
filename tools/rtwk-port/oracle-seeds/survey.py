#!/usr/bin/env python3
"""Survey: which pending oracle tests wait only on a seed or fixture, and how much of a
fixture an oracle test body lets a script infer.

    python3 -I tools/rtwk-port/oracle-seeds/survey.py [--md] [--list CLASS]

Reads codex-rs/tui-rtwk/tests/oracle/*.rs (every `// ORACLE file:line name ...` test; a test
is pending when its body holds `kit::pending(..)`/`kit::missing(..)`) and the oracle's test
body at `file:line` under codex-rs/tui/src.

Blocker class of a pending reason (first match):
  fixture   the reason says the fixture/script is missing or not scripted yet: the only class
            a fixture generator can turn green
  keymap    `.keymap(..)` needs a config the port does not read
  entry     no debug entry with the test's data (a screen directory's entry, src/)
  feature   the screen/widget/flow does not exist in the port
  kit-seed  the kit cannot set the state (disconnect, queued, image file, ...)
  other

Oracle form of the test body (what a generator would have to read):
  notif     builds `ServerNotification::..` literals (still through test helpers)
  events    drives a chat widget (`make_chatwidget_manual`, `chat.on_*`, helpers)
  cell      builds a history cell / exec cell directly
  widget    builds a bottom-pane view, picker, composer or transcript view directly
"""
import collections
import glob
import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
TESTS = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle')
ORACLE = os.path.join(ROOT, 'codex-rs', 'tui', 'src')

CLASSES = [
    ('fixture', r'no fixture|not scripted|fixture yet|no (seed )?script\b|needs a fixture'),
    ('keymap', r'keymap'),
    ('entry', r'\bno (debug )?entry|entry with|no [\w -]*entry'),
    ('feature', r'not built|\bno [\w /`\-.,()]+ (in|on) the (port|chat|ported chat)|has no widget|not dispatched|is not drawn|not drawn|does not exist|no widget'),
    ('kit-seed', r'\bno seed|seed (does not|is not|for)|kit (cannot|seeds no|has no)|not seedable|cannot (set|drive|hold|attach)|no [\w -]*seed'),
]


def classify(reason):
    for name, rx in CLASSES:
        if re.search(rx, reason, re.I):
            return name
    return 'other'


def oracle_body(loc):
    path, line = loc.rsplit(':', 1)
    try:
        lines = open(os.path.join(ORACLE, path)).read().split('\n')
    except OSError:
        return ''
    i = int(line) - 1
    while i < len(lines) and 'fn ' not in lines[i]:
        i += 1
    out, depth, started = [], 0, False
    for text in lines[i:]:
        out.append(text)
        depth += text.count('{') - text.count('}')
        started = started or '{' in text
        if started and depth <= 0:
            break
    return '\n'.join(out)


def oracle_form(body):
    if 'ServerNotification::' in body or 'handle_server_notification' in body:
        return 'notif'
    if 'make_chatwidget_manual' in body:
        return 'events'
    if re.search(r'\bnew_\w+\(|HistoryCell|ExecCell|McpToolCallCell|\bcell\b', body):
        return 'cell'
    return 'widget'


def tests():
    for f in sorted(glob.glob(os.path.join(TESTS, '*.rs'))):
        text = open(f).read()
        for part in re.split(r'(?m)^// ORACLE ', text)[1:]:
            m = re.match(r'(\S+) (\S+) kind=(\S+) owner=(\S+)', part.split('\n', 1)[0])
            if not m:
                continue
            pm = re.search(r'kit::(pending|missing)\(\s*(?:Owner::(\w+),\s*)?((?:"(?:[^"\\]|\\.)*"\s*)+),?\s*\)', part, re.S)
            reason = ''.join(re.findall(r'"((?:[^"\\]|\\.)*)"', pm.group(3))) if pm else ''
            after = re.search(r'kit::(?:pending|missing)\(.*?\n    \);\n(.*?)\n\}\n', part, re.S)
            stub = bool(pm) and not re.sub(r'(?m)^// ----.*$', '', after.group(1) if after else '').strip()
            yield dict(shard=os.path.basename(f)[:-3], loc=m.group(1), name=m.group(2), owner=m.group(4),
                       pending=bool(pm), reason=reason, stub=stub,
                       fixture=bool(re.search(r'include_str!', part)))


def main():
    args = sys.argv[1:]
    rows = list(tests())
    pend = [r for r in rows if r['pending']]
    for r in pend:
        r['cls'] = classify(r['reason'])
        r['form'] = oracle_form(oracle_body(r['loc']))
    if '--list' in args:
        want = args[args.index('--list') + 1]
        for r in pend:
            if r['cls'] == want:
                print(f"{r['shard']}::{r['name']} [{r['owner']}/{r['form']}] {r['reason'][:140]}")
        return
    out = []
    out.append(f"oracle tests {len(rows)}; pending {len(pend)} (stubs with no body {sum(r['stub'] for r in pend)}); "
               f"tests with a fixture file {sum(r['fixture'] for r in rows)}")
    cls = collections.Counter(r['cls'] for r in pend)
    out.append('blocker class: ' + ', '.join(f'{k} {v}' for k, v in cls.most_common()))
    out.append('')
    forms = sorted({r['form'] for r in pend})
    out.append('| class | ' + ' | '.join(forms) + ' | total |')
    out.append('|---|' + '---|' * (len(forms) + 1))
    for k, _ in cls.most_common():
        cells = [sum(1 for r in pend if r['cls'] == k and r['form'] == f) for f in forms]
        out.append(f'| {k} | ' + ' | '.join(map(str, cells)) + f' | {sum(cells)} |')
    out.append('')
    out.append('| owner | ' + ' | '.join(k for k, _ in cls.most_common()) + ' | total |')
    out.append('|---|' + '---|' * (len(cls) + 1))
    owners = collections.Counter(r['owner'] for r in pend)
    for o, n in owners.most_common():
        out.append(f'| {o} | ' + ' | '.join(str(sum(1 for r in pend if r['owner'] == o and r['cls'] == k)) for k, _ in cls.most_common()) + f' | {n} |')
    out.append('')
    out.append('fixture-class tests (the generator-addressable set):')
    for r in pend:
        if r['cls'] == 'fixture':
            out.append(f"- {r['shard']}::{r['name']} ({r['owner']}, {r['form']}): {r['reason'][:110]}")
    print('\n'.join(out))


if __name__ == '__main__':
    main()
