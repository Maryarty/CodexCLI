//! Fixtures: the data a screen's mock state is made of. It is real core
//! model state, built by the same `apply` as the live app's, never data
//! handed to a widget.
//!
//! - `builders`: protocol payloads as the app-server sends them.
//! - `script` (JSONL event scripts): `Seed`, `Notify`, `Request`, `Key`,
//!   `Resize`, applied through `apply_app`.

pub mod builders;
