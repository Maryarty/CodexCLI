//! The registration lists of the core and the UI crate, written from the files (fw-hubs-c5): a
//! new slice, key action, transcript mode or cell is a file, never a line of a list. `build.rs` of
//! `codex-tui-core` and of `codex-tui-rtwk` write the texts below into OUT_DIR and the hub
//! `include!`s them; the rules (`tests/rules`) call the same functions, so what a rule reads is
//! what the compiler got (one source, `#[path]` of all of them). std only.
//!
//! What a file says about itself, and so where it lands:
//!   core/src/features/<f>/mod.rs   a feature slice by being a directory with a `mod.rs`; its
//!                                  marker is the type of its `impl Feature for <Marker>`, and it
//!                                  opens a sheet when a `const OPENS` of it is `Some(..)`.
//!                                  `feature_list!`, the `pub mod` lines, and (UI crate) the
//!                                  `pub mod` lines of `src/features/<f>` and `sheet_list!`.
//!   key_actions/<file>.rs          every file is a key action: `impl KeyAction for <Marker>`.
//!                                  `action_list!` and the `pub mod` lines.
//!   transcript/<file>.rs           a mode when the file holds `impl TranscriptMode for Mode`.
//!                                  `mode_list!` and its `pub mod` lines.
//!   cells/<file>.rs, cells/<f>/mod.rs   a cell when one of its first lines is
//!                                  `// cell: <Item pattern>` (e.g. `// cell: Item::User(_)`).
//!                                  `cell_list!` and its `pub mod` lines.
//! The lists are ordered by file name. A list is written so that its generated macro has the shape
//! the rules parse (`macro_rules! <list> {` .. a `}` at the start of a line).
//!
//! rustfmt does not follow a module that a macro declares: the files of the generated modules
//! are formatted by `tools/rtwk-port/registry/fmt.sh` (the README there).

#![allow(clippy::expect_used, clippy::disallowed_methods, dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// How many lines from the top of a file the `// cell:` line may be on.
const HEAD: usize = 15;

/// The prefix of the header line of a cell file.
pub const CELL_MARKER: &str = "// cell: ";

/// The spellings of the line that makes a type a feature.
const FEATURE_IMPL: &[&str] = &["impl Feature for ", "impl super::Feature for "];

/// One feature slice of the core.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slice {
    /// The directory name, the module and the list name (`goal`).
    pub name: String,
    /// The marker type that implements `Feature` (`Goal`).
    pub marker: String,
    /// The slash command of the slice opens a sheet of its own (`Feature::OPENS` is `Some`).
    pub opens: bool,
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn quoted(path: &Path) -> String {
    format!("{:?}", path.to_string_lossy())
}

/// The names (sorted) of the entries of `dir` that are `.rs` files (stem) and, when
/// `with_dirs`, directories that hold a `mod.rs`; each with the path of the module file.
fn module_files(dir: &Path, with_dirs: bool) -> Result<Vec<(String, PathBuf)>, String> {
    let mut found = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") && name != "mod" {
            found.push((name, path));
        } else if with_dirs && path.is_dir() && path.join("mod.rs").is_file() {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            found.push((name, path.join("mod.rs")));
        }
    }
    found.sort();
    Ok(found)
}

fn mod_line(name: &str, file: &Path) -> String {
    format!("#[path = {}]\npub mod {name};\n", quoted(file))
}

/// The type after `prefix` on the first code line of `text` that starts with it.
fn marker_after(text: &str, prefixes: &[&str]) -> Option<String> {
    text.lines()
        .map(str::trim_start)
        .filter(|line| !line.starts_with("//"))
        .find_map(|line| prefixes.iter().find_map(|prefix| line.strip_prefix(prefix)))
        .map(|rest| {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        })
        .filter(|marker: &String| !marker.is_empty())
}

