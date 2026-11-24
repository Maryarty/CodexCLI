#!/usr/bin/env python3
"""Pass / n/a / pending / todo / red per shard of the oracle suite.

    cd codex-rs/tui-rtwk
    ~/.cache/rtwk/acargo nextest run --profile oracle --no-fail-fast      # fails by design
    python3 -I tests/oracle/tools/summary.py                              # the table
    python3 -I tests/oracle/tools/summary.py --list red                   # the red tests, with the first line of why
    python3 -I tests/oracle/tools/summary.py --list pending               # pending tests: owner and reason
    python3 -I tests/oracle/tools/summary.py --list stale                 # pending tests that now pass (see --probe)
    python3 -I tests/oracle/tools/summary.py --list stale-real            # ... with a body that asserts something present (un-pend these)

Run a shard (or several) and read the result in one command, safe to run beside
other agents (each run writes its own junit file, target/nextest/oracle/junit-<run>.xml):

    python3 -I tests/oracle/tools/summary.py --run pager resume_1         # runs nextest, prints a line per shard
    python3 -I tests/oracle/tools/summary.py --run pager --list red       # ... and lists the red tests

The shard line is always printed, also when no test ran (a wrong shard name says so).

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
  stale     passed while a kit::pending marker was probing; a marker with no code after it (a usage stub)
            passes vacuously: that test is counted `pending`, not stale. A stale test whose body only asserts
            absence (assert_not_*, assert_no_*, assert_absent, assert!(!..)) or asserts nothing is listed with
            the tag `{absence-only}` / `{no-assert}`: un-pending it would prove nothing, strengthen it first
            (tools/rtwk-port/unpend-stale/unpend.py leaves such tests alone). The tag `{weak-body}` marks a
            marker whose own reason says the body checks less than the oracle claims (`probe: the body passes
            without it`, `the probe passes on the text alone`, `not verified`, `half pass`, `not written`, `holds only`) or names a gap row (G61, G-syntax):
            the pass is not the oracle's, the marker stays.

The guards (tests/oracle/guards.rs) count like any shard: a guard that fails is red, under the
shard `guards`, with the first line of its failure.
"""
import collections
import glob
import os
import re
import subprocess
import sys
import tempfile
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


# ---- the body of a stale test: does passing under the probe prove anything?

def _skip_lexeme(src, i):
    """If a comment, string, raw string or char literal starts at src[i], returns the index after it, else None."""
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
    if src[i] == '"' or src.startswith('b"', i):
        j = i + (2 if src[i] == 'b' else 1)
        while j < len(src) and src[j] != '"':
            j += 2 if src[j] == '\\' else 1
        return j + 1
    m = re.compile(r"b?'(?:\\.[^']*|[^\\'])'").match(src, i)
    if m:
        return m.end()
    return None


def _match(src, i, open_c, close_c):
    """src[i] == open_c: the index of its closing partner (strings and comments skipped)."""
    depth = 0
    while i < len(src):
        j = _skip_lexeme(src, i)
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


def test_body(shard, test):
    """The text of `fn <test>` in tests/oracle/<shard>.rs after its kit::pending( .. ); call (None: not found)."""
    path = os.path.join(CRATE, 'tests', 'oracle', shard + '.rs')
    if not os.path.exists(path):
        return None
    src = open(path, encoding='utf-8').read()
    m = re.search(r'\bfn %s\s*\(' % re.escape(test), src)
    if not m:
        return None
    start = src.find('{', m.end())
    end = _match(src, start, '{', '}')
    if start < 0 or end < 0:
        return None
    body = src[start + 1:end]
    k = re.search(r'\bkit::pending\s*\(', body)
    if not k:
        return body
    close = _match(body, k.end() - 1, '(', ')')
    return body[close + 1:] if close >= 0 else body[k.end():]


WEAK_REASON = re.compile(r'probe passes|probe: the body passes|checks less than|not verified|half pass|not written|holds only|visible half|differs only by|\bpending\(G|\bG-?[\w]*\d+:|\bG-[a-z]+')
NEGATIVE = re.compile(r'\bassert_(?:not_\w*|no_\w*|absent|title_lacks)\s*\(|\bassert(?:_eq)?!\s*\(\s*!|\bassert_ne!')
ASSERT = re.compile(r'\bassert\w*!?\s*\(|\bexpect_\w+\s*\(')


def body_kind(shard, test):
    """'stub' (nothing after the marker), 'no-assert' (code, no assertion), 'absence-only' (only negative
    assertions), 'real' (at least one assertion of something present) or None (the test was not found)."""
    body = test_body(shard, test)
    if body is None:
        return None
    # drop strings and comments so that words inside them do not count
    code, i = [], 0
    while i < len(body):
        j = _skip_lexeme(body, i)
        if j is not None:
            code.append(' ')
            i = j
        else:
            code.append(body[i])
            i += 1
    code = ''.join(code)
    if not code.strip(' \t\r\n;'):
        return 'stub'
    calls = [m for m in ASSERT.finditer(code)]
    if not calls:
        return 'no-assert'
    positive = [m for m in calls if not NEGATIVE.match(code, m.start())
                and not re.match(r'assert_text_count\s*\([^;]*,\s*0\s*\)', code[m.start():])]
    return 'real' if positive else 'absence-only'


