#!/usr/bin/env python3
"""Where porter wall time goes: classify the time between tool calls in
codex-porter subagent transcripts.

Usage: python3 -I porter_time.py [--since 2026-11-22] [--top 40] [--per-agent] [--calls CAT]

Timeline sweep per transcript: every interval between two consecutive events
(assistant tool_use, user tool_result, other user message) is charged to
  * the outstanding tool call (tool execution), category by tool/command;
    with several outstanding, to the slowest category (priority list);
  * else, when the next event is an assistant tool_use: model time, charged
    as "model" (LLM thinking/writing before the call);
  * else (agent idle until a wake-up message): "idle".
Intervals over IDLE_CAP seconds are charged to "pause>15m" (agent parked,
orchestrator away), never to a category.
"""
import argparse, collections, glob, json, os, re, statistics, sys
from datetime import datetime

ROOT = os.path.expanduser('~/.claude/projects/-Users-admin-Documents-dev-projects-CodexCLI')
IDLE_CAP = 15 * 60

def ts(s):
    return datetime.fromisoformat(s.replace('Z', '+00:00')).timestamp()

BG = {}  # bgrun name -> category of its command (per transcript)

HEREDOC = re.compile(r"<<-?\s*['\"]?(\w+)['\"]?[^\n]*\n.*?\n\s*\1\s*(?:\n|$)", re.S)

def strip_heredocs(cmd):
    return HEREDOC.sub('<<HEREDOC\n', cmd)

def bash_cat(cmd):
    c = strip_heredocs(cmd)
    if re.search(r'bgrun\.sh\s+wait', c):
        names = re.findall(r'bgrun\.sh\s+wait\s+(\S+)', c)
        cats = [BG.get(n, 'wait:?') for n in names]
        return 'wait:' + (cats[0].replace('wait:', '') if cats else '?')
    if re.search(r'bgrun\.sh\s+(tail|status|log|cat|ls)', c):
        return 'wait-poll'
    rc = re.findall(r'rtwk/bg/\S*?([A-Za-z0-9_.]+)\.(?:rc|log)', c)
    if rc and re.search(r'until|while|sleep', c):
        hit = [v for k, v in BG.items() if any(r.endswith(k) for r in rc)]
        return 'wait:' + (hit[0].replace('wait:', '') if hit else '?')
    if 'land-check' in c:
        return 'land-check'
    if 'summary.py' in c:
        return 'oracle-shard'
    if re.search(r'check\.py|mock-model', c):
        return 'pty-scenario'
    if 'nextest' in c or re.search(r'cargo\S*\s+(-q\s+)?test\b', c) or 'acargo test' in c:
        if re.search(r'--test[ =]rules\b|-E\s*[\'"]?binary\(rules', c):
            return 'test:rules'
        if re.search(r'--workspace|--all\b', c) or not re.search(r'-p\s|--package|--test|-E\s|--\s+\S', c):
            return 'test:wide'
        return 'test:scoped'
    if re.search(r'\bcargo\b|acargo', c):
        if 'clippy' in c:
            return 'cargo:clippy'
        if 'fmt' in c and not re.search(r'\b(build|check)\b', c):
            return 'cargo:fmt'
        return 'cargo:build'
    if re.search(r'(^|[;&|]\s*)sleep\b|until\s|while\s', c):
        return 'wait-poll'
    if re.search(r'(^|\s)git\s', c):
        if re.search(r'git\s+(merge|rebase|cherry-pick|commit|add|checkout|switch|worktree|reset|stash)', c):
            return 'git:write'
        return 'git:read'
    if re.search(r'(^|\s)tm\s', c):
        return 'tm'
    if re.search(r"sed -i|python3?\s.*(replace|write)|cat\s*>|tee\s|perl -pi", cmd, re.S):
        return 'edit'
    if re.search(r'(^|[\s;&|(])(grep|rg|cat|sed|head|tail|ls|find|wc|sym|awk|diff|jq|bat|tree|stat)\b', c):
        return 'read'
    return 'other'

