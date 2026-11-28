#!/usr/bin/env python3
"""Moves oracle tests from `kit::pending` to `kit::delta` where only the screen differs (kit-delta, item 5).

    cd codex-rs/tui-rtwk
    # 1. probe: the whole body of every pending test runs, a junit file is written
    KIT_PROBE_PENDING=1 ~/.cache/rtwk/acargo nextest run -p codex-tui-rtwk --test oracle --profile oracle \
        --no-fail-fast -E 'test(/^(shard_a|shard_b)::/)'       # junit: target/nextest/oracle/junit.xml
    # 2. the candidates: a pending test whose probe failure is a comparison of rows or cell styles
    python3 -I ../../tools/rtwk-port/oracle-delta/delta.py candidates target/nextest/oracle/junit.xml --shard shard_a
    # 3. convert them (kit::pending -> kit::delta, the reason becomes `what`), writing the ledger
    python3 -I ../../tools/rtwk-port/oracle-delta/delta.py apply target/nextest/oracle/junit.xml --shard shard_a
    # 4. run the shards WITHOUT the probe, then settle: a converted test that is not green goes back to
    #    kit::pending with its own reason (the kit refuses rows that say other things, a functional check
    #    that fails stays a failure), the rest are deltas
    python3 -I ../../tools/rtwk-port/oracle-delta/delta.py settle target/nextest/oracle/junit.xml
    # 5. read what each delta is: its `what` and the rows that differ
    python3 -I ../../tools/rtwk-port/oracle-delta/delta.py show target/nextest/oracle/junit.xml
    # 6. a delta you judge not small (docs/rtwk-port/visual-deltas.md) goes back by name
    python3 -I ../../tools/rtwk-port/oracle-delta/delta.py revert shard_a::a_test

The ledger (`--ledger`, default `target/oracle-delta-ledger.json`) keeps the original call of each converted test.
`fmt-all.sh` formats the shard files afterwards. Stdlib only.
"""
import json
import os
import re
import sys
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
CRATE = os.path.join(ROOT, 'codex-rs', 'tui-rtwk')
ORACLE = os.path.join(CRATE, 'tests', 'oracle')
LEDGER = os.path.join(CRATE, 'target', 'oracle-delta-ledger.json')


def skip_lexeme(src, i):
    """If a comment or string literal starts at src[i], the index after it, else None."""
    if src.startswith('//', i):
        j = src.find('\n', i)
        return len(src) if j < 0 else j
    if src.startswith('/*', i):
        j = src.find('*/', i + 2)
        return len(src) if j < 0 else j + 2
    m = re.compile(r'b?r(#*)"').match(src, i)
    if m:
        end = src.find('"' + m.group(1), m.end())
        return len(src) if end < 0 else end + 1 + len(m.group(1))
    if src[i] == '"':
        j = i + 1
        while j < len(src) and src[j] != '"':
            j += 2 if src[j] == '\\' else 1
        return j + 1
    m = re.compile(r"'(?:\\.[^']*|[^\\'])'").match(src, i)
    if m:
        return m.end()
    return None


def match(src, i, open_c, close_c):
    depth = 0
    while i < len(src):
        j = skip_lexeme(src, i)
        if j is not None:
            i = j
            continue
        if src[i] == open_c:
            depth += 1
        elif src[i] == close_c:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def fn_span(src, test):
    """(start, end) of the text of `fn <test>` up to the next top-level fn (None: not found)."""
    m = re.search(r'^fn %s\s*\(' % re.escape(test), src, re.M)
    if not m:
        return None
    nxt = re.search(r'^fn \w+', src[m.end():], re.M)
    return m.start(), (m.end() + nxt.start() if nxt else len(src))


def marker_call(src, test, name):
    """(start, end) of the first `kit::<name>(..)` call inside `fn <test>` (None: no such call)."""
    span = fn_span(src, test)
    if not span:
        return None
    at = re.compile(r'\bkit::%s\s*\(' % name).search(src, span[0], span[1])
    if not at:
        return None
    close = match(src, at.end() - 1, '(', ')')
    return (at.start(), close + 1) if close >= 0 else None


def reason_of(call):
    """The words of the string literals of a marker call, whitespace squeezed."""
    parts = re.findall(r'"((?:[^"\\]|\\.)*)"', call, re.S)
    text = ' '.join(parts)
    text = re.sub(r'\\\n\s*', '', text)
    return ' '.join(text.split())


def probe_failures(path):
    """{shard::test: kind} for the tests that failed in a probe run: 'rows', 'styles' or 'other'."""
    out = {}
    for case in ET.parse(path).getroot().iter('testcase'):
        f = case.find('failure')
        if f is None:
            f = case.find('error')
        if f is None:
            continue
        text = '\n'.join([f.text or '', case.findtext('system-out') or '', case.findtext('system-err') or ''])
        if 'PENDING[' in text or 'TODO[port]' in text or 'PROBE[' not in text:
            continue
        if re.search(r'ours \((left|right)\) differs from the oracle', text):
            out[case.get('name')] = 'rows'
        elif 'cell styles differ' in text:
            out[case.get('name')] = 'styles'
        else:
            out[case.get('name')] = 'other'
    return out


