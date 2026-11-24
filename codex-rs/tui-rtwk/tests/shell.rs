//! The shell's tests (backend, connection, cycle, agents services), one binary: each file of
//! `tests/services/` is a module here (docs/rtwk-port/build-speed.md; the list is written by
//! `build.rs` of `test-mods`, a new service test file needs no line).

tui_test_mods::shell!();
