//! The oracle suite: the oracle's screen-level tests (codex-rs/tui at the
//! tag in `docs/rtwk-port/ORACLE_REF`) ported as black-box screen tests, one
//! module per shard (`tests/oracle/<shard>.rs`, listed in
//! `docs/rtwk-port/test-shards.md`). A shard file holds one test per oracle
//! test, named after it, written with `kit` only. The recipe:
//! `docs/rtwk-port/refs/screen-tests.md`.
//!
//! The modules are the files of `tests/oracle/` (`tui_test_mods::oracle!`, written by
//! `build.rs` of `test-mods`), so a shard written by `tests/oracle/tools/shards.py` or by hand is
//! a module without a line here.

mod common;
mod kit;

tui_test_mods::oracle!();
