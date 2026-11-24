//! A test file nobody registered is never built, and its tests never fail: the merged test
//! binaries (docs/rtwk-port/build-speed.md) list their files, so this keeps the lists whole. The
//! roots that list by hand: `core/tests/all.rs` and `tests/rules/main.rs` list every file of their
//! directory. The roots of the UI crate that test files are modules of (`screens`, `pages`,
//! `oracle`, `shell`) have no list: `build.rs` of `test-mods` writes it from the files, and
//! `tests/rules/test_roots.rs` keeps hand-written `mod` lines out of them.

#![allow(clippy::expect_used, clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rs_files(dir: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(crate_dir().join(dir))
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    files
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .expect("a file name")
        .to_string_lossy()
        .into_owned()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Files of `dir` (but `root_file`) that `root_file` does not list as `mod <stem>;`.
fn unlisted(dir: &str, root_file: &str) -> Vec<String> {
    let root = read(&crate_dir().join(root_file));
    rs_files(dir)
        .iter()
        .map(|path| stem(path))
        .filter(|name| {
            format!("{name}.rs")
                != Path::new(root_file)
                    .file_name()
                    .expect("a root file")
                    .to_string_lossy()
        })
        .filter(|name| {
            !root
                .lines()
                .any(|line| line.trim() == format!("mod {name};"))
        })
        .map(|name| format!("{dir}/{name}.rs is not a `mod {name};` of {root_file}"))
        .collect()
}

#[test]
fn every_test_file_is_registered() {
    let mut missing = Vec::new();
    missing.extend(unlisted("core/tests", "core/tests/all.rs"));
    missing.extend(unlisted("tests/rules", "tests/rules/main.rs"));
    assert!(
        missing.is_empty(),
        "test files nobody builds (register them, or delete them):\n{}",
        missing.join("\n")
    );
}

/// `[[test]]` targets of Cargo.toml and files agree: a target whose file is gone is a build
/// error, a root file without a target is dead.
#[test]
fn every_test_root_has_a_target() {
    let manifest = read(&crate_dir().join("Cargo.toml"));
    let roots = [
        "tests/screens.rs",
        "tests/pages.rs",
        "tests/shell.rs",
        "tests/rules/main.rs",
        "tests/kit_proof.rs",
        "tests/oracle.rs",
        "tests/scenarios/main.rs",
    ];
    for root in roots {
        assert!(
            manifest.contains(&format!("path = \"{root}\"")),
            "{root} has no [[test]] target in Cargo.toml"
        );
    }
    assert!(
        read(&crate_dir().join("core/Cargo.toml")).contains("path = \"tests/all.rs\""),
        "core/tests/all.rs has no [[test]] target in core/Cargo.toml"
    );
}
