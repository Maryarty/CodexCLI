# oracle-pending: the pending list follows the markers

`sync_tsv.py [--check] [shard::test ...]`: `map-full/pending-tests.tsv` (the pending oracle tests, their slice and reason)
re-read from the `kit::pending(Owner::.., reason)` markers of `tests/oracle/<shard>.rs`: a test with no marker leaves
the list, a renamed reason is taken over. No run: the marker is read from the source. Afterwards
`python3 -I tools/rtwk-port/map-full/build.py` rewrites the map and `map-full/unpend/*.txt`.
