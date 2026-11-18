//! Support for the pty scenarios (ADR-007 §2, research/test-infra.md §D3):
//! a backend (the mock model and a fresh `CODEX_HOME`), a driver (rtwk-term's
//! `TerminalSession` over the binary's pty), the JSONL key script, the
//! normalizer and the comparison.
#![allow(clippy::disallowed_methods)]

pub mod backend;
pub mod compare;
pub mod normalize;
pub mod pty;
pub mod run;
pub mod script;
pub mod seed;
