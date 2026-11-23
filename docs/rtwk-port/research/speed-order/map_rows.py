#!/usr/bin/env python3
"""Rows of docs/rtwk-port/reports/chat-feature-map.md by section and by task alias.

Usage: python3 -I map_rows.py <chat-feature-map.md>
"""
import collections, re, sys

def main(path):
    st = collections.Counter()
    per = collections.defaultdict(collections.Counter)
    sec = None
    for line in open(path):
        if line.startswith('## '):
            sec = line[3:].strip()
        if not line.startswith('|') or line.startswith('|---') or 'Функция' in line:
            continue
        cells = [c.strip() for c in line.strip().strip('|').split('|')]
        if len(cells) < 5:
            continue
        s = {'✅': 'ok', '🟡': 'part', '❌': 'no'}.get(cells[1][:1], 'q')
        tasks = [t.strip() for t in re.split(r'[,;]', cells[-1]) if t.strip() and t.strip() not in ('—', '-')]
        for t in tasks or ['—']:
            per[t][s] += 1
        st[(sec, s)] += 1
    secs = collections.defaultdict(dict)
    for (sec, s), v in st.items():
        secs[sec][s] = v
    print('rows by section (ok / part / no / q):')
    tot = collections.Counter()
    for sec, d in secs.items():
        tot.update(d)
        print(f'  {sec[:55]:55} {d.get("ok",0):4} {d.get("part",0):4} {d.get("no",0):4} {d.get("q",0):3}')
    print(f'  {"total":55} {tot["ok"]:4} {tot["part"]:4} {tot["no"]:4} {tot["q"]:3}')
    print('\nopen rows per task alias (no + part), a row with two aliases counts for both:')
    for t, c in sorted(per.items(), key=lambda kv: -(kv[1]['part'] + kv[1]['no'])):
        if c['part'] + c['no']:
            print(f'  {t[:48]:48} no={c["no"]:3} part={c["part"]:3} ok={c["ok"]:3}')

if __name__ == '__main__':
    main(sys.argv[1])