def tool_cat(name, inp):
    if name == 'Bash':
        cmd = inp.get('command', '')
        m = re.search(r'bgrun\.sh\s+(run|start)\s+(\S+)\s+(?:\d+\s+)?--\s+(.*)', cmd, re.S)
        if m:
            BG[m.group(2)] = bash_cat(m.group(3))
            BG['__last__'] = BG[m.group(2)]
            return BG[m.group(2)] if m.group(1) == 'run' else 'bg-start'
        cat = bash_cat(cmd)
        if inp.get('run_in_background'):
            BG['__last__'] = cat
            return 'bg-start'
        return cat
    if name in ('Read', 'Grep', 'Glob', 'LS'):
        return 'read'
    if name in ('Edit', 'Write', 'MultiEdit', 'NotebookEdit'):
        return 'edit'
    if name.startswith('mcp__tm__') or name == 'ToolSearch':
        return 'tm'
    if name in ('Monitor', 'TaskOutput', 'BashOutput'):
        return 'wait:' + BG.get('__last__', '?').replace('wait:', '')
    return 'other:' + name

PRIO = ['land-check', 'oracle-shard', 'test:wide', 'test:scoped', 'test:rules', 'pty-scenario',
        'cargo:clippy', 'cargo:build']

def prio(cat):
    base = cat.replace('wait:', '')
    return PRIO.index(base) if base in PRIO else len(PRIO)

def load(path):
    ev = []
    for line in open(path):
        try:
            d = json.loads(line)
        except Exception:
            continue
        t = d.get('type')
        if t not in ('assistant', 'user') or 'timestamp' not in d:
            continue
        c = d.get('message', {}).get('content')
        when = ts(d['timestamp'])
        if isinstance(c, list):
            uses = [x for x in c if x.get('type') == 'tool_use']
            res = [x for x in c if x.get('type') == 'tool_result']
            for x in uses:
                ev.append((when, 'use', x['id'], x['name'], x.get('input', {})))
            for x in res:
                body = x.get('content')
                if isinstance(body, list):
                    body = ' '.join(b.get('text', '') for b in body if isinstance(b, dict))
                ev.append((when, 'res', x.get('tool_use_id'), None, str(body or '')))
            if t == 'user' and not res:
                ev.append((when, 'msg', None, None, None))
            if t == 'assistant' and not uses:
                ev.append((when, 'say', None, None, None))
        elif t == 'user':
            ev.append((when, 'msg', None, None, None))
    ev.sort(key=lambda e: e[0])
    return ev