def read(shard):
    with open(os.path.join(ORACLE, shard + '.rs'), encoding='utf-8') as f:
        return f.read()


def write(shard, src):
    with open(os.path.join(ORACLE, shard + '.rs'), 'w', encoding='utf-8') as f:
        f.write(src)


def candidates(junit, shards):
    out = []
    for name, kind in sorted(probe_failures(junit).items()):
        shard, _, test = name.partition('::')
        if kind == 'other' or (shards and shard not in shards):
            continue
        if not os.path.exists(os.path.join(ORACLE, shard + '.rs')):
            continue
        span = marker_call(read(shard), test, 'pending')
        if span:
            out.append((shard, test, kind, reason_of(read(shard)[span[0]:span[1]])))
    return out


def load(path):
    return json.load(open(path)) if os.path.exists(path) else {}


def save(path, ledger):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    json.dump(ledger, open(path, 'w'), indent=1, ensure_ascii=False)


def apply(junit, shards, only, ledger_path):
    ledger = load(ledger_path)
    done = 0
    for shard, test, kind, reason in candidates(junit, shards):
        if only and '%s::%s' % (shard, test) not in only:
            continue
        src = read(shard)
        start, end = marker_call(src, test, 'pending')
        call = src[start:end]
        ledger['%s::%s' % (shard, test)] = call
        write(shard, src[:start] + 'kit::delta' + src[start + len('kit::pending'):])
        done += 1
        print('delta   %s::%s (%s)' % (shard, test, kind))
    save(ledger_path, ledger)
    print('%d converted; ledger %s' % (done, ledger_path))


def revert_one(key, ledger):
    shard, _, test = key.partition('::')
    src = read(shard)
    span = marker_call(src, test, 'delta')
    if not span or key not in ledger:
        return False
    write(shard, src[:span[0]] + ledger[key] + src[span[1]:])
    return True


def outcomes(junit):
    out = {}
    for case in ET.parse(junit).getroot().iter('testcase'):
        failed = case.find('failure') is not None or case.find('error') is not None
        out[case.get('name')] = (failed, case)
    return out


def settle(junit, ledger_path):
    ledger = load(ledger_path)
    res = outcomes(junit)
    kept, back, unseen = 0, 0, []
    for key in sorted(ledger):
        if key not in res:
            unseen.append(key)
            continue
        failed, case = res[key]
        if failed:
            f = case.find('failure') if case.find('failure') is not None else case.find('error')
            why = next((l.strip() for l in (f.text or '').splitlines()
                        if l.strip() and not l.startswith(('thread ', 'note:'))), '')
            if revert_one(key, ledger):
                back += 1
                print('pending %s: %s' % (key, why[:150]))
        else:
            kept += 1
    for key in unseen:
        print('unseen  %s (not in the junit: run its shard)' % key)
    print('%d stay deltas, %d went back to pending, %d not run' % (kept, back, len(unseen)))


def show(junit, ledger_path):
    ledger = load(ledger_path)
    for key, (failed, case) in sorted(outcomes(junit).items()):
        text = '\n'.join([case.findtext('system-out') or '', case.findtext('system-err') or ''])
        m = re.search(r'^DELTA\[([\w-]+)\]: (.*)', text, re.M)
        if failed or not m:
            continue
        print('== %s [%s] %s' % (key, m.group(1), m.group(2)[:300]))
        shown = 0
        for line in text.splitlines():
            if re.match(r'^  \| [<>]', line) and shown < 8:
                print('   ' + line.strip())
                shown += 1


def main(argv):
    args = argv[1:]
    if not args:
        sys.exit(__doc__)
    cmd, args = args[0], args[1:]
    shards, only, ledger, rest = [], [], LEDGER, []
    while args:
        a = args.pop(0)
        if a == '--shard':
            shards.append(args.pop(0))
        elif a == '--only':
            only.append(args.pop(0))
        elif a == '--ledger':
            ledger = args.pop(0)
        else:
            rest.append(a)
    if cmd == 'candidates' and rest:
        for shard, test, kind, reason in candidates(rest[0], shards):
            print('%s::%s [%s] %s' % (shard, test, kind, reason[:200]))
    elif cmd == 'apply' and rest:
        apply(rest[0], shards, set(only), ledger)
    elif cmd == 'settle' and rest:
        settle(rest[0], ledger)
    elif cmd == 'show' and rest:
        show(rest[0], ledger)
    elif cmd == 'revert' and rest:
        led = load(ledger)
        for key in rest:
            print(('pending %s' if revert_one(key, led) else 'not a converted test: %s') % key)
    else:
        sys.exit(__doc__)


if __name__ == '__main__':
    main(sys.argv)
