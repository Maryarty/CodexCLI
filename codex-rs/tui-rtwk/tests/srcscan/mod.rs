//! Source scanning shared by the rules tests (`rules.rs`, `layers.rs`): the
//! crates' sources with their text, comment-free lines, pattern scans.

#![allow(dead_code)]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

pub fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

pub fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read_dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The sources below `rel` (relative to the crate directory, e.g. `src` or
/// `core/src`) as `(path relative to the crate directory, text)`.
pub fn sources_of(rel: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(&crate_dir().join(rel), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(crate_dir())
                .expect("inside")
                .to_string_lossy()
                .replace('\\', "/");
            let text = std::fs::read_to_string(&path).expect("read");
            (name, text)
        })
        .collect()
}

/// The UI crate's sources (`src/`).
pub fn sources() -> Vec<(String, String)> {
    sources_of("src")
}

/// Lines of `text` that are code (not `//` comments).
pub fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
}

/// Every code line of `files` that holds one of `patterns`, as
/// `path:line: text`, for the files `skip` does not exempt.
pub fn scan(
    files: &[(String, String)],
    patterns: &[&str],
    skip: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        if skip(name) {
            continue;
        }
        for (index, line) in code_lines(text) {
            for pattern in patterns {
                if line.contains(pattern) {
                    found.push(format!("{name}:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }
    found
}
