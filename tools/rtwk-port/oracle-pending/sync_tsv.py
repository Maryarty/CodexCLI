#!/usr/bin/env python3
"""Syncs map-full/pending-tests.tsv with the kit::pending markers of tests/oracle/<shard>.rs.

    cd <worktree>
    python3 -I tools/rtwk-port/oracle-pending/sync_tsv.py --check   # what would change, nothing written
    python3 -I tools/rtwk-port/oracle-pending/sync_tsv.py           # rewrite the tsv, then map-full/build.py
    python3 -I tools/rtwk-port/oracle-pending/sync_tsv.py shard::test ...   # only these rows

A row whose test has no kit::pending marker any more leaves the tsv; a row whose marker reason (a string literal
or a `const X: &str` of the shard) differs from the row's takes the marker's text (its first line, the way
summary.py shows it; a row that is a prefix of it, or the other way, stays). The owner column is the marker's `Owner::..` slice name when the row has none.
Rerunnable, no run needed: the marker is read from the source, so a renamed reason or a new feature name is one
rerun, not a probe.
"""
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, '..', '..', '..'))
CRATE = os.path.join(ROOT, 'codex-rs', 'tui-rtwk')
sys.path.insert(0, os.path.join(CRATE, 'tests', 'oracle', 'tools'))
import summary  # noqa: E402

TSV = os.path.join(ROOT, 'tools', 'rtwk-port', 'map-full', 'pending-tests.tsv')
ESCAPES = {'n': '\n', 't': '\t', '"': '"', "'": "'", '\\': '\\', '0': '\0'}


def unescape(lit):
    """The value of the body of a Rust string literal (no quotes)."""
    out = []
    i = 0
    while i < len(lit):
        c = lit[i]
        if c != '\\':
            out.append(c)
            i += 1
            continue
        i += 1
        n = lit[i] if i < len(lit) else ''
        if n == '\n':  # a line continuation eats the next line's indentation
            i += 1
            while i < len(lit) and lit[i] in ' \t\n':
                i += 1
        elif n == 'u':
            j = lit.index('}', i)
            out.append(chr(int(lit[i + 2:j], 16)))
            i = j + 1
        else:
            out.append(ESCAPES.get(n, n))
            i += 1
    return ''.join(out)


def literal_at(src, i):
    """(value, end) of the string literal starting at src[i] (a `"` or `r#"`), None otherwise."""
    m = re.compile(r'r(#*)"').match(src, i)
    if m:
        close = '"' + m.group(1)
        end = src.index(close, m.end())
        return src[m.end():end], end + len(close)
    if src[i:i + 1] != '"':
        return None
    j = i + 1
    while src[j] != '"':
        j += 2 if src[j] == '\\' else 1
    return unescape(src[i + 1:j]), j + 1


def marker_of(src, test):
    """(owner, reason) of the kit::pending marker of `fn test`; None when it has none or is not found."""
    m = re.search(r'\bfn %s\s*\(' % re.escape(test), src)
    if not m:
        return None
    start = src.find('{', m.end())
    end = summary._match(src, start, '{', '}')
    body = src[start + 1:end]
    k = re.search(r'\bkit::pending\s*\(', body)
    if not k:
        return None
    close = summary._match(body, k.end() - 1, '(', ')')
    args = body[k.end():close]
    om = re.match(r'\s*Owner::(\w+)\s*,\s*', args)
    if not om:
        return None
    rest = args[om.end():]
    lit = literal_at(rest, 0)
    if lit is None:
        name = re.match(r'\w+', rest)
        cm = name and re.search(r'^const %s: &str =\s*' % re.escape(name.group(0)), src, re.M)
        lit = cm and literal_at(src, cm.end())
    if lit is None:
        return None
    return om.group(1), lit[0]


def main(argv):
    check = '--check' in argv
    only = [a for a in argv if not a.startswith('--')]  # shard::test names: the other rows stay as they are
    rows = []
    shards = {}
    changed = dropped = 0
    for line in open(TSV, encoding='utf-8'):
        name, slice_, reason = line.rstrip('\n').split('\t')
        shard, test = name.split('::', 1)
        if only and name not in only:
            rows.append((name, slice_, reason))
            continue
        if shard not in shards:
            path = os.path.join(CRATE, 'tests', 'oracle', shard + '.rs')
            shards[shard] = open(path, encoding='utf-8').read() if os.path.exists(path) else ''
        found = marker_of(shards[shard], test)
        if found is None:
            if re.search(r'\bfn %s\s*\(' % re.escape(test), shards[shard]):
                dropped += 1
                print('drop    ', name)
                continue
            rows.append((name, slice_, reason))  # a marker this script cannot read: kept as it is
            continue
        text = summary.first_line(found[1])
        if text.startswith(reason) or reason.startswith(text):  # a row cut short by an earlier re-cut stays
            text = reason
        if text != reason:
            changed += 1
            print('reason  ', name)
        rows.append((name, slice_, text))
    print('%d dropped, %d reasons changed' % (dropped, changed))
    if not check:
        with open(TSV, 'w', encoding='utf-8') as f:
            for r in rows:
                f.write('\t'.join(r) + '\n')


if __name__ == '__main__':
    main(sys.argv[1:])
