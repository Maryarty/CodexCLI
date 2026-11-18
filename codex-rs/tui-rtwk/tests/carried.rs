//! Drift test of the items core carries verbatim from the oracle: every
//! `// oracle: <file> <kind> <name>` marker in `core/src` names an item of
//! `codex-rs/tui/src/app/<file>`; its text must equal the oracle's once the
//! visibility keywords are dropped and whitespace is collapsed. A repin that
//! edits one of them fails here, which is the point (reuse-plan §1, route b).

use std::fs;
use std::path::Path;
use std::path::PathBuf;

const MARKER: &str = "// oracle: ";

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Removes comments-free noise: visibility, then collapses whitespace.
fn normalise(text: &str) -> String {
    let mut text = text.to_owned();
    for vis in ["pub(super) ", "pub(crate) ", "pub "] {
        text = text.replace(vis, "");
    }
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The item that starts at `start` (a line index): its attribute lines, then
/// up to the brace that closes its body. Strings and chars are skipped.
fn item_from(lines: &[&str], start: usize) -> String {
    let mut text = String::new();
    let mut depth = 0_i32;
    let mut opened = false;
    for line in &lines[start..] {
        text.push_str(line);
        text.push('\n');
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut in_str = false;
        while i < bytes.len() {
            let c = bytes[i];
            if in_str {
                if c == b'\\' {
                    i += 1;
                } else if c == b'"' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == b'/' && bytes.get(i + 1) == Some(&b'/') {
                break;
            } else if c == b'\'' && bytes.get(i + 2) == Some(&b'\'') {
                i += 2;
            } else if c == b'{' {
                depth += 1;
                opened = true;
            } else if c == b'}' {
                depth -= 1;
            }
            i += 1;
        }
        if opened && depth == 0 {
            break;
        }
    }
    text
}

/// The carried copy: the lines after the marker (the marker itself and the
/// comment lines are not part of the item).
fn carried_item(lines: &[&str], marker: usize) -> String {
    item_from(lines, marker + 1)
}

fn oracle_item(source: &str, kind: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let needle_a = format!("{kind} {name}(");
    let needle_b = format!("{kind} {name} ");
    let needle_c = format!("{kind} {name}<");
    let at = lines.iter().position(|line| {
        let trimmed = line.trim_start();
        let trimmed = trimmed
            .strip_prefix("pub(super) ")
            .or_else(|| trimmed.strip_prefix("pub(crate) "))
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        trimmed.starts_with(&needle_a)
            || trimmed.starts_with(&needle_b)
            || trimmed.starts_with(&needle_c)
    })?;
    let mut start = at;
    while start > 0 && lines[start - 1].trim_start().starts_with("#[") {
        start -= 1;
    }
    Some(item_from(&lines, start))
}

#[test]
fn carried_fragments_equal_the_oracle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("core/src"), &mut files);
    let mut checked = 0;
    let mut failures = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("read");
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some(rest) = line.trim_start().strip_prefix(MARKER) else {
                continue;
            };
            let parts: Vec<&str> = rest.split_whitespace().collect();
            let [source, kind, name] = parts[..] else {
                continue;
            };
            let oracle_path = root.join("../tui/src/app").join(source);
            let oracle = fs::read_to_string(&oracle_path)
                .unwrap_or_else(|error| panic!("{}: {error}", oracle_path.display()));
            let Some(expected) = oracle_item(&oracle, kind, name) else {
                failures.push(format!("{source}: no `{kind} {name}` in the oracle"));
                continue;
            };
            let have = carried_item(&lines, index);
            if normalise(&have) != normalise(&expected) {
                failures.push(format!(
                    "{source} {kind} {name} drifted\n--- carried\n{have}\n--- oracle\n{expected}"
                ));
            }
            checked += 1;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    assert!(checked >= 12, "only {checked} carried items were checked");
}
