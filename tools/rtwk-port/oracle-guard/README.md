# oracle-guard

Rewrites of `codex-rs/tui-rtwk/tests/oracle/*.rs` so that `guards::shards_are_black_box_and_never_skipped`
holds: each `assert!`/`assert_eq!`/core type of a shard became a kit helper (`tests/kit/expect.rs`,
`sparkle.rs`, `settings.rs`, the plugin helpers in `fixtures/chat_popups_3/seed.rs`). One script per
batch of shards, run in order from this directory with `python3 -I batchN.py`; `lib.py` is the
`Shard` editor (`sub`, `rx`). The scripts are the record of the rewrite (applied once, with the
hand edits that followed noted in the commits), not a pipeline to re-run on current files.
