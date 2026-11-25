# unpend-stale: the kit::pending markers of tests that already pass

`python3 -I tools/rtwk-port/unpend-stale/unpend.py [--probe] [--junit F] [--dry-run] [--commit]`: reads a
`KIT_PROBE_PENDING=1` run of the oracle suite (`--probe` makes the run; one run, `PORTER_RUN_OK=1`), takes
`summary.py --list stale-real` (no stub, no absence-only or assert-free body, no `{weak-body}` marker), removes the
`kit::pending(..)` statement of each in `tests/oracle/<shard>.rs`, drops the row from `map-full/pending-tests.tsv`, and with
`--commit` commits per W2 package (`map-full/tests_rules.py`), then reruns `map-full/build.py`. Rerun after every landing.
