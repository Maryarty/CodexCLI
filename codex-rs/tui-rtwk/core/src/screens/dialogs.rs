//! `dialogs` (S23, small dialogs: shortcut overlay, warnings, feedback): the core half of the screen's directory. The model
//! part, its events and intents, its RPCs and their protocol wiring live here,
//! with no rtwk; the view half is `src/screens/dialogs/` in the UI crate. Nothing
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
}

/// The events of this screen (backend results, tickets); a variant of the
/// root `ScreenEv`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ev {
    /// A small dialog was opened.
    Opened(Open),
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
}

pub fn apply(model: &mut Model, ev: Ev, fx: &mut AppFx) {
    let _ = fx;
    match ev {
        Ev::Opened(open) => model.open = Some(open),
    }
}

pub fn apply_intent(model: &mut Model, intent: Intent, fx: &mut AppFx) {
    let _ = (model, intent, fx);
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
/// `{"kind":"feedback_categories"}` and
/// `{"kind":"feedback_consent","category":"bug","can_save":true,..}`.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SeedEv {
    Shortcuts,
    Warnings { entries: Vec<SeedWarning> },
    FeedbackCategories,
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
