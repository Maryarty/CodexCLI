//! Page tests that bring their own harness setup, one binary: the mouse tests of the pages,
//! the composer draft, and the probes of the open rtwk gaps (`pending(Gxx)`). The page tests
//! of `tests/screens.rs` stay there; this binary holds what once had a binary each
//! (docs/rtwk-port/build-speed.md). A file under `tests/pages/` whose first line is
//! `// test root: pages` is a module here, without a line in this file (`test-mods/scan.rs`).

// the kit and the harness it drives are crate-level modules (`crate::common`), as in `screens`
#[path = "common/mod.rs"]
mod common;
#[path = "kit/mod.rs"]
mod kit;

tui_test_mods::pages!();
