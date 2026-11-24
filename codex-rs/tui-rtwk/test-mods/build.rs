//! Writes `macros.rs` (OUT_DIR): the module list of each test root, from `scan.rs`. It reruns when
//! a file of the scanned directories changes (cargo watches directories by the mtime of their
//! files: an edit of a test file reruns it too, which costs this crate and the roots using it,
//! not the library of `codex-tui-rtwk`).

#![allow(clippy::expect_used, clippy::disallowed_methods)]

#[path = "scan.rs"]
mod scan;

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=scan.rs");
    println!("cargo:rerun-if-changed=../core/tests");
    for dir in scan::dirs() {
        println!("cargo:rerun-if-changed=../tests/{dir}");
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests = manifest
        .parent()
        .expect("test-mods sits in the package directory")
        .join("tests");
    let found = scan::scan(&tests).unwrap_or_else(|e| panic!("test roots: {e}"));
    let core = scan::scan_core(
        &manifest
            .parent()
            .expect("the package directory")
            .join("core")
            .join("tests"),
    )
    .unwrap_or_else(|e| panic!("core test modules: {e}"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("macros.rs"), scan::macros(&found, &core)).expect("write macros.rs");
}