fn rs_files_below(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
        let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
        if path.is_dir() {
            rs_files_below(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    out.sort();
    Ok(())
}

/// Whether some `const OPENS` of `text` is `Some(..)`: the text from the name to the next `;`.
fn opens_a_sheet(text: &str) -> bool {
    let mut rest = text;
    while let Some(at) = rest.find("const OPENS") {
        let tail = &rest[at..];
        let end = tail.find(';').unwrap_or(tail.len());
        if tail[..end].contains("= Some(")
            || tail[..end].contains("=\n") && tail[..end].contains("Some(")
        {
            return true;
        }
        rest = &tail[end..];
    }
    false
}

/// The feature slices of `core/src/features`, by name.
pub fn core_features(dir: &Path) -> Result<Vec<Slice>, String> {
    let mut slices = Vec::new();
    for (name, module) in module_files(dir, true)? {
        let Some(root) = module.parent().filter(|_| module.ends_with("mod.rs")) else {
            continue;
        };
        if root == dir {
            continue;
        }
        let mut files = Vec::new();
        rs_files_below(root, &mut files)?;
        let mut markers = Vec::new();
        let mut opens = false;
        for file in files {
            let text = read(&file)?;
            if let Some(marker) = marker_after(&text, FEATURE_IMPL) {
                markers.push((file.clone(), marker));
            }
            opens |= opens_a_sheet(&text);
        }
        match markers.as_slice() {
            [(_, marker)] => slices.push(Slice {
                name,
                marker: marker.clone(),
                opens,
            }),
            [] => {
                return Err(format!(
                    "{}: a directory of core/src/features with no `impl Feature for <Marker>`",
                    root.display()
                ));
            }
            many => {
                return Err(format!(
                    "{}: more than one `impl Feature for`: {many:?}",
                    root.display()
                ));
            }
        }
    }
    Ok(slices)
}

/// `core/src/features`: the `pub mod` line of every slice, then `feature_list!`.
pub fn core_features_text(dir: &Path, slices: &[Slice]) -> String {
    let mut out = String::from("// generated by build.rs from registry.rs: do not edit\n\n");
    for slice in slices {
        out.push_str(&mod_line(
            &slice.name,
            &dir.join(&slice.name).join("mod.rs"),
        ));
    }
    out.push_str(
        "\n/// The list of the features, in hook order (by name), handed to `callback` as\n\
         /// `name => Type;` entries: `name` is the module `core/src/features/<name>/`, `Type` the\n\
         /// marker type in it that implements `Feature`. The UI crate calls it with its own macro\n\
         /// for the view half (`src/features/<name>/`), so the two lists cannot differ. Written by\n\
         /// `build.rs` from the directories: a slice is a directory.\n\
         #[macro_export]\n\
         macro_rules! feature_list {\n    ($callback:ident) => {\n        $callback! {\n",
    );
    for slice in slices {
        out.push_str(&format!(
            "            {} => {};\n",
            slice.name, slice.marker
        ));
    }
    out.push_str("        }\n    };\n}\n");
    out
}

/// `src/features`: the `pub mod` line of every slice's view half, then `sheet_list!` (the slices
/// whose core half opens a sheet).
pub fn ui_features_text(ui_dir: &Path, slices: &[Slice]) -> Result<String, String> {
    let mut out = String::from("// generated by build.rs from registry.rs: do not edit\n\n");
    for slice in slices {
        let dir = ui_dir.join(&slice.name).join("mod.rs");
        let file = ui_dir.join(format!("{}.rs", slice.name));
        let module = if dir.is_file() {
            dir
        } else if file.is_file() {
            file
        } else {
            return Err(format!(
                "{} or {}: the view half of the feature `{}`",
                dir.display(),
                file.display(),
                slice.name
            ));
        };
        out.push_str(&mod_line(&slice.name, &module));
    }
    out.push_str(
        "\n/// The slices whose command opens a sheet of their own (`Feature::OPENS` is `Some`), one\n\
         /// line each, in the form of core's `feature_list!`. Written by `build.rs` from core's\n\
         /// slices; a slice joins it with its `impl FeatureSheet` (the rule\n\
         /// `feature_sheet_only_with_opens` checks the impls against core).\n\
         macro_rules! sheet_list {\n    ($m:ident) => {\n        $m! {\n",
    );
    for slice in slices.iter().filter(|slice| slice.opens) {
        out.push_str(&format!(
            "            {} => {};\n",
            slice.name, slice.marker
        ));
    }
    out.push_str("        }\n    };\n}\n");
    Ok(out)
}

/// `key_actions`: every file is an action.
pub fn actions_text(dir: &Path) -> Result<String, String> {
    let mut out = String::from("// generated by build.rs from registry.rs: do not edit\n\n");
    let mut entries = String::new();
    for (name, file) in module_files(dir, false)? {
        let text = read(&file)?;
        let marker = marker_after(&text, &["impl KeyAction for "]).ok_or_else(|| {
            format!(
                "{}: a file of key_actions with no `impl KeyAction for <Marker>`",
                file.display()
            )
        })?;
        out.push_str(&mod_line(&name, &file));
        entries.push_str(&format!("            {name} => {marker};\n"));
    }
    out.push_str(
        "\n/// The list of the key actions, in the order of their bindings (by file name), handed to\n\
         /// `callback` as `file => Marker;` entries. Written by `build.rs`: an action is a file.\n\
         macro_rules! action_list {\n    ($callback:ident) => {\n        $callback! {\n",
    );
    out.push_str(&entries);
    out.push_str("        }\n    };\n}\n\npub(super) use action_list;\n");
    Ok(out)
}

/// `transcript`: a file with `impl TranscriptMode for Mode` is a mode.
pub fn modes_text(dir: &Path) -> Result<String, String> {
    let mut out = String::from("// generated by build.rs from registry.rs: do not edit\n\n");
    let mut entries = String::new();
    for (name, file) in module_files(dir, false)? {
        if marker_after(&read(&file)?, &["impl TranscriptMode for "]).is_none() {
            continue;
        }
        out.push_str(&mod_line(&name, &file));
        entries.push_str(&format!("            {name};\n"));
    }
    out.push_str(
        "\n/// The list of the transcript's modes (by file name), handed to `callback` as `file;`\n\
         /// entries: `file` is the module `transcript/<file>.rs` with a `Mode` marker that\n\
         /// implements [`TranscriptMode`]. Written by `build.rs`: a mode is a file.\n\
         macro_rules! mode_list {\n    ($callback:ident) => {\n        $callback! {\n",
    );
    out.push_str(&entries);
    out.push_str("        }\n    };\n}\n");
    Ok(out)
}

/// The pattern of the `// cell: <pattern>` line among the first lines of `text`.
pub fn cell_pattern(text: &str) -> Option<String> {
    text.lines()
        .take(HEAD)
        .find_map(|line| line.strip_prefix(CELL_MARKER))
        .map(|pattern| pattern.trim().to_owned())
        .filter(|pattern| !pattern.is_empty())
}

/// `cells`: a file (or directory) whose head holds `// cell: <pattern>` is a cell.
pub fn cells_text(dir: &Path) -> Result<String, String> {
    let mut out = String::from("// generated by build.rs from registry.rs: do not edit\n\n");
    let mut entries = String::new();
    for (name, file) in module_files(dir, true)? {
        let Some(pattern) = cell_pattern(&read(&file)?) else {
            continue;
        };
        out.push_str(&mod_line(&name, &file));
        entries.push_str(&format!("            {pattern} => {name};\n"));
    }
    out.push_str(
        "\n/// The list of the kinds of transcript item, one line per `Item` pattern (by file name),\n\
         /// handed to `callback` as `pattern => file;` entries: `file` is the module\n\
         /// `cells/<file>.rs` whose `view(item, env, place)` builds the cell of the items the\n\
         /// pattern matches. Written by `build.rs` from the `// cell: <pattern>` line of the files'\n\
         /// heads; the match the callback makes is exhaustive, so an `Item` no file names does not\n\
         /// compile. What a feature draws for a note of its own is `FeatureView::cell`.\n\
         macro_rules! cell_list {\n    ($callback:ident) => {\n        $callback! {\n",
    );
    out.push_str(&entries);
    out.push_str("        }\n    };\n}\n\nuse cell_list;\n");
    Ok(out)
}

/// The text of a hub file with the generated text below it, as the compiler sees it after the
/// `include!`: what the rules read.
pub fn with_generated(hub: &str, generated: &str) -> String {
    format!("{hub}\n{generated}")
}
