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
//! A submit is the slice's own RPC (`feedback/upload`; for a ZIP `fs/createDirectory` of the
//! feedback folder under the server's `$CODEX_HOME`, then `feedback/save`), and the answer is
//! said in the transcript of the thread the report was composed in, even when another thread is
//! in use by then (the oracle buffers the submission event on its origin thread). The client's
//! own logs are not staged: the host has no tracing layer feeding `CodexFeedback`, so the logs
//! that go with `include_logs` are the server's. Words and the success cell's copy:
//! `src/features/feedback/strings.rs`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use codex_app_server_protocol::{
    ClientRequest, FeedbackSaveParams, FeedbackSaveResponse, FeedbackUploadParams,
    FeedbackUploadResponse, FsCreateDirectoryParams, RequestId,
};
use serde::Deserialize;

use super::{AppCx, Feature, Never};
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Environment {
    /// The account's audience (`AppServerSession::feedback_audience`).
    pub audience: Audience,
    /// Uploads are on (`config.feedback_enabled`); off, only a local ZIP is offered.
    pub uploads: bool,
    /// The workspace is remote: a ZIP cannot be saved on this machine
    /// (`AppServerSession::uses_remote_workspace`).
    pub remote: bool,
    /// The server's `$CODEX_HOME`, where the ZIPs are saved (none: the host did not say).
    pub home: Option<String>,
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            audience: Audience::External,
            uploads: true,
            remote: false,
            home: None,
        }
    }
}

/// What the host tells the feature, a part at a time: the config's (`Fact::config`, at start
/// and after a config write) and the account's (`Fact::account`, once it is read). A key left
/// out keeps what the state has; as a script's `fact` step every key is optional.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct Fact {
    #[serde(default)]
    audience: Option<Audience>,
    #[serde(default)]
    uploads: Option<bool>,
    #[serde(default)]
    remote: Option<bool>,
    #[serde(default)]
    home: Option<String>,
}

impl Fact {
    /// What the config and the connection say: uploads on (`config.feedback_enabled`), the
    /// server is remote, its `$CODEX_HOME`.
    pub fn config(uploads: bool, remote: bool, home: Option<String>) -> Self {
        Self {
            audience: None,
            uploads: Some(uploads),
            remote: Some(remote),
            home,
        }
    }

    /// The account's e-mail (none for an API key): an `@openai.com` one is an employee
    /// (`FeedbackAudience`).
    pub fn account(email: Option<&str>) -> Self {
        let employee = email.is_some_and(|email| email.trim().ends_with("@openai.com"));
        Self {
            audience: Some(if employee {
                Audience::OpenAiEmployee
            } else {
                Audience::External
            }),
            ..Self::default()
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

/// The slice's RPCs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `feedback/upload`.
    Upload(Upload),
    /// `fs/createDirectory` of the folder the ZIP goes in (the server saves into an existing
    /// one).
    MakeDirectory { path: String },
    /// `feedback/save` to `path`, a fresh file on the server's host.
    Save { upload: Upload, path: String },
}

/// What an RPC answered with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// The server took the report: the id it filed it under.
    Uploaded {
        report: String,
    },
    Directory,
    /// The ZIP was written here.
    Saved {
        path: String,
    },
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
        /// The name of the ZIP a save writes (time and a suffix that makes it unused); the
        /// clock is the surface's, not the domain's.
        file_name: String,
    },
}

/// What an RPC was for: the report composed in `origin`, at `step`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    pub origin: Option<ThreadId>,
    pub category: FeedbackCategory,
    pub include_logs: bool,
    pub step: Step,
}

/// Where a delivery is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    Upload,
    /// The folder is being made; the save follows.
    Directory {
        upload: Upload,
        directory: String,
        path: String,
    },
    Save,
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
        report: String,
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

/// The folder of the saved ZIPs under `$CODEX_HOME` (`FEEDBACK_DIR`).
const FEEDBACK_DIR: &str = "feedback";

