//! The rtwk pin: the commit `docs/rtwk-port/RTWK_REF` names is the commit
//! of the checkout the manifest's rtwk path dependencies point at, and no
//! dependency points at a moving rtwk checkout.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn manifest() -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("Cargo.toml")
}

fn rtwk_ref() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/rtwk-port/RTWK_REF");
    std::fs::read_to_string(path)
        .expect("RTWK_REF")
        .trim()
        .to_string()
}

/// The directories the manifest's `rtwk*` path dependencies live in.
fn rtwk_paths() -> Vec<PathBuf> {
    manifest()
        .lines()
        .filter(|line| line.trim_start().starts_with("rtwk"))
        .filter_map(|line| {
            let start = line.find("path = \"")? + "path = \"".len();
            let end = line[start..].find('"')? + start;
            Some(PathBuf::from(&line[start..end]))
        })
        .collect()
}

#[test]
fn every_rtwk_dependency_is_a_path_into_one_checkout() {
    let paths = rtwk_paths();
    assert!(
        paths.len() >= 4,
        "rtwk, rtwk-core, rtwk-tokio, rtwk-test: {paths:?}"
    );
    let roots: std::collections::BTreeSet<_> = paths
        .iter()
        .map(|path| {
            assert!(
                path.is_absolute(),
                "{path:?} must be absolute (see the parity pack)"
            );
            path.parent()
                .and_then(Path::parent)
                .expect("crates/<name>")
                .to_path_buf()
        })
        .collect();
    assert_eq!(roots.len(), 1, "one rtwk checkout: {roots:?}");
}

#[test]
fn the_checkout_is_at_the_commit_rtwk_ref_names() {
    let root = rtwk_paths()
        .first()
        .and_then(|path| path.parent())
        .and_then(Path::parent)
        .expect("a root")
        .to_path_buf();
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git runs");
    assert!(out.status.success(), "{root:?} is not a git checkout");
    let head = String::from_utf8(out.stdout).expect("utf8");
    assert_eq!(head.trim(), rtwk_ref());
}

#[test]
fn no_dependency_points_at_the_moving_rtwk_checkout() {
    let manifest = manifest();
    assert!(
        !manifest.contains("dev_projects/rtwk/"),
        "the live rtwk checkout moves; depend on the pin"
    );
    assert_eq!(rtwk_ref().len(), 40, "RTWK_REF holds one full commit id");
}

/// The text of the table `[header]` of `manifest`, up to the next table.
fn table(manifest: &str, header: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in manifest.lines() {
        if line.trim_end() == header {
            inside = true;
        } else if inside && line.starts_with('[') {
            break;
        }
        if inside {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    out.trim_end().to_string()
}

/// ADR-001: codex-rs's patches are not applied to another workspace, so
/// ours copies them verbatim and this test diffs the two.
#[test]
fn the_patch_tables_are_codex_rs_s_verbatim() {
    let outer =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../Cargo.toml"))
            .expect("codex-rs/Cargo.toml");
    for header in [
        "[patch.crates-io]",
        "[patch.\"ssh://git@github.com/openai-oss-forks/tungstenite-rs.git\"]",
    ] {
        let theirs = table(&outer, header);
        assert!(theirs.lines().count() > 1, "{header} missing in codex-rs");
        assert_eq!(table(&manifest(), header), theirs, "{header} differs");
    }
}

/// The backend pin (docs/rtwk-port/README.md): every codex backend crate is a
/// path dependency into ONE checkout, `~/.cache/codexcli/backend-pin-<BACKEND_REF[..8]>`, never
/// `../..` of the worktree, so every worktree names the same backend packages
/// at the same paths and sccache serves them to each worktree's own target.
fn backend_paths() -> Vec<(String, PathBuf)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    ["Cargo.toml", "host/Cargo.toml", "core/Cargo.toml"]
        .into_iter()
        .flat_map(|file| {
            let text = std::fs::read_to_string(dir.join(file)).expect(file);
            text.lines()
                .filter(|line| line.trim_start().starts_with("codex-"))
                .filter_map(|line| {
                    let start = line.find("path = \"")? + "path = \"".len();
                    let end = line[start..].find('"')? + start;
                    Some((
                        format!("{file}: {}", line.trim()),
                        PathBuf::from(&line[start..end]),
                    ))
                })
                .collect::<Vec<_>>()
        })
        // our own crates stay relative
        .filter(|(line, _)| {
            !line.contains("codex-tui-core =") && !line.contains("codex-tui-rtwk =")
        })
        .collect()
}

fn backend_ref() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/rtwk-port/BACKEND_REF");
    std::fs::read_to_string(path)
        .expect("BACKEND_REF")
        .trim()
        .to_string()
}

