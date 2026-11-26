//! `codex resume <id>` and `codex fork <id>` at launch: the session the command
//! line names is read once the sign-in steps are over, the working directory
//! it opens in is decided (a prompt when it differs from this one and nothing
//! was remembered), and the app opens it in place of a fresh thread. Oracle:
//! `lib.rs` `run_ratatui_app` (`lookup_session_target_with_app_server`,
//! `missing_session_exit`, `resolve_cwd_for_resume_or_fork_selection`).
//!
//! The read is the app's (`thread/read` is a thread RPC: only `ThreadFx` sends
//! it); this model keeps where the launch stands and says what to ask next.

use super::{Ask, Model, ask};
use crate::app::AppFx;
use crate::screens::startup::cwd::{self, Plan};
use crate::screens::startup::{Action, Answer, Prompt};

/// The session the command line names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Start {
    pub action: Action,
    pub id: String,
}

impl Start {
    /// The words of the oracle's refusal for an id nothing is saved under.
    fn missing(&self) -> String {
        let verb = match self.action {
            Action::Resume => "resume",
            Action::Fork => "fork",
        };
        format!(
            "No saved session found with ID {}. Run `codex {verb}` without an ID to choose from existing sessions.",
            self.id
        )
    }
}

/// What the read found of the session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    /// The directory it recorded (none where the server gave none).
    pub cwd: Option<String>,
    /// The thread's provider and whether the server holds it in memory.
    pub thread: Option<Resumed>,
}

/// The provider of a thread the launch resumes: the device-code sign-in is
/// for it when the launch chose none, or when the server still has the thread
/// loaded (`device_login::start_if_needed`: a thread in memory is reattached
/// with its current provider).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resumed {
    pub provider: String,
    pub loaded: bool,
}

/// Where the launch stands with that session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Phase {
    /// No session was named: a fresh thread starts.
    #[default]
    None,
    /// Named; the app has not asked for it yet (or the connection that was
    /// asked for it is gone).
    Wanted(Start),
    /// `thread/read` is out.
    Reading(Start),
    /// Read.
    Read(Start, Found),
    /// Nothing is saved under that id: the launch ends once its screens are.
    Missing(Start),
    /// Read, with the opening decided: the directory (`None`: the server's),
    /// and the prompt that asks for it, until it is answered.
    Planned {
        start: Start,
        cwd: Option<String>,
        prompt: Option<crate::screens::startup::CwdPrompt>,
        thread: Option<Resumed>,
    },
    /// The launch ends with a message (asked of the host once).
    Ended,
    /// The session was handed to the app.
    Opened,
}

/// What the app opens: the session and the directory it opens in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opening {
    pub start: Start,
    pub cwd: Option<String>,
}

/// The command line names `start`: set once, before the app connects.
pub fn want(model: &mut Model, start: Start) {
    model.session = Phase::Wanted(start);
}

/// The session to ask the server for, once: `thread/read`. Asked again after
/// the connection was lost with the read in flight (`disconnected`).
pub fn to_read(model: &mut Model) -> Option<Start> {
    let Phase::Wanted(start) = &model.session else {
        return None;
    };
    let start = start.clone();
    model.session = Phase::Reading(start.clone());
    Some(start)
}

/// The read's answer: what the session recorded, or why it could not be read
/// (an id nothing is saved under).
pub fn read(model: &mut Model, answer: Result<Found, ()>, fx: &mut AppFx) {
    let Phase::Reading(start) = &model.session else {
        return;
    };
    let start = start.clone();
    model.session = match answer {
        Ok(found) => Phase::Read(start, found),
        Err(()) => Phase::Missing(start),
    };
    super::follow(model, fx);
}

/// The read was lost with the connection: it is asked again.
pub(crate) fn lost(model: &mut Model) {
    if let Phase::Reading(start) = &model.session {
        model.session = Phase::Wanted(start.clone());
    }
}

/// What the device-code sign-in waits for.
pub(super) enum Provider {
    /// A resumed session is not read yet: its provider is not known.
    Wait,
    /// Ask now: the resumed thread's provider (none for a new thread, a fork,
    /// or a session that could not be read).
    Ask(Option<Resumed>),
}

/// The provider of a resumed thread, for the device ask: the oracle reads the
/// thread before it asks (`device_login::start_if_needed`); a fork does not.
pub(super) fn provider(model: &Model) -> Provider {
    match &model.session {
        Phase::Wanted(start) | Phase::Reading(start) if start.action == Action::Resume => {
            Provider::Wait
        }
        Phase::Read(start, found) if start.action == Action::Resume => {
            Provider::Ask(found.thread.clone())
        }
        Phase::Planned { start, thread, .. } if start.action == Action::Resume => {
            Provider::Ask(thread.clone())
        }
        _ => Provider::Ask(None),
    }
}

/// Where the launch goes on after the steps: `Wait` until the session is read
/// and decided (or the launch ended), `Go` otherwise.
pub(super) enum Step {
    Go,
    Wait,
}

pub(super) fn follow(model: &mut Model, fx: &mut AppFx) -> Step {
    match std::mem::take(&mut model.session) {
        Phase::None => Step::Go,
        phase @ (Phase::Wanted(_) | Phase::Reading(_)) => {
            model.session = phase;
            Step::Wait
        }
        phase @ (Phase::Planned { .. } | Phase::Opened) => {
            model.session = phase;
            Step::Go
        }
        Phase::Ended => {
            model.session = Phase::Ended;
            Step::Wait
        }
        Phase::Missing(start) => {
            ask(
                fx,
                Ask::FailLaunch {
                    message: start.missing(),
                },
            );
            model.session = Phase::Ended;
            Step::Wait
        }
        Phase::Read(start, found) => {
            let Found {
                cwd: session,
                thread,
            } = found;
            match cwd::plan(start.action, &model.resume, session.as_deref()) {
                Plan::Open(cwd) => {
                    model.session = Phase::Planned {
                        start,
                        cwd,
                        prompt: None,
                        thread,
                    };
                    Step::Go
                }
                Plan::Ask(prompt) => {
                    model.prompts.insert(0, Prompt::Cwd(prompt.clone()));
                    model.session = Phase::Planned {
                        start,
                        cwd: None,
                        prompt: Some(prompt),
                        thread,
                    };
                    Step::Go
                }
                Plan::Fail(message) => {
                    ask(fx, Ask::FailLaunch { message });
                    model.session = Phase::Ended;
                    Step::Wait
                }
            }
        }
    }
}

/// The session to open, once its directory is decided: asked for by the
/// prompt, the answer is the last of the startup screen's answers.
pub fn opening(model: &Model, answers: &[Answer]) -> Option<Opening> {
    let Phase::Planned {
        start, cwd, prompt, ..
    } = &model.session
    else {
        return None;
    };
    let cwd = match prompt {
        None => cwd.clone(),
        Some(prompt) => {
            let selection = answers.iter().rev().find_map(|answer| match answer {
                Answer::Cwd(selection) => Some(*selection),
                _ => None,
            })?;
            Some(cwd::chosen(prompt, selection).to_owned())
        }
    };
    Some(Opening {
        start: start.clone(),
        cwd,
    })
}

/// The session was handed to the app: no second open.
pub fn opened(model: &mut Model) {
    model.session = Phase::Opened;
}

/// Whether a session was named and is not opened yet: the first thread is
/// not a fresh one.
pub fn pending(model: &Model) -> bool {
    !matches!(model.session, Phase::None | Phase::Opened | Phase::Ended)
}
