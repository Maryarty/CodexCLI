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
