//! A kind of app-scoped ask says what a lost connection does to it (core-review-a F1): no `match`
//! over `AppCtx` has a catch-all arm, so that a new kind has to choose (`AppCtx::on_lost`, in
//! `core/src/app/connection.rs`) and an ask in flight is never kept by a `_ => true` that nobody
//! meant for it.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use crate::layers::srcscan::sources_of;

/// The text with its line comments blanked (lengths kept).
fn without_comments(text: &str) -> String {
    text.lines()
        .map(|line| match line.find("//") {
            Some(at) => format!("{}{}", &line[..at], " ".repeat(line.len() - at)),
            None => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The arms of the `match` whose body opens at `open` (the `{`): each arm's pattern (guard
/// included), at the top level of the body.
fn arm_patterns(text: &str, open: usize) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut patterns = Vec::new();
    let mut depth = 0usize;
    let mut pattern = String::new();
    let mut in_pattern = true;
    let mut block_body = false;
    let mut at = open;
    while at < bytes.len() {
        let ch = bytes[at] as char;
        match ch {
            '{' | '(' | '[' => {
                depth += 1;
                if depth > 1 && in_pattern {
                    pattern.push(ch);
                }
            }
            '}' | ')' | ']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                if depth == 1 && !in_pattern && block_body {
                    in_pattern = true;
                    block_body = false;
                    pattern.clear();
                } else if in_pattern {
                    pattern.push(ch);
                }
            }
            '=' if depth == 1 && in_pattern && bytes.get(at + 1) == Some(&b'>') => {
                patterns.push(pattern.trim().to_owned());
                in_pattern = false;
                at += 2;
                while bytes.get(at).is_some_and(u8::is_ascii_whitespace) {
                    at += 1;
                }
                block_body = bytes.get(at) == Some(&b'{');
                continue;
            }
            ',' if depth == 1 && !in_pattern && !block_body => {
                in_pattern = true;
                pattern.clear();
            }
            ',' if depth == 1 && in_pattern && pattern.is_empty() => {}
            _ if in_pattern => pattern.push(ch),
            _ => {}
        }
        at += 1;
    }
    patterns
}

/// Whether the pattern takes every kind that is left: `_`, a binding, either with a guard.
fn catches_all(pattern: &str) -> bool {
    let head = pattern.split(" if ").next().unwrap_or(pattern).trim();
    head.split('|').any(|alt| {
        let alt = alt.trim();
        alt == "_"
            || (!alt.is_empty()
                && alt
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
                && alt != "None")
    })
}

/// The matches over `AppCtx` of `text` that have a catch-all arm: the line of the `match`.
fn wildcards(text: &str) -> Vec<usize> {
    let code = without_comments(text);
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = code[from..].find("match ") {
        let start = from + at;
        from = start + "match ".len();
        let before_ok = start == 0 || !code.as_bytes()[start - 1].is_ascii_alphanumeric();
        let Some(brace) = code[from..].find('{').map(|b| from + b) else {
            break;
        };
        // a scrutinee has no `;` before its body
        if !before_ok || code[from..brace].contains(';') {
            continue;
        }
        let patterns = arm_patterns(&code, brace);
        let over_app_ctx = patterns.iter().any(|p| p.starts_with("AppCtx::"));
        if over_app_ctx && patterns.iter().any(|p| catches_all(p)) {
            found.push(code[..start].matches('\n').count() + 1);
        }
    }
    found
}

/// The matches over `AppCtx` that keep a catch-all, by file and the text of the arm: named, with
/// why.
const EXEMPT: &[(&str, &str)] = &[(
    "core/src/app.rs",
    // `may_attach`: asks whether this ask is one of the attaching kinds (`is_attach`'s list),
    // a question about three kinds; the others answer `false` in `is_attach`.
    "_ => is_attach(ctx)",
)];

#[test]
fn no_match_over_app_ctx_has_a_catch_all_arm() {
    let mut bad = Vec::new();
    for (name, text) in sources_of("core/src") {
        let lines = wildcards(&text);
        if lines.is_empty() {
            continue;
        }
        let exempt: Vec<&str> = EXEMPT
            .iter()
            .filter(|(file, _)| *file == name)
            .map(|(_, arm)| *arm)
            .collect();
        let allowed = exempt.iter().filter(|arm| text.contains(**arm)).count();
        if lines.len() > allowed {
            bad.push(format!("{name}:{lines:?}"));
        }
    }
    assert!(
        bad.is_empty(),
        "a match over `AppCtx` with a catch-all arm (name every kind: a new one has to choose what a lost connection does to it, `AppCtx::on_lost`): {bad:?}"
    );
}

#[test]
fn the_exemptions_are_still_catch_alls() {
    for (file, arm) in EXEMPT {
        let found = sources_of("core/src").into_iter().any(|(name, text)| {
            name == *file && text.contains(arm) && !wildcards(&text).is_empty()
        });
        assert!(
            found,
            "{file}: `{arm}` no longer is a catch-all over AppCtx; strike it out"
        );
    }
}

#[test]
fn the_scan_bites() {
    let wild = "fn f(c: &AppCtx) -> bool { match c { AppCtx::A { x } => *x, AppCtx::B(_) => { true } _ => true } }";
    assert_eq!(wildcards(wild), vec![1]);
    let bound = "match ctx {\n AppCtx::A => 1,\n other => 2,\n}";
    assert_eq!(wildcards(bound), vec![1]);
    let clean = "match ctx {\n AppCtx::A | AppCtx::B => 1,\n AppCtx::C { n } if *n > 1 => 2,\n AppCtx::C { .. } => 3,\n}";
    assert!(wildcards(clean).is_empty());
    // a match over something else, or a wildcard inside an arm's own match, is not it
    let other = "match x {\n Some(_) => 1,\n _ => 2,\n}";
    assert!(wildcards(other).is_empty());
    let nested = "match ctx {\n AppCtx::A => match y { _ => 1 },\n AppCtx::B => 2,\n}";
    assert!(wildcards(nested).is_empty());
    let commented = "match ctx {\n AppCtx::A => 1, // _ => 2\n AppCtx::B => 2,\n}";
    assert!(wildcards(commented).is_empty());
}
