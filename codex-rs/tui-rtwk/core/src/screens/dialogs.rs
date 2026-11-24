//! `dialogs` (S23, small dialogs: shortcut overlay, warnings, feedback): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/sheets/dialogs/` in the UI crate. Nothing
//! outside these two directories (bar their test files) needs an edit when it
//! grows.

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::RequestId;

use crate::app::AppFx;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::Kind;
use crate::threads::ThreadFx;

pub const NAME: &str = "dialogs";
pub const ID: &str = "S23";
pub const KIND: Kind = Kind::Dialog;

use std::collections::BTreeSet;

use codex_app_server_protocol::{ModelVerification, ServerNotification};

/// The oracle's `TRUSTED_ACCESS_FOR_CYBER_VERIFICATION_WARNING` (chatwidget.rs).
const TRUSTED_ACCESS_FOR_CYBER_WARNING: &str = "Your conversations have multiple flags for possible cybersecurity risk. Responses may take longer because extra safety checks are on. To get authorized for security work, join the Trusted Access for Cyber program: https://chatgpt.com/cyber";
/// The oracle's `StrictReviewRequired` warning (chatwidget/protocol.rs).
const STRICT_REVIEW_WARNING: &str =
    "This request requires additional safety checks, some tool calls might take extra time";
/// A guardian approval is not a warning (`GuardianWarning` in chatwidget/protocol.rs).
const GUARDIAN_APPROVED_PREFIX: &str = "Automatic approval review approved (";
const FALLBACK_MODEL_METADATA_PREFIX: &str = "Model metadata for `";
const FALLBACK_MODEL_METADATA_SUFFIX: &str = "` not found. Defaulting to fallback metadata; this can degrade performance and cause issues.";
/// The source label of a warning the session raised after startup, and of one raised during it.
const SOURCE_WARNING: &str = "Warning";
const SOURCE_STARTUP: &str = "Startup";

/// A diagnostic of the server (oracle `ServerNotification::Warning`, `ConfigWarning`,
/// `GuardianWarning`, `StrictReviewRequired`, `ModelVerification`): the thread it names, if any,
/// and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incoming {
    pub thread: Option<String>,
    pub message: String,
    /// A `configWarning`: during startup it also hides the same text arriving as a thread warning.
    pub config: bool,
}

/// What `notification` warns of, in the oracle's words.
pub fn incoming(notification: &ServerNotification) -> Option<Incoming> {
    let warn = |thread: Option<String>, message: String, config: bool| {
        Some(Incoming {
            thread,
            message,
            config,
        })
    };
    match notification {
        ServerNotification::Warning(n) => warn(n.thread_id.clone(), n.message.clone(), false),
        ServerNotification::ConfigWarning(n) => warn(
            None,
            match &n.details {
                Some(details) => format!("{}: {details}", n.summary),
                None => n.summary.clone(),
            },
            true,
        ),
        ServerNotification::GuardianWarning(n) if !n.message.starts_with(GUARDIAN_APPROVED_PREFIX) => {
            warn(Some(n.thread_id.clone()), n.message.clone(), false)
        }
        ServerNotification::StrictReviewRequired(n) => warn(
            Some(n.thread_id.clone()),
            STRICT_REVIEW_WARNING.to_owned(),
            false,
        ),
        ServerNotification::ModelVerification(n)
            if n.verifications.contains(&ModelVerification::TrustedAccessForCyber) =>
        {
            warn(
                Some(n.thread_id.clone()),
                TRUSTED_ACCESS_FOR_CYBER_WARNING.to_owned(),
                false,
            )
        }
        _ => None,
    }
}

/// The model slug of a fallback-metadata warning (`fallback_model_metadata_warning_slug`).
fn fallback_model_slug(message: &str) -> Option<&str> {
    message
        .strip_prefix(FALLBACK_MODEL_METADATA_PREFIX)?
        .strip_suffix(FALLBACK_MODEL_METADATA_SUFFIX)
}

/// The oracle's `WarningDisplayState`: how the session takes in diagnostics.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Intake {
    /// A turn started (or a history was replayed): later diagnostics are runtime warnings.
    pub startup_complete: bool,
    /// Initialization config warnings, which may also arrive as ordinary thread warnings.
    startup_config: BTreeSet<String>,
    fallback_models: BTreeSet<String>,
    /// Texts of the warnings the user dismissed: an unchanged diagnostic stays hidden.
    dismissed: BTreeSet<String>,
}

/// The diagnostic is new to the session: it is retained, and its row belongs to the thread.
/// `false` for one startup already showed, one dismissed unchanged, or a fallback-metadata
/// warning of a model seen before (`WarningDisplayState::should_display`, `retain`).
pub fn take_in(model: &mut Model, incoming: &Incoming) -> bool {
    let intake = &mut model.intake;
    let message = &incoming.message;
    if intake.startup_config.contains(message)
        || intake.dismissed.contains(message)
        || fallback_model_slug(message).is_some_and(|slug| !intake.fallback_models.insert(slug.to_owned()))
    {
        return false;
    }
    let source = if intake.startup_complete {
        SOURCE_WARNING
    } else {
        if incoming.config {
            intake.startup_config.insert(message.clone());
        }
        SOURCE_STARTUP
    };
    apply_retained(
        model,
        vec![Warning {
            source: source.to_owned(),
            details: message.clone(),
        }],
    );
    true
}

