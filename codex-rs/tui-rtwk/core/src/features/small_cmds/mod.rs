//! The small commands as a feature slice (ADR-012; oracle `app/event_dispatch.rs` `Logout`,
//! `chatwidget/ide_context.rs` `handle_ide_command`, `chatwidget/slash_dispatch.rs` +
//! `app/daybreak.rs` (`/daybreak`), `debug_config.rs` (`/debug-config`)): `/logout`, `/ide`,
//! `/daybreak` and `/debug-config`. Each asks the host for what is its IO (the account's logout,
//! the IDE's context, the thread's Daybreak choice saved, the config as the host holds it) and
//! says how it went as a note of the thread in use.
//!
//! What is kept here is what IO needs: whether `/ide` is on (a message that goes out asks the
//! host for the IDE's context while it is) and, in each thread's part, the Daybreak choice saved
//! for it, which the thread's metadata does not know until it is read again
//! (`daybreak_of`). The
//! indicator of the IDE context, the status line's Daybreak and the notes' words are the UI
//! half's (`src/features/small_cmds/`); a logout that succeeded is a `Wish::Shutdown` the
//! surface carries out (the oracle shows its shutdown feedback and exits).
//!
//! `/debug-m-drop` and `/debug-m-update` are the oracle's own stub (`Memory maintenance: Not
//! available in TUI yet.`, `Stub::MemoryMaintenance`) and stay so.

mod data;
mod ide;
mod rpcs;
mod seed;

pub(crate) use ide::{ide_opted_in, ide_permit, settle_ide_submit};

use codex_app_server_protocol::{ClientRequest, RequestId};

use super::{AppCx, Connection, Feature, Never, Slash, ThreadCx, ThreadView};
use crate::app::effects::Done;
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

pub use crate::app::effects::{IdeFailure, IdeReading};
pub use data::{DaybreakFailure, DefaultFailure, Row, Style};
pub use ide::Ide;
pub use rpcs::{Command, Reply};

/// The marker type of the feature.
pub struct SmallCmds;

/// The Daybreak choice of the thread: what `/daybreak` saved for it, else what its metadata
/// holds (`Metadata::daybreak`).
pub fn daybreak_of(thread: ThreadView<'_>) -> Option<bool> {
    thread
        .parts()
        .and_then(|parts| parts.small_cmds.saved())
        .or(thread.slots().chat.daybreak)
}

/// The thread is not kept (`config.ephemeral`): it has no metadata to save a choice in.
pub fn ephemeral(thread: ThreadView<'_>) -> bool {
    thread.slots().chat.ephemeral
}

/// What the user asks of the small commands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/logout`.
    Logout,
    /// `/daybreak`, typed in the thread `source`.
    Daybreak { source: Option<ThreadId> },
    /// `/ide`, `/ide on`, `/ide off`, `/ide status`: `args` as typed after the command.
    Ide {
        args: Option<String>,
        source: Option<ThreadId>,
    },
    /// `/debug-config`.
    DebugConfig,
}

/// What the app asks of a thread's part of the feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadIntent {
    /// The choice `/daybreak` saved for the thread.
    Daybreak(bool),
}

/// What the feature keeps per thread: the Daybreak choice `/daybreak` saved, which the thread's
/// metadata does not know until it is read again (`daybreak_of` reads it first).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {
    daybreak: Option<bool>,
}

impl ThreadPart {
    /// The choice `/daybreak` saved for the thread, if it did.
    pub fn saved(&self) -> Option<bool> {
        self.daybreak
    }
}

/// The app-scoped state of the feature.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub ide: Ide,
    /// The rows of `/debug-config` as the host last built them (`Fact::DebugConfig`).
    debug_config: Vec<Row>,
}

/// What an ask of the server was for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Logout,
    /// The thread's metadata was asked to hold the choice.
    DaybreakSave {
        thread: ThreadId,
        enabled: bool,
    },
    /// The default preference was asked to be written.
    DaybreakDefault {
        enabled: bool,
    },
    IdeProbe {
        thread: ThreadId,
        generation: u64,
    },
}

/// What the host tells the feature of what only it can read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    /// The rows of `/debug-config` for the config in force.
    DebugConfig(Vec<Row>),
    /// `/ide` is on and an IDE answers: the state a script starts a thread in, the indicator
    /// shown and no note said (what `/ide on` leaves, bar the line in the transcript).
    IdeOn(IdeReading),
}

impl Fact {
    /// The rows of `/debug-config`, each with the style name (`plain`, `bold`, `dim`,
    /// `magenta`) of its line.
    pub fn debug_config<'a>(rows: impl IntoIterator<Item = (String, &'a str)>) -> Fact {
        Fact::DebugConfig(
            rows.into_iter()
                .map(|(text, style)| Row {
                    text,
                    style: match style {
                        "bold" => Style::Bold,
                        "dim" => Style::Dim,
                        "magenta" => Style::Magenta,
                        _ => Style::Plain,
                    },
                })
                .collect(),
        )
    }
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
    /// The command has no conversation on a local filesystem.
    IdeNoLocalWorkspace,
    /// The original message went through after a failed or timed-out read.
    IdeSkipped { hint: Option<String> },
    /// `Usage: /ide [on|off|status]` (error).
    IdeUsage,
    /// The config as the host holds it, one row a line.
    DebugConfig(Vec<Row>),
}