/// The pin of `BACKEND_REF` is its own directory (as the rtwk pin is): a new backend is a
/// new directory, so a worktree still on the old `BACKEND_REF` keeps building on the old one.
fn backend_pin() -> String {
    format!(
        "/.cache/codexcli/backend-pin-{}/codex-rs/",
        &backend_ref()[..8]
    )
}

#[test]
fn every_backend_dependency_is_a_path_into_the_backend_pin() {
    let paths = backend_paths();
    assert!(paths.len() >= 10, "the backend crates: {paths:?}");
    let pin = backend_pin();
    for (line, path) in &paths {
        assert!(
            path.is_absolute() && path.to_string_lossy().contains(&pin),
            "{line}: a backend crate is a path into ~{pin}, not into this worktree"
        );
    }
}

#[test]
fn the_backend_pin_is_at_the_commit_backend_ref_names() {
    let (_, first) = backend_paths().into_iter().next().expect("a backend crate");
    let s = first.to_string_lossy().into_owned();
    let pin = backend_pin();
    let root = PathBuf::from(&s[..s.find(&pin).expect("pin") + pin.len()]);
    let out = Command::new("git")
        .arg("-C")
        .arg(&root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git runs");
    assert!(out.status.success(), "{root:?} is not a git checkout");
    let head = String::from_utf8(out.stdout).expect("utf8");
    assert_eq!(
        backend_ref().len(),
        40,
        "BACKEND_REF holds one full commit id"
    );
    assert_eq!(head.trim(), backend_ref());
}

// ---- the backend patch catalog (docs/rtwk-port/backend-patches.md) ----
//
// The backend pin is upstream plus the patch series of `tools/rtwk-port/backend-patches/series`,
// so `BACKEND_REF` is the tip of that series and the catalog names it. Moving the pin without the
// series (or the series without the catalog) fails here, so a backend update never loses a patch
// silently.

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo(path: &str) -> String {
    std::fs::read_to_string(repo_root().join(path)).expect(path)
}

fn commit_of(path: &str) -> String {
    let text = read_repo(path).trim().to_string();
    assert!(
        text.len() == 40 && text.bytes().all(|b| b.is_ascii_hexdigit()),
        "{path} holds one full commit id, got {text:?}"
    );
    text
}

fn series_files() -> Vec<String> {
    let dir = repo_root().join("tools/rtwk-port/backend-patches/series");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the series directory")
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.ends_with(".patch"))
        .collect();
    names.sort();
    names
}

#[test]
fn backend_ref_is_the_tip_of_the_patch_series() {
    assert_eq!(
        commit_of("docs/rtwk-port/BACKEND_REF"),
        commit_of("tools/rtwk-port/backend-patches/series/TIP"),
        "BACKEND_REF moves with the series: apply.sh, then export.sh <base> <tip> (backend-patches.md)"
    );
}

#[test]
fn the_catalog_names_the_base_the_tip_and_every_patch_of_the_series() {
    let catalog = read_repo("docs/rtwk-port/backend-patches.md");
    for (what, path) in [
        ("base", "tools/rtwk-port/backend-patches/series/BASE"),
        ("tip", "tools/rtwk-port/backend-patches/series/TIP"),
    ] {
        let sha = commit_of(path);
        assert!(catalog.contains(&sha), "the catalog lacks the {what} {sha}");
    }
    let patches = series_files();
    assert!(!patches.is_empty(), "the series holds patch files");
    for name in &patches {
        assert!(
            catalog.contains(name.as_str()),
            "the catalog lacks the series file {name}"
        );
    }
}

#[test]
fn the_series_files_are_numbered_without_a_gap() {
    for (index, name) in series_files().iter().enumerate() {
        let want = format!("{:04}-", index + 1);
        assert!(
            name.starts_with(&want),
            "{name} should be number {}: export.sh numbers from 1",
            index + 1
        );
    }
}