/// A retained list holds each text once (`warning_entries` joins duplicates by identity).
fn apply_retained(model: &mut Model, entries: Vec<Warning>) {
    for entry in entries {
        if !model.retained.iter().any(|held| held.details == entry.details) {
            model.retained.push(entry);
        }
    }
}

/// One warning of the viewer (`history_cell::WarningEntry`): where it comes
/// from and its full text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub source: String,
    pub details: String,
}

/// What the user decided about the warnings when the viewer closed
/// (`AppEvent::UpdateWarnings`): the pages it dismissed (they leave the retained list and stay
/// hidden while unchanged) and the pages it saved for later. Which pages were shown, the page
/// on screen and `k` are the viewer's own state (`src/sheets/dialogs/warnings.rs`); only the
/// outcome reaches the domain, as the verb `Intent::Decide`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub dismissed: Vec<Warning>,
    pub saved: Vec<Warning>,
}

/// The feedback categories of `/feedback` (`FeedbackCategory`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackCategory {
    Bug,
    BadResult,
    GoodResult,
    SafetyCheck,
    Other,
}

impl FeedbackCategory {
    /// The rows of the category picker, in the oracle's order.
    pub const ALL: [FeedbackCategory; 5] = [
        FeedbackCategory::Bug,
        FeedbackCategory::BadResult,
        FeedbackCategory::GoodResult,
        FeedbackCategory::SafetyCheck,
        FeedbackCategory::Other,
    ];
}

/// What the consent step lists (`feedback_consent_params`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Attachments {
    /// Saving a ZIP is offered.
    #[serde(default)]
    pub can_save: bool,
    #[serde(default)]
    pub windows_sandbox_log: bool,
    /// File names of the rollouts that would be attached.
    #[serde(default)]
    pub rollout: Option<String>,
    #[serde(default)]
    pub auto_review: Option<String>,
    /// The connectivity diagnostics the host collected (`FeedbackDiagnostics`): attached as a
    /// file, and listed under "Connectivity diagnostics" for every category but a good result.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
}

/// One connectivity diagnostic (`codex_feedback::FeedbackDiagnostic`): a headline, and the
/// details under it (`NAME = value` for a proxy variable).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Diagnostic {
    pub headline: String,
    #[serde(default)]
    pub details: Vec<String>,
}

/// The note the flow ends in (`AppEvent::OpenFeedbackNote`): the note view
/// itself is not carried yet (parity pack, Open).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRequest {
    pub category: FeedbackCategory,
    pub include_logs: bool,
}

/// The app-scoped model state of this screen, changed only by `apply`.
///
/// What is drawn (which dialog is up, the warnings viewer's page, the keyboard page of the
/// shortcut overlay, the feedback step) is the sheet's own state, ADR-013; the model keeps what
/// the app and the host read: the warnings the session retains, and the host's list of attachments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    /// The note the feedback flow asked for last.
    pub note: Option<NoteRequest>,
    /// What the consent step lists: the host's fact of the files a report would attach.
    pub attachments: Attachments,
    /// The warnings the session keeps (startup and config warnings): the
    /// footer counts them and F2 / `/warnings` shows them. A page seen and
    /// left is dismissed and leaves the list.
    pub retained: Vec<Warning>,
    /// How diagnostics are taken in (startup or runtime, what was seen, what was dismissed).
    pub intake: Intake,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// Warnings the session retains (a startup warning of the config).
    Retained(Vec<Warning>),
    /// What a feedback report would attach (the host's files).
    Attachments(Attachments),
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The warnings viewer closed: what the user dismissed and saved.
    Decide(Decision),
    /// The consent step was answered: the note the feedback flow ends in.
    RequestNote(NoteRequest),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Retained(entries) => apply_retained(model, entries),
        Ev::Attachments(attachments) => model.attachments = attachments,
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Decide(decision) => decide(model, decision),
        Intent::RequestNote(note) => model.note = Some(note),
    }
}

/// A closed warnings viewer's outcome (`WarningsView::close`): the dismissed pages leave the
/// retained list and stay hidden while unchanged; saved pages stay in it.
fn decide(model: &mut Model, decision: Decision) {
    for gone in &decision.dismissed {
        model.intake.dismissed.insert(gone.details.clone());
        if let Some(at) = model.retained.iter().position(|w| w == gone) {
            model.retained.remove(at);
        }
    }
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

/// The seed forms of this screen's events: `{"kind":"retained","entries":[{"source":..,
/// "details":..}]}` (the retained list; `warnings` is the same, for the scripts of the oracle
/// tests) and `{"kind":"attachments","can_save":true,..}` (what the consent step lists).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    #[serde(alias = "warnings")]
    Retained { entries: Vec<SeedWarning> },
    Attachments {
        #[serde(flatten)]
        attachments: Attachments,
    },
}

#[derive(Clone, Debug, serde::Deserialize)]
struct SeedWarning {
    source: String,
    details: String,
}

/// A script's `screen` step for this directory.
pub fn seed(value: serde_json::Value) -> Result<Ev, crate::screens::SeedError> {
    let seed: SeedEv = serde_json::from_value(value)
        .map_err(|err| crate::screens::SeedError::Bad(err.to_string()))?;
    Ok(match seed {
        SeedEv::Retained { entries } => Ev::Retained(
            entries
                .into_iter()
                .map(|entry| Warning {
                    source: entry.source,
                    details: entry.details,
                })
                .collect(),
        ),
        SeedEv::Attachments { attachments } => Ev::Attachments(attachments),
    })
}
