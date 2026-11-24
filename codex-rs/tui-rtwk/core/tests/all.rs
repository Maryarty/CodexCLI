//! The core's integration tests, one binary: every `tests/<name>.rs` (and every `tests/<dir>/mod.rs`)
//! is a module here, listed by `core_all!` from the files (`test-mods/build.rs`, `scan.rs`): a new
//! test file needs no line in this one.
//!
//! One binary instead of one per file: each test binary is a rustc run and a link of the
//! whole dependency set, and every edit of the core relinked all 25 of them (the
//! build-speed task, docs/rtwk-port/build-speed.md). rustfmt does not follow the generated
//! modules: `tools/rtwk-port/registry/fmt.sh` formats them.

tui_test_mods::core_all!();
