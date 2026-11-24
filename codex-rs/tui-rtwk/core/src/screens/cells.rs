//! `cells` (S11, streaming and history cells): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/pages/session/cells/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

mod clock;
pub use crate::threads::local_links;
mod metrics;

pub use clock::{ClockFormat, ViewerClock};
pub use metrics::{RuntimeMetrics, Totals};

use std::collections::BTreeMap;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::{ThreadId, TurnId};
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "cells";
pub const ID: &str = "S11";
pub const KIND: Kind = Kind::Part;

/// What the Daybreak setting said when a cybersecurity request was refused
/// (oracle `daybreak::Notice`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Daybreak {
    /// Not enrolled: apply for broader access.
    Apply,
    /// Enrolled and switched off.
    Disabled,
    /// The Astra model has no Daybreak.
    Astra,
    /// Switched on, and the request was limited anyway.
    Enabled,
}

/// One question of a `request_user_input` exchange as the result cell
/// shows it (oracle `RequestUserInputResultCell`: the question, whether its
/// answer is secret and whether it offered options, and the answers the
/// user gave: option labels and a `user_note: ` entry for a typed note).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnsweredQuestion {
    pub question: String,
    pub secret: bool,
    pub has_options: bool,
    pub answers: Vec<String>,
}

impl AnsweredQuestion {
    /// Whether the user answered it (an empty list is no answer).
    pub fn answered(&self) -> bool {
        !self.answers.is_empty()
    }

    /// The answers split into option labels and the typed note (oracle
    /// `split_request_user_input_answer`).
    pub fn split(&self) -> (Vec<&str>, Option<&str>) {
        let mut options = Vec::new();
        let mut note = None;
        for entry in &self.answers {
            match entry.strip_prefix("user_note: ") {
                Some(text) => note = Some(text),
                None => options.push(entry.as_str()),
            }
        }
        (options, note)
    }
}

/// What the startup card adds under the session header (oracle
/// `new_session_info`): the help of the first session, else a tip and the
/// note that the server used another model than the one asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionInfo {
    /// The first event of the session with tooltips on: the getting-started
    /// help (and no tip).
    pub help: bool,
    /// The tip's Markdown source (the host picks it from the tips catalog).
    pub tip: Option<String>,
    /// `(requested, used)` when the model differs.
    pub model_changed: Option<(String, String)>,
}

impl SessionInfo {
    /// What a conversation opens with (oracle `new_session_info`): the getting-started help
    /// on the user's first run with tooltips on; on a later run the tip (`tip` is asked only
    /// then, with tooltips on) and the note that the server used another model than the one
    /// asked for. `None`: the header stands alone (first run with tooltips off, or no tip and
    /// the same model).
    pub fn opening(
        first_run: bool,
        show_tooltips: bool,
        tip: impl FnOnce() -> Option<String>,
        requested: &str,
        used: &str,
    ) -> Option<SessionInfo> {
        let info = if first_run {
            SessionInfo {
                help: show_tooltips,
                ..SessionInfo::default()
            }
        } else {
            SessionInfo {
                help: false,
                tip: if show_tooltips { tip() } else { None },
                model_changed: (requested != used)
                    .then(|| (requested.to_owned(), used.to_owned())),
            }
        };
        (info != SessionInfo::default()).then_some(info)
    }
}

pub use crate::data::links::LinkDisplay;

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// How web links of a message are written; the oracle's default until the host knows.
    pub link_display: LinkDisplay,
    /// The viewer's 12/24-hour setting (and zone), for the separator's time.
    pub clock: ViewerClock,
    /// The runtime metrics of a turn (its id), as the host last reported them.
    pub runtime_metrics: BTreeMap<TurnId, RuntimeMetrics>,
    /// `show_raw_agent_reasoning` of the config: a reasoning block also shows the model's raw
    /// reasoning after its summary (off, as the oracle's default, until the host says).
    pub show_raw_reasoning: bool,
    /// How this run started, as the host found out; none until it says, and then no
    /// conversation opens with a startup card.
    pub startup: Option<Startup>,
}

