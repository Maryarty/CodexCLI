import csv, sys, os, collections
d = sys.argv[1]; P = lambda f: os.path.join(d, f)
def rows(f, n): return [l.rstrip('\n').split('|', n-1) for l in open(P(f)) if l.strip()]
sub = {r['id']: r for r in csv.DictReader(open(P('subclasses.tsv')), delimiter='\t')}
cls = {r[1]: (r[0], r[2], r[3]) for r in rows('../cls.txt', 4)}  # hash -> (class,date,subject)
dirs = {r[0]: (r[1], r[2]) for r in rows('dirs_all.txt', 3)}
full = {}
for f in ('full508.txt', 'fullsupp.txt'):
    for l in open(P(f)): full[l.strip()[:9]] = l.strip()
up = {}
for f in ('upstream508.txt', 'upstreamsupp.txt'):
    for l in open(P(f)):
        s, v = l.strip().split('|'); up[s] = v
supp = {r[0]: (r[1], r[2]) for r in rows('supp_meta.txt', 3)}
out = csv.writer(open(P('commits.csv'), 'w', newline=''))
out.writerow(['hash','full_sha','upstream','date','subject','set','files_top','areas','subclass','subclass_name','root_cause','verdict_M','verdict_R','verdict_mix_ADR003','rule','verdict_after_rules'])
counts = collections.Counter()
for f, setname in (('cls_manual.txt', 'fix-set'), ('supp_manual.txt', 'outside-fix-set')):
    for h, sc, rc in rows(f, 3):
        date, subj = (cls[h][1], cls[h][2]) if h in cls else supp[h]
        fs = full[h]; s = sub[sc]
        out.writerow([h, fs, up.get(fs, '?'), date, subj, setname, dirs.get(h, ('',''))[0], dirs.get(h, ('',''))[1], sc, s['name'], rc, s['M'], s['R'], s['mix'], s['rule'], s['after']])
        counts[(setname, sc)] += 1
iss = {r[0]: r for r in rows('iss_meta.txt', 4)}
o2 = csv.writer(open(P('issues.csv'), 'w', newline=''))
o2.writerow(['number','title','state','created','subclass','subclass_name','mechanism','verdict_M','verdict_R','verdict_mix_ADR003','verdict_after_rules'])
for n, sc, m in rows('iss_manual.txt', 3):
    _, st, cr, t = iss[n]; s = sub[sc]
    o2.writerow([n, t, st.lower(), cr, sc, s['name'], m, s['M'], s['R'], s['mix'], s['after']])
    counts[('issues', sc)] += 1
classified = {}
for f in ('cls_manual.txt',):
    for h, sc, rc in rows(f, 3): classified[h] = sc
o3 = csv.writer(open(P('fix-set.csv'), 'w', newline=''))
o3.writerow(['hash','full_sha','upstream','date','subject','primary_class_by_subject','files_top','areas','state_subclass'])
for l in open(P('../cls.txt')):
    c, h, dt, s = l.rstrip('\n').split('|', 3); fs = full[h]
    o3.writerow([h, fs, up.get(fs, '?'), dt, s, c, dirs.get(h, ('',''))[0], dirs.get(h, ('',''))[1], classified.get(h, '')])
# summary table
ids = list(sub)
w = csv.writer(open(P('subclass-counts.csv'), 'w', newline=''))
w.writerow(['subclass','name','fix_set','outside_fix_set','issues','M','R','mix_ADR003','rule','after_rules','F2_store','F3_signals','F4_entities','F5_actors'])
for i in ids:
    s = sub[i]
    w.writerow([i, s['name'], counts[('fix-set', i)], counts[('outside-fix-set', i)], counts[('issues', i)], s['M'], s['R'], s['mix'], s['rule'], s['after'], s['F2_store'], s['F3_signals'], s['F4_entities'], s['F5_actors']])
for i in ids: print(i, counts[('fix-set', i)], counts[('outside-fix-set', i)], counts[('issues', i)], sub[i]['mix'], '->', sub[i]['after'], '| M', sub[i]['M'], '| R', sub[i]['R'])
