# w2-cells: helpers for the cells package

- `fixture.py <shard>::<test> ...`: copies the oracle's own recorded fixture of a test
  (`~/.cache/codexcli/oracle-gen/fixtures`) into `tests/oracle/fixtures/<shard>/`, with the recorder's empty thread id
  made `thread-1`.
- `unpend.py <shard>::<test> ...`: removes the `kit::pending` marker and the `pending-tests.tsv` row of the named tests.
- `fixture.py --turn <id> ...`: the same, with a `turn/started` of that turn after the session seed (the oracle's
  test marks the task running by hand; the recorder has no notification for it).
- `ps_fixtures.py`: writes the scripts of the `/ps` snapshot tests of `cells_3` (the sessions the oracle's test
  builds by hand, as `thread/backgroundTerminals` facts and a `/ps` command).
- `guardian_fixtures.py`: writes the scripts of the guardian tests of `chat_approvals_2` that the recorder left
  partial (`item/autoApprovalReview/*`, `guardianWarning`, MCP startup) as the denied test's recorded script shapes them.
