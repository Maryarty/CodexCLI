#!/usr/bin/env python3
"""Landings on claude/tui-rtwk and the hub files they touch.

Usage (from the checkout root): python3 -I docs/rtwk-port/research/speed-order/landings.py [--since 2026-11-22]

For each first-parent commit since --since: time, files changed against its first
parent, and which hub files it touched; then per hub file: landings touching it,
lines now, and the gaps between landings.
"""
import argparse, collections, subprocess

HUBS = [
    'src/pages/session/session_page.rs', 'src/pages/session/actions.rs',
    'core/src/threads.rs', 'core/src/commands/dispatch.rs', 'core/src/app.rs',
    'core/src/app/session.rs', 'src/sheets/select/lists.rs', 'core/src/commands.rs',
    'tests/kit/seed.rs', 'tests/kit/mod.rs', 'src/services/backend/fake.rs',
    'src/pages/session/strings.rs', 'core/src/features.rs', 'tests/oracle.rs',
]
PRE = 'codex-rs/tui-rtwk/'

def git(*a):
    return subprocess.run(['git', *a], capture_output=True, text=True, check=True).stdout

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--since', default='2026-11-22T00:00')
    ap.add_argument('--branch', default='claude/tui-rtwk')
    a = ap.parse_args()
    log = git('log', '--first-parent', f'--since={a.since}', '--format=%H|%ad|%s', '--date=format:%m-%d %H:%M', a.branch)
    hub_hits = collections.defaultdict(list)
    rows = []
    for line in log.splitlines():
        h, when, subj = line.split('|', 2)
        files = git('diff', '--name-only', f'{h}^1', h).split()
        lines = git('diff', '--shortstat', f'{h}^1', h).strip()
        hit = [f for f in HUBS if PRE + f in files]
        for f in hit:
            hub_hits[f].append(when)
        kinds = collections.Counter()
        for f in files:
            if f.startswith(PRE + 'core/src'):
                kinds['core'] += 1
            elif f.startswith(PRE + 'src'):
                kinds['view'] += 1
            elif f.startswith(PRE + 'tests') or f.startswith(PRE + 'core/tests'):
                kinds['tests'] += 1
            elif f.startswith('docs/') or f.startswith('.tm') or f.startswith('.claude'):
                kinds['docs'] += 1
            else:
                kinds['other'] += 1
        rows.append((when, h[:9], subj[:90], dict(kinds), len(hit), lines))
    for r in rows:
        print(f'{r[0]} {r[1]} hubs={r[4]} {r[3]} | {r[2]}')
    print('\nhub file: landings touching it since', a.since, '/ lines now')
    for f in HUBS:
        try:
            n = len(open(PRE + f).read().splitlines())
        except OSError:
            n = 0
        print(f'  {len(hub_hits[f]):3}  {n:6}  {f}')

if __name__ == '__main__':
    main()
