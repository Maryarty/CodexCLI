//! JSONL event scripts: a screen's mock state as the events that make it.
//!
//! A script is a text of JSON lines, one step each; blank lines and lines
//! starting with `#` are skipped. Steps (`"step"` names the kind):
//!
//! - `seed`: model state the oracle's tests start from, made by `apply`
//!   (`{"step":"seed","seed":"session","thread":"thread-1"}`, `resume_list`).
//! - `notify`: a server notification, the protocol's own JSON in `message`.
//! - `request`: a server request, the protocol's own JSON in `message`.
//! - `key` and `resize`: inputs for a tester; the model has no use for them,
//!   `play` hands them back in order.
//!
//! The steps change the model only through `CodexApp::ingest`, the path the
//! live app takes: a script cannot build a state the app cannot reach.

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use serde::Deserialize;

use crate::app::CodexApp;

/// One line of a script.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Step {
    Seed(Seed),
    Notify { message: ServerNotification },
    Request { message: ServerRequest },
    Key { keys: String },
    Resize { width: u16, height: u16 },
}

/// Model state a script starts from.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "seed", rename_all = "snake_case")]
pub enum Seed {
    /// The session is ready with `thread` active: `Connected` and the reply
    /// to its `StartThread`, applied.
    Session { thread: String },
    /// The resume picker's list, as the fake rollouts of the app-server's
    /// tests (`create_fake_rollout`) make it.
    ResumeList { rows: Vec<RolloutSeed> },
}

/// One fake rollout: the fields `create_fake_rollout` takes.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct RolloutSeed {
    pub id: String,
    pub preview: String,
    pub model_provider: String,
    /// RFC 3339, the rollout's start.
    pub timestamp: String,
}

/// A parsed script.
#[derive(Clone, Debug)]
pub struct Script {
    pub steps: Vec<(usize, Step)>,
}

/// An input for a tester, in the order the script gave it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Key(String),
    Resize { width: u16, height: u16 },
}

/// Why a script did not parse or play; `line` counts from 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    pub line: usize,
    pub message: String,
}

/// A script played over a fresh model.
pub struct Played {
    pub app: CodexApp,
    pub inputs: Vec<Input>,
}

/// Parses `text`; the first bad line stops it.
pub fn parse(text: &str) -> Result<Script, ScriptError> {
    let _ = text;
    todo!("parse")
}

/// Plays `text` over a fresh, attached model.
pub fn build(text: &str) -> Result<Played, ScriptError> {
    let _ = text;
    todo!("build")
}

impl Script {
    /// Applies the steps to `app` through `ingest`; returns the inputs.
    pub fn play(&self, app: &mut CodexApp) -> Result<Vec<Input>, ScriptError> {
        let _ = app;
        todo!("play")
    }
}
