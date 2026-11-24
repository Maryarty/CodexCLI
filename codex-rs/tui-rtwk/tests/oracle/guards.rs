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

/// The code of a shard with its comments and the insides of its string
/// literals blanked (a pending reason may say `rtwk` or `oracle_snap`; the
/// rules are about what the code names), lines kept.
fn code_only(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2;
        } else if c == '\'' && next == Some('"') && chars.get(i + 2) == Some(&'\'') {
            out.push_str("' '");
            i += 3;
        } else if c == 'r' && matches!(next, Some('"' | '#')) && is_raw_start(&chars, i) {
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            j += 1; // the opening quote
            out.push_str("\"\"");
            loop {
                match chars.get(j) {
                    None => break,
                    Some('"') if (1..=hashes).all(|k| chars.get(j + k) == Some(&'#')) => {
                        j += 1 + hashes;
                        break;
                    }
                    Some('\n') => out.push('\n'),
                    _ => {}
                }
                j += 1;
            }
            i = j;
        } else if c == '"' {
            out.push_str("\"\"");
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                if chars.get(i) == Some(&'\n') {
                    out.push('\n');
                }
                i += 1;
            }
            i += 1;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `r"` or `r#"` that starts a raw string (not the end of an identifier).
fn is_raw_start(chars: &[char], i: usize) -> bool {
    let before = i.checked_sub(1).and_then(|b| chars.get(b));
    if before.is_some_and(|c| c.is_alphanumeric() || *c == '_') {
        return false;
    }
    let mut j = i + 1;
    while chars.get(j) == Some(&'#') {
        j += 1;
    }
    chars.get(j) == Some(&'"')
}

fn code_lines(text: &str) -> impl Iterator<Item = (usize, String)> {
    code_only(text)
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect::<Vec<_>>()
        .into_iter()
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

/// Every shard file is a module of `tests/oracle.rs`: the root says `oracle!()`, whose list
/// `build.rs` of `test-mods` writes from the files (`tests/rules/test_roots.rs` checks the scan),
/// and holds no `mod` line of a shard by hand.
#[test]
fn every_shard_is_registered() {
    let root = std::fs::read_to_string(dir().join("../oracle.rs")).expect("oracle.rs");
    assert!(
        root.lines()
            .any(|line| line.trim() == "tui_test_mods::oracle!();"),
        "tests/oracle.rs does not list its shards through `tui_test_mods::oracle!();`"
    );
    let by_hand: Vec<String> = shards()
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| root.contains(&format!("mod {name};")))
        .collect();
    assert!(
        by_hand.is_empty(),
        "shard modules written by hand in tests/oracle.rs: {by_hand:?}"
    );
}

/// What a pending reason says is not code: a reason may name `rtwk` or
/// `oracle_snap`; a call in the code still is seen, and so is code after a
/// string, a char literal holding a quote, and a raw string.
#[test]
fn words_in_strings_and_comments_are_not_code() {
    let src = concat!(
        "kit::pending(O, \"the oracle_snap and rtwk words, an \\\" quote\"); // assert!(c)\n",
        "let raw = r#\"std::fs \"inside\" \"#; let q = '\"'; assert!(real);\n",
        "/* block\n assert_eq!(b) */ let z = codex_tui_core::x;\n",
    );
    let code = code_only(src);
    for word in [
        "oracle_snap",
        "rtwk",
        "std::fs",
        "assert!(c)",
        "assert_eq!(b)",
    ] {
        assert!(!code.contains(word), "`{word}` is in:\n{code}");
    }
    assert!(code.contains("assert!(real)"), "{code}");
    assert!(code.contains("codex_tui_core"), "{code}");
    assert_eq!(code.lines().count(), 4, "lines are kept:\n{code}");
}
