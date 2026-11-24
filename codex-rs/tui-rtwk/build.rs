//! Writes the registration lists of the UI crate (OUT_DIR), from the files (`test-mods/registry.rs`,
//! fw-hubs-c5): `features.rs` (the `pub mod` lines of `src/features/<f>` and `sheet_list!`),
//! `key_actions.rs` (`action_list!`), `transcript.rs` (`mode_list!`), `cells.rs` (`cell_list!`).
//! Each hub `include!`s its file. It watches `src/` directories only (the sources of this crate
//! anyway) and the slices of core's `src/features`, never a test file.

#![allow(clippy::expect_used, clippy::disallowed_methods)]

#[path = "test-mods/registry.rs"]
mod registry;

use std::path::{Path, PathBuf};

fn write(out: &Path, name: &str, text: Result<String, String>) {
    let text = text.unwrap_or_else(|e| panic!("{name}: {e}"));
    std::fs::write(out.join(name), text).unwrap_or_else(|e| panic!("write {name}: {e}"));
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=test-mods/registry.rs");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let core = manifest.join("core").join("src").join("features");
    let src = manifest.join("src");
    let session = src.join("pages").join("session");
    for dir in [&core, &src.join("features")] {
        println!("cargo:rerun-if-changed={}", dir.display());
    }
    for sub in ["key_actions", "transcript", "cells"] {
        println!("cargo:rerun-if-changed={}", session.join(sub).display());
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let slices = registry::core_features(&core).unwrap_or_else(|e| panic!("core features: {e}"));
    write(
        &out,
        "features.rs",
        registry::ui_features_text(&src.join("features"), &slices),
    );
    write(
        &out,
        "key_actions.rs",
        registry::actions_text(&session.join("key_actions")),
    );
    write(
        &out,
        "transcript.rs",
        registry::modes_text(&session.join("transcript")),
    );
    write(
        &out,
        "cells.rs",
        registry::cells_text(&session.join("cells")),
    );
}
