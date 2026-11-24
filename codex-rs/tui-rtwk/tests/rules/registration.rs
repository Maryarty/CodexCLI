//! A test file nobody registered is never built, and its tests never fail: the merged test
//! binaries (docs/rtwk-port/build-speed.md) list their files, so this keeps the lists whole. The
//! root that lists by hand: `tests/rules/main.rs` lists every file of its directory. The roots of
//! the UI crate that test files are modules of (`screens`, `pages`, `oracle`, `shell`) and the
//! core's `tests/all.rs` have no list: `build.rs` of `test-mods` writes it from the files
//! (`core_all!` for the core), and `tests/rules/test_roots.rs` keeps hand-written `mod` lines out
//! of the UI roots. The registration lists of the sources (`feature_list!`, `action_list!`,
//! `mode_list!`, `cell_list!`, `sheet_list!`) are written the same way (`test-mods/registry.rs`);
//! the tests at the end bite on its scans.

#![allow(clippy::expect_used, clippy::disallowed_methods)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::layers::registry;
use crate::test_roots::scan;

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
    missing.extend(unlisted("tests/rules", "tests/rules/main.rs"));
    assert!(
        missing.is_empty(),
        "test files nobody builds (register them, or delete them):\n{}",
        missing.join("\n")
    );
}

/// `core/tests/all.rs` lists no module by hand: `core_all!` writes them from the files of
/// `core/tests`, and the scan of them is whole.
#[test]
fn core_tests_are_listed_by_their_files() {
    let all = read(&crate_dir().join("core/tests/all.rs"));
    assert!(
        all.lines()
            .any(|line| line.trim() == "tui_test_mods::core_all!();"),
        "core/tests/all.rs does not list its modules by `tui_test_mods::core_all!();`"
    );
    for line in all.lines().map(str::trim) {
        assert!(
            !line.starts_with("mod ") && !line.starts_with("pub mod "),
            "core/tests/all.rs: `{line}` written by hand (a test file is a module by being a file)"
        );
    }
    let found = scan::scan_core(&crate_dir().join("core/tests"))
        .unwrap_or_else(|e| panic!("core test scan: {e}"));
    assert!(found.iter().any(|m| m.name == "goal"), "{found:?}");
    assert!(found.iter().all(|m| m.name != "all"));
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

fn put(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    fs::write(path, text).expect("write");
}

/// A new slice is a directory: the registry finds its marker and whether it opens a sheet, and
/// lists the slices by name; a directory without an `impl Feature for` is an error, not a gap.
#[test]
fn the_feature_scan_bites() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();
    put(
        root,
        "zeta/mod.rs",
        "impl Feature for Zeta {\n    const OPENS: Option<(A, B)> = None;\n}\n",
    );
    put(
        root,
        "alpha/mod.rs",
        "feature! {\n    impl Feature for Alpha {\n        const OPENS: Option<(SlashCommand, Self::Target)> =\n            Some((SlashCommand::Ps, Sheet));\n    }\n}\n",
    );
    put(root, "alpha/more.rs", "// impl Feature for Commented\n");
    put(root, "cx.rs", "pub struct Cx;\n");
    let slices = registry::core_features(root).expect("a scan");
    let names: Vec<(&str, &str, bool)> = slices
        .iter()
        .map(|s| (s.name.as_str(), s.marker.as_str(), s.opens))
        .collect();
    assert_eq!(names, [("alpha", "Alpha", true), ("zeta", "Zeta", false)]);
    let text = registry::core_features_text(root, &slices);
    assert!(
        text.contains("pub mod alpha;") && text.contains("pub mod zeta;"),
        "{text}"
    );
    assert!(
        text.contains("            alpha => Alpha;\n            zeta => Zeta;\n"),
        "{text}"
    );
    put(root, "orphan/mod.rs", "pub struct Nothing;\n");
    assert!(registry::core_features(root).is_err());
}

/// The cell head line is read from the first lines only; an action file and a mode file are found
/// by their impl.
#[test]
fn the_ui_list_scans_bite() {
    assert_eq!(
        registry::cell_pattern("// cell: Item::Error { .. } | Item::HighLoad\n//! doc\n")
            .as_deref(),
        Some("Item::Error { .. } | Item::HighLoad")
    );
    assert_eq!(registry::cell_pattern("//! doc\n// cell:\n"), None);
    let late = format!("{}// cell: Item::User(_)\n", "//\n".repeat(20));
    assert_eq!(registry::cell_pattern(&late), None);
    let dir = tempfile::tempdir().expect("a temp dir");
    let root = dir.path();
    put(root, "find.rs", "impl KeyAction for Find {}\n");
    put(root, "mod.rs", "pub trait KeyAction {}\n");
    let text = registry::actions_text(root).expect("actions");
    assert!(
        text.contains("find => Find;") && text.contains("pub mod find;"),
        "{text}"
    );
    put(root, "stray.rs", "pub struct NotAnAction;\n");
    assert!(registry::actions_text(root).is_err());
    let modes = tempfile::tempdir().expect("a temp dir");
    put(
        modes.path(),
        "pager.rs",
        "impl TranscriptMode for Mode {}\n",
    );
    put(modes.path(), "history.rs", "pub fn not_a_mode() {}\n");
    let text = registry::modes_text(modes.path()).expect("modes");
    assert!(
        text.contains("pager;") && !text.contains("history"),
        "{text}"
    );
}