/// What the host knows of how this run started, for the card a conversation opens with (oracle
/// `ChatWidget::show_welcome_banner` and `startup_tooltip_override`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Startup {
    /// The project's trust level was never set: the run is the user's first, and every
    /// conversation opens with the getting-started help instead of a tip (oracle `is_first_run`).
    pub first_run: bool,
    /// The model-availability notice that replaces the tip of the first conversation (oracle
    /// `prepare_startup_tooltip_override`); the first conversation takes it.
    pub tooltip: Option<String>,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// What the host found out about the terminal's hyperlinks.
    LinkDisplay(LinkDisplay),
    /// What the host found out about how the viewer writes a time (the 12-hour or 24-hour
    /// clock, the zone): a display fact, not the time (`AppEv::Clock`).
    Viewer(ViewerClock),
    /// `show_raw_agent_reasoning` of the config.
    RawReasoning(bool),
    /// How the run started.
    Startup(Startup),
    /// The totals so far of one turn's runtime metrics (they replace the earlier ones).
    RuntimeMetrics {
        turn: TurnId,
        metrics: RuntimeMetrics,
    },
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::LinkDisplay(display) => model.link_display = display,
        Ev::Viewer(clock) => model.clock = clock,
        Ev::RawReasoning(show) => model.show_raw_reasoning = show,
        Ev::Startup(startup) => model.startup = Some(startup),
        Ev::RuntimeMetrics { turn, metrics } => {
            model.runtime_metrics.insert(turn, metrics);
        }
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, fx);
    match intent {}
}

/// Per-thread state of this screen: a part of every `ThreadModel`, changed
/// only by `apply_thread` and `apply_reply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ThreadPart {}

/// An event of one thread for this screen; a variant of `ThreadEv::Screen`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ThreadEv {}

/// The RPCs of this screen: requested with `ThreadFx::request`, which stamps
/// the thread and the ticket; the shell sends them through `wire`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {}

/// The replies of those RPCs, decoded by `decode`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {}

/// What an outgoing RPC of this screen was for, kept with its ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {}

pub fn apply_thread(part: &mut ThreadPart, ev: ThreadEv, fx: &mut ThreadFx<'_>) {
    let _ = (part, fx);
    match ev {}
}

pub fn apply_reply(
    part: &mut ThreadPart,
    ctx: Ctx,
    result: Result<Reply, RpcError>,
    fx: &mut ThreadFx<'_>,
) {
    let _ = (part, result, fx);
    match ctx {}
}

/// The wire request of `command` for `thread`.
pub fn wire(command: &Command, thread: &ThreadId, id: RequestId) -> ClientRequest {
    let _ = (thread, id);
    match *command {}
}

/// The reply `result` (the JSON result of the request) is for `command`.
pub fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
    let _ = result;
    match *command {}
}

/// A script's `screen` step for this directory: the event `value` stands for
/// (`fixtures::script`). None is seedable yet; the slice gives its events a
/// serde form here and reads it, so the shared loader never changes.
/// `{"kind": "link_display", "display": "label_only" | "with_destination"}`,
/// `{"kind": "clock", "format": "twelve_hour" | "twenty_four_hour", "utc_offset": seconds}`,
/// `{"kind": "runtime_metrics", "turn": id, "metrics": {"tool_calls": {"count": n, "duration_ms": n}, ..}}`.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::LinkDisplay { display } => Ev::LinkDisplay(match display {
            SeedLinkDisplay::LabelOnly => LinkDisplay::LabelOnly,
            SeedLinkDisplay::WithDestination => LinkDisplay::WithDestination,
        }),
        SeedEv::Clock {
            format,
            utc_offset,
        } => Ev::Viewer(ViewerClock {
            format,
            utc_offset,
        }),
        SeedEv::RuntimeMetrics { turn, metrics } => Ev::RuntimeMetrics {
            turn: TurnId::new(turn),
            metrics,
        },
    })
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    LinkDisplay {
        display: SeedLinkDisplay,
    },
    Clock {
        format: ClockFormat,
        #[serde(default)]
        utc_offset: Option<i32>,
    },
    RuntimeMetrics {
        turn: String,
        metrics: RuntimeMetrics,
    },
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum SeedLinkDisplay {
    LabelOnly,
    WithDestination,
}
