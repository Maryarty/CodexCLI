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

/// The sources of both crates (`src/`, `core/src/`) with their text,
/// `src/shell*` apart from the rest.
fn sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    rust_files(&crate_dir().join("src"), &mut files);
    rust_files(&crate_dir().join("core/src"), &mut files);
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
        code_lines(&view).any(|(_, line)| line.contains("with_key(app.ui.nav_key.clone())")),
        "the Navigator needs `.with_key(app.ui.nav_key.clone())`"
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
    let threads =
        std::fs::read_to_string(crate_dir().join("core/src/threads.rs")).expect("threads");
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
    let found = violations(
        &["use_state(", "use_state::", "Obs<", "rtwk::Obs", "use_ref"],
        |_| false,
    );
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

/// The types core's screens export (their models, rows, menus, cards,
/// prompts): application data a widget `State` never holds a copy of (R14;
/// the value is read with `select` in `build`). The per-screen plumbing types
/// are no data.
fn screen_model_types() -> Vec<String> {
    let plumbing = [
        "Ev",
        "Intent",
        "Command",
        "Reply",
        "Ctx",
        "ThreadEv",
        "Kind",
        "SeedError",
    ];
    let mut types = Vec::new();
    for (name, text) in sources() {
        if !name.starts_with("core/src/screens/") {
            continue;
        }
        for (_, line) in code_lines(&text) {
            let declared = line
                .trim()
                .trim_start_matches("pub(crate) ")
                .trim_start_matches("pub ");
            for keyword in ["struct ", "enum "] {
                if let Some(rest) = declared.strip_prefix(keyword) {
                    let name = ident(rest);
                    if !plumbing.contains(&name.as_str()) {
                        types.push(name);
                    }
                }
            }
        }
    }
    types.sort();
    types.dedup();
    types
}

/// Whether `line` names one of `types` as a whole word.
fn names_type(line: &str, types: &[String]) -> bool {
    line.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|word| types.iter().any(|ty| ty == word))
}

