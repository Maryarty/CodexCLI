//! `codex-tui-rtwk-core`: the UI-free core of the Codex terminal UI on rtwk.
//! The models and their one writer (`app`, `threads`, `screens`), ids and
//! RPC tickets (`ids`, `rpc`), the mapping from and to the app-server
//! protocol (`wire`, `threads::project`) and the fixture scripts that seed a
//! model (`fixtures`). It depends on no rtwk crate and no async runtime, and
//! does no IO: the UI crate (`codex-tui-rtwk`) draws it, its `shell` runs the
//! effects. Architecture: `docs/rtwk-port/architecture.md`, "Layers".

pub mod app;
pub mod commands;
pub mod fixtures;
pub mod ids;
pub mod rpc;
pub mod screens;
pub mod threads;
pub mod wire;
