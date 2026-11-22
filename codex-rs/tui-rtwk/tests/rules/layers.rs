//! The layers of the port (architecture.md "Layers"): the core crate
//! `codex-tui-core` holds the model, `apply`, tickets and the protocol
//! mapping and has no rtwk; the screen directories of the UI crate hold view
//! files only; `shell` runs effects and names no widget. The compiler holds
//! the first (core's dependencies); these tests hold the rest, and the first
//! as well, since a Cargo.toml edit is the one way round it.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

#[path = "../srcscan/mod.rs"]
mod srcscan;

use srcscan::{code_lines, crate_dir, scan, sources, sources_of};

fn read(rel: &str) -> String {
    std::fs::read_to_string(crate_dir().join(rel)).unwrap_or_else(|_| panic!("read {rel}"))
}

/// The UI crate's view files: everything under `src/` except services/, the
/// model root (`app/mod.rs`), the composition root (`app/run.rs`) and `lib.rs`. An
/// allowlist of the non-views, so a new directory is a view until it is named
/// here.
fn is_view(name: &str) -> bool {
    name.starts_with("src/")
        && !is_shell(name)
        && !matches!(name, "src/app/mod.rs" | "src/app/run.rs" | "src/lib.rs")
}

fn in_pages_sheets_widgets(name: &str) -> bool {
    ["src/pages/", "src/sheets/", "src/widgets/"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn is_shell(name: &str) -> bool {
    name.starts_with("src/services/")
}

/// The direct dependencies core may have, by name: the protocol (which pulls
/// tokio and others in transitively; core's sources name none, see
/// `core_sources_name_no_ui_runtime_or_io`), serde, and the test helpers.
/// Anything else is a decision, made here.
const CORE_DEPS: &[&str] = &[
    "codex-app-server-protocol",
    // The person (2026-11-18): core may take the protocol crates the carried
    // oracle code names (ThreadId, SubAgentSource); still no rtwk, no IO.
    "codex-protocol",
    "serde",
    "serde_json",
    "pretty_assertions",
];

#[test]
fn core_manifest_has_no_ui_or_runtime_dependency() {
    let manifest = read("core/Cargo.toml");
    assert!(manifest.contains("name = \"codex-tui-core\""));
    let mut found = Vec::new();
    let mut in_deps = false;
    for (index, line) in manifest.lines().enumerate() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line.contains("dependencies");
            continue;
        }
        let key = line.split('=').next().unwrap_or_default().trim();
        if in_deps && line.contains('=') && !CORE_DEPS.contains(&key) {
            found.push(format!("core/Cargo.toml:{}: {line}", index + 1));
        }
    }
    assert!(
        found.is_empty(),
        "core has a direct dependency outside the allowlist (CORE_DEPS): {found:#?}"
    );
    // Core is a member of the workspace and the UI crate depends on it.
    let root = read("Cargo.toml");
    assert!(root.contains("members = [\"host\", \"core\"]"));
    assert!(root.contains("codex-tui-core = { path = \"core\" }"));
}

#[test]
fn core_sources_name_no_ui_runtime_or_io() {
    let files = sources_of("core/src");
    assert!(!files.is_empty(), "core/src has no sources");
    let found = scan(
        &files,
        &[
            "rtwk",
            "tokio",
            "futures::",
            "std::{",
            "std::io",
            "std::fs",
            "std::net",
            "std::process",
            "std::thread",
            "std::env",
            "Instant::now",
            "SystemTime::now",
            "File::open",
            "block_on(",
        ],
        |_| false,
    );
    assert!(
        found.is_empty(),
        "core names UI, runtime, IO or clock: {found:#?}"
    );
}

