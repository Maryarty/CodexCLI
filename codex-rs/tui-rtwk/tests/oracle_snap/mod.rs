//! The replay helper of the screen tests of the route screens: reads a
//! codex-tui insta `.snap` (the oracle, `../tui/src/**/snapshots/`) and
//! compares rows of the screen under test with it. The expected rows are the
//! oracle's text; nothing here is ours to edit when a screen changes.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::PathBuf;

use rtwk_test::Tester;

fn oracle_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tui/src")
        .join(rel)
}

/// The body of an insta snapshot: the lines after the front matter.
fn body(rel: &str) -> Vec<String> {
    let text = std::fs::read_to_string(oracle_path(rel))
        .unwrap_or_else(|err| panic!("oracle snapshot {rel}: {err}"));
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("---"), "{rel}: front matter");
    for line in lines.by_ref() {
        if line == "---" {
            break;
        }
    }
    lines.map(str::to_string).collect()
}

/// A `Debug`-quoted row (`"text   "`) as the text it holds.
fn unquote(row: &str) -> String {
    match row.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        Some(inner) => inner.replace("\\\"", "\"").replace("\\\\", "\\"),
        None => row.to_string(),
    }
}

fn trimmed(rows: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut rows: Vec<String> = rows.into_iter().map(|r| r.trim_end().to_string()).collect();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows
}

/// The rows of the oracle snapshot `rel` (relative to `codex-rs/tui/src`),
/// unquoted, trailing blanks trimmed.
pub fn rows(rel: &str) -> Vec<String> {
    trimmed(body(rel).iter().map(|r| unquote(r)))
}

/// The rows of the section `name` of a snapshot made of `--- name ---`
/// sections (`transcript_overlay_paginated_history_states`).
pub fn section(rel: &str, name: &str) -> Vec<String> {
    let marker = format!("--- {name} ---");
    let body = body(rel);
    let start = body
        .iter()
        .position(|line| *line == marker)
        .unwrap_or_else(|| panic!("{rel}: no section {name}"));
    trimmed(
        body[start + 1..]
            .iter()
            .take_while(|line| !(line.starts_with("--- ") && line.ends_with(" ---")))
            .cloned()
            .collect::<Vec<_>>(),
    )
}

/// The screen's rows, trailing blanks trimmed.
pub fn screen(t: &Tester) -> Vec<String> {
    trimmed(t.lines())
}

/// Compares whole screens: the rows equal, line by line.
pub fn assert_same(label: &str, actual: &[String], expected: &[String]) {
    pretty_assertions::assert_eq!(
        expected.join("\n"),
        actual.join("\n"),
        "{label}: ours (right) differs from the oracle (left)"
    );
}

/// Compares `expected` with the screen rows from `at` on (a snapshot that is a
/// fragment of the oracle's screen).
pub fn assert_fragment(label: &str, actual: &[String], at: usize, expected: &[String]) {
    let got: Vec<String> = actual
        .iter()
        .skip(at)
        .take(expected.len())
        .cloned()
        .collect();
    assert_same(label, &trimmed(got), &trimmed(expected.iter().cloned()));
}
