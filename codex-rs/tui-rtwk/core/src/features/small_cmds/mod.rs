//! The small commands as a feature slice (ADR-012; oracle `app/event_dispatch.rs` `Logout`,
//! `chatwidget/ide_context.rs` `handle_ide_command`, `chatwidget/slash_dispatch.rs` +
//! `app/daybreak.rs` (`/daybreak`), `debug_config.rs` (`/debug-config`)): `/logout`, `/ide`,
//! `/daybreak` and `/debug-config`. Each asks the host for what is its IO (the account's logout,
//! the IDE's context, the thread's Daybreak choice saved, the config as the host holds it) and
//! says how it went as a note of the thread in use.
//!
//! What is kept here is what IO needs: whether `/ide` is on (a message that goes out asks the
//! host for the IDE's context while it is) and the Daybreak choice saved for a thread, which
//! the thread's metadata does not know until it is read again (`State::daybreak`). The
//! indicator of the IDE context, the status line's Daybreak and the notes' words are the UI
//! half's (`src/features/small_cmds/`); a logout that succeeded is a `Wish::Shutdown` the
//! surface carries out (the oracle shows its shutdown feedback and exits).
//!
//! `/debug-m-drop` and `/debug-m-update` are the oracle's own stub (`Memory maintenance: Not
//! available in TUI yet.`, `Stub::MemoryMaintenance`) and stay so.

mod data;

use std::collections::BTreeMap;

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Feature, Never};
use crate::app::effects::{Ask, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

pub use data::{
    DaybreakFailure, DaybreakSaved, DefaultFailure, IdeFailure, IdeReading, Row, Style,
};

/// The marker type of the feature.
pub struct SmallCmds;

/// What the user asks of the small commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/logout`.
    Logout,
    /// `/daybreak`, typed in the thread `source`.
    Daybreak { source: Option<ThreadId> },
    /// `/ide`, `/ide on`, `/ide off`, `/ide status`: `args` as typed after the command.
    Ide { args: Option<String> },
    /// `/debug-config`.
    DebugConfig,
}

/// What `/ide` keeps: whether the IDE's context goes with the messages.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ide {
    pub enabled: bool,
    /// A message went out without the IDE's context and the line saying so was shown: it is
    /// not said again until the context is read again (`prompt_fetch_warned`).
    pub warned: bool,
}

/// The app-scoped state of the feature.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub ide: Ide,
    /// The Daybreak choice saved for a thread by `/daybreak`: a request record by thread,
    /// read before the thread's metadata (`State::saved_daybreak`).
    daybreak: BTreeMap<ThreadId, bool>,
}

impl State {
    /// The choice `/daybreak` saved for `thread`, if it did.
    pub fn saved_daybreak(&self, thread: &ThreadId) -> Option<bool> {
        self.daybreak.get(thread).copied()
    }
}

/// What an ask of the host was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Logout,
    /// The IDE context read for `/ide`'s status line.
    Ide,
    /// Daybreak set to `enabled` for `thread`.
    Daybreak { thread: ThreadId, enabled: bool },
    DebugConfig,
}

/// What the surface is asked to show once: a logout succeeded, the app shuts down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    Shutdown,
}

/// What the small commands say in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Logout failed: {error}` (error).
    LogoutFailed(String),
    /// `Daybreak requires the OpenAI provider with either ChatGPT sign-in or an API key with
    /// Daybreak support enabled.` (error)
    DaybreakNeedsProvider,
    /// `Daybreak is not available for this account. Apply at ...` (info).
    DaybreakNotAvailable,
    /// `Daybreak availability could not be determined from the connected host's model catalog.`
    /// (error)
    DaybreakUndetermined,
    /// `Daybreak is unavailable until the thread starts.` (error)
    DaybreakNoThread,
    /// `Failed to save Daybreak for this thread: {error}` / `Failed to read the thread to save
    /// Daybreak: {error}` (error).
    DaybreakFailed(DaybreakFailure),
    /// `Failed to save the default Daybreak preference: {error}` / `Failed to read the default
    /// Daybreak preference: {error}` (error).
    DaybreakDefault(DefaultFailure),
    /// `Daybreak {on|off}. Applies to new turns.` (info)
    DaybreakSet(bool),
    /// `IDE context is off.` (info)
    IdeOff,
    /// `IDE context is on.` with a hint by whether the IDE has context to send (info).
    IdeOn { has_context: bool },
    /// `IDE context could not be enabled.` with the failure's hint (info).
    IdeCouldNotEnable { hint: String },
    /// `Usage: /ide [on|off|status]` (error).
    IdeUsage,
    /// The config as the host holds it, one row a line.
    DebugConfig(Vec<Row>),
}

fn ide_status(state: &mut State, cx: &mut AppCx<'_>) {
    if !state.ide.enabled {
        return cx.note(None, Note::IdeOff);
    }
    let cwd = cx.view().working_directory(cx.view().active());
    cx.effect(Ask::IdeContext { cwd }, Ctx::Ide);
}