/// `/feedback` and its steps' last one: the checks of the state, then the first RPC.
fn submit(state: &State, cx: &mut AppCx<'_>, upload: Upload, file_name: &str) {
    let origin = cx.view().active().cloned();
    let upload = Upload {
        thread: origin.clone(),
        ..upload
    };
    let mut ctx = Ctx {
        origin: origin.clone(),
        category: upload.category,
        include_logs: upload.include_logs,
        step: Step::Upload,
    };
    match upload.destination {
        Destination::OpenAi if !state.can_upload() => {}
        Destination::OpenAi => cx.ask(origin.as_ref(), Command::Upload(upload), ctx),
        Destination::LocalZip if !state.can_save() => cx.note(origin, Note::ZipNeedsLocalServer),
        Destination::LocalZip => {
            let Some(home) = state
                .environment
                .home
                .as_deref()
                .filter(|home| is_absolute(home))
            else {
                let cause = "The app server did not report its Codex home directory";
                return cx.note(origin, Note::Failed(format!("{SAVE_FAILED}{cause}")));
            };
            let directory = join(home, FEEDBACK_DIR);
            let path = join(&directory, file_name);
            ctx.step = Step::Directory {
                upload,
                directory: directory.clone(),
                path,
            };
            cx.ask(
                origin.as_ref(),
                Command::MakeDirectory { path: directory },
                ctx,
            );
        }
    }
}

const UPLOAD_FAILED: &str = "Failed to upload feedback: ";
const SAVE_FAILED: &str = "Failed to save feedback ZIP: ";

fn is_absolute(path: &str) -> bool {
    std::path::Path::new(path).is_absolute()
}

fn join(directory: &str, name: &str) -> String {
    PathBuf::from(directory)
        .join(name)
        .to_string_lossy()
        .into_owned()
}

impl Feature for Feedback {
    const NAME: &'static str = "feedback";
    const SEED: Option<fn(serde_json::Value) -> Result<Fact, SeedError>> = Some(seed);

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Command;
    type Reply = Reply;
    type Ctx = Ctx;
    type Note = Note;
    type Fact = Fact;
    type Wish = Never<Self>;
    type Interest = Never<Self>;
    type Event = Never<Self>;

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
                file_name,
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
                &file_name,
            ),
        }
    }

    fn on_fact(state: &mut State, fact: Fact, _cx: &mut AppCx<'_>) {
        let environment = &mut state.environment;
        environment.audience = fact.audience.unwrap_or(environment.audience);
        environment.uploads = fact.uploads.unwrap_or(environment.uploads);
        environment.remote = fact.remote.unwrap_or(environment.remote);
        if fact.uploads.is_some() {
            // The config part says the home too: none is none.
            environment.home = fact.home;
        }
    }

    /// How the delivery went, said in the thread the report was composed in; a ZIP's folder
    /// made, the save follows.
    fn answered(state: &mut State, ctx: Ctx, result: Result<Reply, RpcError>, cx: &mut AppCx<'_>) {
        let Ctx {
            origin,
            category,
            include_logs,
            step,
        } = ctx;
        let note = match (step, result) {
            (Step::Upload, Ok(Reply::Uploaded { report })) => Note::Uploaded {
                category,
                include_logs,
                report,
                audience: state.environment.audience,
            },
            (Step::Upload, Err(error)) => Note::Failed(format!("{UPLOAD_FAILED}{}", cause(&error))),
            (
                Step::Directory {
                    upload,
                    directory,
                    path,
                },
                result,
            ) => match result {
                Ok(_) => {
                    let ctx = Ctx {
                        origin: origin.clone(),
                        category,
                        include_logs,
                        step: Step::Save,
                    };
                    return cx.ask(origin.as_ref(), Command::Save { upload, path }, ctx);
                }
                Err(error) => Note::Failed(format!(
                    "{SAVE_FAILED}Could not create the feedback directory at {directory}: {}",
                    cause(&error)
                )),
            },
            (Step::Save, Ok(Reply::Saved { path })) => Note::Saved(path),
            (Step::Save, Err(error)) => Note::Failed(format!("{SAVE_FAILED}{}", cause(&error))),
            // An answer of another shape than its ask: nothing to say.
            (_, Ok(_)) => return,
        };
        cx.note(origin, note);
    }

    fn note_tone(note: &Note) -> Tone {
        match note {
            Note::Failed(_) => Tone::Error,
            _ => Tone::Info,
        }
    }

    fn wire(command: &Command, _thread: &ThreadId, request_id: RequestId) -> ClientRequest {
        match command {
            Command::Upload(upload) => ClientRequest::FeedbackUpload {
                request_id,
                params: params_of(upload),
            },
            Command::Save { upload, path } => save_request(request_id, params_of(upload), path),
            Command::MakeDirectory { path } => {
                let params = serde_json::from_value::<FsCreateDirectoryParams>(
                    serde_json::json!({ "path": path, "recursive": true }),
                );
                match params {
                    Ok(params) => ClientRequest::FsCreateDirectory { request_id, params },
                    Err(_) => refused(request_id),
                }
            }
        }
    }

    fn decode(command: &Command, result: serde_json::Value) -> Result<Reply, RpcError> {
        let malformed = |error: serde_json::Error| RpcError::Malformed(error.to_string());
        match command {
            Command::Upload(_) => serde_json::from_value::<FeedbackUploadResponse>(result)
                .map(|response| Reply::Uploaded {
                    report: response.thread_id,
                })
                .map_err(malformed),
            Command::MakeDirectory { .. } => Ok(Reply::Directory),
            Command::Save { .. } => serde_json::from_value::<FeedbackSaveResponse>(result)
                .map(|response| Reply::Saved {
                    path: response.saved_path.to_string(),
                })
                .map_err(malformed),
        }
    }
}

