//! ADR-012, "the wire is generated" (arch-pillars, `rpc_list!`): a slice's RPCs are one
//! `rpc_list!` (core/src/protocol/rpc_list.rs), one line per RPC; no `wire.rs` and no hand-made
//! `ClientRequest` is written in `core/src/features/`.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read_dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The files of `core/src/features/`, by path from the crate and their text.
fn feature_files() -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("core/src/features"), &mut files);
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

/// The byte ranges of the `rpc_list! { ... }` invocations of `text`.
fn lists(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = text[from..].find("rpc_list! {") {
        let start = from + at;
        let open = start + "rpc_list! ".len();
        let mut depth = 0usize;
        let mut end = text.len();
        for (offset, ch) in text[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + offset;
                        break;
                    }
                }
                _ => {}
            }
        }
        found.push(start..end);
        from = end;
    }
    found
}

/// The lines where a `ClientRequest::Method` is made outside an `rpc_list!`, comments left out.
fn hand_made(text: &str) -> Vec<usize> {
    let blocks = lists(text);
    let mut offset = 0;
    let mut lines = Vec::new();
    for (number, line) in text.lines().enumerate() {
        let at = offset;
        offset += line.len() + 1;
        if line.trim_start().starts_with("//") || !line.contains("ClientRequest::") {
            continue;
        }
        if !blocks.iter().any(|block| block.contains(&at)) {
            lines.push(number + 1);
        }
    }
    lines
}

/// The slices that still write their requests by hand (a ratchet: the line only shrinks; a slice
/// that moved to `rpc_list!` is struck out, and a stale entry fails). `framework.rs` is the
/// framework's own test probe, which is no slice.
const HAND_WRITTEN: &[&str] = &[
    "core/src/features/feedback/mod.rs",
    "core/src/features/framework.rs",
    "core/src/features/side/mod.rs",
    "core/src/features/windows_sandbox/mod.rs",
];

/// The slices whose RPCs are generated.
const GENERATED: &[&str] = &[
    "goal",
    "mcp_inventory",
    "plugins",
    "rename",
    "settings",
    "small_cmds",
];

#[test]
fn wire_is_generated() {
    let files = feature_files();
    let walls: Vec<_> = files
        .iter()
        .filter(|(name, _)| name.ends_with("/wire.rs"))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        walls.is_empty(),
        "a slice's RPCs are one `rpc_list!`, not a `wire.rs`: {walls:?}"
    );
    for (name, text) in &files {
        let lines = hand_made(text);
        let allowed = HAND_WRITTEN.contains(&name.as_str());
        assert!(
            lines.is_empty() || allowed,
            "{name}:{lines:?}: a `ClientRequest` is made by hand; write the RPC as a line of `rpc_list!`"
        );
        assert!(
            !lines.is_empty() || !allowed,
            "{name} no longer makes a request by hand: strike it out of HAND_WRITTEN"
        );
    }
    for slice in GENERATED {
        assert!(
            files.iter().any(|(name, text)| name
                .starts_with(&format!("core/src/features/{slice}/"))
                && !lists(text).is_empty()),
            "core/src/features/{slice}/ has no `rpc_list!`"
        );
    }
}

#[test]
fn the_list_finder_reads_braces_of_the_blocks_only() {
    let text = "a\nrpc_list! {\n  X => ClientRequest::A { v: json!({ \"k\": 1 }) }\n}\nClientRequest::B {}\n";
    assert_eq!(hand_made(text), vec![5]);
    assert_eq!(lists(text).len(), 1);
}
