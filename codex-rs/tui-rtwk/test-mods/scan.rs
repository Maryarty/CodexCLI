//! Which test file is a module of which test root: the scan `build.rs` writes the root macros
//! from, and `tests/rules/test_roots.rs` checks the roots against (one source, `#[path]` of
//! both). std only.
//!
//! The directories below `tests/` that hold test modules and their roots:
//!   `oracle/*.rs`   the `oracle` root, `services/*.rs` the `shell` root;
//!   `pages/`, `sheets/`, `app/`  the `screens` root, or the `pages` root for a file whose first
//!                   lines hold the marker `// test root: pages`;
//!   a file that a sibling names as `mod <stem>;` (a page's own children) is that sibling's.
//! A module is named after its file stem, so a test id (`chat::transitions::<test>`) does not
//! depend on the directory it is in.

#![allow(clippy::expect_used, clippy::disallowed_methods, dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// The roots whose module lists are generated: the macro of each name is `lib.rs`'s.
pub const ROOTS: [&str; 4] = ["screens", "pages", "oracle", "shell"];

/// The first words of the marker line: `// test root: pages`.
pub const MARKER: &str = "// test root: ";

/// The roots a marker may name (`screens` is what no marker means).
const MARKED: [&str; 2] = ["screens", "pages"];

/// How many lines from the top of a file the marker may be on.
const HEAD: usize = 10;

/// The directories below `tests/` and the root a file of each goes to; `None`: the marker says.
const DIRS: [(&str, Option<&str>); 5] = [
    ("app", None),
    ("pages", None),
    ("sheets", None),
    ("oracle", Some("oracle")),
    ("services", Some("shell")),
];

/// The module names a root declares by hand (directories with a `mod.rs`: the harness the test
/// files share): a file of these stems would clash with one.
pub const SUPPORT: [&str; 4] = ["common", "kit", "oracle_snap", "unit"];

/// One test file as a module of a root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Module {
    pub root: &'static str,
    pub name: String,
    pub path: PathBuf,
}

/// The directories `scan` reads, below `tests/`.
pub fn dirs() -> impl Iterator<Item = &'static str> {
    DIRS.iter().map(|(dir, _)| *dir)
}

fn marked_root(text: &str, path: &Path) -> Result<&'static str, String> {
    for line in text.lines().take(HEAD) {
        if let Some(value) = line.trim_end().strip_prefix(MARKER) {
            return MARKED
                .iter()
                .find(|root| **root == value.trim())
                .copied()
                .ok_or_else(|| {
                    format!(
                        "{}: `{line}` names no root of {MARKED:?}",
                        path.display()
                    )
                });
        }
    }
    Ok("screens")
}

/// Every test file below `tests` with the root it is a module of, ordered by root and name.
pub fn scan(tests: &Path) -> Result<Vec<Module>, String> {
    let mut found: Vec<Module> = Vec::new();
    for (dir, fixed) in DIRS {
        let dir = tests.join(dir);
        let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut files: Vec<(PathBuf, String, String)> = Vec::new();
        for entry in entries {
            let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            if !path.is_file() || path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let stem = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .ok_or_else(|| format!("{}: no file name", path.display()))?;
            let text =
                fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.push((path, stem, text));
        }
        files.sort();
        for (path, stem, text) in &files {
            let child = files.iter().any(|(other, _, other_text)| {
                other != path
                    && other_text
                        .lines()
                        .any(|line| line.trim() == format!("mod {stem};"))
            });
            if child {
                continue;
            }
            let root = match fixed {
                Some(root) => root,
                None => marked_root(text, path)?,
            };
            found.push(Module {
                root,
                name: stem.clone(),
                path: path.clone(),
            });
        }
    }
    found.sort_by(|a, b| (a.root, &a.name).cmp(&(b.root, &b.name)));
    for pair in found.windows(2) {
        if pair[0].root == pair[1].root && pair[0].name == pair[1].name {
            return Err(format!(
                "two test files make the module `{}` of the `{}` root: {} and {}",
                pair[0].name,
                pair[0].root,
                pair[0].path.display(),
                pair[1].path.display()
            ));
        }
    }
    if let Some(clash) = found.iter().find(|m| SUPPORT.contains(&m.name.as_str())) {
        return Err(format!(
            "{}: the stem `{}` is a support module of the roots",
            clash.path.display(),
            clash.name
        ));
    }
    Ok(found)
}

/// The text of `macros.rs`: one `macro_rules!` per root, declaring its modules by absolute path.
pub fn macros(found: &[Module]) -> String {
    let mut out = String::from("// generated by build.rs from scan.rs: do not edit\n\n");
    for root in ROOTS {
        out.push_str(&format!(
            "/// The test modules of the `{root}` root (`tests/` files, see `scan.rs`).\n\
             #[macro_export]\n\
             macro_rules! {root} {{\n    () => {{\n"
        ));
        for module in found.iter().filter(|m| m.root == root) {
            out.push_str(&format!(
                "        #[path = {:?}]\n        mod {};\n",
                module.path.to_string_lossy(),
                module.name
            ));
        }
        out.push_str("    };\n}\n\n");
    }
    out
}
