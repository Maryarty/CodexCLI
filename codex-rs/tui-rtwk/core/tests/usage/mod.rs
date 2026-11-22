//! State tests of S20 (the `/usage` dashboard): the pure logic of the oracle's
//! `analytics/*` carried into `codex_tui_core::screens::usage`, and the model
//! the way a fixture script seeds it.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

mod normalize;
mod plot;
mod report;
mod seed;
mod state;
mod tokens;
