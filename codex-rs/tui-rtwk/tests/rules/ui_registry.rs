//! The UI hubs' registries (ADR-012 for the UI, `docs/rtwk-port/architecture.md`): what a key
//! does, where a target opens, what an item or a mode of the transcript draws is a file and one
//! line of a list, never an arm of a hub's match. Each list has a registration test (its files
//! exist and have the shape the list's generator needs, no file is left out), each hub a
//! ratchet (no arm of a registry's kind in the hub, its line count only falls), and the
//! `FeatureView` doors (`cell`, `slot`, `actions`) that a feature takes instead of a hub edit are
//! checked to stay.
//!
//! The sources are read as text and never run; the macros are parsed with the lines' own shape
//! (`entry;` per line), which the lists keep.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::collections::BTreeSet;
use std::path::Path;

// the lists that `build.rs` writes from the files: the rules read the same text (fw-hubs-c5)
use crate::layers::registry;

fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    std::fs::read_to_string(crate_dir().join(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// A hub's `mod.rs` as the compiler sees it: the file, then the lists `build.rs` writes from the
/// files of its directory (`pub mod` lines and the list macro).
fn hub_text(dir: &str) -> String {
    let generated = match dir {
        KEY_ACTIONS => registry::actions_text(&crate_dir().join(dir)),
        TRANSCRIPT => registry::modes_text(&crate_dir().join(dir)),
        CELLS => registry::cells_text(&crate_dir().join(dir)),
        _ => panic!("{dir} has no generated list"),
    }
    .unwrap_or_else(|e| panic!("{dir}: {e}"));
    registry::with_generated(&read(&format!("{dir}/mod.rs")), &generated)
}

fn exists(rel: &str) -> bool {
    crate_dir().join(rel).is_file()
}

/// The code lines of `text` (no `//` comments), with their trailing comment cut.
fn code(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .map(|line| line.split(" // ").next().unwrap_or(line).to_owned())
        .collect()
}

/// The entries of the list `list` (`macro_rules! <list>`) in `text`: the lines between
/// `$callback! {` and its closing brace, each without its trailing `;`.
fn entries(text: &str, list: &str) -> Vec<String> {
    let lines = code(text);
    let start = lines
        .iter()
        .position(|line| line.contains(&format!("macro_rules! {list}")))
        .unwrap_or_else(|| panic!("macro_rules! {list} is not there"));
    let open = lines[start..]
        .iter()
        .position(|line| line.contains("$callback! {"))
        .unwrap_or_else(|| panic!("{list} has no `$callback! {{`"))
        + start;
    lines[open + 1..]
        .iter()
        .take_while(|line| line.trim() != "}")
        .map(|line| line.trim().trim_end_matches(';').to_owned())
        .filter(|line| !line.is_empty())
        .collect()
}

/// The variants of `pub enum name` in `text`: the identifiers that start a line at the enum's own
/// nesting depth.
fn variants(text: &str, name: &str) -> Vec<String> {
    let lines = code(text);
    let start = lines
        .iter()
        .position(|line| line.contains(&format!("pub enum {name} ")))
        .unwrap_or_else(|| panic!("enum {name} is not there"));
    let mut depth = 0i32;
    let mut found = Vec::new();
    for (index, line) in lines[start..].iter().enumerate() {
        let trimmed = line.trim_start();
        if index > 0 && depth == 1 && trimmed.starts_with(|c: char| c.is_uppercase()) {
            let ident: String = trimmed
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            found.push(ident);
        }
        for c in line.chars() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                _ => {}
            }
        }
        if index > 0 && depth <= 0 {
            break;
        }
    }
    found
}

fn lines_of(rel: &str) -> usize {
    read(rel).lines().count()
}

