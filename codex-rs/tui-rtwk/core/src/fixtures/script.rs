//! JSONL event scripts: a screen's mock state as the events that make it.
//!
//! A script is a text of JSON lines, one step each; blank lines and lines
//! starting with `#` are skipped. Steps (`"step"` names the kind):
//!
//! - `seed`: model state the oracle's tests start from, made by `apply`
//!   (`{"step":"seed","seed":"session","thread":"thread-1"}`).
//! - `screen`: an event of one screen directory, read by that directory's own
//!   `seed` (`core/src/screens/<dir>.rs`), so a slice never edits this file.
//! - `notify`: a server notification, the protocol's own JSON in `message`.
//! - `request`: a server request, the protocol's own JSON in `message`.
//! - `lagged`: the app server's event stream dropped `skipped` events
//!   (`{"step":"lagged","skipped":3}`; oracle: the `Lagged` event).
//! - `key` and `resize`: inputs for a tester; the model has no use for them,
//!   `play` hands them back in order.
//!
//! The steps change the model only through `CodexApp::ingest`, the path the
//! live app takes: a script cannot build a state the app cannot reach.

use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::ServerRequest;
use serde::Deserialize;

use crate::app::AppEv;
use crate::app::AppFx;
use crate::app::BackendEv;
use crate::app::CodexApp;
use crate::app::Command;
use crate::app::Ingress;
use crate::app::Session;
use crate::ids::ThreadId;
use crate::rpc::Reply;
use crate::screens;

/// One line of a script.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum Step {
    Seed(Seed),
    /// An event of a screen directory, read by that directory's own `seed`
    /// (`{"step":"screen","dir":"resume","ev":{..}}`).
    Screen {
        dir: String,
        ev: serde_json::Value,
    },
    Notify {
        message: Box<ServerNotification>,
    },
    Request {
        message: Box<ServerRequest>,
    },
    Lagged {
        skipped: usize,
    },
    Key {
        keys: String,
    },
    Resize {
        width: u16,
        height: u16,
    },
}

/// Model state a script starts from.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "seed", rename_all = "snake_case")]
pub enum Seed {
    /// The session is ready with `thread` active: `Connected` and the reply
    /// to its `StartThread`, applied.
    Session { thread: String },
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
    let mut steps = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let step = serde_json::from_str::<Step>(line).map_err(|err| ScriptError {
            line: line_no,
            message: err.to_string(),
        })?;
        steps.push((line_no, step));
    }
    Ok(Script { steps })
}

/// Plays `text` over a fresh, attached model.
pub fn build(text: &str) -> Result<Played, ScriptError> {
    let script = parse(text)?;
    let mut app = CodexApp::new();
    app.attach();
    let inputs = script.play(&mut app)?;
    Ok(Played { app, inputs })
}

impl Script {
    /// Applies the steps to `app` (attached to a connection) through
    /// `ingest`; returns the `key` and `resize` inputs in order.
    pub fn play(&self, app: &mut CodexApp) -> Result<Vec<Input>, ScriptError> {
        let mut inputs = Vec::new();
        for (line, step) in &self.steps {
            let fail = |message: &str| ScriptError {
                line: *line,
                message: message.to_string(),
            };
            match step {
                Step::Seed(Seed::Session { thread }) => {
                    if app.session().status != Session::Starting {
                        return Err(fail("the session is already seeded"));
                    }
                    let fx = ingest(app, AppEv::Connected);
                    let ticket = fx
                        .commands()
                        .iter()
                        .find_map(|command| match command {
                            Command::StartThread { ticket, .. } => Some(*ticket),
                            _ => None,
                        })
                        .ok_or_else(|| fail("`Connected` asked for no thread"))?;
                    let reply = Reply::thread_started(ThreadId::new(thread));
                    ingest(app, AppEv::Reply(ticket, reply));
                }
                Step::Screen { dir, ev } => {
                    let ev = screens::seed_ev(dir, ev.clone()).map_err(|err| ScriptError {
                        line: *line,
                        message: format!("{err:?}"),
                    })?;
                    ingest(app, AppEv::Screen(ev));
                }
                Step::Notify { message } => {
                    let ev = BackendEv::Notification(message.clone());
                    ingest(app, AppEv::Backend(ev));
                }
                Step::Request { message } => {
                    let ev = BackendEv::Request(message.clone());
                    ingest(app, AppEv::Backend(ev));
                }
                Step::Lagged { skipped } => {
                    ingest(app, AppEv::Backend(BackendEv::Lagged(*skipped)));
                }
                Step::Key { keys } => inputs.push(Input::Key(keys.clone())),
                Step::Resize { width, height } => inputs.push(Input::Resize {
                    width: *width,
                    height: *height,
                }),
            }
        }
        Ok(inputs)
    }
}

/// One event through the live app's router, under the current epoch. The
/// effects (RPCs) a script's events cause are returned, not run: a fixture
/// has no backend.
fn ingest(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let epoch = app.epoch();
    app.ingest(Ingress::new(epoch, ev)).unwrap_or_default()
}
