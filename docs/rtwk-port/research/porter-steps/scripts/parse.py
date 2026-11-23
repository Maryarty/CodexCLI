import json, sys, re, collections, datetime as dt
def ts(s): return dt.datetime.fromisoformat(s.replace('Z','+00:00'))
FLAKE = re.compile(r'flak|known.red|known-red|not mine|outside my area|under load|load avg|uptime|pre-existing|preexisting|unrelated|on the tip without|passes alone|a_remembered_mode|starting…|starting\.\.\.', re.I)
def readkind(c):
    if re.search(r'\.cache/rtwk/bg|\.output|\.log\b|/tmp/', c): return 'read:logs'
    if re.search(r'codex-rs/tui/|cd [^;&]*codex-rs/tui( |;|&|/src)|\.\./tui/|tui/src', c): return 'read:oracle'
    if re.search(r'dev_projects/rtwk|skills/rtwk|refs/', c): return 'read:rtwk'
    if re.search(r'docs/|briefs|parity/|\.md\b', c): return 'read:docs'
    return 'read:own-code'
def bucket(name, inp):
    if name.startswith('mcp__tm__') or name=='ToolSearch': return 'tm'
    if name in ('Edit','Write','NotebookEdit'): return 'edit'
    if name in ('Read','Grep','Glob'): return readkind(str(inp))
    if name!='Bash': return 'other:'+name
    c = inp.get('command','')
    cs = re.sub(r'^cd [^;&]+(&&|;)\s*','',c.strip())
    if re.search(r'bgrun\.sh\s+(wait|log)|\bsleep\b|until |while .*sleep|tail .*(\.log|/bg/)', c): return 'wait'
    if re.search(r'land-check', c): return 'build:landcheck'
    if re.search(r'check\.py', c): return 'build:pty'
    if re.search(r'summary\.py|shards\.py|--profile oracle', c): return 'build:oracle-tools'
    if re.search(r'acargo|cargo |nextest', c):
        if re.search(r'\bfmt\b', c) and not re.search(r'nextest|clippy|check ', c): return 'build:fmt'
        if re.search(r'clippy', c) and re.search(r'--workspace', c): return 'build:full-gate'
        if re.search(r'nextest', c) and not re.search(r"-E|--test |-p |test\(", c): return 'build:full-gate'
        if re.search(r'clippy', c): return 'build:scoped'
        if re.search(r'nextest|test ', c): return 'build:scoped'
        if re.search(r'\bcheck\b', c): return 'build:check'
        if re.search(r'\bbuild\b', c): return 'build:bin'
        return 'build:other'
    if re.search(r'^\s*git |&& git |; git ', c) or cs.startswith('git '):
        if re.search(r'rebase|fetch|reset --hard|merge-base', c): return 'git:rebase'
        if re.search(r'commit|add ', c): return 'git:commit'
        return 'git:other'
    if re.search(r'sed -i|perl -pi|> *[\w/.]+\.rs\b|cat > ', c): return 'edit'
    if re.search(r'python3?', c) and re.search(r'\.write\(|write_text|/tmp/[\w_]+edit|_edit\d*\.py|cc_s\d|\.py [\w/.-]*\.rs', c): return 'edit'
    if re.search(r'python3?', c): return 'script'
    if re.search(r'\b(rg|grep|sed -n|cat|head|tail|ls|find|wc|awk)\b', c): return readkind(c)
    return 'bash:other'
def run(path):
    lines=[json.loads(l) for l in open(path) if l.strip()]
    calls={}; order=[]; last_t=None; texts=[]
    prev_end=None; model_time=0.0
    compacts=0
    for d in lines:
        t=d.get('timestamp')
        if d.get('type')=='system' and 'compact' in json.dumps(d)[:400].lower(): compacts+=1
        m=d.get('message')
        if not m or not t: continue
        T=ts(t)
        if d['type']=='assistant':
            for b in m.get('content',[]) if isinstance(m.get('content'),list) else []:
                if b.get('type') in ('text','thinking'): texts.append(b.get('text') or b.get('thinking') or '')
                if b.get('type')=='tool_use':
                    ctx=' '.join(texts[-2:]); 
                    calls[b['id']]=dict(name=b['name'],inp=b.get('input',{}),t0=T,t1=None,ctx=ctx,res='')
                    order.append(b['id'])
            if prev_end and order: pass
        elif d['type']=='user' and isinstance(m.get('content'),list):
            for b in m['content']:
                if b.get('type')=='tool_result' and b.get('tool_use_id') in calls:
                    c=calls[b['tool_use_id']]; c['t1']=T
                    r=b.get('content'); c['res']=r if isinstance(r,str) else json.dumps(r)[:3000]
            texts=[]
    return lines, calls, order, compacts
if __name__=='__main__':
    path=sys.argv[1]
    lines,calls,order,compacts=run(path)
    t_start=ts(lines[0]['timestamp']); t_end=ts([l for l in lines if l.get('timestamp')][-1]['timestamp'])
    agg=collections.defaultdict(lambda:[0,0.0]); flake=[0,0.0]
    first_edit=None; ckpts=[]; model=0.0; prev=t_start
    for i,k in enumerate(order):
        c=calls[k]; b=bucket(c['name'],c['inp'])
        dur=((c['t1'] or c['t0'])-c['t0']).total_seconds()
        gap=(c['t0']-prev).total_seconds()
        if gap>0: model+=gap
        prev=max(prev, c['t1'] or c['t0'])
        agg[b][0]+=1; agg[b][1]+=dur
        if b!='edit' and FLAKE.search(c['ctx']): flake[0]+=1; flake[1]+=dur
        if b=='edit' and first_edit is None: first_edit=(i, (c['t0']-t_start).total_seconds())
        if c['name']=='mcp__tm__task_checkpoint': ckpts.append((i,(c['t0']-t_start).total_seconds()))
    N=len(order); wall=(t_end-t_start).total_seconds()
    print(f"# {path.split('/')[-1]} calls={N} wall={wall/60:.0f}min model_gap={model/60:.0f}min compacts={compacts}")
    print(f"first_edit at call {first_edit[0] if first_edit else None} ({(first_edit[1]/60 if first_edit else 0):.0f} min); checkpoints at calls {[(a,round(b/60)) for a,b in ckpts]}")
    for b,(n,s) in sorted(agg.items(), key=lambda x:-x[1][1]):
        print(f"{b:22s} {n:4d} {100*n/N:5.1f}%  {s/60:6.1f} min")
    print(f"{'flake-ctx (overlay)':22s} {flake[0]:4d} {100*flake[0]/N:5.1f}%  {flake[1]/60:6.1f} min")