#[test]
fn r14_widget_state_fields_hold_no_screen_model_types() {
    let types = screen_model_types();
    assert!(
        types.iter().any(|ty| ty == "Menu"),
        "the scan sees core's screens"
    );
    let mut found = Vec::new();
    for (name, text) in sources() {
        if !(name.starts_with("src/ui/") || name.starts_with("src/screens/")) {
            continue;
        }
        for state in state_struct_names(&text) {
            for (index, line) in struct_fields(&text, &state) {
                if names_type(line, &types) {
                    found.push(format!("{name}:{}: {line}", index + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "R14: {found:#?}");
}

/// The rule bites: the copied menu of the first S14 dialog (G5) is caught.
#[test]
fn r14_the_screen_scan_catches_a_copied_menu() {
    let text = "pub struct MenuDialogState {\n    menu: Option<Menu>,\n    shown: bool,\n}\n\
                impl State for MenuDialogState {}\n";
    let types = vec!["Menu".to_string()];
    let caught = struct_fields(text, "MenuDialogState")
        .into_iter()
        .filter(|(_, line)| names_type(line, &types))
        .count();
    assert_eq!(caught, 1);
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
        // The agents screen's list cursor (`SelectionController`) and its
        // search/rename line (`TextEditingController`) are view state of the
        // screen's widget, not app data: the model keeps the highlighted task
        // and the search text it filters by; so is the cursor of the
        // confirmation dialog's two rows (`agents/dialogs.rs`); so is the
        // cursor of the request dialog's option rows (`approvals/dialog.rs`).
        |name| {
            matches!(
                name,
                "src/ui/thread_ui.rs"
                    | "src/screens/agents/mod.rs"
                    | "src/screens/agents/dialogs.rs"
                    | "src/screens/approvals/dialog.rs"
            )
        },
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
/// the code. The theme file alone holds colour values: the per-thread accents
/// (`ThreadPalette`), which rtwk's `ThemeData` has no slot for.
#[test]
fn no_literal_colours_anywhere() {
    let found = violations(
        &["Color::", "Color::rgb", "rgb(", "Style::new().fg("],
        |name| name == "src/theme.rs",
    );
    assert!(found.is_empty(), "literal colours: {found:#?}");
}

/// R15: a view is built of rtwk's widgets (`Text` with wrap and overflow,
/// `Row`, `Column`, `Expanded`, `Divider`, `Table`, `show_dialog`), never of
/// hand text and width math: no width counting by `.chars().count()` or the
/// unicode crates, no blank or rule strings made by `.repeat(`, no function
/// whose name is wrap, ellipsis, truncate, pad or clip. An entry in
/// `R15_GAPS` is a documented rtwk gap; one in `R15_NOT_YET` is a screen
/// whose own rework task has not landed (it leaves the list when it does).
const R15_GAPS: &[(&str, &str)] = &[
    (
        "src/screens/agents/path.rs",
        "G37: no TextOverflow mode cuts a path by its segments (oracle center_truncate_path)",
    ),
    (
        "src/screens/agents/tabs.rs",
        "G36: TabBar has no gap, no outside marks, no whole-tab window (oracle render_filled_tab_bar)",
    ),
    (
        "src/screens/agents/soft.rs",
        "G38: no widget tells how many rows a Text takes; the facade does not export break_lines",
    ),
];
const R15_NOT_YET: &[&str] = &[
    "src/screens/chat/home.rs",
    "src/screens/dialogs/feedback.rs",
    "src/screens/pager/mod.rs",
    "src/screens/pets/mod.rs",
    "src/screens/resume/layout.rs",
    "src/screens/resume/mod.rs",
    "src/screens/select/dialog.rs",
    "src/screens/status/card.rs",
    "src/screens/status/format.rs",
    "src/screens/usage/mod.rs",
];
const R15_PATTERNS: &[&str] = &[
    ".chars().count()",
    "UnicodeWidth",
    "unicode_width",
    "TextSegmenter",
    "fit_text",
    ".repeat(",
];
const R15_FN_WORDS: &[&str] = &["wrap", "ellips", "truncat", "pad", "clip"];

fn r15_found(name: &str, text: &str) -> Vec<String> {
    let code = text.split("#[cfg(test)]").next().unwrap_or(text);
    let mut found = Vec::new();
    for (index, line) in code_lines(code) {
        let fn_name = line
            .split("fn ")
            .nth(1)
            .map(|rest| rest.split(['(', '<']).next().unwrap_or(""));
        let by_name = line.contains("fn ")
            && fn_name.is_some_and(|n| R15_FN_WORDS.iter().any(|word| n.contains(word)));
        if by_name || R15_PATTERNS.iter().any(|pattern| line.contains(pattern)) {
            found.push(format!("{name}:{}: {}", index + 1, line.trim()));
        }
    }
    found
}

#[test]
fn r15_views_have_no_hand_text_or_width_math() {
    let mut found = Vec::new();
    for (name, text) in sources() {
        let in_scope = name.starts_with("src/screens/") || name.starts_with("src/ui/");
        let allowed =
            R15_GAPS.iter().any(|(file, _)| *file == name) || R15_NOT_YET.contains(&name.as_str());
        if in_scope && !allowed {
            found.extend(r15_found(&name, &text));
        }
    }
    assert!(found.is_empty(), "R15 hand text/width math: {found:#?}");
}

#[test]
fn r15_the_not_yet_list_names_only_files_that_still_violate() {
    let sources = sources();
    for name in R15_NOT_YET {
        let text = sources
            .iter()
            .find(|(file, _)| file == name)
            .map(|(_, text)| text.as_str());
        assert!(
            text.is_some_and(|text| !r15_found(name, text).is_empty()),
            "{name} no longer violates R15 (or is gone): remove it from R15_NOT_YET"
        );
    }
}

#[test]
fn r15_the_gap_entries_are_small_and_documented() {
    let sources = sources();
    for (name, why) in R15_GAPS {
        assert!(why.starts_with('G'), "{name}: a gap entry names its Gxx");
        let text = sources.iter().find(|(file, _)| file == name);
        assert!(text.is_some(), "{name}: the gap entry's file is gone");
        let code = text.map_or(0, |(_, text)| {
            text.split("#[cfg(test)]")
                .next()
                .unwrap_or("")
                .lines()
                .filter(|line| {
                    let line = line.trim();
                    !line.is_empty() && !line.starts_with("//")
                })
                .count()
        });
        assert!(
            code <= 170,
            "{name}: {code} lines, a gap entry is one small piece"
        );
    }
}

/// ADR-010: no user-visible string literal in widget code. The strings are
/// in `src/l10n` and the per-directory `strings.rs`, which are the only
/// files that may hold them.
#[test]
fn adr010_no_user_visible_literal_outside_the_string_tables() {
    let found = violations(
        &[
            "Text::new(\"",
            "Text::new(format!(\"",
            "Text::new(&format!(\"",
            ".title(\"",
            ".placeholder(\"",
            ".label(\"",
            ".hint(\"",
            "Button::new(\"",
            "Button::new(format!(\"",
            "Span::new(\"",
            "Span::new(format!(\"",
            "Span::styled(\"",
            "Span::styled(format!(\"",
            "Text::styled(\"",
            "Text::styled(format!(\"",
            "Text::rich(\"",
            "Text::rich_line(\"",
            ".title(format!(\"",
            ".title(&format!(\"",
            ".placeholder(format!(\"",
            ".label(format!(\"",
            ".hint(format!(\"",
            ".tooltip(\"",
            "const TITLE",
            "const LABEL",
            "const HINT",
            "const PLACEHOLDER",
        ],
        in_the_tables,
    );
    assert!(found.is_empty(), "ADR-010: {found:#?}");
}

fn in_the_tables(name: &str) -> bool {
    name == "src/l10n.rs" || name.starts_with("src/l10n/") || name.ends_with("/strings.rs")
}

/// A `format!` whose literal holds words is user-visible text (or becomes
/// it): it belongs in a table as a `Msg`. The shell and the tables are
/// exempt; ids and column layouts (`"turn-{n}"`, `"{:<11} {}"`) hold no
/// word of three letters outside the braces.
#[test]
fn adr010_no_format_literal_with_words_outside_the_tables() {
    let mut found = Vec::new();
    for (name, text) in sources() {
        if in_the_tables(&name) || is_shell(&name) {
            continue;
        }
        for (index, line) in code_lines(&text) {
            let mut rest = line;
            while let Some(at) = rest.find("format!(\"") {
                rest = &rest[at + "format!(\"".len()..];
                let literal = rest.split('"').next().unwrap_or_default();
                let mut outside = String::new();
                let mut depth = 0;
                for c in literal.chars() {
                    match c {
                        '{' => depth += 1,
                        '}' if depth > 0 => depth -= 1,
                        _ if depth == 0 => outside.push(c),
                        _ => {}
                    }
                }
                let has_word = outside
                    .split(|c: char| !c.is_alphabetic())
                    .any(|word| word.chars().count() >= 3);
                if has_word {
                    found.push(format!("{name}:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "ADR-010: {found:#?}");
}

// The pending screen tests (a `pending(..)` marker fails the test by design)
// run in the nextest profile `pending`, out of the default one. The list in
// `.config/nextest.toml` and the markers in `tests/agents.rs` must agree, so
// that nothing hides in the list and nothing pending is left in the gate.

/// The test fns of `tests/agents.rs` whose first statement is `pending(`.
fn pending_markers() -> Vec<String> {
    let text = std::fs::read_to_string(crate_dir().join("tests/agents.rs")).expect("read");
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("fn ") else {
            continue;
        };
        if lines
            .get(i + 1)
            .is_some_and(|next| next.trim() == "pending(")
        {
            found.push(rest.split('(').next().expect("a name").to_owned());
        }
    }
    found.sort();
    found
}

/// The names the nextest config lists as pending (`test(=agents::<name>)`).
fn pending_listed() -> Vec<String> {
    let text = std::fs::read_to_string(crate_dir().join(".config/nextest.toml")).expect("read");
    let mut found: Vec<String> = text
        .split("test(=agents::")
        .skip(1)
        .map(|rest| rest.split(')').next().expect("a name").to_owned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    found.sort();
    found
}

#[test]
fn the_pending_profile_lists_exactly_the_pending_markers() {
    let markers = pending_markers();
    assert!(!markers.is_empty(), "the marker scan found nothing");
    assert_eq!(pending_listed(), markers);
}

#[test]
fn the_default_profile_leaves_out_the_pending_group_and_the_pending_profile_runs_it() {
    let text = std::fs::read_to_string(crate_dir().join(".config/nextest.toml")).expect("read");
    let section = |name: &str| -> String {
        let start = text.find(&format!("[profile.{name}]")).expect("a profile");
        let rest = &text[start + 1..];
        let end = rest.find("\n[").unwrap_or(rest.len());
        rest[..end].to_owned()
    };
    let default = section("default");
    let pending = section("pending");
    assert!(default.contains("binary(oracle)"), "{default}");
    assert!(
        default.contains("not (") && default.contains("agents::"),
        "{default}"
    );
    assert!(pending.contains("default-filter"), "{pending}");
    assert!(pending.contains("agents::"), "{pending}");
    assert!(pending.contains("fail-fast = false"), "{pending}");
}
