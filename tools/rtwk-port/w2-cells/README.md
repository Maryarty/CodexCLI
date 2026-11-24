# w2-cells: helpers for the cells package

- `fixture.py <shard>::<test> ...`: copies the oracle's own recorded fixture of a test
  (`~/.cache/codexcli/oracle-gen/fixtures`) into `tests/oracle/fixtures/<shard>/`, with the recorder's empty thread id
  made `thread-1`.
- `unpend.py <shard>::<test> ...`: removes the `kit::pending` marker and the `pending-tests.tsv` row of the named tests.
