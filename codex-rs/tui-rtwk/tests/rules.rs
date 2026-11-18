//! ADR-003's rules that a lint cannot hold alone, as source checks (the CI
//! greps of architecture.md §4): the allowlist of `#[allow]`s, the spawn
//! paths, the clock and the file system, the transcript's API, local state.

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

/// The crate's sources with their text, `src/shell*` apart from the rest.
fn sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
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

/// The shell and its backends: the only files that spawn, read the clock or
/// touch the file system.
fn is_shell(name: &str) -> bool {
    name == "src/shell.rs" || name.starts_with("src/shell/")
}

/// Lines of `text` that are code (not `//` comments).
fn code_lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
}

fn violations(patterns: &[&str], allow: impl Fn(&str) -> bool) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in sources() {
        if allow(&name) {
            continue;
        }
        for (index, line) in code_lines(&text) {
            for pattern in patterns {
                if line.contains(pattern) {
                    found.push(format!("{name}:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }
    found
}

#[test]
fn r1_only_the_shell_allows_disallowed_methods() {
    let allows = violations(
        &["allow(clippy::disallowed", "expect(clippy::disallowed"],
        |name| name == "src/shell.rs",
    );
    assert!(allows.is_empty(), "R1: {allows:#?}");
    let shell = std::fs::read_to_string(crate_dir().join("src/shell.rs")).expect("shell.rs");
    assert!(shell.contains("#![allow(clippy::disallowed_methods"));
}

#[test]
fn r1_spawn_paths_are_the_shells() {
    let found = violations(
        &[
            "thread::spawn",
            "tokio::spawn",
            "spawn_tokio",
            ".subscribe(",
            "subscribe_with",
            ".follow(",
            "cx.spawn(",
            ".channel(",
            "block_on(",
        ],
        is_shell,
    );
    assert!(found.is_empty(), "R1: {found:#?}");
}

/// Architecture §A6: `unchanged()` is banned in feature code. The one use is
/// the ingress router's drop of an older connection's event, in `src/app.rs`
/// (the root, not a feature); a second use there fails this test too.
#[test]
fn a6_unchanged_is_banned_in_feature_code() {
    let found = violations(&[".unchanged("], |name| name == "src/app.rs");
    assert!(found.is_empty(), "A6: {found:#?}");
    let app = std::fs::read_to_string(crate_dir().join("src/app.rs")).expect("app.rs");
    let uses = code_lines(&app)
        .filter(|(_, line)| line.contains(".unchanged("))
        .count();
    assert_eq!(uses, 1, "A6: only the epoch drop in `ingest`");
}

/// ADR-002 binding condition 3 (G15): the Navigator carries a `GlobalKey`
/// kept in the app, so a change of the provider list cannot drop its stack.
#[test]
fn adr002_the_navigator_is_keyed() {
    let view = std::fs::read_to_string(crate_dir().join("src/ui/view.rs")).expect("view.rs");
    assert!(
        code_lines(&view).any(|(_, line)| line.contains("with_key(app.nav_key.clone())")),
        "the Navigator needs `.with_key(app.nav_key.clone())`"
    );
    let app = std::fs::read_to_string(crate_dir().join("src/app.rs")).expect("app.rs");
    assert!(app.contains("pub nav_key: GlobalKey"));
}

/// The host runs a future the shell hands to rtwk; it spawns nothing and
/// allows no lint (R1).
#[test]
fn r1_the_host_neither_spawns_nor_allows() {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("host/src"), &mut files);
    assert!(!files.is_empty());
    for path in files {
        let text = std::fs::read_to_string(&path).expect("read");
        for (index, line) in code_lines(&text) {
            for pattern in ["spawn(", "#![allow", "#[allow", "block_in_place"] {
                assert!(
                    !line.contains(pattern),
                    "R1: {}:{}: {line}",
                    path.display(),
                    index + 1
                );
            }
        }
    }
}

#[test]
fn r11_r12_no_clock_and_no_file_system_outside_the_shell() {
    let found = violations(
        &[
            "Instant::now",
            "SystemTime::now",
            "std::fs::",
            "File::open",
            "tokio::fs",
        ],
        is_shell,
    );
    assert!(found.is_empty(), "R11/R12: {found:#?}");
}

#[test]
fn r4_the_transcript_has_no_push() {
    let threads = std::fs::read_to_string(crate_dir().join("src/threads/mod.rs")).expect("threads");
    let transcript = threads
        .split("impl Transcript")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("impl Transcript");
    assert!(!transcript.contains("pub fn push"), "R4: upsert only");
    assert!(transcript.contains("pub fn upsert"));
}

#[test]
fn r13_ids_are_newtypes_not_strings() {
    let found = violations(
        &[
            "thread_id: String",
            "turn_id: String",
            "item_id: String",
            "request_id: String",
        ],
        |name| name == "src/shell/fake.rs" || name == "src/shell.rs",
    );
    assert!(found.is_empty(), "R13: {found:#?}");
}

#[test]
fn r14_no_app_data_in_local_state() {
    let found = violations(&["use_state", "Obs<", "rtwk::Obs", "use_ref"], |_| false);
    assert!(found.is_empty(), "R14: {found:#?}");
}

#[test]
fn r14_widget_state_fields_hold_no_model_types() {
    // `State` structs under src/ui and src/screens: no model type in a field.
    let model_types = [
        "ThreadModel",
        "Transcript",
        "CodexApp",
        "RequestQueue",
        "SessionSlice",
    ];
    let mut found = Vec::new();
    for (name, text) in sources() {
        if !(name.starts_with("src/ui/") || name.starts_with("src/screens/")) {
            continue;
        }
        let mut in_state = false;
        for (index, line) in code_lines(&text) {
            let trimmed = line.trim();
            if trimmed.starts_with("pub struct ")
                && trimmed.contains("State")
                && trimmed.ends_with('{')
            {
                in_state = true;
                continue;
            }
            if in_state && trimmed == "}" {
                in_state = false;
            }
            if in_state && model_types.iter().any(|ty| trimmed.contains(ty)) {
                found.push(format!("{name}:{}: {trimmed}", index + 1));
            }
        }
    }
    assert!(found.is_empty(), "R14: {found:#?}");
}

#[test]
fn the_lint_lists_name_the_rules_banned_paths() {
    let clippy = std::fs::read_to_string(crate_dir().join("clippy.toml")).expect("clippy.toml");
    for path in [
        "std::thread::spawn",
        "tokio::spawn",
        "rtwk::app::AppCx::spawn",
        "rtwk::app::AppCx::channel",
        "rtwk_tokio::TokioApp::spawn_tokio",
        "rtwk_tokio::TokioApp::subscribe",
        "std::time::Instant::now",
        "std::time::SystemTime::now",
        "rtwk::hooks::use_state",
        "rtwk::Obs",
    ] {
        assert!(clippy.contains(path), "clippy.toml bans `{path}`");
    }
}
