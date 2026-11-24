#!/usr/bin/env python3
"""Un-pends the named oracle tests (the ones a W2 cells change makes pass).

    python3 -I tools/rtwk-port/w2-cells/unpend.py cells_3::ps_output_empty_snapshot [...]

For each `<shard>::<test>`: the `kit::pending(..);` statement leaves the test fn of
tests/oracle/<shard>.rs (tools/rtwk-port/unpend-stale/unpend.py `remove_marker`, `tidy_imports`,
`tidy_consts`) and its row leaves tools/rtwk-port/map-full/pending-tests.tsv. The rerunnable form of the
sweep of `unpend-stale` for a list instead of a probe run.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', 'unpend-stale'))
import unpend  # noqa: E402


def main(names):
    by_shard = {}
    for name in names:
        shard, test = name.split('::')
        by_shard.setdefault(shard, []).append(test)
    for shard, tests in by_shard.items():
        path = os.path.join(unpend.CRATE, 'tests', 'oracle', shard + '.rs')
        src = open(path, encoding='utf-8').read()
        for test in tests:
            new = unpend.remove_marker(src, test)
            if new is None:
                print('no marker in', shard + '::' + test)
                continue
            src = new
            print('un-pended', shard + '::' + test)
        src = unpend.tidy_consts(unpend.tidy_imports(src))
        open(path, 'w', encoding='utf-8').write(src)
    gone = set(names)
    rows = open(unpend.TSV, encoding='utf-8').read().splitlines(keepends=True)
    keep = [row for row in rows if row.split('\t')[0] not in gone]
    open(unpend.TSV, 'w', encoding='utf-8').write(''.join(keep))


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    main(sys.argv[1:])
