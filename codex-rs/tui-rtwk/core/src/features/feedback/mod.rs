//! `/feedback` as a feature slice (ADR-012; oracle `bottom_pane/feedback_view.rs`,
//! `bottom_pane/feedback_note_view.rs`, `app/feedback_upload.rs`, `chatwidget.rs`
//! `open_feedback`): the report the user composes (a category, the consent to attach logs, a
//! note), its delivery to the server or to a local ZIP, and the cell that says how it went.
//!
//! The three steps (category list, consent list, note field) are the UI half's: what the user
//! picked is drawn state until the note is submitted (ADR-013), and only the submit crosses into
//! the domain, as the verb `Intent::Submit`. What the host knows and the session cannot ask for
//! is the feature's fact (`Environment`: whether uploads are enabled, whether the workspace is
//! remote, who the account is for); what the steps list as attachments is the dialogs' fact
//! (`screens::dialogs::Attachments`), which stays where its sheets read it.
//!
//! A submit is one ask of the host (`Ask::FeedbackUpload`: it stages the client logs, uploads or
//! saves through the server and cleans up), answered once (`Done::Feedback`), and the answer is
//! said in the transcript of the thread the report was composed in, even when another thread is
//! in use by then (the oracle buffers the submission event on its origin thread). Words and the
//! success cell's copy: `src/features/feedback/strings.rs`.

use codex_app_server_protocol::{ClientRequest, RequestId};
use serde::Deserialize;

use super::{AppCx, Feature, Never};
use crate::app::effects::{Ask, Done};
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::SeedError;
use crate::screens::dialogs::FeedbackCategory;
use crate::threads::Tone;

/// The marker type of the feature.
pub struct Feedback;

/// Who the report is for: the copy of the success cell and its follow-up link follow it
/// (`FeedbackAudience`; the account's e-mail ends with `@openai.com` for an employee).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    OpenAiEmployee,
    #[default]
    External,
}

/// Where a report goes (`FeedbackDestination`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    /// The server's feedback upload.
    OpenAi,
    /// A ZIP of the diagnostics saved on this machine (`Ctrl+S` of the note step).
    LocalZip,
}

/// What the host tells the feature about the session (`AppEv::Feature`, `Fact`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct Environment {
    /// The account's audience (`AppServerSession::feedback_audience`).
    #[serde(default)]
    pub audience: Audience,
    /// Uploads are on (`config.feedback_enabled`); off, only a local ZIP is offered.
    #[serde(default = "enabled")]
    pub uploads: bool,
    /// The workspace is remote: a ZIP cannot be saved on this machine
    /// (`AppServerSession::uses_remote_workspace`).
    #[serde(default)]
    pub remote: bool,
}

fn enabled() -> bool {
    true
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            audience: Audience::External,
            uploads: true,
            remote: false,
        }
    }
}

/// Why `/feedback` offers nothing (`open_feedback`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unavailable {
    /// Remote workspace with uploads disabled: nothing can be delivered.
    RemoteWithoutUploads,
}

/// The app state: what the host said of the session.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub environment: Environment,
}

impl State {
    /// Whether `/feedback` can open at all (`ChatWidget::open_feedback`): a remote workspace
    /// with uploads disabled can neither upload nor save a ZIP. (The oracle's cloud threads have
    /// no counterpart: this app has no cloud backend.)
    pub fn unavailable(&self) -> Option<Unavailable> {
        (self.environment.remote && !self.environment.uploads)
            .then_some(Unavailable::RemoteWithoutUploads)
    }

    /// What a report submitted now may do: upload (uploads on) and save a ZIP (workspace local).
    pub fn can_upload(&self) -> bool {
        self.environment.uploads
    }

    pub fn can_save(&self) -> bool {
        !self.environment.remote
    }
}

/// The report, as the host delivers it (`SubmitFeedback`, `build_feedback_params`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upload {
    pub category: FeedbackCategory,
    /// The note, trimmed; none when it was empty.
    pub reason: Option<String>,
    /// The thread the report was composed in (none before the session started).
    pub thread: Option<ThreadId>,
    /// The thread's last turn, tagged on the report (`turn_id`).
    pub turn_id: Option<String>,
    /// The client logs and the thread's rollout go with the report.
    pub include_logs: bool,
    /// The thread's rollout file, attached to an upload that includes logs.
    pub rollout: Option<String>,
    pub destination: Destination,
}

/// `FeedbackReceipt`: what the host's delivery came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Receipt {
    /// The server took the report: the id it filed it under.
    Uploaded { thread_id: String },
    /// The ZIP was written here.
    Saved { path: String },
}

/// What the user or a view asks of `/feedback`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    /// `/feedback` was typed in `source`: says why nothing opens when the session cannot
    /// deliver a report (`State::unavailable`).
    Open { source: Option<ThreadId> },
    /// The note step was submitted (Enter, or `Ctrl+S` for a ZIP): the report goes out.
    Submit {
        category: FeedbackCategory,
        /// The note as typed; an empty one is no reason.
        note: String,
        turn_id: Option<String>,
        rollout: Option<String>,
        include_logs: bool,
        destination: Destination,
    },
}