/// `SlashCommand::Daybreak`: the checks of the chat widget, then the choice saved for the
/// thread (`persist_daybreak_selection`). Turned on needs an eligible account and a catalog
/// that advertises Daybreak; turned off needs nothing.
fn daybreak(source: Option<ThreadId>, cx: &mut AppCx<'_>) {
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
        .and_then(|id| view.thread(id).and_then(daybreak_of))
        .unwrap_or(false);
    if !enabled {
        let access = view.facts().access();
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
    // an ephemeral thread has no metadata to save the choice in (`thread.ephemeral`)
    if view.thread(&thread).is_some_and(ephemeral) {
        return daybreak_saved(thread, !enabled, cx);
    }
    let at = thread.clone();
    cx.ask(
        Some(&at),
        Command::SetDaybreak(!enabled),
        Ctx::DaybreakSave {
            thread,
            enabled: !enabled,
        },
    );
}

/// The choice is saved in the thread (or the thread has no metadata to save it in): the thread
/// shows it, the default preference follows (`persist_daybreak_selection`).
fn daybreak_saved(thread: ThreadId, enabled: bool, cx: &mut AppCx<'_>) {
    cx.thread(&thread, ThreadIntent::Daybreak(enabled));
    cx.ask(
        None,
        Command::SaveDefault(enabled),
        Ctx::DaybreakDefault { enabled },
    );
}

impl Feature for SmallCmds {
    const NAME: &'static str = "small_cmds";
    const SEED: Option<crate::features::SeedFn<Fact>> = Some(seed::seed);
    const RUNS: &'static [SlashCommand] = &[
        SlashCommand::Logout,
        SlashCommand::Ide,
        SlashCommand::Daybreak,
        SlashCommand::DebugConfig,
    ];

    type State = State;
    type ThreadPart = ThreadPart;
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = ThreadIntent;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Fact;
    type Wish = Wish;
    type Interest = Never<Self>;
    type Event = Never<Self>;

    /// `/logout`, `/ide [on|off|status]`, `/daybreak`, `/debug-config`.
    fn slash(state: &mut State, slash: Slash, cx: &mut AppCx<'_>) {
        let intent = match slash.command {
            SlashCommand::Logout => Intent::Logout,
            SlashCommand::Ide => Intent::Ide {
                args: (!slash.args.is_empty()).then_some(slash.args),
                source: slash.source,
            },
            SlashCommand::Daybreak => Intent::Daybreak {
                source: slash.source,
            },
            SlashCommand::DebugConfig => Intent::DebugConfig,
            _ => return,
        };
        Self::intent(state, intent, cx);
    }

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Logout => cx.ask(None, Command::Logout, Ctx::Logout),
            Intent::Daybreak { source } => daybreak(source, cx),
            Intent::Ide { args, source } => state.ide.command(args, source, cx),
            Intent::DebugConfig => {
                let rows = state.debug_config.clone();
                cx.note(None, Note::DebugConfig(rows));
            }
        }
    }

    fn on_fact(state: &mut State, fact: Fact, _cx: &mut AppCx<'_>) {
        match fact {
            Fact::DebugConfig(rows) => state.debug_config = rows,
            Fact::IdeOn(_) => {
                state.ide.enabled = true;
            }
        }
    }

    fn on_connection(state: &mut State, connection: Connection, _cx: &mut AppCx<'_>) {
        if connection == Connection::ShuttingDown {
            state.ide.retire();
        }
    }

    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        if let (Ctx::IdeProbe { thread, generation }, Done::IdeContext(reading)) = (ctx, done) {
            state.ide.probed(thread, generation, reading, cx);
        }
    }

    fn answered(_state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        let failure = |error: RpcError| match error {
            RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => text,
            RpcError::Mismatch | RpcError::Provider(_) => String::new(),
        };
        match (ctx, result) {
            (Ctx::Logout, Ok(_)) => {
                let scope = cx.view().active().cloned().unwrap_or_default();
                cx.wish(scope, Wish::Shutdown);
            }
            (Ctx::Logout, Err(error)) => cx.note(None, Note::LogoutFailed(failure(error))),
            (Ctx::DaybreakSave { thread, enabled }, Ok(_)) => daybreak_saved(thread, enabled, cx),
            (Ctx::DaybreakSave { .. }, Err(error)) => cx.note(
                None,
                Note::DaybreakFailed(DaybreakFailure::Save(failure(error))),
            ),
            (Ctx::DaybreakDefault { enabled }, result) => {
                if let Err(error) = result {
                    cx.note(
                        None,
                        Note::DaybreakDefault(DefaultFailure::Save(failure(error))),
                    );
                }
                cx.note(None, Note::DaybreakSet(enabled));
            }
            (Ctx::IdeProbe { .. }, _) => {}
        }
    }

    fn thread_intent(part: &mut ThreadPart, intent: ThreadIntent, _cx: &mut ThreadCx<'_, '_>) {
        let ThreadIntent::Daybreak(enabled) = intent;
        part.daybreak = Some(enabled);
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::DaybreakNotAvailable
            | Note::DaybreakSet(_)
            | Note::IdeOff
            | Note::IdeOn { .. }
            | Note::IdeCouldNotEnable { .. }
            | Note::IdeNoLocalWorkspace
            | Note::IdeSkipped { .. }
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

    fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
        command.request(thread, id)
    }

    fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
        command.reply(result)
    }
}
