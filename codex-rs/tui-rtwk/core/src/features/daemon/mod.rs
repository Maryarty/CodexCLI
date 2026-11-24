//! `/daemon` as a feature slice (ADR-012; oracle `app/daemon_menu.rs`, `event_dispatch.rs`
//! `RunDaemonRestart`): the maintenance menu of the local background server. Core keeps what IO
//! needs and the decisions: the connection the host reports (`Fact`, `Facts`), the probe of
//! whether the server is a managed daemon (`Ask::BackgroundServer(Verify)`), the restart that runs
//! (`Ask::BackgroundServer(Restart)`), and the update that ends Codex (`State::update_wanted`, which the
//! shell reads once Codex has exited). The sheet, its rows' words and its confirmations are the UI
//! half's (`src/features/daemon/`): it reads `State::page`, says the user's choice as a `Verb`
//! and shows the `Wish::Menu` pages the feature asks for.

mod model;

use codex_app_server_protocol::{ClientRequest, RequestId};

pub use self::model::{
    Cli, Connection, Facts, Guidance, Menu, Page, Reason, Row, RowState, Source, State, Status,
    Step, Target, Verb,
};
use super::{AppCx, Feature, Never, Scope};
use crate::app::effects::{Ask, DaemonAsk, DaemonDone, Done};
use crate::commands::SlashCommand;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::threads::Tone;

/// The marker type of the feature.
pub struct Daemon;

/// What `/daemon` names (`FeatureTarget`): the feature's own sheet, opened by its UI half.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sheet;

/// What an ask of the host was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ctx {
    /// The probe of whether the server is a managed daemon: the menu opens when it answers.
    Verify,
    /// The restart of the server.
    Restart,
}

/// The page the surface is asked to open (`Scope::Thread` of the thread in use).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    Menu(Menu),
}

/// What `/daemon` says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Could not restart the background server: {error}`.
    RestartFailed(String),
}

impl Feature for Daemon {
    const NAME: &'static str = "daemon";
    const OPENS: Option<(SlashCommand, Sheet)> = Some((SlashCommand::Daemon, Sheet));

    type State = State;
    type ThreadPart = ();
    type Target = Sheet;
    type Intent = Verb;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Facts;
    type Wish = Wish;
    type Interest = Never<Self>;
    type Event = Never<Self>;

    fn intent(state: &mut State, verb: Verb, cx: &mut AppCx<'_>) {
        match state.verb(verb) {
            Some(Step::Show(menu)) => show(cx, menu),
            Some(Step::Ask(DaemonAsk::Verify)) => {
                cx.effect(Ask::BackgroundServer(DaemonAsk::Verify), Ctx::Verify)
            }
            Some(Step::Ask(DaemonAsk::Restart)) => {
                cx.effect(Ask::BackgroundServer(DaemonAsk::Restart), Ctx::Restart)
            }
            None => {}
        }
    }

    fn on_fact(state: &mut State, facts: Facts, _cx: &mut AppCx<'_>) {
        state.told(facts);
    }

    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        match (ctx, done) {
            (Ctx::Verify, Done::BackgroundServer(DaemonDone::Managed(managed))) => {
                if let Some(menu) = state.verified(managed) {
                    show(cx, menu);
                }
            }
            (Ctx::Restart, Done::BackgroundServer(DaemonDone::Restarted(result))) => {
                if let Some(error) = state.restarted(result) {
                    cx.note(None, Note::RestartFailed(error));
                }
            }
            _ => {}
        }
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::RestartFailed(_) => Tone::Error,
        }
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}

/// Asks the surface to open `menu`, for the thread in use.
fn show(cx: &mut AppCx<'_>, menu: Menu) {
    let thread = cx.view().active().cloned().unwrap_or_default();
    cx.wish(Scope::Thread(thread), Wish::Menu(menu));
}
