//! Registry 4, tests (fw-ui-reg-2): the roots of the UI crate's test files (`screens`, `pages`,
//! `oracle`, `shell`) hold no `mod` line of a test file. `build.rs` of `test-mods` writes their
//! module lists from the files of `tests/{app,pages,sheets,oracle,services}` (`scan.rs`, the one
//! source of both), so a new test file never edits a root and two branches that each add a file do
//! not conflict on one. `tests/rules/main.rs` and `core/tests/all.rs` stay hand lists
//! (`registration.rs` checks them): the rules root must not depend on the crate that rebuilds when
//! a test file changes, and core's lists are fw-hubs-c's.

#![allow(clippy::expect_used, clippy::disallowed_methods)]

#[path = "../../test-mods/scan.rs"]
mod scan;

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The names of the `mod x;` lines (with or without `pub`, any `#[path]` above) of a root file.
fn hand_mods(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("pub ").or(Some(line)))
        .filter_map(|line| line.strip_prefix("mod "))
        .filter_map(|rest| rest.strip_suffix(';'))
        .map(str::to_owned)
        .collect()
}

#[test]
fn test_roots_have_no_hand_mods() {
    let mut fails = Vec::new();
    for root in scan::ROOTS {
        let rel = format!("tests/{root}.rs");
        let text = fs::read_to_string(crate_dir().join(&rel)).expect("a test root");
        let call = format!("tui_test_mods::{root}!();");
        if !text.lines().any(|line| line.trim() == call) {
            fails.push(format!("{rel} does not list its modules by `{call}`"));
        }
        for name in hand_mods(&text) {
            if !scan::SUPPORT.contains(&name.as_str()) {
                fails.push(format!(
                    "{rel}: `mod {name};` written by hand (a test file is a module by being a file of tests/{{app,pages,sheets,oracle,services}}; only the support modules {:?} are lines)",
                    scan::SUPPORT
                ));
            }
        }
    }
    // the scan reads every file of the directories: a marker that names no root, two files of
    // one module name, a file that clashes with a support module are errors here, not in a build
    if let Err(error) = scan::scan(&crate_dir().join("tests")) {
        fails.push(format!("the test scan fails: {error}"));
    }
    assert!(
        fails.is_empty(),
        "test roots with a hand-written module list:\n{}",
        fails.join("\n")
    );
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    fs::write(path, text).expect("write");
}

fn layout(dir: &Path) {
    for sub in ["app", "pages", "sheets", "oracle", "services"] {
        fs::create_dir_all(dir.join(sub)).expect("mkdir");
    }
}

fn modules(found: &[scan::Module], root: &str) -> Vec<String> {
    found
        .iter()
        .filter(|m| m.root == root)
        .map(|m| m.name.clone())
        .collect()
}

/// The scan sees a new file: a test file nobody lists is found by what it is and where.
#[test]
fn the_test_scan_bites() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let tests = dir.path();
    layout(tests);
    write(tests, "pages/marked.rs", "// test root: pages\n//! doc\n");
    write(
        tests,
        "pages/plain.rs",
        "//! no marker: screens\nmod plain_child;\n",
    );
    write(tests, "pages/plain_child.rs", "// a child of plain\n");
    write(tests, "sheets/sheet.rs", "");
    write(tests, "app/part.rs", "");
    write(tests, "oracle/shard_1.rs", "");
    write(tests, "services/svc.rs", "");
    write(tests, "services/notes.txt", "not a test file");
    let found = scan::scan(tests).expect("a clean layout scans");
    assert_eq!(modules(&found, "pages"), ["marked"]);
    assert_eq!(modules(&found, "screens"), ["part", "plain", "sheet"]);
    assert_eq!(modules(&found, "oracle"), ["shard_1"]);
    assert_eq!(modules(&found, "shell"), ["svc"]);
    let text = scan::macros(&found);
    assert!(text.contains("macro_rules! pages"), "{text}");
    assert!(text.contains("mod marked;"), "{text}");
    assert!(!text.contains("mod plain_child;"), "{text}");

    // a file added later is a module of the next scan, whatever root it is for
    write(tests, "pages/later.rs", "// test root: pages\n");
    let found = scan::scan(tests).expect("scans");
    assert_eq!(modules(&found, "pages"), ["later", "marked"]);
}

#[test]
fn the_test_scan_refuses_what_it_cannot_place() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let tests = dir.path();
    layout(tests);
    write(tests, "pages/bad.rs", "// test root: oracle\n");
    let error = scan::scan(tests).expect_err("a marker naming another root");
    assert!(error.contains("names no root"), "{error}");

    fs::remove_file(tests.join("pages/bad.rs")).expect("rm");
    write(tests, "pages/twin.rs", "");
    write(tests, "sheets/twin.rs", "");
    let error = scan::scan(tests).expect_err("two files of one module name");
    assert!(error.contains("module `twin`"), "{error}");

    fs::remove_file(tests.join("sheets/twin.rs")).expect("rm");
    write(tests, "app/kit.rs", "");
    let error = scan::scan(tests).expect_err("a file with a support module's name");
    assert!(error.contains("support module"), "{error}");
}
