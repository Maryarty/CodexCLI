//! The layers of the port (architecture.md "Layers"): the core crate
//! `codex-tui-core` holds the model, `apply`, tickets and the protocol
//! mapping and has no rtwk; the screen directories of the UI crate hold view
//! files only; `shell` runs effects and names no widget. The compiler holds
//! the first (core's dependencies); these tests hold the rest, and the first
//! as well, since a Cargo.toml edit is the one way round it.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

// shared with adr013.rs: a file is one module of the binary (clippy::duplicate_mod)
#[path = "../srcscan/mod.rs"]
pub(crate) mod srcscan;

use srcscan::{code_lines, crate_dir, scan, sources, sources_of};

fn read(rel: &str) -> String {
    std::fs::read_to_string(crate_dir().join(rel)).unwrap_or_else(|_| panic!("read {rel}"))
}

// the lists that `build.rs` writes from the files: the rules read the same text (fw-hubs-c5);
// shared with ui_registry.rs and feature_sheet.rs (clippy::duplicate_mod)
#[path = "../../test-mods/registry.rs"]
pub(crate) mod registry;

/// `core/src/features/mod.rs` as the compiler sees it: the hub, then the `pub mod` lines and
/// `feature_list!` that `core/build.rs` writes from the slices' directories.
fn features_mod() -> String {
    let dir = crate_dir().join("core/src/features");
    let slices = registry::core_features(&dir).unwrap_or_else(|e| panic!("core features: {e}"));
    registry::with_generated(
        &read("core/src/features/mod.rs"),
        &registry::core_features_text(&dir, &slices),
    )
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

/// The view folders that hold no model code: the page, sheet and widget folders and the view
/// half of the feature slices (ADR-012).
fn in_pages_sheets_widgets(name: &str) -> bool {
    ["src/pages/", "src/sheets/", "src/widgets/", "src/features/"]
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
    // f-elicitation-dialogs: the oracle validates an app link's URL with `url::Url`; pure, no IO.
    "url",
    // fw-hubs-c5: the module list of `tests/all.rs` (a dev-dependency without dependencies of its
    // own, `test-mods/`: std only).
    "tui-test-mods",
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
/// `l10n/`, `oracle_strings/` (generated literal tables of the oracle, W2 porters consume them), `keymap/`, `features/` (the view half of the core's feature slices, ADR-012:
/// the words and views of `core/src/features/<f>/`). A core feature module need not mirror
/// a UI folder (a feature may feed a page, a sheet or a cell).
#[test]
fn ui_crate_has_only_the_known_folders() {
    let known = [
        "app",
        "features",
        "keymap",
        "l10n",
        "oracle_strings",
        "pages",
        "services",
        "sheets",
        "widgets",
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

// `core_screen_models_hold_no_ui_state_fields` (a denylist over `pub struct Model` of the screens) is
// `adr013_domain_types_hold_no_draw_state` now (tests/rules/adr013.rs): the same words and more, over
// every type the app's, a thread's, a screen's and a feature's state reaches, with the families
// that still hold drawn state as a ratchet.

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

/// The thread RPCs of the thread's life: resume, fork, start, read, turns, subscribe, the
/// turn calls and the review start, which starts a turn and switches review mode (ADR-003).
/// Built in the thread catalog, whoever asks: a feature slice cannot make them either.
const LIFECYCLE_RPCS: &[&str] = &[
    "ClientRequest::ThreadResume",
    "ClientRequest::ThreadFork",
    "ClientRequest::ThreadStart",
    "ClientRequest::ThreadRead",
    "ClientRequest::ThreadTurnsList",
    "ClientRequest::ThreadUnsubscribe",
    "ClientRequest::TurnStart",
    "ClientRequest::TurnInterrupt",
    "ClientRequest::TurnSteer",
    "ClientRequest::ReviewStart",
    "ThreadResumeParams",
    "ThreadForkParams",
    "ThreadStartParams",
    "ThreadUnsubscribeParams",
    "TurnInterruptParams",
    "TurnStartParams",
    "TurnSteerParams",
    "ReviewStartParams",
];

/// Thread RPCs that are a feature's, not the lifecycle's (`/compact`, `/rename`, `!cmd`,
/// `/undo`): until their features are slices they are built in the thread catalog; a slice under
/// `core/src/features/<f>/` builds its own in `Feature::wire` (ADR-012), and its asks go out
/// only through `AppCx::ask` / `ThreadCx::request` (`feature_slices_ask_only_through_the_outbox`).
const FEATURE_RPCS: &[&str] = &[
    "ClientRequest::ThreadCompactStart",
    "ClientRequest::ThreadSetName",
    "ClientRequest::ThreadShellCommand",
    "ClientRequest::ThreadRevert",
    "ThreadCompactStartParams",
    "ThreadSetNameParams",
    "ThreadShellCommandParams",
    "ThreadRevertParams",
];

fn thread_rpcs_outside(files: &[(String, String)]) -> Vec<String> {
    let mut found = scan(files, LIFECYCLE_RPCS, |name| {
        name == "core/src/threads/asks.rs"
    });
    found.extend(scan(files, FEATURE_RPCS, |name| {
        name == "core/src/threads/asks.rs" || name.starts_with("core/src/features/")
    }));
    found
}

/// A thread RPC is asked for as data (`threads::ThreadAsk`) and made into a
/// wire request only in the thread catalog.
/// No screen, view or shell file builds `ClientRequest::Thread*`/`Turn*`. The lifecycle
/// RPCs stay here whoever asks; the feature RPCs may also be built inside a feature slice.
#[test]
fn thread_rpcs_only_in_threads() {
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host"));
    let found = thread_rpcs_outside(&files);
    assert!(
        found.is_empty(),
        "thread RPC built outside the thread catalog: {found:#?}"
    );
}

/// A feature slice cannot build a lifecycle RPC, but builds its own.
#[test]
fn the_thread_rpc_scan_lets_a_slice_build_its_own_and_not_the_lifecycle() {
    let slice = |text: &str| {
        vec![(
            "core/src/features/goal/wire.rs".to_string(),
            text.to_string(),
        )]
    };
    assert!(thread_rpcs_outside(&slice("ClientRequest::ThreadSetName {")).is_empty());
    assert_eq!(
        thread_rpcs_outside(&slice("ClientRequest::TurnStart {")).len(),
        1
    );
    assert_eq!(
        thread_rpcs_outside(&slice("let p = ThreadResumeParams {")).len(),
        1
    );
    // `review/start` starts a turn and switches review mode: a lifecycle RPC, not a slice's
    assert_eq!(
        thread_rpcs_outside(&slice("ClientRequest::ReviewStart {")).len(),
        1
    );
    assert_eq!(
        thread_rpcs_outside(&slice("let p = ReviewStartParams {")).len(),
        1
    );
    let elsewhere = vec![(
        "core/src/screens/x.rs".to_string(),
        "ClientRequest::ThreadSetName {".to_string(),
    )];
    assert_eq!(thread_rpcs_outside(&elsewhere).len(), 1);
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

// ---- Core feature slices (ADR-012) ----

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// `word` stands in `line` as a whole word (`ThreadModel` is not `ThreadModels`).
fn names_word(line: &str, word: &str) -> bool {
    line.match_indices(word).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        !before.is_some_and(is_ident) && !after.is_some_and(is_ident)
    })
}

/// Every code line of `files` that names one of `words` as a whole word, for the files `skip`
/// does not exempt.
fn scan_words(
    files: &[(String, String)],
    words: &[&str],
    skip: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        if skip(name) {
            continue;
        }
        for (index, line) in code_lines(text) {
            if words.iter().any(|word| names_word(line, word)) {
                found.push(format!("{name}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    found
}

/// The `name => Type;` entries of `feature_list!`, in list order, out of the text of
/// `core/src/features/mod.rs`.
fn listed_entries(mod_rs: &str) -> Vec<(String, String)> {
    let (_, body) = mod_rs
        .split_once("macro_rules! feature_list {")
        .expect("macro_rules! feature_list");
    let (body, _) = body.split_once("\n}\n").expect("end of feature_list!");
    code_lines(body)
        .filter_map(|(_, line)| {
            let (name, rest) = line.trim().split_once(" => ")?;
            let ty = rest.strip_suffix(';')?;
            (name.chars().all(is_ident) && ty.chars().all(is_ident))
                .then(|| (name.to_string(), ty.to_string()))
        })
        .collect()
}

/// The module names of `listed_entries`.
fn listed_features(mod_rs: &str) -> Vec<String> {
    listed_entries(mod_rs)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn all_sources() -> Vec<(String, String)> {
    let mut files = sources_of("core/src");
    files.extend(sources_of("src"));
    files.extend(sources_of("host"));
    files
}

/// The framework's own files: the list and the generated dispatch, the contexts, the probes
/// that test them. Every other file of `core/src/features/` belongs to a slice.
fn is_framework_file(name: &str) -> bool {
    matches!(
        name,
        "core/src/features/mod.rs" | "core/src/features/cx.rs" | "core/src/features/framework.rs"
    )
}

/// `core/src/features/<f>/` or `<f>.rs`, `src/features/<f>/` or `<f>.rs`, and
/// `host/src/feeds/<f>/` or `<f>.rs`: the three halves of feature `f` (the model, the view, the
/// host's feed).
fn is_own(name: &str, feature: &str) -> bool {
    ["core/src/features/", "src/features/", "host/src/feeds/"]
        .iter()
        .any(|root| {
            name.strip_prefix(root).is_some_and(|rest| {
                rest == format!("{feature}.rs") || rest.starts_with(&format!("{feature}/"))
            })
        })
}

/// `line` names feature `feature` by path: `features::goal` or `features::{.., goal, ..}`.
fn names_feature_path(line: &str, feature: &str) -> bool {
    names_path_after(line, "features::", feature)
}

/// `line` names `feature` right after `marker` (`features::`, `super::`): `<marker>goal` or
/// `<marker>{.., goal, ..}`.
fn names_path_after(line: &str, marker: &str, feature: &str) -> bool {
    line.match_indices(marker).any(|(at, pattern)| {
        let rest = &line[at + pattern.len()..];
        let tokens = |text: &str| {
            text.split(|c: char| !is_ident(c))
                .filter(|token| !token.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        match rest.strip_prefix('{') {
            Some(group) => tokens(group.split('}').next().unwrap_or_default())
                .iter()
                .any(|token| token == feature),
            None => tokens(rest).first().is_some_and(|token| token == feature),
        }
    })
}

/// The framework's own tests: the outbox (`Out`) and the app's `Effects` are typed by the real
/// list's roots, while the probes' list is a second one, so the test of a door builds a ctx of
/// a real feature. The only files besides a feature's own that name one, and they are test
/// files (`#[cfg(test)]`).
const ROOT_TESTS: &[&str] = &[
    "core/src/features/framework.rs",
    "core/src/app/features/tests.rs",
    "core/src/app/connection/tests.rs",
];

/// The lines of `files` that name a listed feature by path outside its own files and the two
/// lists (`core/src/features/mod.rs`, `src/features/mod.rs`).
fn features_named_outside_the_list(files: &[(String, String)], features: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        if name == "core/src/features/mod.rs"
            || name == "src/features/mod.rs"
            || ROOT_TESTS.contains(&name.as_str())
        {
            continue;
        }
        for feature in features {
            if is_own(name, feature) {
                continue;
            }
            for (index, line) in code_lines(text) {
                if names_feature_path(line, feature) {
                    found.push(format!("{name}:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }
    found
}

/// A feature is reached through the list: the generated `FeatureIntent`, `FeatureMenu`...
/// carry one variant per feature and the dispatch is generated, so no hub, screen, view or
/// shell file names `features::<f>` (its types, its functions). The feature's own two folders
/// do, and the host's feed of the feature (`host/src/feeds/<f>.rs`, the one file that names it
/// on the host's side); a slash command is carried to its owner by
/// `SessionIntent::Feature` and `apply_slash`, so no router names a feature's intent.
#[test]
fn features_are_named_only_through_the_list() {
    let mod_rs = features_mod();
    let features = listed_features(&mod_rs);
    for feature in &features {
        let slice = format!("core/src/features/{feature}");
        assert!(
            crate_dir().join(format!("{slice}.rs")).exists()
                || crate_dir().join(format!("{slice}/mod.rs")).exists(),
            "{feature} is in feature_list! but has no core/src/features/{feature}"
        );
    }
    let found = features_named_outside_the_list(&all_sources(), &features);
    assert!(
        found.is_empty(),
        "a feature is named outside the list and its own files: {found:#?}"
    );
}

/// The names of `host_feed_list!` in `host/src/feeds.rs`, in list order.
fn listed_feeds() -> Vec<String> {
    let text = read("host/src/feeds.rs");
    let (_, body) = text
        .split_once("host_feed_list! {")
        .expect("host_feed_list! { .. } in host/src/feeds.rs");
    let (body, _) = body.split_once("\n}\n").expect("end of host_feed_list!");
    code_lines(body)
        .filter_map(|(_, line)| line.trim().strip_suffix(';').map(str::to_owned))
        .filter(|name| name.chars().all(is_ident))
        .collect()
}

/// The host half of a slice is one file, `host/src/feeds/<f>.rs`, a line of `host_feed_list!`,
/// and it names the types of that slice and of no other (rule `features_are_named_only_through_the_list`
/// reads every other host file as a framework file that names none): `in_process.rs` tells the
/// feeds what happened and delivers what they tell (arch-pillars P7).
#[test]
fn host_half_names_one_feature() {
    let feeds = listed_feeds();
    assert!(!feeds.is_empty(), "host_feed_list! lists the feeds");
    let features = listed_features(&features_mod());
    for feed in &feeds {
        assert!(
            features.contains(feed),
            "host/src/feeds/{feed}.rs feeds `{feed}`, which feature_list! does not list"
        );
    }
    let mut files = sources_of("host/src/feeds");
    files.retain(|(name, _)| name != "host/src/feeds.rs");
    for (name, _) in &files {
        let stem = name
            .strip_prefix("host/src/feeds/")
            .and_then(|rest| rest.split(['/', '.']).next())
            .unwrap_or_default();
        assert!(
            feeds.iter().any(|feed| feed == stem),
            "{name}: not a line of host_feed_list!"
        );
    }
    for feed in &feeds {
        assert!(
            crate_dir()
                .join(format!("host/src/feeds/{feed}.rs"))
                .exists(),
            "host_feed_list! lists {feed} but host/src/feeds/{feed}.rs is not there"
        );
    }
    let found = features_named_outside_the_list(&files, &features);
    assert!(
        found.is_empty(),
        "a feed names a feature other than its own: {found:#?}"
    );
}

/// The slash commands a slice claims, by claim: the `SlashCommand::X` names inside its
/// `const RUNS ... ;` (verbs it runs with their arguments) or its `const OPENS ... ;` (the one
/// command that opens its sheet), read out of the code of one file.
fn command_claims(text: &str, constant: &str) -> Vec<String> {
    let code = code_lines(text)
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    let mut claims = Vec::new();
    for (at, _) in code.match_indices(&format!("const {constant}:")) {
        let span = code[at..].split(';').next().unwrap_or_default();
        for (found, pattern) in span.match_indices("SlashCommand::") {
            let name: String = span[found + pattern.len()..]
                .chars()
                .take_while(|c| is_ident(*c))
                .collect();
            claims.push(name);
        }
    }
    claims
}

/// The commands that more than one feature claims (`RUNS` with `RUNS`, `OPENS` with `OPENS`);
/// one feature may open a sheet bare and run the command with arguments.
fn commands_with_two_owners(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for constant in ["RUNS", "OPENS"] {
        let mut owners: std::collections::BTreeMap<String, Vec<String>> = Default::default();
        for (name, text) in files {
            let Some(rest) = name.strip_prefix("core/src/features/") else {
                continue;
            };
            let feature = rest.split(['/', '.']).next().unwrap_or_default();
            for command in command_claims(text, constant) {
                let own = owners.entry(command).or_default();
                if !own.iter().any(|owner| owner == feature) {
                    own.push(feature.to_owned());
                }
            }
        }
        for (command, owners) in owners {
            if owners.len() > 1 {
                found.push(format!("{constant} {command}: {owners:?}"));
            }
        }
    }
    found
}

/// A slash command has one owner (ADR-012, "the slash door"): `SessionIntent::Feature` is carried
/// to the one feature whose `RUNS` holds the command, so two claims would make the later one dead
/// code. The same holds for `OPENS`, the sheet a bare command opens.
#[test]
fn one_owner_per_command() {
    let found = commands_with_two_owners(&sources_of("core/src/features"));
    assert!(
        found.is_empty(),
        "a slash command is claimed by two features: {found:#?}"
    );
}

#[test]
fn the_command_owner_scan_bites() {
    let file = |name: &str, text: &str| (name.to_owned(), text.to_owned());
    let runs = |a: &str, b: &str| {
        vec![
            file(
                "core/src/features/goal/mod.rs",
                &format!("    const RUNS: &'static [SlashCommand] = &[{a}];\n"),
            ),
            file(
                "core/src/features/rename/mod.rs",
                &format!("    const RUNS: &'static [SlashCommand] = &[{b}];\n"),
            ),
        ]
    };
    assert_eq!(
        commands_with_two_owners(&runs("SlashCommand::Goal", "SlashCommand::Rename")),
        Vec::<String>::new()
    );
    assert_eq!(
        commands_with_two_owners(&runs(
            "SlashCommand::Goal, SlashCommand::Rename",
            "SlashCommand::Rename"
        ))
        .len(),
        1
    );
    // a feature that opens a sheet bare and runs the command with arguments owns both
    let both = vec![file(
        "core/src/features/export/mod.rs",
        "const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::Export, Sheet));\n    const RUNS: &'static [SlashCommand] = &[SlashCommand::Export];\n",
    )];
    assert!(commands_with_two_owners(&both).is_empty());
    // a comment is not a claim
    let comment = [file(
        "core/src/features/export/mod.rs",
        "// const RUNS: &'static [SlashCommand] = &[SlashCommand::Goal];\n",
    )];
    assert!(command_claims(&comment[0].1, "RUNS").is_empty());
}

/// The plain `pub mod` lines of `core/src/features/mod.rs` (written out so that rustfmt follows
/// them), sorted.
fn declared_feature_mods(mod_rs: &str) -> Vec<String> {
    let mut mods: Vec<String> = code_lines(mod_rs)
        .filter_map(|(_, line)| {
            let line = line.trim();
            // any visibility: `pub mod`, `pub(crate) mod`, `pub(super) mod`
            let rest = line.strip_prefix("pub")?;
            let rest = match rest.strip_prefix('(') {
                Some(restricted) => restricted.split_once(')')?.1,
                None => rest,
            };
            let name = rest.strip_prefix(" mod ")?.strip_suffix(';')?;
            name.chars().all(is_ident).then(|| name.to_string())
        })
        .collect();
    mods.sort();
    mods
}

/// The two lists of a feature agree: a module line without a `feature_list!` entry compiles
/// (the other way round it does not), so an extra plain `pub mod` is caught here.
#[test]
fn the_plain_feature_mods_are_the_feature_list() {
    let mod_rs = features_mod();
    let mut listed = listed_features(&mod_rs);
    listed.sort();
    assert_eq!(
        declared_feature_mods(&mod_rs),
        listed,
        "the `pub mod` lines of core/src/features/mod.rs and the entries of feature_list! differ"
    );
}

#[test]
fn the_feature_mod_scan_bites() {
    let text = "// pub mod commented;\nmod private;\npub mod goal;\npub(crate) mod hidden;\npub mod queue;\npub use goal::Goal;\n";
    assert_eq!(declared_feature_mods(text), ["goal", "hidden", "queue"]);
}

#[test]
fn the_feature_naming_scan_bites() {
    let list = "#[macro_export]\nmacro_rules! feature_list {\n    ($callback:ident) => {\n        $callback! {\n        goal => Goal;\n        queue => Queue;\n        }\n    };\n}\n";
    let features = listed_features(list);
    assert_eq!(features, ["goal", "queue"], "the list is read in order");
    assert_eq!(
        listed_features(
            "macro_rules! feature_list {\n    ($c:ident) => {\n        $c! {}\n    };\n}\n"
        ),
        Vec::<String>::new()
    );
    let file = |name: &str, text: &str| vec![(name.to_string(), text.to_string())];
    let found = |name: &str, text: &str| {
        features_named_outside_the_list(&file(name, text), &features).len()
    };
    // outside: a hub, a screen, a view
    assert_eq!(
        found("core/src/app.rs", "use crate::features::goal::Model;"),
        1
    );
    assert_eq!(
        found(
            "src/pages/session/x.rs",
            "use codex_tui_core::features::queue::Note;"
        ),
        1
    );
    assert_eq!(
        found("core/src/app.rs", "use crate::features::{AppCx, queue};"),
        1
    );
    assert_eq!(
        found("core/src/app.rs", "let x = crate::features::goal::apply();"),
        1
    );
    // another feature's folder is outside too
    assert_eq!(
        found(
            "core/src/features/queue/mod.rs",
            "use super::super::goal; use crate::features::goal::Model;"
        ),
        1
    );
    // inside: the feature's two halves, the framework names, a comment, another word
    assert_eq!(
        found(
            "core/src/features/goal/menu.rs",
            "use crate::features::goal::Model;"
        ),
        0
    );
    assert_eq!(
        found(
            "src/features/goal/strings.rs",
            "use codex_tui_core::features::goal::Note;"
        ),
        0
    );
    assert_eq!(
        found("core/src/app.rs", "use crate::features::{AppCx, AppView};"),
        0
    );
    assert_eq!(found("core/src/app.rs", "// the features::goal module"), 0);
    assert_eq!(
        found("core/src/app.rs", "use crate::features::goalie::X;"),
        0
    );
    assert_eq!(
        found(
            "core/src/features/mod.rs",
            "pub mod goal; features::goal::X"
        ),
        0
    );
    // the tests of the framework's roots build a ctx of a real feature; the app's code does not
    assert_eq!(
        found("core/src/app/features/tests.rs", "features::goal::Ctx {"),
        0
    );
    assert_eq!(
        found(
            "core/src/features/framework.rs",
            "crate::features::goal::Ctx {"
        ),
        0
    );
    assert_eq!(
        found("core/src/app/features.rs", "features::goal::Ctx {"),
        1
    );
    // the host's feed of a feature (`host/src/feeds/<f>.rs`) names that feature and no other; the
    // pump (`in_process.rs`) names none, not even a `Fact`
    assert_eq!(
        found(
            "host/src/feeds/goal.rs",
            "FeatureFact::from(features::goal::Fact::Seen)"
        ),
        0
    );
    assert_eq!(
        found(
            "host/src/feeds/rename.rs",
            "FeatureFact::from(features::goal::Fact::Seen)"
        ),
        1
    );
    assert_eq!(
        found(
            "host/src/in_process.rs",
            "FeatureFact::from(features::goal::Fact::Seen)"
        ),
        1
    );
    assert_eq!(
        found("host/src/in_process.rs", "features::goal::Model::new()"),
        1
    );
    assert_eq!(
        found("host/src/in_process.rs", "features::goal::Intent::Open"),
        1
    );
    assert_eq!(
        found(
            "core/src/app.rs",
            "FeatureFact::from(features::goal::Fact::Seen)"
        ),
        1
    );
    assert_eq!(
        found("core/src/app/session.rs", "features::goal::Fact::Seen"),
        1
    );
    // the slash door is generic: no hub names a feature's intent or thread intent, the session
    // file included (`SessionIntent::Feature`, `features::apply_slash`)
    assert_eq!(
        found(
            "core/src/app/session.rs",
            "features::goal::ThreadIntent::Set(name).into()"
        ),
        1
    );
    assert_eq!(
        found("core/src/app.rs", "features::goal::ThreadIntent::Set(name)"),
        1
    );
    assert_eq!(
        found("core/src/app/session.rs", "features::goal::Intent::Open"),
        1
    );
    assert_eq!(
        found(
            "core/src/app/session.rs",
            "features::goal::Model::default()"
        ),
        1
    );
    assert_eq!(found("core/src/app.rs", "features::goal::Intent::Open"), 1);
}

/// A slice reads the app through `AppView` / `ThreadView` and changes it through the outbox:
/// no file of `core/src/features/` names `CodexApp` or `ThreadModel`.
#[test]
fn features_hold_no_app() {
    let files = sources_of("core/src/features");
    assert!(!files.is_empty(), "core/src/features has no sources");
    let found = scan_words(&files, &["CodexApp", "ThreadModel"], |_| false);
    assert!(
        found.is_empty(),
        "a feature names the app or a thread's model: {found:#?}"
    );
}

#[test]
fn the_feature_app_scan_bites() {
    let files = vec![
        (
            "core/src/features/goal/mod.rs".to_string(),
            "fn go(app: &mut CodexApp) {}\nfn two(m: &ThreadModel) {}\n// CodexApp in a comment\nstruct ThreadModels;\n".to_string(),
        ),
        (
            "core/src/features/cx.rs".to_string(),
            "use crate::app::CodexApp;\n".to_string(),
        ),
    ];
    let found = scan_words(&files, &["CodexApp", "ThreadModel"], |_| false);
    assert_eq!(found.len(), 3, "{found:#?}");
}

/// What a slice must not name: the effects of the app and of a thread (`AppFx`, `ThreadFx`),
/// the thread's ask enum, the set of models, and the calls that apply an event. Its RPCs go
/// out through `AppCx::ask` / `ThreadCx::request` and nothing else, so each one is a ticket
/// with a context the framework routes back (ADR-003: thread RPCs only through `ThreadFx`).
const OUTBOX_ONLY: &[&str] = &[
    "AppFx",
    "ThreadFx",
    "ThreadAsk",
    "ThreadSet",
    "apply_app",
    "apply_ev",
    // the host's answers (`AppEv::Effect`) and asks (`Command::Effect`, `Intent::Effect`) are the
    // app's: a feature asks through `AppCx::effect` and is answered in `Feature::effect_done`
    "AppEv",
];

fn slice_outside_the_outbox(files: &[(String, String)]) -> Vec<String> {
    let mut found = scan_words(files, OUTBOX_ONLY, |name| {
        !name.starts_with("core/src/features/") || is_framework_file(name)
    });
    found.extend(scan(
        files,
        &[
            "threads::apply(",
            "app::Command",
            "rpc::Reply",
            "Intent::Effect",
            "Command::Effect",
        ],
        |name| !name.starts_with("core/src/features/") || is_framework_file(name),
    ));
    found
}

#[test]
fn feature_slices_ask_only_through_the_outbox() {
    let found = slice_outside_the_outbox(&sources_of("core/src/features"));
    assert!(
        found.is_empty(),
        "a feature slice reaches the effects directly: {found:#?}"
    );
}

#[test]
fn the_outbox_scan_bites_and_spares_the_framework() {
    let file = |name: &str, text: &str| vec![(name.to_string(), text.to_string())];
    for text in [
        "fn go(fx: &mut ThreadFx<'_>) {}",
        "fn go(fx: &mut AppFx) {}",
        "let ask = ThreadAsk::Resume { thread };",
        "fn go(set: &mut ThreadSet) {}",
        "threads::apply(model, ev, fx);",
        "use crate::app::Command;",
        "fn go(ev: AppEv) {}",
        "let ask = Intent::Effect { request, ask };",
        "Command::Effect { request, ask } => {}",
    ] {
        assert_eq!(
            slice_outside_the_outbox(&file("core/src/features/goal/mod.rs", text)).len(),
            1,
            "{text}"
        );
    }
    assert!(
        slice_outside_the_outbox(&file("core/src/features/cx.rs", "ThreadFx ThreadSet")).is_empty()
    );
    assert!(
        slice_outside_the_outbox(&file("core/src/threads.rs", "ThreadFx ThreadSet")).is_empty()
    );
}

#[test]
fn the_layer_scans_cover_the_feature_dirs() {
    // core pure: the scan of core/src reads the feature folder
    assert!(
        sources_of("core/src")
            .iter()
            .any(|(name, _)| name.starts_with("core/src/features/"))
    );
    let core = vec![(
        "core/src/features/goal/mod.rs".to_string(),
        "use tokio::sync::mpsc;\nlet t = Instant::now();\n".to_string(),
    )];
    assert_eq!(scan(&core, &["tokio", "Instant::now"], |_| false).len(), 2);
    // views without protocol: the view half is a view, holds no model code, and is a known folder
    assert!(is_view("src/features/goal/strings.rs"));
    assert!(is_view("src/features/mod.rs"));
    assert!(in_pages_sheets_widgets("src/features/goal/strings.rs"));
    let view = vec![(
        "src/features/goal/strings.rs".to_string(),
        "use codex_app_server_protocol::ThreadGoal;\npub struct Model;\n".to_string(),
    )];
    assert_eq!(
        scan(&view, &["codex_app_server_protocol"], |name| !is_view(name)).len(),
        1
    );
    assert_eq!(
        scan(&view, &["pub struct Model"], |name| {
            !in_pages_sheets_widgets(name)
        })
        .len(),
        1
    );
}

// ---- No enum with a variant per feature is written by hand ----

/// The enums of `text` that are declared in code, each with the names of its variants (the
/// uppercase identifiers that start a line at the enum's own depth).
fn enums_with_variants(text: &str) -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    let mut open: Option<(String, Vec<String>, i32)> = None;
    for (_, line) in code_lines(text) {
        let line = bare(line);
        if let Some((name, names, depth)) = open.as_mut() {
            if *depth == 1 && line.trim_start().starts_with(|c: char| c.is_uppercase()) {
                let ident: String = line
                    .trim_start()
                    .chars()
                    .take_while(|c| is_ident(*c))
                    .collect();
                names.push(ident);
            }
            *depth += line
                .chars()
                .map(|c| match c {
                    '(' | '[' | '{' => 1,
                    ')' | ']' | '}' => -1,
                    _ => 0,
                })
                .sum::<i32>();
            if *depth <= 0 {
                found.push((std::mem::take(name), std::mem::take(names)));
                open = None;
            }
            continue;
        }
        let trimmed = line.trim_start();
        let trimmed = trimmed
            .strip_prefix("pub(crate) ")
            .or_else(|| trimmed.strip_prefix("pub "))
            .unwrap_or(trimmed);
        if let Some(rest) = trimmed.strip_prefix("enum ") {
            let name: String = rest.chars().take_while(|c| is_ident(*c)).collect();
            if !name.is_empty() && rest.contains('{') {
                let depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
                open = Some((name, Vec::new(), depth));
            }
        }
    }
    found
}

/// The enums that name the listed features' marker types (`Goal`, `Rename`...) as variants,
/// two or more at once: the per-feature enum that `feature_list!` generates (`FeatureIntent`,
/// `FeatureTarget`, `OpenArgs`...) written by hand. The oracle's slash list is the one that
/// names commands, not features.
fn per_feature_enums(files: &[(String, String)], markers: &[String]) -> Vec<String> {
    // the oracle's slash list (a case per command), and the import screen's kinds of item (its
    // plugins and settings are things found in another tool's config, not these features)
    const NAMES_THINGS: &[&str] = &[
        "core/src/commands.rs:SlashCommand",
        "core/src/commands/dispatch.rs:SessionIntent",
        "core/src/screens/import/item.rs:Kind",
    ];
    let mut found = Vec::new();
    for (file, text) in files {
        for (name, variants) in enums_with_variants(text) {
            let named = variants.iter().filter(|v| markers.contains(v)).count();
            if named >= 2 && !NAMES_THINGS.contains(&format!("{file}:{name}").as_str()) {
                found.push(format!(
                    "{file}: enum {name} has {named} variants named like features"
                ));
            }
        }
    }
    found
}

/// A new thing that is a case per feature (the target of a slash command, the arguments of a
/// sheet, a menu...) is a type of the `Feature` trait that `feature_list!` collects into one
/// generated enum, never an enum written by hand beside the list.
#[test]
fn no_enum_with_a_variant_per_feature_is_written_by_hand() {
    let markers: Vec<String> = listed_entries(&features_mod())
        .into_iter()
        .map(|(_, ty)| ty)
        .collect();
    assert!(markers.len() >= 5, "the list has its features: {markers:?}");
    let found = per_feature_enums(&all_sources(), &markers);
    assert!(
        found.is_empty(),
        "a hand-written per-feature enum (make it an associated type of `Feature` / \
         `FeatureView` that the list generates): {found:#?}"
    );
}

#[test]
fn the_per_feature_enum_scan_bites() {
    let markers = ["Goal", "Rename", "Export"].map(str::to_owned).to_vec();
    let file = |text: &str| vec![("x.rs".to_string(), text.to_string())];
    let hand = "pub enum Target {\n    Goal(u8),\n    Export { a: u8 },\n    Other,\n}\n";
    assert_eq!(per_feature_enums(&file(hand), &markers).len(), 1);
    let one = "pub enum Target {\n    Goal(u8),\n    Other,\n}\n";
    assert!(per_feature_enums(&file(one), &markers).is_empty());
    let slash = vec![(
        "core/src/commands/dispatch.rs".to_string(),
        "pub enum SessionIntent {\n    Goal,\n    Rename,\n}\n".to_string(),
    )];
    assert!(per_feature_enums(&slash, &markers).is_empty());
    let generated = "pub enum FeatureTarget {\n    $($variant(u8),)*\n}\n";
    assert!(per_feature_enums(&file(generated), &markers).is_empty());
}

// ---- The hub budget: the hub enums only shrink ----

/// `enum Name {` as the line starts it (after `pub`, `pub(crate)`).
fn starts_enum(line: &str, name: &str) -> bool {
    let line = line.trim_start();
    let line = match line.strip_prefix("pub") {
        Some(rest) => rest
            .strip_prefix(|c: char| c == ' ')
            .or_else(|| rest.split_once(") ").map(|(_, rest)| rest))
            .unwrap_or(rest),
        None => line,
    };
    line.strip_prefix(&format!("enum {name}"))
        .is_some_and(|rest| rest.trim_start().starts_with('{'))
}

/// `line` without its `//` comment and the contents of its string literals.
fn bare(line: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if in_string {
            match (escaped, c) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '/' if chars.peek() == Some(&'/') => break,
            _ => out.push(c),
        }
    }
    out
}

/// The number of variants of `enum name` in `text`: the lines that start an uppercase
/// identifier at the enum's own nesting depth.
fn variants(text: &str, name: &str) -> usize {
    let mut lines = text.lines().skip_while(|line| !starts_enum(line, name));
    let header = lines.next().expect("the enum is in the file");
    let mut depth: i32 = 0;
    let mut count = 0;
    let opens = |text: &str| -> i32 {
        text.chars()
            .map(|c| match c {
                '(' | '[' | '{' => 1,
                ')' | ']' | '}' => -1,
                _ => 0,
            })
            .sum()
    };
    depth += opens(&bare(header));
    for line in lines {
        if depth <= 0 {
            break;
        }
        let line = bare(line);
        if depth == 1 && line.trim_start().starts_with(|c: char| c.is_uppercase()) {
            count += 1;
        }
        depth += opens(&line);
    }
    count
}

/// What a ratchet says of `current` against its `budget`: it must not grow, and a shrink is
/// written down so it cannot come back.
fn ratchet(what: &str, current: usize, budget: usize) -> Option<String> {
    match current.cmp(&budget) {
        std::cmp::Ordering::Equal => None,
        std::cmp::Ordering::Greater => Some(format!(
            "{what}: {current} > the budget {budget}; the hub only shrinks, so the new case is a feature slice (ADR-012)"
        )),
        std::cmp::Ordering::Less => Some(format!(
            "{what}: {current} < the budget {budget}; lower its line in tests/rules/hub_budget.txt to {current}"
        )),
    }
}

/// The lines of `tests/rules/hub_budget.txt` of one kind: `enum <file> <Name> <n>` or
/// `arms <file> <fn> <pattern> <n>`, as the words of the line after the kind.
fn budget_lines(kind: &str) -> Vec<Vec<String>> {
    read("tests/rules/hub_budget.txt")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .filter_map(|line| {
            let mut words = line.split_whitespace().map(str::to_owned);
            (words.next().as_deref() == Some(kind)).then(|| words.collect())
        })
        .collect()
}

/// The hub's enums have one variant per case of every feature that went through them; the
/// features of ADR-012 add none (`Menu::Feature`, `NoteKey::Feature`... are one variant
/// each). The budget is the count at the day the framework landed, lowered as features move
/// into slices; it never rises.
#[test]
fn hub_enums_only_shrink() {
    let lines = budget_lines("enum");
    assert!(lines.len() >= 15, "the budget lists the hub enums");
    let mut found = Vec::new();
    for words in lines {
        let [file, name, budget] = words.as_slice() else {
            panic!("`enum <file> <Name> <n>`: {words:?}");
        };
        let current = variants(&read(file), name);
        found.extend(ratchet(
            &format!("{file} {name}"),
            current,
            budget.parse().expect("a count"),
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// The entries of the macro list `name! { .. }` in `text`: the lines at the list's own depth that
/// start a lowercase method name and carry `=>` (the shape of `host_capability_list!`), with
/// multi-line entries counted once.
fn list_entries(text: &str, name: &str) -> usize {
    let start = format!("{name}! {{");
    let mut lines = text
        .lines()
        .skip_while(|line| !line.trim_start().starts_with(&start));
    lines.next().expect("the list is in the file");
    let mut depth: i32 = 1;
    let mut count = 0;
    for line in lines {
        let line = bare(line);
        if depth == 1
            && line.trim_start().starts_with(|c: char| c.is_lowercase())
            && line.contains("=>")
        {
            count += 1;
        }
        depth += line
            .chars()
            .map(|c| match c {
                '(' | '[' | '{' => 1,
                ')' | ']' | '}' => -1,
                _ => 0,
            })
            .sum::<i32>();
        if depth <= 0 {
            break;
        }
    }
    count
}

/// The host's capability list (`host_capability_list!`, ADR-012 "the host's doors") is the one
/// enum of what the host can do: it only shrinks, and a capability added is a decision that
/// raises its line (its `Ask` and `Done` variants are the same line of the list).
#[test]
fn hub_lists_only_shrink() {
    let lines = budget_lines("list");
    assert!(!lines.is_empty(), "the budget lists the host capabilities");
    let mut found = Vec::new();
    for words in lines {
        let [file, name, budget] = words.as_slice() else {
            panic!("`list <file> <macro> <n>`: {words:?}");
        };
        found.extend(ratchet(
            &format!("{file} {name}!"),
            list_entries(&read(file), name),
            budget.parse().expect("a count"),
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
}

/// The number of methods of `trait name` in `text`: the `fn` items at the trait's own depth
/// (four spaces in), up to the `}` that closes it at column 0.
fn trait_methods(text: &str, name: &str) -> usize {
    let header = |line: &str| {
        let line = line.trim_start();
        let line = line
            .strip_prefix("pub(crate) ")
            .or(line.strip_prefix("pub "))
            .unwrap_or(line);
        line.strip_prefix(&format!("trait {name}"))
            .is_some_and(|rest| rest.starts_with([':', ' ', '<', '{']))
    };
    text.lines()
        .skip_while(|line| !header(line))
        .skip(1)
        .take_while(|line| *line != "}")
        .filter(|line| {
            line.strip_prefix("    fn ")
                .is_some_and(|rest| !rest.starts_with(' '))
        })
        .count()
}

/// Every cross-feature nudge that becomes a method of `Feature` on all features, or a variant of
/// the generated roots, grows the hub under another name (review of the framework, R4/R5): a
/// hook is added only for a thread or app lifecycle fact (a turn ended, a thread resumed, the
/// input released), never for one producer and one consumer, and each one is a line of the
/// budget raised with its ADR-012 paragraph.
#[test]
fn the_feature_trait_has_the_hooks_the_budget_says() {
    let lines = budget_lines("methods");
    assert!(!lines.is_empty(), "the budget lists the Feature trait");
    let mut found = Vec::new();
    for words in lines {
        let [file, name, budget] = words.as_slice() else {
            panic!("`methods <file> <Trait> <n>`: {words:?}");
        };
        let current = trait_methods(&read(file), name);
        found.extend(ratchet(
            &format!("{file} trait {name}"),
            current,
            budget.parse().expect("a count"),
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_trait_method_count_bites() {
    let text = "pub trait Hooks: 'static {\n    const NAME: &'static str;\n\n    fn a() {}\n\n    fn b(\n        x: u8,\n    ) {\n        fn inner() {}\n    }\n}\n\nfn outside() {}\n";
    assert_eq!(trait_methods(text, "Hooks"), 2);
    assert_eq!(
        trait_methods(text, "Hook"),
        0,
        "a longer name is another trait"
    );
}

#[test]
fn the_variant_count_and_the_ratchet_bite() {
    let text = "/// docs\n#[derive(Clone, Debug)]\npub enum Hub {\n    /// one\n    One,\n    Two(u8, Vec<(u8, u8)>), // trailing\n    #[allow(dead_code)]\n    Three {\n        inner: u8,\n        Fake,\n    },\n    #[cfg_attr(\n        test,\n        derive(Debug)\n    )]\n    Four = 4,\n    Five(\"A { \"),\n}\n\npub enum Next {\n    Other,\n}\n";
    assert_eq!(variants(text, "Hub"), 5);
    assert_eq!(variants(text, "Next"), 1);
    assert_eq!(
        variants("pub(crate) enum Hidden {\n    A,\n    B,\n}\n", "Hidden"),
        2
    );
    assert_eq!(variants("enum Empty {}\n", "Empty"), 0);
    assert_eq!(
        variants("pub enum Hub {\n    One,\n    Two,\n    Three,\n}\n", "Hub"),
        3
    );
    assert!(ratchet("Hub", 5, 5).is_none());
    assert!(ratchet("Hub", 6, 5).is_some_and(|text| text.contains("only shrinks")));
    assert!(ratchet("Hub", 4, 5).is_some_and(|text| text.contains("lower its line")));
}

/// The text of `fn name(`, from its header line to the `}` that closes it at column 0.
fn fn_body<'a>(text: &'a str, name: &str) -> &'a str {
    let header = format!("fn {name}(");
    let start = text
        .lines()
        .scan(0usize, |at, line| {
            let this = *at;
            *at += line.len() + 1;
            Some((this, line))
        })
        .find(|(_, line)| line.contains(&header) && !line.trim_start().starts_with("//"))
        .map(|(at, _)| at)
        .expect("the function is in the file");
    let rest = &text[start..];
    let end = rest.find("\n}\n").map_or(rest.len(), |end| end + 2);
    &rest[..end]
}

/// `threads::apply_ev` matches the thread's own events and hands every intent of a screen or a
/// feature to that screen's or feature's own function. The arms that still name a screen's
/// intent by name are the debt of ADR-012 (the queue, the draft, the mask moves out of
/// `chat::ThreadIntent`): the count only goes down, to zero. A feature's intent is never named
/// there.
#[test]
fn threads_apply_matches_no_screen_or_feature_intent() {
    let threads = read("core/src/threads.rs");
    let body = fn_body(&threads, "apply_ev");
    let lines = budget_lines("arms");
    assert!(!lines.is_empty(), "the budget lists the arms");
    let mut found = Vec::new();
    for words in lines {
        let [file, function, pattern, budget] = words.as_slice() else {
            panic!("`arms <file> <fn> <pattern> <n>`: {words:?}");
        };
        let text = read(file);
        let current: usize = code_lines(fn_body(&text, function))
            .map(|(_, line)| line.matches(pattern.as_str()).count())
            .sum();
        found.extend(ratchet(
            &format!("{file} {function} names {pattern}"),
            current,
            budget.parse().expect("a count"),
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
    assert!(
        !body.contains("FeatureThreadIntent::"),
        "threads::apply_ev names a feature's thread intent"
    );
}

#[test]
fn the_function_body_scan_bites() {
    let text = "fn other() {\n    ScreenThreadIntent::Chat(a);\n}\n\nfn apply_ev(x: u8) {\n    match x {\n        ScreenThreadIntent::Chat(b) => {}\n        // ScreenThreadIntent::Chat(c)\n        ScreenThreadIntent::Backtrack(d) => {}\n    }\n}\n\nfn after() {\n    ScreenThreadIntent::Chat(e);\n}\n";
    let body = fn_body(text, "apply_ev");
    let count: usize = code_lines(body)
        .map(|(_, line)| line.matches("ScreenThreadIntent::").count())
        .sum();
    assert_eq!(count, 2);
    assert!(!body.contains("fn after"));
    assert!(ratchet("arms", 3, 2).is_some());
}

// ---- ADR-012: the doors of the hub (what a slice reads, what it names, what ticks) ----

/// The number of `pub fn`s of the `impl` blocks of `name` in `text`: the methods at the block's
/// own depth (four spaces in), up to the `}` that closes the block at column 0. `pub(crate)`
/// ones are the framework's plumbing and are not a slice's to call.
fn view_methods(text: &str, name: &str) -> usize {
    let header = |line: &str| {
        line.starts_with("impl")
            && !line.contains(" for ")
            && (line.contains(&format!(" {name}<")) || line.contains(&format!(" {name} {{")))
    };
    let mut inside = false;
    let mut count = 0;
    for line in text.lines() {
        if header(line) {
            inside = true;
        } else if inside && line == "}" {
            inside = false;
        } else if inside && line.starts_with("    pub fn ") {
            count += 1;
        }
    }
    count
}

/// `AppView` and `ThreadView` are what a slice reads the app through. A fact only one slice
/// wants is that slice's selector over the handles (`ThreadView::core`, `slots`, `parts`,
/// `AppView::facts`), not a method here: the two views grow by a decision with a line of the
/// budget raised, never by a feature's convenience.
#[test]
fn view_methods_budget() {
    let lines = budget_lines("view");
    assert!(!lines.is_empty(), "the budget lists the views");
    let mut found = Vec::new();
    for words in lines {
        let [file, name, budget] = words.as_slice() else {
            panic!("`view <file> <Type> <n>`: {words:?}");
        };
        found.extend(ratchet(
            &format!("{file} {name} methods"),
            view_methods(&read(file), name),
            budget.parse().expect("a count"),
        ));
    }
    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn the_view_method_count_bites() {
    let text = "impl<'a> View<'a> {\n    pub(crate) fn new() {}\n    pub fn a(&self) {}\n    fn private() {\n        pub fn inner() {}\n    }\n    pub fn b(&self) {}\n}\n\nimpl Views {\n    pub fn other(&self) {}\n}\n\nimpl Trait for View<'_> {\n    pub fn c(&self) {}\n}\n\nimpl View<'_> {\n    pub fn d(&self) {}\n}\n";
    assert_eq!(view_methods(text, "View"), 3);
    assert_eq!(view_methods(text, "Views"), 1);
}

/// The lines of `line` that reach into a feature's state or part: `parts.goal`, `states.goal`,
/// `features.goal` (the fields the generated state and parts have, one per feature).
fn names_slice_state(line: &str, feature: &str) -> bool {
    ["parts.", "states.", "features."]
        .iter()
        .any(|owner| names_word(line, &format!("{owner}{feature}")))
}

/// `line` names a listed feature through `super::<f>` (the sibling module of a file inside
/// `core/src/features/`).
fn names_sibling(line: &str, feature: &str) -> bool {
    names_path_after(line, "super::", feature)
}

/// The lines of the framework's own files (the contexts and the shared types of
/// `core/src/features/`, not the list and not the probes) that name a slice: by path
/// (`super::<f>`, `features::<f>`) or by its field of the states or the parts. The framework
/// holds no slice's name (ADR-012): a selector that needs one is the slice's.
fn framework_names_a_slice(files: &[(String, String)], features: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        let direct = name
            .strip_prefix("core/src/features/")
            .is_some_and(|rest| !rest.contains('/') && rest.ends_with(".rs"));
        if !direct
            || matches!(
                name.as_str(),
                "core/src/features/mod.rs" | "core/src/features/framework.rs"
            )
        {
            continue;
        }
        for (index, line) in code_lines(text) {
            if features.iter().any(|feature| {
                names_sibling(line, feature)
                    || names_feature_path(line, feature)
                    || names_slice_state(line, feature)
            }) {
                found.push(format!("{name}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    found
}

/// `cx.rs`, `surface.rs` and `remote.rs` (what every slice is handed, and what a surface and a
/// remote are) name no slice: the facts a slice reads are selectors the slice writes over the
/// handles. cx.rs carried the exemption while `ThreadView::daybreak` and `AppView::tui` lived
/// there; they are `small_cmds::daybreak_of` and `AppFacts::tui` now.
#[test]
fn framework_names_no_slice() {
    let features = listed_features(&features_mod());
    assert!(features.len() > 5, "the list has its features");
    let found = framework_names_a_slice(&sources_of("core/src/features"), &features);
    assert!(
        found.is_empty(),
        "a framework file names a slice: {found:#?}"
    );
}

#[test]
fn the_framework_scan_bites() {
    let features = vec!["goal".to_string(), "recap".to_string()];
    let file = |name: &str, text: &str| vec![(name.to_string(), text.to_string())];
    for text in [
        "use super::recap::Message;",
        "use super::{AppCx, goal};",
        "use crate::features::goal::Model;",
        "let part = parts.goal.saved();",
        "let state = &states.recap;",
    ] {
        assert_eq!(
            framework_names_a_slice(&file("core/src/features/cx.rs", text), &features).len(),
            1,
            "{text}"
        );
    }
    for (name, text) in [
        (
            "core/src/features/cx.rs",
            "// super::goal\nuse super::{AppCx, Out};",
        ),
        ("core/src/features/cx.rs", "let goals = parts.goals;"),
        ("core/src/features/mod.rs", "use super::goal::Model;"),
        ("core/src/features/framework.rs", "parts.goal"),
        (
            "core/src/features/goal/mod.rs",
            "use super::recap::Message;",
        ),
    ] {
        assert!(
            framework_names_a_slice(&file(name, text), &features).is_empty(),
            "{name}: {text}"
        );
    }
}

/// The lines of the app (`core/src/app.rs` and `core/src/app/`) that reach into a slice's state
/// or part: the hub reads a feature through the generated dispatch or through the feature's own
/// outbox, not by its field (`app.features.side`, `parts.small_cmds`).
fn app_names_slice_state(files: &[(String, String)], features: &[String]) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        if name != "core/src/app.rs" && !name.starts_with("core/src/app/") {
            continue;
        }
        for (index, line) in code_lines(text) {
            if features
                .iter()
                .any(|feature| names_slice_state(line, feature))
            {
                found.push(format!("{name}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    found
}

/// The hub names no slice's state (`app.features.<f>`, `parts.<f>`): the side lifecycle is the
/// slice's (it asks the app through the lifecycle doors of `AppCx`), the thread's Daybreak choice
/// is read through the list (`features::daybreak_of`). It is 0, and the budget line says so.
#[test]
fn app_names_no_feature_state() {
    let features = listed_features(&features_mod());
    let found = app_names_slice_state(&sources_of("core/src"), &features);
    let lines = budget_lines("appstate");
    let [words] = lines.as_slice() else {
        panic!("one `appstate <n>` line in the budget");
    };
    let [budget] = words.as_slice() else {
        panic!("`appstate <n>`: {words:?}");
    };
    let message = ratchet(
        "the app names a feature's state",
        found.len(),
        budget.parse().expect("a count"),
    );
    assert!(message.is_none(), "{message:?}: {found:#?}");
}

#[test]
fn the_app_state_scan_bites() {
    let features = vec!["side".to_string(), "goal".to_string()];
    let files = vec![
        (
            "core/src/app.rs".to_string(),
            "if app.features.side.is_side(&t) {}\nlet n = app.features.sides;\n// app.features.goal\n".to_string(),
        ),
        (
            "core/src/app/notices.rs".to_string(),
            "p.and_then(|parts| parts.goal.saved())\nself.features.insert(1, 2);\n".to_string(),
        ),
        (
            "core/src/threads.rs".to_string(),
            "parts.goal\n".to_string(),
        ),
    ];
    assert_eq!(app_names_slice_state(&files, &features).len(), 2);
}

/// The text of `enum AppCtx { .. }` in `core/src/app.rs`: what an in-flight request of the app
/// is waiting for.
fn app_ctx_body(text: &str) -> &str {
    let start = text
        .find("enum AppCtx {")
        .expect("the app has an AppCtx enum");
    let rest = &text[start..];
    &rest[..rest.find("\n}\n").expect("the enum ends")]
}

/// `line` names a listed feature's module (`side::Opening`, `features::side::Start`), not a
/// longer name that ends in it.
fn names_feature_module(line: &str, feature: &str) -> bool {
    let marker = format!("{feature}::");
    line.match_indices(&marker)
        .any(|(at, _)| !line[..at].chars().next_back().is_some_and(is_ident))
}

/// The lines of `AppCtx` that name a feature: an in-flight request of the hub waits for a
/// context the hub understands (the app's own steps, the generic `FeatureCtx`), never for a
/// slice's own type (`side::Opening` was one).
fn app_ctx_names_a_feature(text: &str, features: &[String]) -> Vec<String> {
    app_ctx_body(text)
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| {
            features.iter().any(|feature| {
                names_feature_module(line, feature) || names_feature_path(line, feature)
            })
        })
        .map(|(index, line)| format!("core/src/app.rs AppCtx +{index}: {}", line.trim()))
        .collect()
}

/// `AppCtx` names no feature (ADR-012, "lifecycle doors"): what a feature asked the app to make
/// or open is told back through `Feature::thread_opened` under the feature's own `FeatureCtx`.
#[test]
fn app_ctx_names_no_feature() {
    let features = listed_features(&features_mod());
    assert!(features.len() > 5, "the list has its features");
    let found = app_ctx_names_a_feature(&read("core/src/app.rs"), &features);
    assert!(found.is_empty(), "AppCtx names a feature: {found:#?}");
}

#[test]
fn the_app_ctx_scan_bites() {
    let features = vec!["side".to_string(), "goal".to_string()];
    let text = "enum AppCtx {\n    A { opening: side::Opening },\n    // goal::Note\n    B(crate::features::goal::Ctx),\n    C(FeatureCtx),\n    D { aside: u8, sides: u8 },\n}\nfn side::x() {}\n";
    assert_eq!(app_ctx_names_a_feature(text, &features).len(), 2);
}

/// Time is told, never ticked (ADR-012, "time as a declared wake"): the shell reads the clock
/// and says it (`AppEv::Clock`), and what wants to be woken declares the moment
/// (`Feature::wake_at`). The count of what still ticks: a `Clock` variant in an enum of a screen
/// or a slice (an event each screen had to be told the time by), `AppEv::Clock` named outside the
/// app's own arm, the script's step and the shell's timer, and a periodic timer
/// (`CLOCK_EVERY`, `tokio::time::interval`). It is 0, and stays 0.
fn clock_loops(files: &[(String, String)]) -> Vec<String> {
    let mut found = Vec::new();
    for (name, text) in files {
        let screen_or_slice = name.starts_with("core/src/screens/")
            || name.starts_with("core/src/features/")
            || name.starts_with("src/features/");
        if screen_or_slice {
            for (enum_name, names) in enums_with_variants(text) {
                if names.iter().any(|variant| variant == "Clock") {
                    found.push(format!("{name}: enum {enum_name} has a Clock variant"));
                }
            }
        }
        let own = matches!(
            name.as_str(),
            "core/src/app.rs" | "core/src/fixtures/script.rs" | "src/services/shell.rs"
        );
        for (index, line) in code_lines(text) {
            let ticks = ["CLOCK_EVERY", "time::interval(", "time::interval_at("]
                .iter()
                .any(|pattern| line.contains(pattern));
            if ticks || (!own && line.contains("AppEv::Clock")) {
                found.push(format!("{name}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    found
}

#[test]
fn no_clock_loops() {
    let found = clock_loops(&all_sources());
    let lines = budget_lines("clock_loops");
    let [words] = lines.as_slice() else {
        panic!("one `clock_loops <n>` line in the budget");
    };
    let [budget] = words.as_slice() else {
        panic!("`clock_loops <n>`: {words:?}");
    };
    let message = ratchet(
        "the app is ticked or told the time by a screen",
        found.len(),
        budget.parse().expect("a count"),
    );
    assert!(message.is_none(), "{message:?}: {found:#?}");
}

#[test]
fn the_clock_loop_scan_bites() {
    let files = vec![
        (
            "core/src/screens/agents/mod.rs".to_string(),
            "pub enum Ev {\n    Home(Option<String>),\n    Clock(i64),\n}\n".to_string(),
        ),
        (
            "core/src/features/goal/mod.rs".to_string(),
            "enum Intent {\n    Open,\n}\nlet ev = AppEv::Clock(now);\n".to_string(),
        ),
        (
            "src/services/shell.rs".to_string(),
            "emit(AppEv::Clock(clock.now()));\nlet mut tick = tokio::time::interval(CLOCK_EVERY);\n"
                .to_string(),
        ),
        (
            "src/services/effects.rs".to_string(),
            "// AppEv::Clock in a comment\nlet ok = Ev::ViewerClock;\n".to_string(),
        ),
    ];
    // the variant, the stray AppEv::Clock, the interval and the constant; not the shell's own
    assert_eq!(clock_loops(&files).len(), 3, "{:#?}", clock_loops(&files));
}
