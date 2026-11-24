//! Writes `features.rs` (OUT_DIR): the `pub mod` line of every slice of `src/features/` and
//! `feature_list!`, from the directories (`test-mods/registry.rs`, fw-hubs-c5). A slice is a
//! directory with a `mod.rs` that holds an `impl Feature for <Marker>`; nothing else registers it.
//! The hub `src/features/mod.rs` `include!`s the file. It watches `src/features` only (the sources
//! of this crate anyway), never a test file.

#![allow(clippy::expect_used, clippy::disallowed_methods)]

#[path = "../test-mods/registry.rs"]
mod registry;

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../test-mods/registry.rs");
    println!("cargo:rerun-if-changed=src/features");
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("src").join("features");
    let slices = registry::core_features(&dir).unwrap_or_else(|e| panic!("core features: {e}"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(
        out.join("features.rs"),
        registry::core_features_text(&dir, &slices),
    )
    .expect("write features.rs");
}
