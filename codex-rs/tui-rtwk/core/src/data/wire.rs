//! The protocol's plain data types a feature reads as they are (ADR-013: a value the protocol
//! already names is not mirrored by an enum of ours). A type listed here is re-exported from the
//! protocol crate, so slices, screens and the UI name it as `core::data::wire::X` and nothing
//! maps it field by field. A type that is not `Eq` (the protocol's `HookMetadata`) or carries
//! more than the screen reads (`McpServerStatus` keeps the tools' schemas) is not listed:
//! the slice keeps its own small value for it.

pub use codex_app_server_protocol::McpAuthStatus;
