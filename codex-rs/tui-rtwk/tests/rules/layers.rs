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
/// `l10n/`, `keymap/`, `features/` (the view half of the core's feature slices, ADR-012:
/// the words and views of `core/src/features/<f>/`). A core feature module need not mirror
/// a UI folder (a feature may feed a page, a sheet or a cell).
#[test]
fn ui_crate_has_only_the_known_folders() {
    let known = [
        "app", "features", "keymap", "l10n", "pages", "services", "sheets", "widgets",
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
/// Built in core/src/threads.rs and
/// core/src/protocol/mod.rs only, whoever asks: a feature slice cannot make them either.
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
/// `/undo`): until their features are slices they are built in threads.rs too; a slice under
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
        name == "core/src/threads.rs" || name == "core/src/protocol/mod.rs"
    });
    found.extend(scan(files, FEATURE_RPCS, |name| {
        name == "core/src/threads.rs"
            || name == "core/src/protocol/mod.rs"
            || name.starts_with("core/src/features/")
    }));
    found
}

/// A thread RPC is asked for as data (`threads::ThreadAsk`) and made into a
/// wire request in core/src/threads.rs only; the two thread commands the app
/// itself mints (`StartThread`, `StartTurn`) are mapped in protocol/mod.rs.
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
        "thread RPC built outside threads.rs: {found:#?}"
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
fn listed_features(mod_rs: &str) -> Vec<String> {
    let (_, body) = mod_rs
        .split_once("macro_rules! feature_list {")
        .expect("macro_rules! feature_list");
    let (body, _) = body.split_once("\n}\n").expect("end of feature_list!");
    code_lines(body)
        .filter_map(|(_, line)| {
            let (name, rest) = line.trim().split_once(" => ")?;
            let ty = rest.strip_suffix(';')?;
            (name.chars().all(is_ident) && ty.chars().all(is_ident)).then(|| name.to_string())
        })
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

/// `core/src/features/<f>/` or `<f>.rs`, and `src/features/<f>/` or `<f>.rs`: both halves of
/// feature `f`.
fn is_own(name: &str, feature: &str) -> bool {
    ["core/src/features/", "src/features/"].iter().any(|root| {
        name.strip_prefix(root).is_some_and(|rest| {
            rest == format!("{feature}.rs") || rest.starts_with(&format!("{feature}/"))
        })
    })
}

/// `line` names feature `feature` by path: `features::goal` or `features::{.., goal, ..}`.
fn names_feature_path(line: &str, feature: &str) -> bool {
    line.match_indices("features::").any(|(at, pattern)| {
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

/// The files that route the oracle's slash list (`SessionIntent`) into a feature's intent:
/// the one place besides a feature's own files that may name `features::<f>::Intent` (and
/// `ThreadIntent`, for what a command asks of the thread it starts: `/new <name>`), and nothing
/// else of the feature.
const SLASH_ROUTERS: &[&str] = &["core/src/app/session.rs"];

/// The file that sends a fact the host read, unasked, to a feature (`AppEv::Feature`): the one
/// place besides a feature's own files that may name `features::<f>::Fact` (the payload type
/// it builds `FeatureFact::from(..)` of), and nothing else of the feature.
const FACT_SENDERS: &[&str] = &["host/src/in_process.rs"];

/// The framework's own tests: the outbox (`Out`) and the app's `Effects` are typed by the real
/// list's roots, while the probes' list is a second one, so the test of a door builds a ctx of
/// a real feature. The only files besides a feature's own that name one, and they are test
/// files (`#[cfg(test)]`).
const ROOT_TESTS: &[&str] = &[
    "core/src/features/framework.rs",
    "core/src/app/features/tests.rs",
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
            let intent = format!("features::{feature}::Intent");
            let thread_intent = format!("features::{feature}::ThreadIntent");
            let fact = format!("features::{feature}::Fact");
            for (index, line) in code_lines(text) {
                let mut line_left = line.to_string();
                if SLASH_ROUTERS.contains(&name.as_str()) {
                    line_left = line_left.replace(&intent, "").replace(&thread_intent, "");
                }
                if FACT_SENDERS.contains(&name.as_str()) {
                    line_left = line_left.replace(&fact, "");
                }
                if names_feature_path(&line_left, feature) {
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
/// do; the slash router may build its intent and the host's sender its fact.
#[test]
fn features_are_named_only_through_the_list() {
    let mod_rs = read("core/src/features/mod.rs");
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
    // the host's fact sender builds the feature's `Fact` and names nothing else; no other file may
    assert_eq!(
        found(
            "host/src/in_process.rs",
            "FeatureFact::from(features::goal::Fact::Seen)"
        ),
        0
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
    // the slash router builds an intent, or a thread intent, and names nothing else
    assert_eq!(
        found(
            "core/src/app/session.rs",
            "features::goal::ThreadIntent::Set(name).into()"
        ),
        0
    );
    assert_eq!(
        found("core/src/app.rs", "features::goal::ThreadIntent::Set(name)"),
        1
    );
    assert_eq!(
        found("core/src/app/session.rs", "features::goal::Intent::Open"),
        0
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
