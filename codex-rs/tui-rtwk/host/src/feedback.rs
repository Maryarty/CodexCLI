//! The host's side of `/feedback` (oracle `app/feedback_upload.rs` `deliver_feedback`,
//! `app/background_requests.rs` `submit_feedback`, `build_feedback_params`): the answer to
//! `Ask::FeedbackUpload`. The client's own logs are staged where the server can read them (a
//! `tui-feedback-<id>` directory under `$CODEX_HOME/tmp`, written through `fs/*`, removed at the
//! end), the report goes to the server's `feedback/upload`, or to `feedback/save` as a ZIP under
//! `$CODEX_HOME/feedback`, and the answer says what the delivery came to. In Flutter this is
//! the feedback repository the report sheet's submit would call.
//!
//! A remote server has no `$CODEX_HOME` this machine can name: the logs are then left out (the
//! report says so in its reason) and a ZIP cannot be saved, as in the oracle.

use std::collections::BTreeMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use codex_app_server_protocol::{
    ClientRequest, FeedbackSaveParams, FeedbackSaveResponse, FeedbackUploadParams,
    FeedbackUploadResponse, FsCreateDirectoryParams, FsRemoveParams, FsWriteFileParams,
};
use codex_feedback::CodexFeedback;
use codex_tui_core::app::AppEv;
use codex_tui_core::app::effects::{Done, Ask};
use codex_tui_core::features::feedback::{Destination, Receipt, Upload, classification};
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_path_uri::LegacyAppPathString;

use crate::startup::{Startup, Target};

/// The client logs sent are the last megabyte: base64 adds a third, which keeps `fs/writeFile`
/// below the 16 MiB frame limit.
const MAX_CLIENT_LOG_SIZE: usize = 1024 * 1024;
const FEEDBACK_DIR: &str = "feedback";
/// What the reason gains when the logs could not be staged.
const OMITTED: &str =
    "\n\nTUI client logs were omitted because they could not be staged on the app-server.";

/// The one `CodexFeedback` of the process: the embedded server is started with it and the
/// report snapshots its logs.
pub(crate) fn shared() -> CodexFeedback {
    static FEEDBACK: OnceLock<CodexFeedback> = OnceLock::new();
    FEEDBACK.get_or_init(CodexFeedback::new).clone()
}

/// `Ask::FeedbackUpload` as the job that answers it: the config's `$CODEX_HOME` is where the
/// logs are staged, unless the server is remote.
pub(crate) fn job<F, Fut>(
    request: u64,
    upload: Upload,
    startup: Startup,
    ids: Ids,
    send: F,
) -> codex_tui_rtwk::services::agents::workspace::Boxed<Option<AppEv>>
where
    F: Fn(ClientRequest) -> Fut + Send + 'static,
    Fut: Future<Output = Result<serde_json::Value, ServerError>> + Send,
{
    Box::pin(async move {
        let home = if startup.target == Target::Remote {
            None
        } else {
            Some(startup.current.read().await.codex_home.to_path_buf())
        };
        let done = deliver(&upload, home.as_deref(), &shared(), &ids, send).await;
        Some(AppEv::Effect { request, done })
    })
}

/// The report, delivered: `Done::Feedback` with the receipt or the failure's text.
pub(crate) async fn deliver<F, Fut>(
    upload: &Upload,
    codex_home: Option<&Path>,
    feedback: &CodexFeedback,
    ids: &Ids,
    send: F,
) -> Done
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let mut params = params_of(upload);
    let staging = codex_home
        .filter(|_| upload.include_logs)
        .map(|home| home.join("tmp").join(format!("tui-feedback-{}", unique())));
    if upload.include_logs {
        match stage(staging.as_deref(), feedback, ids, &send).await {
            Ok(path) => params.extra_log_files.get_or_insert_with(Vec::new).push(path),
            Err(error) => {
                tracing::warn!("failed to stage TUI feedback logs: {error}");
                params.reason.get_or_insert_with(String::new).push_str(OMITTED);
            }
        }
    }
    let result = match upload.destination {
        Destination::OpenAi => send(ClientRequest::FeedbackUpload {
            request_id: ids.next(),
            params,
        })
        .await
        .map_err(|error| error.message)
        .and_then(|value| {
            serde_json::from_value::<FeedbackUploadResponse>(value).map_err(|e| e.to_string())
        })
        .map(|response| Receipt::Uploaded {
            thread_id: response.thread_id,
        })
        .map_err(|cause| format!("Failed to upload feedback: {cause}")),
        Destination::LocalZip => save(codex_home, params, ids, &send)
            .await
            .map_err(|cause| format!("Failed to save feedback ZIP: {cause}")),
    };
    if let Some(directory) = staging
        && let Err(error) = remove(&directory, ids, &send).await
    {
        tracing::warn!("failed to remove staged feedback logs: {error}");
    }
    Done::Feedback(result)
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

