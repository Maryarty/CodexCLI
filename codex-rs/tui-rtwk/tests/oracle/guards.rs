//! The rules the shard files are held to, as tests: black-box (the kit and
//! nothing else), no skipped test, no expectation of our own making, and one
//! test per oracle test of the shard's manifest.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/oracle")
}

/// (shard name, text) of every `tests/oracle/<shard>.rs` but this file.
fn shards() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir()).expect("tests/oracle").flatten() {
        let path = entry.path();
        let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if path.extension().is_some_and(|e| e == "rs") && name != "guards" {
            let text = std::fs::read_to_string(&path).expect("read shard");
            out.push((name.to_string(), text));
        }
    }
    out.sort();
    out
}

/// What a shard file must never name: the way in to core or to the view, a
/// skipped test, an expectation that is not the oracle's.
const FORBIDDEN: [(&str, &str); 14] = [
    ("codex_tui_core", "core's API: seed with the kit"),
    (
        "codex_tui_rtwk",
        "the view's API: drive the screen with the kit",
    ),
    ("rtwk", "the framework: the kit wraps the tester"),
    ("oracle_snap", "compare with the kit's assert_snap*"),
    ("common::", "start the screen with the kit"),
    (
        "#[ignore",
        "never skip a test: kit::pending(owner, reason) first in the body",
    ),
    ("#[cfg", "no conditional tests"),
    ("#[allow", "no lint allowances"),
    (
        "std::fs",
        "no files: the oracle's snapshots come through the kit",
    ),
    (
        "UPDATE_SNAPSHOTS",
        "expected values come from the oracle only",
    ),
    (
        "assert!(",
        "no expectation of our own: use the kit's assert_snap*, assert_contains, assert_row",
    ),
    (
        "assert_eq!(",
        "no expectation of our own: use the kit's assert_snap*, assert_contains, assert_row",
    ),
    (
        "assert_ne!(",
        "no expectation of our own: use the kit's assert_snap*, assert_contains, assert_row",
    ),
    ("unsafe", "no unsafe"),
];

fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with("//"))
        .map(|(i, l)| (i + 1, l))
}

#[test]
fn shards_are_black_box_and_never_skipped() {
    let mut bad = Vec::new();
    for (name, text) in shards() {
        for (line, code) in code_lines(&text) {
            for (token, why) in FORBIDDEN {
                if code.contains(token) {
                    bad.push(format!("tests/oracle/{name}.rs:{line}: `{token}`: {why}"));
                }
            }
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// Every `// ORACLE <file>:<line> <fn> ..` line of a shard's manifest has its
/// `fn <fn>(`: ported, pending or n/a, never dropped.
#[test]
fn every_oracle_test_of_a_manifest_has_its_test() {
    let mut missing = Vec::new();
    for (name, text) in shards() {
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("// ORACLE ") else {
                continue;
            };
            let mut parts = rest.split_whitespace();
            let (Some(origin), Some(func)) = (parts.next(), parts.next()) else {
                missing.push(format!(
                    "tests/oracle/{name}.rs: bad manifest line `{line}`"
                ));
                continue;
            };
            if !text.contains(&format!("fn {func}(")) {
                missing.push(format!(
                    "tests/oracle/{name}.rs: no `fn {func}` for {origin}"
                ));
            }
        }
    }
    assert!(missing.is_empty(), "\n{}", missing.join("\n"));
}

/// Every shard file is a module of `tests/oracle.rs`.
#[test]
fn every_shard_is_registered() {
    let root = std::fs::read_to_string(dir().join("../oracle.rs")).expect("oracle.rs");
    let mut loose = Vec::new();
    for (name, _) in shards() {
        if !root.contains(&format!("#[path = \"oracle/{name}.rs\"]")) {
            loose.push(name);
        }
    }
    assert!(loose.is_empty(), "not in tests/oracle.rs: {loose:?}");
}
