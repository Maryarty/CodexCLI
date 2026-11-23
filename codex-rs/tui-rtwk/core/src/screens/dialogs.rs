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

pub mod prompt;
pub use prompt::{Prompt, PromptKind, Submitted, Suggestion};

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

/// The two pages of the shortcut overlay (`bottom_pane/shortcut_overlay.rs`:
/// `ShortcutView`): the list of bindings and the keyboard diagram.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShortcutView {
    #[default]
    List,
    Keyboard,
}

/// One warning of the viewer (`history_cell::WarningEntry`): where it comes
/// from and its full text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub source: String,
    pub details: String,
}

/// A frozen set of warnings shown one at a time (`warnings_view.rs`:
/// `WarningsView`); the scroll offset inside a page is the view's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warnings {
    pub entries: Vec<Warning>,
    pub current: usize,
    /// Pages that were shown (the oracle: drawn).
    pub visited: BTreeSet<usize>,
    /// Pages the user kept with `k`.
    pub kept: BTreeSet<usize>,
}

impl Warnings {
    pub fn new(entries: Vec<Warning>) -> Self {
        let visited = if entries.is_empty() {
            BTreeSet::new()
        } else {
            BTreeSet::from([0])
        };
        Self {
            entries,
            current: 0,
            visited,
            kept: BTreeSet::new(),
        }
    }

    /// The warning on the page shown.
    pub fn page(&self) -> Option<&Warning> {
        self.entries.get(self.current)
    }
}

/// What the user decided about the warnings when the viewer closed
/// (`AppEvent::UpdateWarnings`): the effect on the transcript is the cells'.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub dismissed: Vec<Warning>,
    pub kept: Vec<Warning>,
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
}

/// A step of the feedback flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feedback {
    /// "How was this?"
    Category,
    /// "Include logs and session history?"
    Consent {
        category: FeedbackCategory,
        attachments: Attachments,
    },
}

/// The small dialog open now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Open {
    Shortcuts(ShortcutView),
    Warnings(Warnings),
    Feedback(Feedback),
    /// A text prompt (`/rename`): the field is the view's.
    Prompt(Prompt),
}

/// The note the flow ends in (`AppEvent::OpenFeedbackNote`): the note view
/// itself is not carried yet (parity pack, Open).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteRequest {
    pub category: FeedbackCategory,
    pub include_logs: bool,
}

/// The app-scoped model state of this screen, changed only by `apply`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Model {
    pub open: Option<Open>,
    /// Decisions made when a warnings viewer closed.
    pub decided: Vec<Decision>,
    /// Warning texts copied (`AppEvent::CopyWarning`).
    pub copied: Vec<String>,
    /// The note the feedback flow asked for last.
    pub note: Option<NoteRequest>,
    /// The warnings the session keeps (startup and config warnings): the
    /// footer counts them and F2 / `/warnings` shows them. A page seen and
    /// left is dismissed and leaves the list.
    pub retained: Vec<Warning>,
    /// The prompt the user submitted and the app has not yet carried out.
    pub submitted: Option<Submitted>,
    /// How diagnostics are taken in (startup or runtime, what was seen, what was dismissed).
    pub intake: Intake,
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A small dialog was opened.
    Opened(Open),
    /// Warnings the session retains (a startup warning of the config).
    Retained(Vec<Warning>),
}

/// The warnings viewer's actions (`WarningsView::handle_key`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageIntent {
    Previous,
    Next,
    /// `k`: keep this one and show the next; on the last page, close.
    KeepAndNext,
    Copy,
}

