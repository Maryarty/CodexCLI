#!/usr/bin/env python3
"""Removes the kit::pending marker of every oracle test that passes under the probe (stale), one commit per W2 package.

    cd <worktree>
    PORTER_RUN_OK=1 python3 -I tools/rtwk-port/unpend-stale/unpend.py --probe --dry-run   # one KIT_PROBE_PENDING=1 run, then the plan
    python3 -I tools/rtwk-port/unpend-stale/unpend.py --junit codex-rs/tui-rtwk/target/nextest/oracle/junit.xml --commit

Input: the junit of a `KIT_PROBE_PENDING=1 acargo nextest run --profile oracle --no-fail-fast` run (`--probe` makes it,
into target/nextest/oracle/junit.xml), read with tests/oracle/tools/summary.py: the tests of `--list stale-real`
(a stub with no body, an absence-only or assert-free body, and a marker whose own reason says the body checks less than
the oracle claims, stay pending: summary.py tags them `{stub}` `{absence-only}` `{no-assert}` `{weak-body}`).
Package of a test: map-full/tests_rules.py (the reason of map-full/pending-tests.tsv when the test is there).
Edit: the `kit::pending( .. );` statement leaves the test fn (strings and comments are skipped, not parsed as code); the
`self` / `Owner` of `use crate::kit::{..}` and a `const X: &str` reason leave with their last user; the row leaves pending-tests.tsv. `--commit` commits
per package (oracle files + the tsv, staged by path); at the end map-full/build.py regenerates the port-map and unpend/*.txt.
Rerunnable after every landing: with no stale test it changes nothing.
"""
import argparse
import collections
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
CRATE = os.path.join(ROOT, 'codex-rs', 'tui-rtwk')
sys.path.insert(0, os.path.join(CRATE, 'tests', 'oracle', 'tools'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full'))
import summary  # noqa: E402
import tests_rules  # noqa: E402

TSV = os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full', 'pending-tests.tsv')
JUNIT = os.path.join(CRATE, 'target', 'nextest', 'oracle', 'junit.xml')


def probe():
    env = dict(os.environ, KIT_PROBE_PENDING='1')
    cmd = [os.path.expanduser('~/.cache/rtwk/acargo'), 'nextest', 'run', '--profile', 'oracle', '--no-fail-fast',
           '--status-level', 'none', '--final-status-level', 'none', '--hide-progress-bar']
    subprocess.run(cmd, cwd=CRATE, env=env, stdout=subprocess.DEVNULL)  # fails by design
    target = os.environ.get('CARGO_TARGET_DIR')
    path = os.path.join(target, 'nextest', 'oracle', 'junit.xml') if target else JUNIT
    if not os.path.exists(path):
        sys.exit('the probe run wrote no %s' % path)
    return path


def tsv_reasons():
    out = {}
    for line in open(TSV, encoding='utf-8'):
        n, _s, r = line.rstrip('\n').split('\t')
        out[n] = r
    return out


def remove_marker(src, test):
    """src without the `kit::pending( .. );` statement of `fn test`; None when not found."""
    m = re.search(r'\bfn %s\s*\(' % re.escape(test), src)
    if not m:
        return None
    start = src.find('{', m.end())
    end = summary._match(src, start, '{', '}')
    body = src[start + 1:end]
    k = re.search(r'\bkit::pending\s*\(', body)
    if not k:
        return None
    close = summary._match(body, k.end() - 1, '(', ')')
    stop = close + 1
    while stop < len(body) and body[stop] in ' \t':
        stop += 1
    if body[stop:stop + 1] == ';':
        stop += 1
    if body[stop:stop + 1] == '\n':
        stop += 1
    begin = body.rfind('\n', 0, k.start()) + 1
    if body[begin:k.start()].strip():
        begin = k.start()
    return src[:start + 1] + body[:begin] + body[stop:] + src[end:]


def tidy_imports(src):
    """Drops `self` / `Owner` from `use crate::kit::{..};` when nothing else of the file uses them."""
    m = re.search(r'^use crate::kit::\{([^}]*)\};\n', src, re.M)
    if not m:
        return src
    rest = src[:m.start()] + src[m.end():]
    rest = '\n'.join(l for l in rest.split('\n') if not l.lstrip().startswith('//'))
    names = [x.strip() for x in m.group(1).split(',') if x.strip()]
    keep = []
    for n in names:
        used = re.search(r'\bkit::', rest) if n == 'self' else re.search(r'\b%s\b' % re.escape(n), rest)
        if used:
            keep.append(n)
    if keep == names:
        return src
    if keep == ['self']:
        new = 'use crate::kit;\n'
    elif keep:
        new = 'use crate::kit::{%s};\n' % ', '.join(keep)
    else:
        new = ''
    return src[:m.start()] + new + src[m.end():]


def tidy_consts(src):
    """Drops `const NAME: &str = ..;` (reasons shared by several markers) that no code of the file names any more."""
    while True:
        for m in re.finditer(r'^const (\w+): &str =', src, re.M):
            name = m.group(1)
            if len(re.findall(r'\b%s\b' % name, src)) > 1:
                continue
            i = m.end()
            while i < len(src) and src[i] != ';':
                j = summary._skip_lexeme(src, i)
                i = j if j is not None else i + 1
            end = i + 1
            if src[end:end + 1] == '\n':
                end += 1
            begin = m.start()
            while True:  # the comment lines right above go with it
                prev = src.rfind('\n', 0, begin - 1) + 1
                if begin > 0 and src[prev:begin].lstrip().startswith('//'):
                    begin = prev
                else:
                    break
            src = src[:begin] + src[end:]
            break
        else:
            return src


def git(*args, check=True):
    return subprocess.run(['git', '-C', ROOT] + list(args), check=check, capture_output=True, text=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--junit', help='junit.xml of a KIT_PROBE_PENDING=1 run (default: target/nextest/oracle/junit.xml)')
    ap.add_argument('--probe', action='store_true', help='run the probe first (one oracle run; PORTER_RUN_OK=1 for the hook)')
    ap.add_argument('--dry-run', action='store_true', help='print the plan, change nothing')
    ap.add_argument('--commit', action='store_true', help='one commit per package')
    ap.add_argument('--trailer', default='', help='text appended to every commit message (attribution lines)')
    ap.add_argument('--only', action='append', default=[], help='only this package (repeatable)')
    a = ap.parse_args()
    path = probe() if a.probe else (a.junit or JUNIT)
    reasons = tsv_reasons()
    rows = [r for r in summary.verdicts(path) if r[2] == 'stale']
    real = [r for r in rows if not r[4].startswith('{')]
    skipped = collections.Counter(re.match(r'\{([\w-]+)\}', r[4]).group(1) for r in rows if r[4].startswith('{'))
    plan = collections.defaultdict(list)
    for shard, test, _k, _o, why in real:
        name = '%s::%s' % (shard, test)
        plan[tests_rules.classify(name, reasons.get(name, why))].append((shard, test))
    print('stale: %d, real-body: %d, left pending (%s)' % (
        len(rows), len(real), ', '.join('%d %s' % (v, k) for k, v in sorted(skipped.items())) or 'none'))
    for pkg in sorted(plan):
        print('  %-14s %3d  %s' % (pkg, len(plan[pkg]), ', '.join(sorted({s for s, _ in plan[pkg]}))))
    if a.dry_run:
        return
    done = 0
    for pkg in sorted(plan):
        if a.only and pkg not in a.only:
            continue
        files = set()
        names = []
        for shard, test in plan[pkg]:
            f = os.path.join(CRATE, 'tests', 'oracle', shard + '.rs')
            src = open(f, encoding='utf-8').read()
            new = remove_marker(src, test)
            if new is None:
                print('  not found / no marker: %s::%s' % (shard, test))
                continue
            open(f, 'w', encoding='utf-8').write(tidy_consts(tidy_imports(new)))
            files.add(f)
            names.append('%s::%s' % (shard, test))
        if not names:
            continue
        lines = open(TSV, encoding='utf-8').read().split('\n')
        gone = set(names)
        open(TSV, 'w', encoding='utf-8').write('\n'.join(l for l in lines if l.split('\t')[0] not in gone))
        done += len(names)
        if a.commit:
            git('add', '--', *sorted(files), TSV)
            msg = 'unpend-stale: %s: %d stale oracle tests lose their kit::pending marker\n\n%s\n' % (
                pkg, len(names), '\n'.join(sorted(names)))
            git('commit', '-q', '-m', msg + (('\n' + a.trailer + '\n') if a.trailer else ''))
        print('  %s: %d tests, %d files' % (pkg, len(names), len(files)))
    if done:
        subprocess.run([sys.executable, '-I', os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full', 'build.py')], check=True)
        if a.commit:
            git('add', '--', 'tools/rtwk-port/map-full', 'docs/rtwk-port/reports')
            git('commit', '-q', '-m', 'unpend-stale: map-full/build.py regenerated (port-map, unpend/*.txt)\n' + (('\n' + a.trailer + '\n') if a.trailer else ''))
    print('un-pended: %d' % done)


if __name__ == '__main__':
    main()
