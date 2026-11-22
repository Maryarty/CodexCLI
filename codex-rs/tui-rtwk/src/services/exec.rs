//! Commands in the workspace of the active session (oracle
//! `workspace_command.rs`, `AppServerWorkspaceCommandRunner`): the one-off
//! `command/exec` request, so the same code follows the session's embedded or
//! remote execution. `/diff` runs through it (`shell::git_diff`); so do the
//! git probes of the status line.
//!
//! The request is non-tty, streams nothing and takes the call's timeout.
//! Sandbox and permission profile are left to the app-server. The client is
//! a closure, as in `backend::rpc`: the host's real one and a test's are the
//! same code.

use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::CommandExecParams;
use codex_app_server_protocol::CommandExecResponse;
use codex_app_server_protocol::RequestId;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::transcript::diff;

use super::backend::rpc::Ids;
use super::shell::WorkspaceExec;

/// `command/exec` over `send`, with the connection's request ids.
pub struct AppServerExec<F> {
    send: F,
    ids: Ids,
}

impl<F> AppServerExec<F> {
    pub fn new(send: F, ids: Ids) -> Self {
        Self { send, ids }
    }
}

impl<F, Fut> WorkspaceExec for AppServerExec<F>
where
    F: Fn(ClientRequest) -> Fut + Send + Sync,
    Fut: Future<Output = Result<serde_json::Value, ServerError>> + Send,
{
    fn run(
        &self,
        call: diff::Call,
        cwd: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<diff::Output, String>> + Send + '_>> {
        let request = request(&call, cwd, self.ids.next());
        Box::pin(async move {
            let value = (self.send)(request).await.map_err(|error| error.message)?;
            let response: CommandExecResponse =
                serde_json::from_value(value).map_err(|error| error.to_string())?;
            Ok(diff::Output {
                exit_code: response.exit_code,
                stdout: response.stdout,
            })
        })
    }
}

fn request(call: &diff::Call, cwd: &Path, request_id: RequestId) -> ClientRequest {
    let timeout = Duration::from_secs(call.limits.timeout_secs());
    let env: HashMap<String, Option<String>> = call
        .env
        .iter()
        .map(|(key, value)| (key.clone(), Some(value.clone())))
        .collect();
    ClientRequest::OneOffCommandExec {
        request_id,
        params: CommandExecParams {
            command: call.argv.clone(),
            process_id: None,
            tty: false,
            stream_stdin: false,
            stream_stdout_stderr: false,
            output_bytes_cap: None,
            disable_output_cap: call.limits.uncapped(),
            disable_timeout: false,
            timeout_ms: Some(i64::try_from(timeout.as_millis()).unwrap_or(i64::MAX)),
            cwd: Some(cwd.to_path_buf()),
            env: (!env.is_empty()).then_some(env),
            size: None,
            sandbox_policy: None,
            permission_profile: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::Mutex;

    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;
    use crate::services::shell::git_diff;

    type Answer = Result<serde_json::Value, ServerError>;
    type Seen = Arc<Mutex<Vec<ClientRequest>>>;
    type Reply = Pin<Box<dyn Future<Output = Answer> + Send>>;

    fn call(limits: diff::Limits) -> diff::Call {
        diff::Call {
            argv: vec!["git".to_owned(), "status".to_owned()],
            env: vec![("GIT_PAGER".to_owned(), "cat".to_owned())],
            limits,
        }
    }

    /// An executor whose app-server answers `answers` in order and records
    /// every request it got.
    fn exec(
        answers: Vec<Answer>,
    ) -> (
        AppServerExec<impl Fn(ClientRequest) -> Reply + Send + Sync>,
        Seen,
    ) {
        let seen: Seen = Arc::default();
        let log = Arc::clone(&seen);
        let answers = Mutex::new(answers.into_iter());
        let send = move |request: ClientRequest| -> Reply {
            log.lock().unwrap().push(request);
            let answer = answers.lock().unwrap().next().expect("an answer");
            Box::pin(async move { answer })
        };
        (AppServerExec::new(send, Ids::default()), seen)
    }

    fn params(seen: &Seen) -> CommandExecParams {
        match seen.lock().unwrap().last().cloned() {
            Some(ClientRequest::OneOffCommandExec { params, .. }) => params,
            other => panic!("not a command/exec: {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_probe_is_a_five_second_capped_one_off_exec_in_the_workspace() {
        let (exec, seen) = exec(vec![Ok(json!({
            "exitCode": 0, "stdout": "true\n", "stderr": ""
        }))]);

        let output = exec
            .run(call(diff::Limits::Probe), &PathBuf::from("/work"))
            .await
            .unwrap();

        assert_eq!(
            output,
            diff::Output {
                exit_code: 0,
                stdout: "true\n".to_owned()
            }
        );
        let params = params(&seen);
        assert_eq!(params.command, ["git", "status"]);
        assert_eq!(params.cwd, Some(PathBuf::from("/work")));
        assert_eq!(params.timeout_ms, Some(5_000));
        assert!(!params.disable_output_cap);
        assert!(!params.tty && !params.stream_stdin && !params.stream_stdout_stderr);
        assert_eq!(
            params.env,
            Some(HashMap::from([(
                "GIT_PAGER".to_owned(),
                Some("cat".to_owned())
            )]))
        );
    }

    #[tokio::test]
    async fn a_diff_has_thirty_seconds_and_no_output_cap_and_no_env_is_none() {
        let (exec, seen) = exec(vec![Ok(json!({
            "exitCode": 1, "stdout": "", "stderr": "x"
        }))]);
        let mut call = call(diff::Limits::Diff);
        call.env.clear();

        let output = exec.run(call, &PathBuf::from("/work")).await.unwrap();

        // A non-zero exit is an output, not a failure to run.
        assert_eq!(output.exit_code, 1);
        let params = params(&seen);
        assert_eq!(params.timeout_ms, Some(30_000));
        assert!(params.disable_output_cap);
        assert_eq!(params.env, None);
    }

    #[tokio::test]
    async fn a_refused_request_is_a_command_that_did_not_run() {
        let (exec, _) = exec(vec![Err(ServerError::from("no such cwd"))]);

        let result = exec
            .run(call(diff::Limits::Probe), &PathBuf::from("/nowhere"))
            .await;

        assert_eq!(result, Err("no such cwd".to_owned()));
    }

    #[tokio::test]
    async fn every_request_has_its_own_id() {
        let ok = || Ok(json!({"exitCode": 0, "stdout": "", "stderr": ""}));
        let (exec, seen) = exec(vec![ok(), ok()]);

        for _ in 0..2 {
            exec.run(call(diff::Limits::Probe), &PathBuf::from("/work"))
                .await
                .unwrap();
        }

        let ids: Vec<_> = seen
            .lock()
            .unwrap()
            .iter()
            .map(|request| match request {
                ClientRequest::OneOffCommandExec { request_id, .. } => request_id.clone(),
                other => panic!("not a command/exec: {other:?}"),
            })
            .collect();
        assert_eq!(ids.len(), 2);
        assert!(ids[0] != ids[1]);
    }

    #[tokio::test]
    async fn the_diff_sequence_runs_through_the_app_server() {
        // Not a work tree: `git diff` stops at the first probe.
        let (exec, seen) = exec(vec![Ok(json!({
            "exitCode": 128, "stdout": "", "stderr": "fatal: not a git repository"
        }))]);

        let report = git_diff(&exec, &PathBuf::from("/work")).await.unwrap();

        assert!(!report.in_repo);
        assert_eq!(params(&seen).cwd, Some(PathBuf::from("/work")));
    }
}
