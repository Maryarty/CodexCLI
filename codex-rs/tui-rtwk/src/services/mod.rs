//! Services: no widget, they own IO and tasks. `shell` pumps the backend's
//! command queue and runs effects (R1); `backend` is the scripted test
//! backend, reconnect backoff and the RPC send; `agents` holds the
//! agents page's history search, version words and machine facts;
//! `cli_help` is the `--help` text; `startup` the startup page's update
//! cache and update run.
//!
//! The clippy `disallowed_methods` allow below is the only one in the
//! library (the shell and its backends are the effect runners, R1);
//! `tests/rules.rs` rejects any other.
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

pub mod agents;
pub mod backend;
pub mod cli_help;
pub mod shell;
pub mod startup;
