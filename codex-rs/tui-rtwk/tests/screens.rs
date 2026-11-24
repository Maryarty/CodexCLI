//! The screen tests: one binary, the skeleton's own tests first, then one
//! module per page, sheet and app part (`tests/{pages,sheets,app}/<name>.rs`). The modules are
//! the files: `build.rs` of `test-mods` lists them (`tui_test_mods::screens!`), so a
//! slice adds tests in its file and edits no root, and the build links one test binary, not one
//! per directory. A file of those directories is a module of this root unless its first lines
//! hold `// test root: pages` (`tests/pages.rs`).

mod common;
mod unit;

tui_test_mods::screens!();