/// Failures and messages are typed values in core; the words are the UI's
/// string tables'. A `format!` literal with words in core is prose.
#[test]
fn core_sources_hold_no_user_visible_prose() {
    let mut found = Vec::new();
    for (name, text) in sources_of("core/src") {
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
                if outside
                    .split(|c: char| !c.is_alphabetic())
                    .any(|word| word.chars().count() >= 3)
                {
                    found.push(format!("{name}:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }
    assert!(found.is_empty(), "prose in core: {found:#?}");
}

#[test]
fn views_never_name_protocol_shell_tokio_or_io() {
    let found = scan(
        &sources(),
        &[
            "codex_app_server_protocol",
            "codex_tui_core::protocol",
            "crate::services::shell",
            "shell::",
            "tokio",
            "fixtures::script",
            "std::{",
            "std::io",
            "stdout(",
            "stderr(",
            "stdin(",
            "std::fs",
            "std::net",
            "std::process",
            "std::thread",
            "std::env",
        ],
        |name| !is_view(name),
    );
    assert!(
        found.is_empty(),
        "a view names protocol, shell, tokio or IO: {found:#?}"
    );
}

/// A view emits typed intents; the model changes in core's `apply` only.
#[test]
fn views_never_apply_the_model() {
    let found = scan(
        &sources(),
        &[
            "apply_app(",
            "apply_intent(",
            "apply_ev(",
            "apply_thread(",
            "apply_reply(",
            "threads::apply(",
            "ingest(",
            "Ingress",
        ],
        |name| !is_view(name),
    );
    assert!(found.is_empty(), "a view calls apply: {found:#?}");
}

/// The folders of `pages/`, `sheets/` and `widgets/` hold widgets and strings;
/// a feature's model, events, intents, RPCs and protocol wiring are core's
/// (`core/src/screens/`, `core/src/protocol/`).
#[test]
fn screen_dirs_hold_view_files_only() {
    let found = scan(
        &sources(),
        &[
            "pub struct Model",
            "pub struct ThreadPart",
            "pub enum Ev ",
            "pub enum Intent ",
            "pub enum ThreadEv ",
            "pub enum Command ",
            "pub enum Reply ",
            "pub enum Ctx ",
            "pub fn apply(",
            "pub fn apply_intent(",
            "pub fn apply_thread(",
            "pub fn apply_reply(",
            "pub fn wire(",
            "pub fn decode(",
        ],
        |name| !in_pages_sheets_widgets(name),
    );
    assert!(found.is_empty(), "model code in a view dir: {found:#?}");
}

/// The UI crate's top-level folders are Flutter's: `app/` (root, theme,
/// debug), `pages/` (the `Navigator`'s routes), `sheets/` (dialog and
/// bottom-sheet bodies), `widgets/` (shared), `services/` (IO, no widget),
/// `l10n/`, `keymap/`. A core feature module need not mirror a UI folder
/// (a feature may feed a page, a sheet or a cell).
#[test]
fn ui_crate_has_only_the_known_folders() {
    let known = [
        "app", "keymap", "l10n", "pages", "services", "sheets", "widgets",
    ];
    let mut found = Vec::new();
    for (name, _) in sources() {
        let Some(rest) = name.strip_prefix("src/") else {
            continue;
        };
        let top = rest.split('/').next().unwrap_or_default();
        if rest != "lib.rs" && !known.contains(&top) {
            found.push(name);
        }
    }
    assert!(
        found.is_empty(),
        "a file outside the known folders: {found:#?}"
    );
}

/// `shell` runs effects and names no widget, screen or view. Dialogs are not
/// its business any more: rtwk's `AppCx::show_dialog_from` shows the request
/// dialog from `ui::presenter` and delivers `closed` to the model
/// (rtwk-repin), so `show_dialog`, `close_dialog` and `DialogOptions` are on
/// this list.
#[test]
fn shell_names_no_widget() {
    let found = scan(
        &sources(),
        &[
            "AnyWidget",
            "StatefulWidget",
            "StatelessWidget",
            "impl_widget",
            "rtwk::widgets",
            "rtwk::layout",
            "rtwk::controls",
            "rtwk::navigation",
            "rtwk::overlay::{",
            "rtwk::focus",
            "rtwk::editing",
            "rtwk::scroll",
            "rtwk::provider",
            "rtwk::prelude",
            "Dialog::",
            "DialogOptions",
            "show_dialog",
            "close_dialog",
            "crate::ui",
            "crate::screens",
            "crate::app::theme",
        ],
        |name| !is_shell(name),
    );
    assert!(found.is_empty(), "shell names a widget or view: {found:#?}");
}

/// The UI crate's model is core's plus UI-only state; core's model has none
/// of the UI's fields.
#[test]
fn the_ui_model_is_the_core_model_plus_ui_state() {
    let app = read("src/app/mod.rs");
    for needle in [
        "pub struct UiApp",
        "pub core: CodexApp",
        "pub ui: UiState",
        "pub struct UiState",
        "pub nav_key: GlobalKey",
        "pub thread_ui: ThreadUiMap",
        "pub lang: Lang",
        "pub debug: bool",
    ] {
        assert!(app.contains(needle), "src/app/mod.rs lacks `{needle}`");
    }
    let core = read("core/src/app.rs");
    for needle in ["nav_key", "thread_ui", "GlobalKey", "Lang", "debug_screens"] {
        assert!(!core.contains(needle), "core/src/app.rs holds `{needle}`");
    }
}

/// UI-only state belongs to the view (widget state, controllers) or `UiApp.ui`
/// (process.md §2, A4.4), never to a screen's model in core. A denylist of the
/// names such state goes by; the query a search runs and the persisted
/// grouping are domain and are not on it.
#[test]
fn core_screen_models_hold_no_ui_state_fields() {
    const DENIED: &[&str] = &[
        "help",
        "cursor",
        "selected",
        "scroll",
        "focus",
        "confirm",
        "searching",
        "draft",
    ];
    let mut found = Vec::new();
    for (name, text) in sources_of("core/src/screens") {
        let mut in_model = false;
        for (index, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.starts_with("pub struct Model {") && !line.ends_with("{}") {
                in_model = true;
            } else if in_model && line == "}" {
                in_model = false;
            } else if in_model
                && !line.starts_with("//")
                && let Some(field) = line.strip_prefix("pub ").and_then(|l| l.split(':').next())
                && DENIED.contains(&field)
            {
                found.push(format!("{name}:{}: {field}", index + 1));
            }
        }
    }
    assert!(
        found.is_empty(),
        "UI-only state in a core model: {found:#?}"
    );
}

/// The view reads what core projects (`RowData`, `Usage` methods), never the
/// protocol type behind it (VS1 review L2): a field access through a core
/// re-export passes the `codex_app_server_protocol` scan.
#[test]
fn views_read_projections_not_core_held_protocol_values() {
    let found = scan(
        &[sources_of("src/pages"), sources_of("src/sheets")].concat(),
        &["row.thread", ".estimate", "RowData { thread"],
        |_| false,
    );
    assert!(
        found.is_empty(),
        "a view reads a protocol value: {found:#?}"
    );
}

// ADR-003: one `ThreadModel` per thread, made in one place.

/// `ThreadModel` is made only by `ThreadSet::open` (core/src/threads.rs): its
/// constructor is private and a private field closes the struct literal. This
/// holds both: no file outside threads.rs names `ThreadModel::new`/`blank` or
/// a `ThreadModel {` literal, and the type still carries its seal.
#[test]
fn thread_model_is_made_only_in_threads_rs() {
    let threads = read("core/src/threads.rs");
    assert!(
        threads.contains("    fn new(id: ThreadId) -> Self {"),
        "ThreadModel::new must stay private"
    );
    assert!(
        threads.contains("    seal: Seal,"),
        "ThreadModel must keep its private seal field"
    );
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host"));
    let found = scan(
        &files,
        &["ThreadModel::new", "ThreadModel::blank", "ThreadModel {"],
        |name| name == "core/src/threads.rs",
    );
    assert!(
        found.is_empty(),
        "ThreadModel made outside threads.rs: {found:#?}"
    );
}

/// The thread map is private to `CodexApp`: a screen or the UI model cannot
/// insert a model into it.
#[test]
fn codex_app_holds_its_threads_privately() {
    let app = read("core/src/app.rs");
    assert!(
        app.contains("\n    threads: ThreadSet,"),
        "CodexApp.threads must be a private ThreadSet"
    );
    for (file, name, source) in [
        ("app.rs", "CodexApp", app),
        ("threads.rs", "ThreadModel", read("core/src/threads.rs")),
    ] {
        let (_, body) = source
            .split_once(&format!("pub struct {name} {{"))
            .expect(name);
        let (fields, _) = body.split_once("\n}").expect("end of model");
        assert!(
            code_lines(fields).all(|(_, field)| !field.trim_start().starts_with("pub")),
            "{file}: {name} must expose read-only methods instead of public fields",
        );
    }
}

/// A thread RPC is asked for as data (`threads::ThreadAsk`) and made into a
/// wire request in core/src/threads.rs only; the two thread commands the app
/// itself mints (`StartThread`, `StartTurn`) are mapped in protocol/mod.rs.
/// No screen, view or shell file builds `ClientRequest::Thread*`/`Turn*`.
#[test]
fn thread_rpcs_only_in_threads() {
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host"));
    let found = scan(
        &files,
        &[
            "ClientRequest::ThreadResume",
            "ClientRequest::ThreadFork",
            "ClientRequest::ThreadCompactStart",
            "ClientRequest::ThreadSetName",
            "ClientRequest::ThreadShellCommand",
            "ClientRequest::ThreadStart",
            "ClientRequest::ThreadRead",
            "ClientRequest::ThreadTurnsList",
            "ClientRequest::ThreadUnsubscribe",
            "ClientRequest::ThreadRevert",
            "ClientRequest::TurnStart",
            "ClientRequest::TurnInterrupt",
            "ClientRequest::TurnSteer",
            "ThreadResumeParams",
            "ThreadForkParams",
            "ThreadCompactStartParams",
            "ThreadSetNameParams",
            "ThreadShellCommandParams",
            "ThreadStartParams",
            "ThreadUnsubscribeParams",
            "ThreadRevertParams",
            "TurnInterruptParams",
            "TurnStartParams",
            "TurnSteerParams",
        ],
        |name| name == "core/src/threads.rs" || name == "core/src/protocol/mod.rs",
    );
    assert!(
        found.is_empty(),
        "thread RPC built outside threads.rs: {found:#?}"
    );
}

/// Which thread the chat shows is decided in `apply_app` (core/src/app.rs)
/// alone: no screen, view or shell file assigns `session.active` or makes the
/// session slice mutable, so "switch or resume once" has one place.
#[test]
fn the_shown_thread_is_set_only_in_apply_app() {
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host"));
    let found = scan(
        &files,
        &[
            "active = Some(",
            ".active.replace(",
            ".active.take(",
            "make_mut(&mut app.session)",
            "make_mut(&mut self.session)",
        ],
        |name| name == "core/src/app.rs",
    );
    assert!(
        found.is_empty(),
        "the shown thread set outside app.rs: {found:#?}"
    );
}

#[test]
fn agents_read_the_apps_thread_state_and_ask_it_to_open_threads() {
    let agents = read("core/src/screens/agents/mod.rs");
    let (_, model) = agents
        .split_once("pub struct Model {")
        .expect("agents Model");
    let (fields, _) = model.split_once("\n}").expect("end of agents Model");
    for field in [
        "current",
        "turning",
        "requests",
        "activity",
        "last_messages",
        "pinned",
    ] {
        assert!(
            !code_lines(fields).any(|(_, line)| line.contains(&format!("{field}:"))),
            "agents has a second copy of {field}"
        );
    }
    let mut files = sources_of("core/src/screens");
    files.extend(sources_of("src"));
    files.extend(sources_of("host"));
    let found = scan(
        &files,
        &[
            "ThreadAsk::Resume",
            "ThreadAsk::Unsubscribe",
            "ThreadSet::",
            "&mut ThreadSet",
        ],
        |_| false,
    );
    assert!(
        found.is_empty(),
        "thread opened or released outside the app: {found:#?}"
    );
}

/// The shared widgets sit below the pages (architecture.md: widgets are
/// shared by more than one page): no file under `src/widgets` names a page.
/// The composer's draft was the one upward import (`thread_ui` named
/// `pages::session::composer::draft`); its type lives in
/// `widgets/composer_draft.rs` now.
#[test]
fn widgets_never_import_pages() {
    let files = sources_of("src/widgets");
    assert!(!files.is_empty(), "src/widgets has no sources");
    let found = scan(
        &files,
        &[
            "crate::pages",
            "super::pages",
            "pages::session",
            "pages::agents",
        ],
        |_| false,
    );
    assert!(found.is_empty(), "a widget imports a page: {found:#?}");
}

/// The scan sees an upward import of any spelling.
#[test]
fn the_widgets_scan_sees_an_upward_import() {
    let files = vec![(
        "src/widgets/x.rs".to_string(),
        "use crate::pages::session::composer::draft::ComposerDraft;\n".to_string(),
    )];
    let found = scan(&files, &["crate::pages", "super::pages"], |_| false);
    assert_eq!(found.len(), 1, "{found:?}");
}
