import sys, json, re, collections, statistics as st
sys.path.insert(0,'/tmp/pstep'); from parse import run, bucket, ts
FR=re.compile(r'^\s*(?:FAIL|TIMEOUT|SIGABRT)\s+\[[^\]]*\]\s+(?:\(\s*\d+/\d+\)\s+)?(\S+)\s+(\S+)', re.M)
ERR={'py39':r"tomllib|unsupported operand type\(s\) for \||Python 3\.9",'timeout_bin':r'timeout: command not found|command not found: timeout',
 'sed_bsd':r'sed: 1:|sed: -e|invalid command code','bg_gone':r'gone \(no exit code\)|already running','lock':r'Blocking waiting for file lock',
 'memcap':r'memcap: waiting','moved_bg':r'moved to the background','to120':r'did not complete within','notfound':r'No such file or directory'}
G=collections.Counter(); RN=collections.Counter(); chk=[]; nx=[]; gate=[]; pre=collections.Counter(); prelist=[]; flake=collections.Counter(); timeouts=collections.Counter(); par=[]
for path in open('list.txt').read().split():
    lines,calls,order,_=run(path)
    first=[l for l in lines if l.get('type')=='user'][0]['message']['content']; first=first if isinstance(first,str) else json.dumps(first)
    relaunch=bool(re.search(r'Continue|checkpoint|re-grant',first[:600]))
    failed=set(); last_ck=0; seen_edit=False; pc=collections.Counter()
    for i,k in enumerate(order):
        c=calls[k]; b=bucket(c['name'],c['inp']); r=c['res'] or ''; cmd=c['inp'].get('command','') if c['name']=='Bash' else ''
        d=((c['t1'] or c['t0'])-c['t0']).total_seconds()
        if c['name']=='Bash': timeouts['with_timeout' if 'timeout' in c['inp'] else 'default']+=1; 
        if c['name']=='Bash' and c['inp'].get('run_in_background'): timeouts['run_in_background']+=1
        for n,p in ERR.items():
            if re.search(p,r): G[n]+=1
        if re.search(r'acargo check\b',cmd) and 'bgrun' not in cmd and d>0: chk.append(d)
        if re.search(r'acargo nextest run',cmd) and re.search(r"-E|--test ",cmd) and 'bgrun' not in cmd and 'oracle' not in cmd: nx.append(d)
        # flake triage: rerun or log-read of a name that failed before, or known-red / stash / tip comparisons
        names=[m[2] for m in FR.finditer(r)]
        hit=[n for n in failed if len(n.split('::')[-1])>12 and n.split('::')[-1] in cmd] if cmd else []
        if hit: flake['rerun_failed_name']+=1; flake['t']+=d; flake['t_'+b.split(':')[0]]+=d; [RN.update([n]) for n in hit[:1]]
        elif re.search(r'known-red|git stash|a_remembered_mode|uptime|loadavg|sysctl -n vm.loadavg',cmd): flake['known_red/stash/load']+=1; flake['t']+=d
        failed.update(names)
        if not seen_edit:
            if b=='edit': seen_edit=True; prelist.append((relaunch,i,dict(pc)))
            else: pc[b.split(':')[0] if not b.startswith('read') else b]+=1
        if c['name'] in ('mcp__tm__task_checkpoint','mcp__tm__task_done'):
            # gate window: first fmt/clippy-ws/full nextest/shards/summary since last checkpoint
            st_i=None
            for j in range(last_ck,i):
                cc=calls[order[j]]; cm=cc['inp'].get('command','') if cc['name']=='Bash' else ''
                if re.search(r'clippy[^|;&]*--workspace',cm) or (re.search(r'nextest run',cm) and not re.search(r"-E|--test |-p |--no-run|oracle",cm)):
                    st_i=j; break
            if st_i is not None:
                w=(c['t0']-calls[order[st_i]]['t0']).total_seconds()/60; gate.append((i-st_i,w,path.split('agent-')[-1][:6]))
            last_ck=i+1
    # parallelism
    msgs=collections.Counter(l['message'].get('id') for l in lines if l.get('type')=='assistant' for b in (l['message'].get('content') or []) if isinstance(b,dict) and b.get('type')=='tool_use')
    par.append(sum(msgs.values())/max(1,len(msgs)))
print('errors', dict(G)); print('bash timeouts', dict(timeouts))
print('acargo check fg: n=%d median=%.0fs p75=%.0fs'%(len(chk),st.median(chk),sorted(chk)[3*len(chk)//4]))
print('scoped nextest fg: n=%d median=%.0fs p75=%.0fs'%(len(nx),st.median(nx),sorted(nx)[3*len(nx)//4]))
print('flake', dict(flake))
print('gate windows n=%d calls median=%d sum=%d  min median=%.0f sum=%.0f'%(len(gate),st.median([g[0] for g in gate]),sum(g[0] for g in gate),st.median([g[1] for g in gate]),sum(g[1] for g in gate)))
print(sorted(gate))
print('calls/turn', [round(p,2) for p in par])
R=[p for p in prelist if p[0]]; print('relaunch pre-edit calls', sorted(p[1] for p in R))
agg=collections.Counter()
for p in R:
    for k,v in p[2].items(): agg[k]+=v
print('relaunch pre-edit composition (sum over %d)'%len(R), agg.most_common())

print('top rerun names', RN.most_common(25))
