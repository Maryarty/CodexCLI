//! The module lists of the test roots of `codex-tui-rtwk`, written by `build.rs` from the files of
//! `tests/{app,pages,sheets,oracle,services}`: a root says `tui_test_mods::pages!();`
//! and a new test file is a file, never a line of a root. Which root a file of `pages`, `sheets`
//! and `app` belongs to is a marker on its first lines (`// test root: pages`; none: `screens`),
//! see `scan.rs`. rustfmt does not follow a module that a macro declares: `acargo fmt` does not
//! reach these files (docs/rtwk-port/build-speed.md says how to format them).

include!(concat!(env!("OUT_DIR"), "/macros.rs"));