/// The client logs written to `directory/client-logs.txt` through the server's filesystem.
async fn stage<F, Fut>(
    directory: Option<&Path>,
    feedback: &CodexFeedback,
    ids: &Ids,
    send: &F,
) -> Result<PathBuf, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let directory = directory
        .ok_or("App server did not report $CODEX_HOME; cannot stage feedback logs")?;
    make_directory(directory, ids, send).await?;
    let path = directory.join("client-logs.txt");
    let mut logs = feedback.snapshot(None).log_attachment(None).buffer;
    logs.drain(..logs.len().saturating_sub(MAX_CLIENT_LOG_SIZE));
    send(ClientRequest::FsWriteFile {
        request_id: ids.next(),
        params: FsWriteFileParams {
            path: absolute(&path)?,
            data_base64: base64(&logs),
        },
    })
    .await
    .map_err(|error| error.message)?;
    Ok(path)
}

/// `save_feedback_zip`: `feedback/save` to a fresh file under `$CODEX_HOME/feedback`.
async fn save<F, Fut>(
    codex_home: Option<&Path>,
    params: FeedbackUploadParams,
    ids: &Ids,
    send: &F,
) -> Result<Receipt, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let home = codex_home.ok_or("The app server did not report its Codex home directory")?;
    let directory = home.join(FEEDBACK_DIR);
    make_directory(&directory, ids, send).await.map_err(|error| {
        format!(
            "Could not create the feedback directory at {}: {error}",
            directory.display()
        )
    })?;
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%SZ");
    let path = directory.join(format!("codex-diagnostics-{timestamp}-{:08x}.zip", unique()));
    let value = send(ClientRequest::FeedbackSave {
        request_id: ids.next(),
        params: FeedbackSaveParams {
            feedback: params,
            save_path: LegacyAppPathString::from_string(path.display().to_string()),
        },
    })
    .await
    .map_err(|error| match error.code {
        Some(METHOD_NOT_FOUND) => "Update the Codex server to save a feedback ZIP".to_owned(),
        _ => error.message,
    })?;
    let response: FeedbackSaveResponse = serde_json::from_value(value).map_err(|e| e.to_string())?;
    Ok(Receipt::Saved {
        path: response.saved_path.to_string(),
    })
}

/// JSON-RPC's "method not found": a server that predates `feedback/save`.
const METHOD_NOT_FOUND: i64 = -32601;

async fn make_directory<F, Fut>(directory: &Path, ids: &Ids, send: &F) -> Result<(), String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    send(ClientRequest::FsCreateDirectory {
        request_id: ids.next(),
        params: FsCreateDirectoryParams {
            path: absolute(directory)?,
            recursive: Some(true),
        },
    })
    .await
    .map(|_| ())
    .map_err(|error| error.message)
}

async fn remove<F, Fut>(directory: &Path, ids: &Ids, send: &F) -> Result<(), String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    send(ClientRequest::FsRemove {
        request_id: ids.next(),
        params: FsRemoveParams {
            path: absolute(directory)?,
            recursive: Some(true),
            force: Some(true),
        },
    })
    .await
    .map(|_| ())
    .map_err(|error| error.message)
}

fn absolute(path: &Path) -> Result<AbsolutePathBuf, String> {
    AbsolutePathBuf::from_absolute_path(path).map_err(|error| error.to_string())
}

/// A number that differs between deliveries (the oracle's uuid): the clock's nanoseconds mixed
/// with the process id.
fn unique() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos() ^ elapsed.as_secs() as u32);
    nanos ^ std::process::id().rotate_left(16)
}

/// Standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, byte)| n | u32::from(*byte) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(TABLE[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use codex_tui_core::features::feedback::Upload;
    use codex_tui_core::screens::dialogs::FeedbackCategory;

    use super::*;

    fn upload(include_logs: bool, destination: Destination) -> Upload {
        Upload {
            category: FeedbackCategory::Bug,
            reason: Some("it broke".into()),
            thread: Some(codex_tui_core::ids::ThreadId::new("thread-1")),
            turn_id: Some("turn-9".into()),
            include_logs,
            rollout: Some("/home/.codex/sessions/rollout.jsonl".into()),
            destination,
        }
    }

    /// Records the methods asked, answers with `reply` for the delivery's own request.
    async fn run(
        upload: Upload,
        home: Option<&Path>,
        reply: serde_json::Value,
    ) -> (Done, Vec<ClientRequest>) {
        let asked = Mutex::new(Vec::new());
        let done = deliver(&upload, home, &CodexFeedback::new(), &Ids::default(), |request| {
            let answer = match &request {
                ClientRequest::FeedbackUpload { .. } | ClientRequest::FeedbackSave { .. } => {
                    reply.clone()
                }
                _ => serde_json::json!({}),
            };
            asked.lock().unwrap().push(request);
            async move { Ok(answer) }
        })
        .await;
        (done, asked.into_inner().unwrap())
    }

    #[tokio::test]
    async fn an_upload_without_logs_is_one_request_with_the_oracles_params() {
        let (done, asked) = run(
            upload(false, Destination::OpenAi),
            None,
            serde_json::json!({"threadId": "filed-1"}),
        )
        .await;
        assert_eq!(
            done,
            Done::Feedback(Ok(Receipt::Uploaded {
                thread_id: "filed-1".into()
            }))
        );
        let [ClientRequest::FeedbackUpload { params, .. }] = asked.as_slice() else {
            panic!("not one upload: {asked:?}");
        };
        assert_eq!(params.classification, "bug");
        assert_eq!(params.reason.as_deref(), Some("it broke"));
        assert_eq!(params.thread_id.as_deref(), Some("thread-1"));
        assert_eq!(params.extra_log_files, None);
        assert_eq!(params.tags.as_ref().unwrap()["turn_id"], "turn-9");
    }

    #[tokio::test]
    async fn logs_without_a_home_are_omitted_and_the_reason_says_so() {
        let (done, asked) = run(
            upload(true, Destination::OpenAi),
            None,
            serde_json::json!({"threadId": "filed-1"}),
        )
        .await;
        assert!(matches!(done, Done::Feedback(Ok(_))));
        let [ClientRequest::FeedbackUpload { params, .. }] = asked.as_slice() else {
            panic!("not one upload: {asked:?}");
        };
        assert!(params.reason.as_ref().unwrap().ends_with("staged on the app-server."));
        // the rollout still goes
        assert_eq!(params.extra_log_files.as_ref().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn staged_logs_are_written_attached_and_removed() {
        let home = tempfile::tempdir().unwrap();
        let (done, asked) = run(
            upload(true, Destination::OpenAi),
            Some(home.path()),
            serde_json::json!({"threadId": "filed-1"}),
        )
        .await;
        assert!(matches!(done, Done::Feedback(Ok(_))));
        let kinds: Vec<_> = asked
            .iter()
            .map(|request| match request {
                ClientRequest::FsCreateDirectory { .. } => "mkdir",
                ClientRequest::FsWriteFile { .. } => "write",
                ClientRequest::FeedbackUpload { .. } => "upload",
                ClientRequest::FsRemove { .. } => "remove",
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(kinds, ["mkdir", "write", "upload", "remove"]);
        let ClientRequest::FeedbackUpload { params, .. } = &asked[2] else {
            unreachable!()
        };
        let files = params.extra_log_files.as_ref().unwrap();
        assert_eq!(files.len(), 2, "{files:?}");
        assert!(files[0].ends_with("rollout.jsonl") && files[1].ends_with("client-logs.txt"));
    }

    #[tokio::test]
    async fn a_zip_is_saved_under_the_feedback_directory() {
        let home = tempfile::tempdir().unwrap();
        let (done, asked) = run(
            upload(false, Destination::LocalZip),
            Some(home.path()),
            serde_json::json!({"savedPath": "/saved.zip", "threadId": "t"}),
        )
        .await;
        assert_eq!(
            done,
            Done::Feedback(Ok(Receipt::Saved {
                path: "/saved.zip".into()
            }))
        );
        let [ClientRequest::FsCreateDirectory { .. }, ClientRequest::FeedbackSave { params, .. }] =
            asked.as_slice()
        else {
            panic!("{asked:?}");
        };
        // a ZIP never attaches the rollout by path: the server selects it by thread id
        assert_eq!(params.feedback.extra_log_files, None);
        assert!(
            params
                .save_path
                .to_string()
                .contains("feedback/codex-diagnostics-")
        );
    }

    #[tokio::test]
    async fn a_zip_without_a_home_fails_with_the_oracles_text() {
        let (done, _) = run(upload(false, Destination::LocalZip), None, serde_json::json!({})).await;
        assert_eq!(
            done,
            Done::Feedback(Err(
                "Failed to save feedback ZIP: The app server did not report its Codex home directory"
                    .into()
            ))
        );
    }

    #[test]
    fn base64_pads_like_the_standard_alphabet() {
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
    }

    #[allow(dead_code)]
    fn _ask(upload: Upload) -> Ask {
        Ask::FeedbackUpload(upload)
    }
}