def verdicts(path):
    root = ET.parse(path).getroot()
    out = []
    for case in root.iter('testcase'):
        name = case.get('name', '')
        shard, _, test = name.partition('::')
        if not test:
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
                kind = body_kind(shard, test)
                if kind == 'stub':
                    # nothing after the marker: the pass proves nothing, the test is still pending
                    out.append((shard, test, 'pending', probe.group(1), '{stub} ' + probe.group(2)))
                elif kind in ('absence-only', 'no-assert'):
                    out.append((shard, test, 'stale', probe.group(1), '{%s} %s' % (kind, probe.group(2))))
                elif WEAK_REASON.search(probe.group(2)):
                    out.append((shard, test, 'stale', probe.group(1), '{weak-body} ' + probe.group(2)))
                else:
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


def run_shards(shards):
    """Runs the oracle profile for `shards` with a junit file of this run's own; returns its path."""
    run = '%s-%d' % ('+'.join(shards), os.getpid())
    config = open(os.path.join(CRATE, '.config', 'nextest.toml')).read()
    config = config.replace('path = "junit.xml"', 'path = "junit-%s.xml"' % run)
    filt = 'test(/^(%s)::/)' % '|'.join(re.escape(x) for x in shards)
    with tempfile.NamedTemporaryFile('w', suffix='.toml', delete=False) as f:
        f.write(config)
        cfg = f.name
    try:
        cmd = [os.path.expanduser('~/.cache/rtwk/acargo'), 'nextest', 'run', '--profile', 'oracle',
               '--config-file', cfg, '--no-fail-fast', '--status-level', 'none',
               '--final-status-level', 'none', '--hide-progress-bar', '-E', filt]
        proc = subprocess.run(cmd, cwd=CRATE, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    finally:
        os.unlink(cfg)
    found = glob.glob(os.path.join(CRATE, 'target', 'nextest', 'oracle', 'junit-%s.xml' % run))
    target = os.environ.get('CARGO_TARGET_DIR')
    if not found and target:
        found = glob.glob(os.path.join(target, 'nextest', 'oracle', 'junit-%s.xml' % run))
    if not found:
        sys.stderr.write(proc.stderr[-3000:])
        sys.exit('nextest wrote no junit-%s.xml: the build failed? run it by hand: acargo nextest run --profile oracle -E \'%s\'' % (run, filt))
    return found[0]


def main(argv):
    path = None
    want = None
    shards = []
    run = False
    args = argv[1:]
    while args:
        a = args.pop(0)
        if a == '--junit':
            path = args.pop(0)
        elif a == '--list':
            want = args.pop(0)
        elif a == '--run':
            run = True
        elif a == '--shard':
            shards.append(args.pop(0))
        elif run and not a.startswith('-'):
            shards.append(a)
        else:
            sys.exit(__doc__)
    if run:
        if not shards:
            sys.exit('--run needs the shard names: --run pager resume_1')
        path = run_shards(shards)
    if path is None:
        path = JUNIT
    if not os.path.exists(path):
        sys.exit('no %s: run `summary.py --run <shard>` (or `acargo nextest run --profile oracle --no-fail-fast`) first' % path)
    rows = verdicts(path)
    if shards:
        rows = [r for r in rows if r[0] in shards]
    if want:
        real_only = want == 'stale-real'
        if real_only:
            want = 'stale'
        for shard, test, kind, owner, why in rows:
            if kind == want and not (real_only and why.startswith('{')):
                print('%s::%s  %s%s' % (shard, test, ('[%s] ' % owner) if owner else '', why))
        return
    per = collections.OrderedDict((s, collections.Counter()) for s in shards)
    for shard, test, kind, owner, why in rows:
        per.setdefault(shard, collections.Counter())[kind] += 1
    total = collections.Counter()
    print('%-24s %5s %5s %7s %5s %5s %6s %6s' % ('shard', 'pass', 'n/a', 'pending', 'todo', 'red', 'stale', 'total'))
    for shard, c in sorted(per.items()):
        total.update(c)
        note = '' if sum(c.values()) else '   <- no test of this shard ran (wrong name?)'
        print('%-24s %5d %5d %7d %5d %5d %6d %6d%s' % ((shard,) + tuple(c[k] for k in KINDS) + (sum(c.values()), note)))
    print('%-24s %5d %5d %7d %5d %5d %6d %6d' % (('TOTAL',) + tuple(total[k] for k in KINDS) + (sum(total.values()),)))
    owners = collections.Counter(owner for _s, _t, kind, owner, _w in rows if kind == 'pending')
    if owners:
        print('\npending by slice: ' + ', '.join('%s %d' % kv for kv in sorted(owners.items())))


if __name__ == '__main__':
    main(sys.argv)
