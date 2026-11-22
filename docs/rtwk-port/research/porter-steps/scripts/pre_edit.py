import sys, collections, json, re
sys.path.insert(0, sys.argv[1])
from parse import run, bucket
for p in sys.argv[2:]:
    lines,calls,order,_=run(p)
    t0=calls[order[0]]['t0']
    pre=[]; fe=None
    for i,k in enumerate(order):
        c=calls[k]; b=bucket(c['name'],c['inp'])
        if b=='edit': fe=i; break
        pre.append((i,b,c))
    # finer: classify reads
    agg=collections.Counter(); 
    for i,b,c in pre:
        s=json.dumps(c['inp'])
        if 'fleet-brief' in s: b='fleet-brief'
        elif 'AGENTS.md' in s or 'architecture.md' in s or 'process.md' in s: b='docs:arch/process'
        agg[b]+=1
    tot=len(order)
    tmin=(calls[order[fe]]['t0']-t0).total_seconds()/60 if fe else 0
    wall=((calls[order[-1]]['t1'] or calls[order[-1]]['t0'])-t0).total_seconds()/60
    print(f"## {p.split('/')[-1][:23]} first_edit={fe} ({tmin:.0f}m) total={tot} wall={wall:.0f}m")
    print('  ', dict(agg.most_common()))
    if '-v' in sys.argv[1:2] or True:
        for i,b,c in pre:
            inp=c['inp']; s=inp.get('command') or inp.get('file_path') or json.dumps(inp)[:120]
            extra=f" off={inp.get('offset')} lim={inp.get('limit')}" if c['name']=='Read' else ''
            rl=len(c['res'] or '')
            print(f"   {i:3d} {b:16s} {rl:6d} {str(s)[:150].replace(chr(10),' ')}{extra}")