fn ide_off(state: &mut State, cx: &mut AppCx<'_>) {
    state.ide = Ide::default();
    cx.note(None, Note::IdeOff);
}

fn ide_on(state: &mut State, cx: &mut AppCx<'_>) {
    state.ide = Ide {
        enabled: true,
        warned: false,
    };
    ide_status(state, cx);
}

/// `handle_ide_command_args`.
fn ide(state: &mut State, args: Option<String>, cx: &mut AppCx<'_>) {
    match args.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
        "" if state.ide.enabled => ide_off(state, cx),
        "" | "on" => ide_on(state, cx),
        "off" => ide_off(state, cx),
        "status" => ide_status(state, cx),
        _ => cx.note(None, Note::IdeUsage),
    }
}

/// `SlashCommand::Daybreak`: the checks of the chat widget, then the choice saved for the
/// thread (`persist_daybreak_selection`). Turned on needs an eligible account and a catalog
/// that advertises Daybreak; turned off needs nothing.
fn daybreak(state: &State, source: Option<ThreadId>, cx: &mut AppCx<'_>) {
    let view = cx.view();
    // The command is listed only when the feature is on (`CliDaybreak`).
    if view
        .screens()
        .chat
        .flags
        .daybreak_command_description
        .is_none()
    {
        return;
    }
    let thread = source.filter(|id| view.thread(id).is_some());
    let enabled = thread
        .as_ref()
        .and_then(|id| {
            state
                .saved_daybreak(id)
                .or_else(|| view.thread(id).and_then(|thread| thread.daybreak()))
        })
        .unwrap_or(false);
    if !enabled {
        let access = view.daybreak();
        if !access.eligible {
            return cx.note(None, Note::DaybreakNeedsProvider);
        }
        if !access.available() {
            return cx.note(
                None,
                if access.availability() == Some(false) {
                    Note::DaybreakNotAvailable
                } else {
                    Note::DaybreakUndetermined
                },
            );
        }
    }
    let Some(thread) = thread else {
        return cx.note(None, Note::DaybreakNoThread);
    };
    cx.effect(
        Ask::DaybreakSave {
            thread: thread.clone(),
            enabled: !enabled,
        },
        Ctx::Daybreak {
            thread,
            enabled: !enabled,
        },
    );
}

impl Feature for SmallCmds {
    const NAME: &'static str = "small_cmds";

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Never<Self>;
    type Wish = Wish;
    type Interest = Never<Self>;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Logout => cx.effect(Ask::Logout, Ctx::Logout),
            Intent::Daybreak { source } => daybreak(state, source, cx),
            Intent::Ide { args } => ide(state, args, cx),
            Intent::DebugConfig => cx.effect(Ask::DebugConfig, Ctx::DebugConfig),
        }
    }

    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        match (ctx, done) {
            (Ctx::Logout, Done::LoggedOut(Ok(()))) => {
                let scope = cx.view().active().cloned().unwrap_or_default();
                cx.wish(scope, Wish::Shutdown);
            }
            (Ctx::Logout, Done::LoggedOut(Err(error))) => {
                cx.note(None, Note::LogoutFailed(error))
            }
            (Ctx::Ide, Done::Ide(Ok(reading))) => {
                state.ide.warned = false;
                cx.note(
                    None,
                    Note::IdeOn {
                        has_context: reading.has_context,
                    },
                );
            }
            (Ctx::Ide, Done::Ide(Err(failure))) => {
                state.ide = Ide::default();
                cx.note(None, Note::IdeCouldNotEnable { hint: failure.hint });
            }
            (Ctx::Daybreak { thread, enabled }, Done::Daybreak(Ok(saved))) => {
                state.daybreak.insert(thread, enabled);
                for failure in saved.defaults {
                    cx.note(None, Note::DaybreakDefault(failure));
                }
                cx.note(None, Note::DaybreakSet(enabled));
            }
            (Ctx::Daybreak { .. }, Done::Daybreak(Err(failure))) => {
                cx.note(None, Note::DaybreakFailed(failure))
            }
            (Ctx::DebugConfig, Done::DebugConfig(rows)) => {
                cx.note(None, Note::DebugConfig(rows))
            }
            _ => {}
        }
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::DaybreakNotAvailable
            | Note::DaybreakSet(_)
            | Note::IdeOff
            | Note::IdeOn { .. }
            | Note::IdeCouldNotEnable { .. }
            | Note::DebugConfig(_) => Tone::Info,
            Note::LogoutFailed(_)
            | Note::DaybreakNeedsProvider
            | Note::DaybreakUndetermined
            | Note::DaybreakNoThread
            | Note::DaybreakFailed(_)
            | Note::DaybreakDefault(_)
            | Note::IdeUsage => Tone::Error,
        }
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}
