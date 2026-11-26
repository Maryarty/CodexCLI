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

/// The hand-written requests, counted per file in `tests/rules/wire_budget.txt` (`hand <file> <n>`,
/// the way `hub_budget.txt` counts): a slice that still writes `ClientRequest`s by hand has a
/// line, a count that rises is a new hand-made request, one that falls is written down, and a
/// slice that moved to `rpc_list!` is struck out. `framework.rs` is the framework's own test
/// probe, which is no slice.
fn hand_budget() -> std::collections::BTreeMap<String, usize> {
    let text = std::fs::read_to_string(crate_dir().join("tests/rules/wire_budget.txt"))
        .expect("read wire_budget.txt");
    text.lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            match words.as_slice() {
                ["hand", file, count] => Some((
                    (*file).to_owned(),
                    count.parse().expect("a count in wire_budget.txt"),
                )),
                _ => None,
            }
        })
        .collect()
}

/// What `counts` (file, hand-made requests in it) say against the budget: more is refused, fewer
/// is written down, and a file with none and a line is stale.
fn hand_report(
    counts: &[(String, usize)],
    budget: &std::collections::BTreeMap<String, usize>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (name, current) in counts {
        let allowed = budget.get(name).copied().unwrap_or(0);
        match current.cmp(&allowed) {
            std::cmp::Ordering::Greater => out.push(format!(
                "{name}: {current} requests made by hand, {allowed} allowed; write the RPC as a line of `rpc_list!`"
            )),
            std::cmp::Ordering::Less => out.push(format!(
                "{name}: {current} requests made by hand, {allowed} allowed; lower its line in tests/rules/wire_budget.txt to {current} (delete it at 0)"
            )),
            std::cmp::Ordering::Equal => {}
        }
    }
    for name in budget.keys() {
        if !counts.iter().any(|(file, _)| file == name) {
            out.push(format!("{name}: in wire_budget.txt but no such file"));
        }
    }
    out
}

/// The slices whose RPCs are generated.
const GENERATED: &[&str] = &[
    "goal",
    "mcp_inventory",
    "plugins",
    "rename",
    "settings",
    "small_cmds",
    "thread_usage",
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
    let counts: Vec<(String, usize)> = files
        .iter()
        .map(|(name, text)| (name.clone(), hand_made(text).len()))
        .collect();
    let report = hand_report(&counts, &hand_budget());
    assert!(report.is_empty(), "{report:#?}");
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
fn the_thread_catalog_generates_its_requests() {
    let catalog = include_str!("../../core/src/threads/asks.rs");
    assert_eq!(lists(catalog).len(), 1, "one generated thread catalog");
    let lines: Vec<_> = catalog.lines().collect();
    for number in hand_made(catalog) {
        let line = lines[number - 1].trim();
        // The host projects already encoded turn input into Cloud admission.
        assert!(
            matches!(
                line,
                "wire::ClientRequest::TurnStart { params, .. } => Some(params),"
                    | "wire::ClientRequest::TurnSteer { params, .. } => Some(wire::TurnStartParams {"
            ),
            "thread catalog line {number} makes a request outside rpc_list!: {line}"
        );
    }
}

#[test]
fn the_list_finder_reads_braces_of_the_blocks_only() {
    let text = "a\nrpc_list! {\n  X => ClientRequest::A { v: json!({ \"k\": 1 }) }\n}\nClientRequest::B {}\n";
    assert_eq!(hand_made(text), vec![5]);
    assert_eq!(lists(text).len(), 1);
}

#[test]
fn the_hand_written_budget_bites() {
    let budget = std::collections::BTreeMap::from([("a.rs".to_owned(), 2usize)]);
    let counts = |a: usize, b: usize| [("a.rs".to_owned(), a), ("b.rs".to_owned(), b)];
    assert!(hand_report(&counts(2, 0), &budget).is_empty());
    // one more in a budgeted file, and a first one in a file with none
    assert_eq!(hand_report(&counts(3, 0), &budget).len(), 1);
    assert_eq!(hand_report(&counts(2, 1), &budget).len(), 1);
    // one fewer must be written down
    let fell = hand_report(&counts(1, 0), &budget);
    assert_eq!(fell.len(), 1);
    assert!(fell[0].contains("lower its line"), "{fell:?}");
    // a line for a file that is gone
    assert_eq!(hand_report(&[("b.rs".to_owned(), 0)], &budget).len(), 1);
}