/// The intents of this screen; a variant of the root `ScreenIntent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// The dialog was closed (Esc, `?`, F2).
    Close,
    /// Tab in the shortcut overlay: list and keyboard swap.
    ToggleKeyboard,
    Page(PageIntent),
    /// A row of the category picker was picked.
    PickCategory(usize),
    /// The consent step was answered: include logs or not.
    Consent(bool),
    /// F2 or `/warnings`: the viewer over the retained warnings (none: its
    /// empty page).
    OpenWarnings,
    /// The prompt's field changed by the user (a key or a paste).
    Typed,
    /// Enter in the prompt: its text; the prompt closes and the app carries it out.
    Submit(String),
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Opened(open) => model.open = Some(open),
        Ev::Retained(entries) => apply_retained(model, entries),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = fx;
    match intent {
        Intent::Close => close(model),
        Intent::OpenWarnings => {
            model.open = Some(Open::Warnings(Warnings::new(model.retained.clone())));
        }
        Intent::ToggleKeyboard => {
            if let Some(Open::Shortcuts(view)) = &mut model.open {
                *view = match view {
                    ShortcutView::List => ShortcutView::Keyboard,
                    ShortcutView::Keyboard => ShortcutView::List,
                };
            }
        }
        Intent::Page(page) => apply_page(model, page),
        Intent::PickCategory(index) => {
            if let (Some(Open::Feedback(step @ Feedback::Category)), Some(category)) =
                (&mut model.open, FeedbackCategory::ALL.get(index))
            {
                *step = Feedback::Consent {
                    category: *category,
                    attachments: Attachments::default(),
                };
            }
        }
        Intent::Typed => {
            if let Some(Open::Prompt(prompt)) = &mut model.open {
                prompt.typed();
            }
        }
        Intent::Submit(text) => {
            if let Some(Open::Prompt(prompt)) = model.open.take() {
                model.submitted = Some(Submitted {
                    kind: prompt.kind,
                    text,
                });
            }
        }
        Intent::Consent(include_logs) => {
            if let Some(Open::Feedback(Feedback::Consent { category, .. })) = &model.open {
                model.note = Some(NoteRequest {
                    category: *category,
                    include_logs,
                });
                model.open = None;
            }
        }
    }
}

/// Closes the dialog; a warnings viewer first records what the user decided
/// (`WarningsView::close`): kept pages are kept, the other shown pages are
/// dismissed, pages never shown are left alone.
fn close(model: &mut Model) {
    if let Some(Open::Warnings(warnings)) = model.open.take() {
        let mut dismissed = Vec::new();
        let mut kept = Vec::new();
        for (index, entry) in warnings.entries.into_iter().enumerate() {
            if warnings.kept.contains(&index) {
                kept.push(entry);
            } else if warnings.visited.contains(&index) {
                dismissed.push(entry);
            }
        }
        for gone in &dismissed {
            model.intake.dismissed.insert(gone.details.clone());
            if let Some(at) = model.retained.iter().position(|w| w == gone) {
                model.retained.remove(at);
            }
        }
        if !dismissed.is_empty() || !kept.is_empty() {
            model.decided.push(Decision { dismissed, kept });
        }
    }
    model.open = None;
}

/// `WarningsView::handle_key`: pages, keep and copy.
fn apply_page(model: &mut Model, page: PageIntent) {
    let Some(Open::Warnings(warnings)) = &mut model.open else {
        return;
    };
    let last = warnings.entries.len().saturating_sub(1);
    match page {
        PageIntent::Previous => warnings.current = warnings.current.saturating_sub(1),
        PageIntent::Next => warnings.current = (warnings.current + 1).min(last),
        PageIntent::KeepAndNext => {
            warnings.kept.insert(warnings.current);
            if warnings.current >= last {
                close(model);
                return;
            }
            warnings.current += 1;
        }
        PageIntent::Copy => {
            if let Some(entry) = warnings.page() {
                let text = entry.details.clone();
                model.copied.push(text);
            }
            return;
        }
    }
    warnings.visited.insert(warnings.current);
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

/// The seed forms of this screen's events: `{"kind":"shortcuts"}`,
/// `{"kind":"warnings","entries":[{"source":..,"details":..}]}`,
/// `{"kind":"retained","entries":[..]}` (the retained list, nothing opened),
/// `{"kind":"feedback_categories"}` and
/// `{"kind":"feedback_consent","category":"bug","can_save":true,..}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Shortcuts,
    Warnings { entries: Vec<SeedWarning> },
    FeedbackCategories,
    Retained { entries: Vec<SeedWarning> },
    FeedbackConsent {
        category: FeedbackCategory,
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
    Ok(Ev::Opened(match seed {
        SeedEv::Retained { entries } => {
            return Ok(Ev::Retained(
                entries
                    .into_iter()
                    .map(|entry| Warning {
                        source: entry.source,
                        details: entry.details,
                    })
                    .collect(),
            ));
        }
        SeedEv::Shortcuts => Open::Shortcuts(ShortcutView::List),
        SeedEv::Warnings { entries } => Open::Warnings(Warnings::new(
            entries
                .into_iter()
                .map(|entry| Warning {
                    source: entry.source,
                    details: entry.details,
                })
                .collect(),
        )),
        SeedEv::FeedbackCategories => Open::Feedback(Feedback::Category),
        SeedEv::FeedbackConsent {
            category,
            attachments,
        } => Open::Feedback(Feedback::Consent {
            category,
            attachments,
        }),
    }))
}
