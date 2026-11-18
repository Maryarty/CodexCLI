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
            "tokio::time::Instant",
            "chrono::Utc",
            "chrono::Local",
            "Command::new",
            "TcpStream",
            "TcpListener",
            "UdpSocket",
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

/// The names of the structs that are widget state in `text`: those named
/// `…State` (any visibility, braces or tuple form) and those given to
/// `impl State for X` or `impl rtwk::State for X`.
fn state_struct_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.split("impl State for ").nth(1)
            && trimmed.starts_with("impl State for ")
        {
            names.push(ident(rest));
        }
        if let Some(rest) = trimmed.split("impl rtwk::State for ").nth(1) {
            names.push(ident(rest));
        }
        let declared = trimmed
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub(super) ")
            .trim_start_matches("pub ");
        if let Some(rest) = declared.strip_prefix("struct ") {
            let name = ident(rest);
            if name.ends_with("State") {
                names.push(name);
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn ident(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// `(line number, line)` of the fields of struct `name` in `text`.
fn struct_fields<'a>(text: &'a str, name: &str) -> Vec<(usize, &'a str)> {
    let mut fields = Vec::new();
    let mut inside = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let declared = trimmed
            .trim_start_matches("pub(crate) ")
            .trim_start_matches("pub(super) ")
            .trim_start_matches("pub ");
        if let Some(rest) = declared.strip_prefix("struct ")
            && ident(rest) == name
        {
            fields.push((index, trimmed)); // tuple form: the line has its fields
            inside = trimmed.ends_with('{');
            continue;
        }
        if inside {
            if trimmed.starts_with('}') {
                inside = false;
            } else {
                fields.push((index, trimmed));
            }
        }
    }
    fields
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
        "ThreadSlots",
        "Pending<",
    ];
    let mut found = Vec::new();
    let mut checked = 0;
    for (name, text) in sources() {
        if !(name.starts_with("src/ui/") || name.starts_with("src/screens/")) {
            continue;
        }
        for state in state_struct_names(&text) {
            for (index, line) in struct_fields(&text, &state) {
                checked += 1;
                if model_types.iter().any(|ty| line.contains(ty)) {
                    found.push(format!("{name}:{}: {line}", index + 1));
                }
            }
        }
    }
    assert!(checked > 0, "the scan found no State struct at all");
    assert!(found.is_empty(), "R14: {found:#?}");
}

#[test]
fn r14_the_state_scan_sees_every_declaration_form() {
    let text = "pub struct AState {\n    a: u8,\n}\nstruct BState(u8);\n\
                pub(crate) struct CState { c: u8 }\nstruct Hidden {\n    d: u8,\n}\n\
                impl State for Hidden {}\nstruct Plain;\n";
    assert_eq!(
        state_struct_names(text),
        ["AState", "BState", "CState", "Hidden"]
    );
    let fields = struct_fields("struct Hidden {\n    d: ThreadModel,\n}\n", "Hidden");
    assert!(fields.iter().any(|(_, line)| line.contains("ThreadModel")));
}

/// R14: a controller (rtwk's local UI state) is made only in the UI-only
/// allowlist; `new`, `default` and the other constructors are caught here,
/// `new` is also in `clippy.toml`.
#[test]
fn r14_controllers_are_made_only_in_the_allowlist() {
    let found = violations(
        &[
            "Controller::new(",
            "Controller::default(",
            "Controller::with_",
            "Controller::from",
            "Controller::<",
        ],
        |name| name == "src/ui/thread_ui.rs",
    );
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
        "std::process::Command::new",
        "tokio::process::Command::new",
        "std::net::TcpStream::connect",
        "std::net::TcpListener::bind",
        "std::net::UdpSocket::bind",
        "tokio::net::TcpStream::connect",
        "tokio::fs::read_to_string",
        "tokio::fs::write",
        "std::fs::File::open",
        "chrono::Utc::now",
        "chrono::Local::now",
        "tokio::time::Instant::now",
        "rtwk::editing::TextEditingController::new",
        "rtwk::scroll::ScrollController::new",
    ] {
        assert!(clippy.contains(path), "clippy.toml bans `{path}`");
    }
}

/// Item 11: colours come from the theme (`ThemeData` at the root), not from
/// the code. Only `src/theme.rs` may name a style, and it names no colour.
#[test]
fn no_literal_colours_anywhere() {
    let found = violations(&["Color::", "Color::rgb", "rgb(", "Style::new().fg("], |_| false);
    assert!(found.is_empty(), "literal colours: {found:#?}");
}