/// Every `.rs` file of `dir` (not recursive), by stem.
fn stems(dir: &str) -> BTreeSet<String> {
    std::fs::read_dir(crate_dir().join(dir))
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .filter_map(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect()
}

/// The sources of the UI crate (`src/`), but those of `skip`, as `(path, code lines)`.
fn ui_sources(skip: impl Fn(&str) -> bool) -> Vec<(String, Vec<String>)> {
    let mut files = Vec::new();
    collect(&crate_dir().join("src"), &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            path.strip_prefix(crate_dir())
                .expect("inside the crate")
                .to_string_lossy()
                .replace('\\', "/")
        })
        .filter(|name| !skip(name))
        .map(|name| {
            let lines = code(&read(&name));
            (name, lines)
        })
        .collect()
}

fn collect(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read_dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn assert_clean(what: &str, fails: Vec<String>) {
    assert!(fails.is_empty(), "{what}:\n{}", fails.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Registry 1: key actions (`src/pages/session/key_actions/`, `action_list!`)

const KEY_ACTIONS: &str = "src/pages/session/key_actions";

/// `(file, Marker)` of every entry of `action_list!`.
fn key_action_entries() -> Vec<(String, String)> {
    entries(&hub_text(KEY_ACTIONS), "action_list")
        .iter()
        .map(|entry| {
            let (file, marker) = entry
                .split_once("=>")
                .unwrap_or_else(|| panic!("action_list! entry `{entry}` is not `file => Marker`"));
            (file.trim().to_owned(), marker.trim().to_owned())
        })
        .collect()
}

#[test]
fn action_list_names_a_file_and_a_key_action_each() {
    let list = key_action_entries();
    assert!(!list.is_empty(), "action_list! has no entry");
    let module = hub_text(KEY_ACTIONS);
    let mut fails = Vec::new();
    let mut ids = BTreeSet::new();
    for (file, marker) in &list {
        let path = format!("{KEY_ACTIONS}/{file}.rs");
        if !exists(&path) {
            fails.push(format!("action_list! names `{file}`, {path} is not a file"));
            continue;
        }
        if !module
            .lines()
            .any(|line| line.trim() == format!("pub mod {file};"))
        {
            fails.push(format!(
                "{path} is not a `pub mod {file};` of key_actions/mod.rs"
            ));
        }
        let text = read(&path);
        if !text.contains(&format!("impl KeyAction for {marker}")) {
            fails.push(format!("{path} has no `impl KeyAction for {marker}`"));
        }
        let id = code(&text)
            .into_iter()
            .find(|line| line.contains("const ID: ActionId"))
            .map(|line| line.trim().to_owned());
        match id {
            Some(id) if !ids.insert(id.clone()) => {
                fails.push(format!("{path}: `{id}` is the id of another action too"));
            }
            Some(_) => {}
            None => fails.push(format!("{path} has no `const ID: ActionId = ..`")),
        }
    }
    assert_clean("the key action list", fails);
}

#[test]
fn every_key_action_file_is_in_the_list() {
    let listed: BTreeSet<String> = key_action_entries().into_iter().map(|(f, _)| f).collect();
    let files: BTreeSet<String> = stems(KEY_ACTIONS)
        .into_iter()
        .filter(|name| name != "mod")
        .collect();
    let orphans: Vec<&String> = files.difference(&listed).collect();
    assert!(
        orphans.is_empty(),
        "files of {KEY_ACTIONS} that action_list! does not name (a file nobody lists is dead): {orphans:?}"
    );
}

/// The hubs match on no action: `keys.rs` generates the enum from the list and `session_page.rs`
/// hands the key to `Action::run`; an arm `Action::<Marker> =>` anywhere in the UI is a route
/// round the registry. (Naming an action as a value, `ChatMsg::Action(Action::Warnings)`, is the
/// verb the registry dispatches; the feature door, `Action::Feature`, is generic.)
#[test]
fn key_hubs_match_on_no_action() {
    let markers: Vec<String> = key_action_entries().into_iter().map(|(_, m)| m).collect();
    let mut fails = Vec::new();
    for (name, lines) in ui_sources(|name| name.starts_with(&format!("{KEY_ACTIONS}/"))) {
        for (index, line) in lines.iter().enumerate() {
            for marker in &markers {
                let needle = format!("Action::{marker}");
                if let Some(at) = line.find(&needle) {
                    let rest = &line[at + needle.len()..];
                    let rest = rest.trim_start_matches([')', ' ']);
                    if rest.starts_with("=>") || rest.starts_with('|') {
                        fails.push(format!("{name}:{}: {}", index + 1, line.trim()));
                    }
                }
            }
        }
    }
    assert_clean(
        "an arm on a key action outside its file (the hub dispatches through `Action::run`)",
        fails,
    );
    let keys = read("src/pages/session/keys.rs");
    assert!(
        keys.contains("key_actions::action_list!(action_enum);"),
        "keys.rs generates `Action` from `action_list!`"
    );
}

// ---------------------------------------------------------------------------------------------
// Registry 2: open targets (`src/pages/session/open.rs`)

/// The body of `pub fn open` of open.rs: its match lines.
fn open_arms() -> Vec<String> {
    let lines = code(&read("src/pages/session/open.rs"));
    let start = lines
        .iter()
        .position(|line| line.starts_with("pub fn open<"))
        .expect("`pub fn open` of open.rs");
    let match_at = lines[start..]
        .iter()
        .position(|line| line.trim() == "match target {")
        .expect("`match target {` in open")
        + start;
    lines[match_at + 1..]
        .iter()
        .take_while(|line| line.trim() != "}")
        .cloned()
        .collect()
}

#[test]
fn every_target_has_one_line_in_open() {
    let targets = variants(&read("core/src/commands/dispatch.rs"), "Target");
    assert!(targets.len() >= 13, "core's Target has {targets:?}");
    let arms = open_arms();
    let mut fails = Vec::new();
    for target in &targets {
        if !arms
            .iter()
            .any(|arm| arm.contains(&format!("Target::{target}")))
        {
            fails.push(format!("Target::{target} has no line in open.rs"));
        }
    }
    // one line per arm: the owner's opener is called, nothing is built in the hub
    for arm in &arms {
        let arm = arm.trim();
        if !arm.contains("=>") || !(arm.ends_with(',') || arm.ends_with("=> {}")) {
            fails.push(format!(
                "open.rs: `{arm}` is not a one-line arm (`Target::X => owner::open(cx),`)"
            ));
        }
        if arm.contains('{') && !arm.ends_with("=> {}") && !arm.ends_with("=> {},") {
            fails.push(format!(
                "open.rs: `{arm}` has a body; the arm calls the owner's opener"
            ));
        }
    }
    assert_clean("the open targets", fails);
}

/// No stub of a screen lives in the hubs: a screen that is not built is its owner's `not_yet`
/// (`pager::open`, `agents::open_picker`, `select::open_presets`), a feature's is its own sheet.
#[test]
fn open_hubs_hold_no_stub() {
    let names = [
        "not_yet",
        "daemon_action",
        "presets_action",
        "action_subagents",
    ];
    let mut fails = Vec::new();
    for rel in ["src/pages/session/open.rs", "src/pages/session/actions.rs"] {
        for (index, line) in code(&read(rel)).iter().enumerate() {
            if names.iter().any(|name| line.contains(name)) {
                fails.push(format!("{rel}:{}: {}", index + 1, line.trim()));
            }
        }
    }
    assert_clean("a stub in a hub of the open targets", fails);
}

/// A match on `Target` is the registry's: outside open.rs (and the slices, which have enums of
/// their own) no arm names a variant of core's `Target`.
#[test]
fn only_open_matches_on_target() {
    let targets = variants(&read("core/src/commands/dispatch.rs"), "Target");
    let mut fails = Vec::new();
    let files = ui_sources(|name| {
        name == "src/pages/session/open.rs"
            || name.starts_with("src/features/")
            || name.starts_with("src/sheets/select/")
    });
    for (name, lines) in files {
        for (index, line) in lines.iter().enumerate() {
            for target in &targets {
                let needle = format!("Target::{target}");
                let mut from = 0;
                while let Some(at) = line[from..].find(&needle) {
                    let end = from + at + needle.len();
                    let rest = line[end..].trim_start();
                    let rest = rest.trim_start_matches(['(', '_', ')']);
                    let word = line[..from + at].chars().next_back();
                    let own = word.is_some_and(|c| c.is_alphanumeric() || c == '_');
                    if !own && (rest.starts_with("=>") || rest.starts_with('|')) {
                        fails.push(format!("{name}:{}: {}", index + 1, line.trim()));
                    }
                    from = end;
                }
            }
        }
    }
    assert_clean("an arm on `Target` outside the registry (open.rs)", fails);
}

// ---------------------------------------------------------------------------------------------
// Registry 3: cells (`src/pages/session/cells/`, `cell_list!`) and the transcript's modes

const CELLS: &str = "src/pages/session/cells";

/// `(patterns, file)` of every entry of `cell_list!`.
fn cell_entries() -> Vec<(String, String)> {
    entries(&hub_text(CELLS), "cell_list")
        .iter()
        .map(|entry| {
            let (pattern, file) = entry
                .rsplit_once("=>")
                .unwrap_or_else(|| panic!("cell_list! entry `{entry}` is not `pattern => file`"));
            (pattern.trim().to_owned(), file.trim().to_owned())
        })
        .collect()
}

#[test]
fn cell_list_covers_every_item_with_a_view_file() {
    let list = cell_entries();
    let mut fails = Vec::new();
    // every kind of item is a pattern of some entry (the generated match is exhaustive, so this
    // is the registry's own record of it)
    let items = variants(&read("core/src/threads.rs"), "Item");
    assert!(items.len() >= 20, "core's Item has {items:?}");
    for item in &items {
        let named = list.iter().any(|(pattern, _)| {
            pattern
                .split('|')
                .any(|alt| alt.trim().starts_with(&format!("Item::{item}")))
        });
        if !named {
            fails.push(format!("Item::{item} is no pattern of cell_list!"));
        }
    }
    let module = hub_text(CELLS);
    let mut seen = BTreeSet::new();
    for (pattern, file) in &list {
        if pattern
            .split('|')
            .any(|alt| !alt.trim().starts_with("Item::"))
        {
            fails.push(format!(
                "cell_list!: `{pattern}` is not an `Item::` pattern"
            ));
        }
        if !seen.insert(file.clone()) {
            fails.push(format!(
                "cell_list!: `{file}` is on two lines (one line per file, patterns joined by `|`)"
            ));
        }
        let path = format!("{CELLS}/{file}.rs");
        if !exists(&path) && !exists(&format!("{CELLS}/{file}/mod.rs")) {
            fails.push(format!(
                "cell_list! names `{file}`, which is no file of {CELLS}"
            ));
            continue;
        }
        if !module
            .lines()
            .any(|line| line.trim() == format!("pub mod {file};"))
        {
            fails.push(format!(
                "`{file}` is not a `pub mod {file};` of cells/mod.rs"
            ));
        }
        let text = if exists(&path) {
            read(&path)
        } else {
            read(&format!("{CELLS}/{file}/mod.rs"))
        };
        if !text.contains("pub fn view(item: &Item, ") {
            fails.push(format!(
                "{file}: no `pub fn view(item: &Item, env, place) -> AnyWidget` (the entry the list's match calls)"
            ));
        }
    }
    assert_clean("the cell list", fails);
}

/// dispatch.rs is the raw-mode step and the generated match: no arm of its own for a kind of
/// item, and its size only falls.
#[test]
fn dispatch_holds_no_item_arm() {
    let lines = code(&read(&format!("{CELLS}/dispatch.rs")));
    let named: Vec<String> = lines
        .iter()
        .filter(|line| line.contains("Item::"))
        .map(|line| line.trim().to_owned())
        .collect();
    // the only kind the dispatcher knows is the user's prompt, for the blank row it does not get
    // when it joins the one above (raw or not)
    assert!(
        named.len() <= 1 && named.iter().all(|line| line.contains("Item::User(_)")),
        "cells/dispatch.rs names the kinds of item {named:?}: a cell is a file and a line of cell_list!"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("super::cell_list!(built);")),
        "cells/dispatch.rs generates its match from cell_list!"
    );
}

const TRANSCRIPT: &str = "src/pages/session/transcript";

#[test]
fn mode_list_names_a_mode_file_each() {
    let text = hub_text(TRANSCRIPT);
    let list = entries(&text, "mode_list");
    assert!(!list.is_empty(), "mode_list! has no entry");
    let mut fails = Vec::new();
    let mut names = BTreeSet::new();
    for file in &list {
        let path = format!("{TRANSCRIPT}/{file}.rs");
        if !exists(&path) {
            fails.push(format!("mode_list! names `{file}`, {path} is not a file"));
            continue;
        }
        if !text.lines().any(|line| {
            line.trim() == format!("pub mod {file};") || line.trim() == format!("mod {file};")
        }) {
            fails.push(format!("{path} is no module of transcript/mod.rs"));
        }
        let source = read(&path);
        if !source.contains("pub struct Mode;") {
            fails.push(format!("{path} has no `pub struct Mode;`"));
        }
        if !source.contains("impl TranscriptMode for Mode") {
            fails.push(format!("{path} has no `impl TranscriptMode for Mode`"));
        }
        let name = code(&source)
            .into_iter()
            .find(|line| line.contains("const NAME: &'static str"))
            .map(|line| line.trim().to_owned());
        match name {
            Some(name) if !names.insert(name.clone()) => {
                fails.push(format!("{path}: `{name}` is the name of another mode too"));
            }
            Some(_) => {}
            None => fails.push(format!("{path} has no `const NAME`")),
        }
    }
    assert_clean("the transcript's mode list", fails);
}

/// The debug menu's entries of the transcript are the modes': `transcript/mod.rs` keeps no
/// fixture of a mode.
#[test]
fn transcript_mod_keeps_no_mode() {
    let text = hub_text(TRANSCRIPT);
    let mut fails = Vec::new();
    for banned in ["DebugEntry::new", "fixture", "fn pager_", "fn sheet_"] {
        if code(&text).iter().any(|line| line.contains(banned)) {
            fails.push(format!(
                "{TRANSCRIPT}/mod.rs holds `{banned}`: a mode's fixture is its file's"
            ));
        }
    }
    assert_clean("the transcript's hub", fails);
}

// ---------------------------------------------------------------------------------------------
// The doors of a feature: `FeatureView::cell`, `slot`, `actions`

#[test]
fn feature_view_has_the_cell_slot_and_actions_doors() {
    let text = read("src/features/mod.rs");
    let mut fails = Vec::new();
    for needle in [
        "fn cell(_note: &Self::Note) -> Option<AnyWidget>",
        "fn slot<S: 'static>(",
        "fn actions() -> Vec<FeatureAction>",
        "feature_list!(view_cells);",
        "feature_list!(view_slots);",
        "feature_list!(view_actions);",
        "pub fn run_action(",
    ] {
        if !text.contains(needle) {
            fails.push(format!("src/features/mod.rs lacks `{needle}`"));
        }
    }
    // the doors are reached: a note of a feature draws through `cell`, a key of a feature runs
    // through `run_action`
    if !read(&format!("{CELLS}/note.rs")).contains("crate::features::cell(") {
        fails.push(format!(
            "{CELLS}/note.rs does not ask `features::cell` for a feature's note"
        ));
    }
    if !read("src/pages/session/keys.rs").contains("features::run_action(") {
        fails.push(
            "src/pages/session/keys.rs does not run a feature's key through `features::run_action`"
                .to_owned(),
        );
    }
    assert_clean("the feature view's doors", fails);
}

// ---------------------------------------------------------------------------------------------
// The lists are the files' (fw-hubs-c5): no hand line of a list, no cell file without its head

/// The four hubs take their list from `build.rs` (`include!`), so a slice, action, mode or cell
/// is a file and two branches that each add one do not meet on a line.
#[test]
fn the_lists_are_generated_from_the_files() {
    let mut fails = Vec::new();
    for (hub, include, list) in [
        (KEY_ACTIONS, "/key_actions.rs", "action_list"),
        (TRANSCRIPT, "/transcript.rs", "mode_list"),
        (CELLS, "/cells.rs", "cell_list"),
    ] {
        let text = read(&format!("{hub}/mod.rs"));
        if !text.contains(&format!(
            "include!(concat!(env!(\"OUT_DIR\"), \"{include}\"));"
        )) {
            fails.push(format!(
                "{hub}/mod.rs does not include `{include}` of build.rs"
            ));
        }
        if text.contains(&format!("macro_rules! {list}")) {
            fails.push(format!("{hub}/mod.rs writes `{list}!` by hand"));
        }
    }
    let text = read("src/features/mod.rs");
    if !text.contains("include!(concat!(env!(\"OUT_DIR\"), \"/features.rs\"));") {
        fails.push("src/features/mod.rs does not include `/features.rs` of build.rs".to_owned());
    }
    if text.contains("macro_rules! sheet_list") {
        fails.push("src/features/mod.rs writes `sheet_list!` by hand".to_owned());
    }
    assert_clean("a registration list written by hand", fails);
}

/// A file of cells/ that has the cell entry (`pub fn view(item: &Item, ..`) has its `// cell:` head
/// line, or `build.rs` never lists it and the match over `Item` does not compile.
#[test]
fn every_cell_view_has_its_head_line() {
    let mut fails = Vec::new();
    let dir = crate_dir().join(CELLS);
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("cells/")
        .flatten()
        .map(|entry| entry.path())
        .collect();
    files.sort();
    for path in files {
        let file = if path.is_dir() {
            path.join("mod.rs")
        } else {
            path.clone()
        };
        if !file.is_file() || file.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("a cell file");
        let has_view = text.contains("pub fn view(item: &Item, ");
        let head = registry::cell_pattern(&text);
        if has_view
            && head.is_none()
            && file
                .file_name()
                .is_some_and(|n| n != "mod.rs" || path.is_dir())
        {
            fails.push(format!(
                "{}: `pub fn view(item: &Item, ..)` without a `{}<Item pattern>` line in its first lines",
                file.display(),
                registry::CELL_MARKER
            ));
        }
        if let Some(pattern) = head {
            if !has_view {
                fails.push(format!(
                    "{}: `// cell: {pattern}` but no `pub fn view(item: &Item, ..)`",
                    file.display()
                ));
            }
            if pattern
                .split('|')
                .any(|alt| !alt.trim().starts_with("Item::"))
            {
                fails.push(format!(
                    "{}: `{pattern}` is not an `Item::` pattern",
                    file.display()
                ));
            }
        }
    }
    assert_clean("the cells' head lines", fails);
}

// ---------------------------------------------------------------------------------------------
// The hubs' size: the line counts only fall (the number is what the file has after the registry
// took its cases out; lower the constant when a hub shrinks further)

const HUBS: &[(&str, usize)] = &[
    ("src/pages/session/open.rs", 78),
    ("src/pages/session/keys.rs", 90),
    ("src/pages/session/actions.rs", 471),
    ("src/pages/session/cells/dispatch.rs", 93),
    ("src/pages/session/transcript/mod.rs", 108),
    ("src/pages/session/session_page.rs", 431),
];

#[test]
fn hubs_only_shrink() {
    let mut fails = Vec::new();
    for (rel, budget) in HUBS {
        let current = lines_of(rel);
        if current > *budget {
            fails.push(format!(
                "{rel}: {current} lines > the budget {budget}; what the hub gained is a file of a registry (key_actions/, cells/, transcript/<mode>.rs, a slice's opener)"
            ));
        }
    }
    assert_clean("a hub grew", fails);
}
