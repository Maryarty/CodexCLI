//! The dev link of `codex-rtwk` pulls in all of codex-core: on macOS `ld`
//! needs over 4 GB for it and warns that `__eh_frame` is too large for the
//! compact unwind table. Without the compact unwind table it links in a few
//! seconds and in far less memory. Every link of this package gets it (the binary and its tests, which
//! link codex-core too); the dependencies' build cache is the same as every
//! other crate's.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg=-Wl,-no_compact_unwind");
    }
}
