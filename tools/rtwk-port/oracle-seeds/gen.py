#!/usr/bin/env python3
"""Oracle recorder log -> `.jsonl` step fixtures (the half of the generator that can be exact).

    python3 -I tools/rtwk-port/oracle-seeds/gen.py LOG [--out DIR] [--force]
    python3 -I tools/rtwk-port/oracle-seeds/gen.py --selftest

Why a log and not the Rust test body: the oracle's tests build their server messages through
test helpers (`handle_turn_started`, `begin_exec`, `test_thread_goal`, `compaction_started`, ...),
so reading the body would mean interpreting Rust. The oracle's own run knows the exact
`ServerNotification` values: a `cfg(test)` recorder in `chatwidget/protocol.rs`
`handle_server_notification` (orchestrator-owned file, see README) appends one line per call:

    {"test": "<fn name>", "replay": null | "<ReplayKind>", "message": <ServerNotification as serde_json>}

(`handle_server_request` the same with `"request": true`). This script groups the lines by test,
finds the test in the shards (`// ORACLE file:line name`) and writes
`codex-rs/tui-rtwk/tests/oracle/fixtures/<shard>/<name>.jsonl`:

    # oracle <file>:<line> <name>
    {"step": "seed", "seed": "session", "thread": "thread-1"}
    {"step": "notify", "message": {...}}          one per recorded call

An existing fixture is never overwritten (hand-written ones carry `screen`/`fact` steps and
sizes the log cannot know) unless --force; the file is then written beside it as `<name>.gen.jsonl`.
A test with a replayed notification (`replay` not null) is reported, not written: the script has
no replay step (gap), so those stay by hand.
"""
import collections
import glob
import json
import os
import re
import sys
import tempfile

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..'))
TESTS = os.path.join(ROOT, 'codex-rs', 'tui-rtwk', 'tests', 'oracle')


def manifest():
    """name -> (shard, loc) over every `// ORACLE loc name ...` line."""
    out = {}
    for f in sorted(glob.glob(os.path.join(TESTS, '*.rs'))):
        for m in re.finditer(r'(?m)^// ORACLE (\S+) (\S+) kind=', open(f).read()):
            out[m.group(2)] = (os.path.basename(f)[:-3], m.group(1))
    return out


def render(loc, name, calls):
    lines = [f'# oracle {loc} {name}', json.dumps({'step': 'seed', 'seed': 'session', 'thread': 'thread-1'})]
    for c in calls:
        step = 'request' if c.get('request') else 'notify'
        lines.append(json.dumps({'step': step, 'message': c['message']}))
    return '\n'.join(lines) + '\n'


def generate(log_path, out_dir, force=False):
    names = manifest()
    by_test = collections.defaultdict(list)
    for line in open(log_path):
        if line.strip():
            c = json.loads(line)
            by_test[c['test']].append(c)
    stats = collections.Counter()
    for test, calls in sorted(by_test.items()):
        if test not in names:
            stats['unknown test'] += 1
            print(f'unknown test {test}', file=sys.stderr)
            continue
        shard, loc = names[test]
        if any(c.get('replay') for c in calls):
            stats['replayed (by hand)'] += 1
            print(f'{shard}::{test}: replayed notification, stays by hand', file=sys.stderr)
            continue
        d = os.path.join(out_dir, shard)
        os.makedirs(d, exist_ok=True)
        path = os.path.join(d, test + '.jsonl')
        if os.path.exists(path) and not force:
            path = os.path.join(d, test + '.gen.jsonl')
            stats['beside existing'] += 1
        else:
            stats['written'] += 1
        with open(path, 'w') as fh:
            fh.write(render(loc, test, calls))
    print(dict(stats))


def selftest():
    """A log made from a fixture regenerates that fixture (modulo its header comment)."""
    names = manifest()
    src = os.path.join(TESTS, 'fixtures', 'cells_1', 'code_mode_output_shares_a_row_budget_across_blocks.jsonl')
    msgs = [json.loads(l)['message'] for l in open(src) if l.startswith('{') and '"notify"' in l]
    test = 'code_mode_output_shares_a_row_budget_across_blocks'
    assert test in names, 'manifest lost the sample test'
    with tempfile.TemporaryDirectory() as tmp:
        log = os.path.join(tmp, 'log.jsonl')
        with open(log, 'w') as fh:
            for m in msgs:
                fh.write(json.dumps({'test': test, 'replay': None, 'message': m}) + '\n')
        generate(log, tmp)
        got = [json.loads(l) for l in open(os.path.join(tmp, names[test][0], test + '.jsonl')) if l.startswith('{')]
    want = [json.loads(l) for l in open(src) if l.startswith('{')]
    assert got == want, 'regenerated steps differ from the hand-written fixture'
    print('selftest ok')


if __name__ == '__main__':
    a = sys.argv[1:]
    if a == ['--selftest']:
        selftest()
    elif a and not a[0].startswith('-'):
        out = a[a.index('--out') + 1] if '--out' in a else os.path.join(TESTS, 'fixtures')
        generate(a[0], out, '--force' in a)
    else:
        print(__doc__)