/// What an ask of the host was for: the report composed in `origin`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Report {
        origin: Option<ThreadId>,
        category: FeedbackCategory,
        include_logs: bool,
    },
}

/// What `/feedback` says in the transcript.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    /// `Feedback uploads are disabled, and saving a ZIP requires a local app server.`
    Unavailable,
    /// `Saving a feedback ZIP requires a local app server.`
    ZipNeedsLocalServer,
    /// The success cell of an upload (`feedback_success_cell`).
    Uploaded {
        category: FeedbackCategory,
        include_logs: bool,
        thread_id: String,
        audience: Audience,
    },
    /// `Feedback ZIP saved to {path}`.
    Saved(String),
    /// The delivery's failure text (`Failed to upload feedback: ...`, an error line).
    Failed(String),
}

/// `feedback_classification`: what the server files the report under.
pub fn classification(category: FeedbackCategory) -> &'static str {
    match category {
        FeedbackCategory::BadResult => "bad_result",
        FeedbackCategory::GoodResult => "good_result",
        FeedbackCategory::Bug => "bug",
        FeedbackCategory::SafetyCheck => "safety_check",
        FeedbackCategory::Other => "other",
    }
}

/// `should_show_feedback_connectivity_details`: connectivity diagnostics are listed for every
/// category but a good result, and only when there are any.
pub fn shows_connectivity(category: FeedbackCategory, diagnostics: usize) -> bool {
    category != FeedbackCategory::GoodResult && diagnostics > 0
}

/// The note as the report's reason: trimmed, none when nothing is left (`submit`).
pub fn reason_of(note: &str) -> Option<String> {
    let note = note.trim();
    (!note.is_empty()).then(|| note.to_owned())
}

/// `/feedback` and its steps' last one: the checks of the state, then the ask.
fn submit(state: &State, cx: &mut AppCx<'_>, upload: Upload) {
    let origin = cx.view().active().cloned();
    match upload.destination {
        Destination::OpenAi if !state.can_upload() => return,
        Destination::LocalZip if !state.can_save() => {
            return cx.note(origin, Note::ZipNeedsLocalServer);
        }
        _ => {}
    }
    let ctx = Ctx::Report {
        origin: origin.clone(),
        category: upload.category,
        include_logs: upload.include_logs,
    };
    let upload = Upload {
        thread: origin,
        ..upload
    };
    cx.effect(Ask::FeedbackUpload(upload), ctx);
}

impl Feature for Feedback {
    const NAME: &'static str = "feedback";
    const SEED: Option<fn(serde_json::Value) -> Result<Environment, SeedError>> = Some(seed);

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Never<Self>;
    type Reply = Never<Self>;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Environment;
    type Wish = Never<Self>;
    type Interest = Never<Self>;

    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        match intent {
            Intent::Open { source } => {
                if state.unavailable().is_some() {
                    cx.note(source, Note::Unavailable);
                }
            }
            Intent::Submit {
                category,
                note,
                turn_id,
                rollout,
                include_logs,
                destination,
            } => submit(
                state,
                cx,
                Upload {
                    category,
                    reason: reason_of(&note),
                    thread: None,
                    turn_id,
                    include_logs,
                    rollout,
                    destination,
                },
            ),
        }
    }

    fn on_fact(state: &mut State, fact: Environment, _cx: &mut AppCx<'_>) {
        state.environment = fact;
    }

    /// How the delivery went, said in the thread the report was composed in.
    fn effect_done(state: &mut State, ctx: Ctx, done: Done, cx: &mut AppCx<'_>) {
        let Ctx::Report {
            origin,
            category,
            include_logs,
        } = ctx;
        let note = match done {
            Done::Feedback(Ok(Receipt::Uploaded { thread_id })) => Note::Uploaded {
                category,
                include_logs,
                thread_id,
                audience: state.environment.audience,
            },
            Done::Feedback(Ok(Receipt::Saved { path })) => Note::Saved(path),
            Done::Feedback(Err(error)) => Note::Failed(error),
            _ => return,
        };
        cx.note(origin, note);
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::Failed(_) => Tone::Error,
            _ => Tone::Info,
        }
    }

    fn wire(command: &Never<Self>, _thread: &ThreadId, _id: RequestId) -> ClientRequest {
        command.absurd()
    }

    fn decode(command: &Never<Self>, _result: serde_json::Value) -> Result<Never<Self>, RpcError> {
        command.absurd()
    }
}

/// The seed form of the fact, for a script's `fact` step:
/// `{"audience":"open_ai_employee","uploads":false,"remote":true}`, every key optional.
fn seed(value: serde_json::Value) -> Result<Environment, SeedError> {
    serde_json::from_value(value).map_err(|error| SeedError::Bad(error.to_string()))
}
