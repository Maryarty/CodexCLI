//! The verdict markers of the oracle shards (`tests/kit/verdict.rs`): `kit::delta(owner, what)`
//! says in words what small visual difference a passing test has (docs/rtwk-port/visual-deltas.md),
//! it is the only verdict of its test (a test is pending, n/a or a delta, never two), and the
//! kit refuses at run time what the document says is not small (other words on the rows).

#![allow(clippy::expect_used, clippy::disallowed_methods)]

use std::path::PathBuf;

fn oracle_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/oracle")
}

/// (shard file name, text) of every `tests/oracle/*.rs`.
fn shards() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(oracle_dir())
        .expect("tests/oracle")
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "rs") {
            let name = entry.file_name().to_string_lossy().into_owned();
            out.push((name, std::fs::read_to_string(&path).expect("read shard")));
        }
    }
    out.sort();
    out
}

/// The test functions of a shard: (name, line of its `fn`, its text up to the next top-level `fn`).
fn functions(text: &str) -> Vec<(String, usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].starts_with("fn "))
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, &at)| {
            let end = starts.get(k + 1).copied().unwrap_or(lines.len());
            let name = lines[at][3..]
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .next()
                .unwrap_or("")
                .to_string();
            (name, at + 1, lines[at..end].join("\n"))
        })
        .collect()
}

/// The code lines of a function (a `//` comment line is not code).
fn code(text: &str) -> String {
    text.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The words of the string literals inside the parentheses that open at `open` (a `\` at a line
/// end continues the literal, as Rust reads it), or None when they do not close.
fn call_text(text: &str, open: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let (mut depth, mut i, mut words, mut in_str) = (0usize, open, String::new(), false);
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_str {
            match c {
                '\\' if bytes.get(i + 1) == Some(&b'\n') => {
                    i += 1;
                    while bytes.get(i + 1).is_some_and(u8::is_ascii_whitespace) {
                        i += 1;
                    }
                }
                '\\' => i += 1,
                '"' => in_str = false,
                _ => words.push(c),
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Some(words);
            }
        }
        i += 1;
    }
    None
}

#[test]
fn a_delta_says_what_differs_and_is_the_only_verdict_of_its_test() {
    let mut bad = Vec::new();
    for (file, text) in shards() {
        for (name, line, body) in functions(&text) {
            let body = code(&body);
            let Some(at) = body.find("kit::delta(") else {
                continue;
            };
            let open = at + "kit::delta".len();
            let what = call_text(&body, open).unwrap_or_default();
            if what.split_whitespace().count() < 3 {
                bad.push(format!(
                    "tests/oracle/{file}:{line} `{name}`: kit::delta needs `what`, at least three words that \
                     say which colour, indent, glyph or wrap differs (visual-deltas.md); got `{what}`"
                ));
            }
            for other in ["kit::pending(", "kit::na(", "kit::missing(", "kit::todo("] {
                if body.contains(other) {
                    bad.push(format!(
                        "tests/oracle/{file}:{line} `{name}`: `{other}` and kit::delta in one test: one verdict per test"
                    ));
                }
            }
            if body.matches("kit::delta(").count() > 1 {
                bad.push(format!(
                    "tests/oracle/{file}:{line} `{name}`: kit::delta twice: say all of it in one `what`"
                ));
            }
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}

/// The rule reads a call as it is written: concatenated literals, a continued line, and a
/// comment that names the marker is not a call.
#[test]
fn the_call_text_is_read_as_written() {
    let src = "kit::delta(\n    Owner::UiPane,\n    \"a glyph \\\n     and a dim\",\n);";
    assert_eq!(
        call_text(src, "kit::delta".len()).as_deref(),
        Some("a glyph and a dim")
    );
    assert!(
        code("// kit::delta(Owner::UiPane, \"x\")\nlet a = 1;")
            .find("kit::delta(")
            .is_none()
    );
    assert_eq!(
        call_text("kit::delta(Owner::UiPane", "kit::delta".len()),
        None
    );
}
