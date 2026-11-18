//! The pty scenarios: `codex` (the oracle) and `codex-rtwk` in a pseudo-terminal
//! against the same mocked model, compared at named checkpoints
//! (ADR-007 §2, research/test-infra.md §D3). The scenario tests are opt-in;
//! `tests/scenarios/README.md` has the command. The unit tests of the
//! harness itself (`unit.rs`) run with the normal suite.
#![allow(clippy::disallowed_methods)]

#[path = "../support/mod.rs"]
mod support;

mod unit;
