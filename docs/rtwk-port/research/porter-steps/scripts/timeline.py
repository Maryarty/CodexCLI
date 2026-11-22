import sys
sys.path.insert(0,'/tmp/pstep'); from parse import run, bucket
lines,calls,order,_=run(sys.argv[1]); skip=tuple(sys.argv[2].split(',')) if len(sys.argv)>2 else ('read','edit')
t0=calls[order[0]]['t0']
for i,k in enumerate(order):
    c=calls[k]; b=bucket(c['name'],c['inp'])
    if b.startswith(skip): continue
    s=c['inp'].get('command') or c['inp'].get('file_path') or str(c['inp'])[:150]
    s=s.replace('/Users/admin/Documents/dev_projects/CodexCLI/.claude/worktrees/','WT/').replace('/Users/admin/','~/')
    import re; s=re.sub(r'^cd \S+ && ','',s)
    r=c['res'].replace('\n',' | ')[:110]
    d=((c['t1'] or c['t0'])-c['t0']).total_seconds()
    print(f"{i:3d} {int((c['t0']-t0).total_seconds()/60):3d}m {d:4.0f}s {b[:12]:12s} {s[:150].replace(chr(10),' | ')}  => {r}")
