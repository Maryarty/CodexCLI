import sys, re, collections
sys.path.insert(0,'/tmp/pstep'); from parse import run, bucket, ts
for path in sys.argv[1:]:
    l,c,o,cp=run(path); T=[ts(x['timestamp']) for x in l if x.get('timestamp')]; wall=(T[-1]-T[0]).total_seconds()
    B=collections.Counter(); Bt=collections.Counter(); N=len(o); tool=0
    for k in o:
        x=c[k]; b=bucket(x['name'],x['inp'])
        if b.startswith('build:landcheck'): b='build:full-gate'
        if b in ('build:fmt','build:other','build:bin'): b='build:other'
        if b.startswith('git'): b='git'
        if b.startswith('other') or b.startswith('bash'): b='other'
        d=((x['t1'] or x['t0'])-x['t0']).total_seconds(); B[b]+=1; Bt[b]+=d; tool+=d
    print(f"\n{path.split('agent-')[-1][:9]} calls={N} wall={wall/60:.0f}m compactions={cp}")
    for b,n in sorted(B.items(), key=lambda x:-Bt[x[0]]): print(f"| {b} | {n} | {100*n/N:.0f}% | {Bt[b]/60:.0f} |")
    print(f"| model (between calls) | - | - | {(wall-tool)/60:.0f} |")