def analyse(path, calls):
    BG.clear()
    ev = load(path)
    out = collections.Counter()
    open_ = {}  # id -> (start, cat, cmd)
    for i, e in enumerate(ev):
        when, kind, tid, name, inp = e
        if i > 0:
            dt = when - ev[i - 1][0]
            if dt > IDLE_CAP:
                out['pause>15m'] += dt
            elif open_:
                cat = min((v[1] for v in open_.values()), key=prio)
                out[cat] += dt
            elif kind in ('use', 'say'):
                out['model'] += dt
            else:
                out['idle'] += dt
        if kind == 'use':
            cat = tool_cat(name, inp)
            cmd = inp.get('command') if name == 'Bash' else (inp.get('file_path') or inp.get('pattern') or inp.get('task') or '')
            open_[tid] = (when, cat, cmd)
        elif kind == 'res' and tid in open_:
            st, cat, cmd = open_.pop(tid)
            m = re.findall(r'Summary \[\s*([\d.]+)s\]\s+(\d+) tests? run', inp or '')
            run = (sum(float(a) for a, _ in m), sum(int(b) for _, b in m)) if m else None
            calls.append((when - st, cat, os.path.basename(path), strip_heredocs(str(cmd)).replace('\n', ' '), run))
    return out, (ev[-1][0] - ev[0][0]) if ev else 0, ev[0][0] if ev else 0

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--since', default='2026-11-22')
    ap.add_argument('--top', type=int, default=40)
    ap.add_argument('--per-agent', action='store_true')
    ap.add_argument('--calls', default=None, help='list calls of this category')
    a = ap.parse_args()
    since = datetime.fromisoformat(a.since + 'T00:00:00+00:00').timestamp()
    tot = collections.Counter()
    calls = []
    n = 0
    span = 0
    for f in sorted(glob.glob(ROOT + '/*/subagents/*.jsonl')):
        try:
            m = json.load(open(f.replace('.jsonl', '.meta.json')))
        except Exception:
            continue
        if m.get('agentType') != 'codex-porter':
            continue
        mine = []
        c, s, first = analyse(f, mine)
        if first < since:
            continue
        calls.extend(mine)
        n += 1
        span += s
        tot += c
        if a.per_agent:
            w = sum(v for k, v in c.items() if k != 'pause>15m') or 1
            top = ', '.join(f'{k} {v/w:.0%}' for k, v in c.most_common(4) if k != 'pause>15m')
            print(f'{w/60:6.1f}m  {m.get("description","")[:45]:45}  {top}')
    work = sum(v for k, v in tot.items() if k != 'pause>15m')
    print(f'\n{n} porter transcripts since {a.since}; active wall time {work/3600:.1f} h (pauses >15m excluded: {tot["pause>15m"]/3600:.1f} h)\n')
    print(f'{"category":18} {"hours":>7} {"share":>6}')
    for k, v in sorted(tot.items(), key=lambda kv: -kv[1]):
        if k == 'pause>15m':
            continue
        print(f'{k:18} {v/3600:7.2f} {v/work:6.1%}')
    bycat = collections.defaultdict(list)
    for d, cat, f, cmd, run in calls:
        bycat[cat].append(d)
    print(f'\n{"category":18} {"calls":>6} {"median s":>9} {"p90 s":>7} {"max s":>7} {"sum h":>6}')
    for cat, ds in sorted(bycat.items(), key=lambda kv: -sum(kv[1])):
        ds.sort()
        print(f'{cat:18} {len(ds):6} {statistics.median(ds):9.1f} {ds[int(len(ds)*0.9)]:7.1f} {ds[-1]:7.0f} {sum(ds)/3600:6.2f}')
    split(calls)
    forms(calls)
    sel = [c for c in calls if (a.calls is None or c[1] == a.calls)]
    print(f'\nTop {a.top} single calls' + (f' in {a.calls}' if a.calls else ''))
    for d, cat, f, cmd, run in sorted(sel, key=lambda c: -c[0])[:a.top]:
        print(f'{d:7.0f}s {cat:14} {f[:22]} {re.sub(r"cd /\S+ && ", "", cmd)[:150]}')

def form(cmd):
    cmd = strip_heredocs(cmd)
    if 'summary.py' in cmd:
        return 'summary.py --run' if '--run' in cmd else 'summary.py (read junit)'
    if '--workspace' in cmd:
        return 'nextest --workspace'
    t = sorted(set(re.findall(r'--test[ =](\w+)', cmd)))
    if t:
        return '--test ' + '+'.join(t)
    if re.search(r'-p\s+codex[-_]tui[-_]core', cmd):
        return '-p codex-tui-core'
    return 'no root named (all 7 roots)'

def forms(calls):
    print('\nTest calls by how the porter named what to build (direct calls only)')
    g = collections.defaultdict(list)
    for d, cat, f, cmd, run in calls:
        if cat in ('test:scoped', 'test:rules', 'test:wide', 'oracle-shard'):
            g[form(cmd)].append((d, run[0] if run else 0))
    print(f'{"form":30} {"calls":>6} {"median s":>9} {"sum h":>6} {"run h":>6}')
    for k, v in sorted(g.items(), key=lambda kv: -sum(x[0] for x in kv[1]))[:12]:
        ds = sorted(x[0] for x in v)
        print(f'{k:30} {len(v):6} {statistics.median(ds):9.1f} {sum(ds)/3600:6.2f} {sum(x[1] for x in v)/3600:6.2f}')

def split(calls):
    print('\nDirect test calls with a nextest Summary line: wall = compile/link + run')
    print(f'{"category":14} {"calls":>6} {"wall h":>7} {"run h":>6} {"compile share":>13} {"median tests":>12}')
    by = collections.defaultdict(list)
    for d, cat, f, cmd, run in calls:
        if run and not cat.startswith('wait'):
            by[cat].append((d, min(run[0], d), run[1]))
    for cat, xs in sorted(by.items(), key=lambda kv: -sum(x[0] for x in kv[1])):
        w = sum(x[0] for x in xs); r = sum(x[1] for x in xs)
        print(f'{cat:14} {len(xs):6} {w/3600:7.2f} {r/3600:6.2f} {1-r/w:13.0%} {statistics.median(x[2] for x in xs):12}')

if __name__ == '__main__':
    main()
