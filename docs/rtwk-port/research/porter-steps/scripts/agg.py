import sys, json, re, collections
sys.path.insert(0,'/tmp/pstep'); from parse import run, bucket, ts
def norm(c):
    c=re.sub(r'^cd \S+ (&&|;) *','',c.strip()); c=re.sub(r'/Users/admin/Documents/dev_projects/CodexCLI/(\.claude/worktrees/[^/]+/)?','',c)
    return c
rows=[]
for path in sys.argv[1:]:
    lines,calls,order,compacts=run(path)
    first=[l for l in lines if l.get('type')=='user'][0]['message']['content']; first=first if isinstance(first,str) else json.dumps(first)
    alias=(re.findall(r'chunk-[a-z0-9-]+|core-wiring',first) or ['?'])[0]
    relaunch=bool(re.search(r'Continue|checkpoint|re-grant',first[:600]))
    T=[ts(l['timestamp']) for l in lines if l.get('timestamp')]; wall=(T[-1]-T[0]).total_seconds()/60
    turns=len({l['message'].get('id') for l in lines if l.get('type')=='assistant' and any(b.get('type')=='tool_use' for b in (l['message'].get('content') or []) if isinstance(b,dict))})
    B=collections.Counter(); Bt=collections.Counter(); first_edit=None; steps=0
    bg=to=memc=fullc=fulln=summ=dup=0; seen=collections.Counter(); prog=0; retest=collections.Counter()
    for i,k in enumerate(order):
        c=calls[k]; b=bucket(c['name'],c['inp']); top=b.split(':')[0]
        d=((c['t1'] or c['t0'])-c['t0']).total_seconds()
        B[top]+=1; Bt[top]+=d
        if top=='edit' and first_edit is None: first_edit=i
        r=c['res'] or ''
        if 'moved to the background' in r: bg+=1
        if 'did not complete within' in r: to+=1
        if 'memcap: waiting' in r: memc+=1
        cmd=c['inp'].get('command','') if c['name']=='Bash' else ''
        if re.search(r'clippy[^|;&]*--workspace',cmd): fullc+=1
        if re.search(r'nextest run',cmd) and not re.search(r"-E|--test |-p |test\(|--no-run",cmd) and 'oracle' not in cmd: fulln+=1
        if 'summary.py --run' in cmd or 'summary.py' in cmd and '--run' in cmd: summ+=1
        if c['name']=='mcp__tm__task_checkpoint' or c['name']=='mcp__tm__task_done': steps+=1
        key=norm(cmd) if top=='read' and cmd else (c['inp'].get('file_path','')+str(c['inp'].get('offset',''))+str(c['inp'].get('limit','')) if c['name']=='Read' else None)
        if key:
            if seen[key]: dup+=1
            seen[key]+=1
    rows.append((alias, 'relaunch' if relaunch else 'fresh', path.split('/')[-3][:8]+'/'+path.split('agent-')[-1][:8], len(order), turns, round(wall), steps, compacts, first_edit, B, Bt, bg, to, memc, fullc, fulln, summ, dup))
tops=['read','edit','build','wait','git','tm','script']
print('alias|run|id|calls|turns|wall_min|ckpts|compacts|first_edit|'+'|'.join(f'{t}(n/min)' for t in tops)+'|bg_moved|timeout120|memcap_wait|clippy_ws|nextest_full|summary_run|dup_reads')
tot=collections.Counter()
for r in rows:
    a,rl,i,n,tu,w,st,cp,fe,B,Bt,bg,to,mc,fc,fn,sm,du=r
    print(f"{a}|{rl}|{i}|{n}|{tu}|{w}|{st}|{cp}|{fe}|"+'|'.join(f"{B[t]}/{Bt[t]/60:.0f}" for t in tops)+f"|{bg}|{to}|{mc}|{fc}|{fn}|{sm}|{du}")
    tot['calls']+=n; tot['wall']+=w; tot['steps']+=st
    for t in tops: tot[t]+=B[t]; tot[t+'_t']+=Bt[t]
    tot['bg']+=bg; tot['to']+=to; tot['mc']+=mc; tot['dup']+=du; tot['fc']+=fc; tot['fn']+=fn; tot['sm']+=sm; tot['turns']+=tu
print('TOTAL', dict(tot))