/// The text of a failed RPC, as the oracle puts the error after its prefix.
fn cause(error: &RpcError) -> String {
    match error {
        RpcError::Remote(text) | RpcError::Malformed(text) | RpcError::Prepare(text) => {
            text.clone()
        }
        other => format!("{other:?}"),
    }
}

/// `build_feedback_params`: the rollout is attached to an upload that includes logs (a save
/// selects the thread's transcripts by its id).
fn params_of(upload: &Upload) -> FeedbackUploadParams {
    let extra_log_files = (upload.include_logs && upload.destination == Destination::OpenAi)
        .then(|| upload.rollout.clone().map(|path| vec![PathBuf::from(path)]))
        .flatten();
    FeedbackUploadParams {
        classification: classification(upload.category).to_owned(),
        reason: upload.reason.clone(),
        thread_id: upload.thread.as_ref().map(ToString::to_string),
        include_logs: upload.include_logs,
        extra_log_files,
        tags: upload
            .turn_id
            .clone()
            .map(|turn_id| BTreeMap::from([("turn_id".to_owned(), turn_id)])),
    }
}

/// `feedback/save` to `path`: the report's own fields and the path, one object.
fn save_request(
    request_id: RequestId,
    feedback: FeedbackUploadParams,
    path: &str,
) -> ClientRequest {
    let mut params = serde_json::to_value(feedback).unwrap_or_default();
    if let Some(object) = params.as_object_mut() {
        object.insert("savePath".to_owned(), path.into());
    }
    match serde_json::from_value::<FeedbackSaveParams>(params) {
        Ok(params) => ClientRequest::FeedbackSave { request_id, params },
        Err(_) => refused(request_id),
    }
}

/// What `submit` guards against (an absolute `$CODEX_HOME`) cannot fail to decode; if it did, a
/// read that changes nothing goes out in place of the RPC, and its answer does not decode as the
/// RPC's: the report says it failed, as the oracle's does for any bad answer.
fn refused(request_id: RequestId) -> ClientRequest {
    ClientRequest::ConfigRequirementsRead {
        request_id,
        params: None,
    }
}

/// The seed form of the fact, for a script's `fact` step:
/// `{"audience":"open_ai_employee","uploads":false,"remote":true,"home":"/home/x/.codex"}`,
/// every key optional (a `home` is read with `uploads` or `remote`: the config's part).
fn seed(value: serde_json::Value) -> Result<Fact, SeedError> {
    serde_json::from_value(value).map_err(|error| SeedError::Bad(error.to_string()))
}
