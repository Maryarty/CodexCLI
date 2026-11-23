use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;

use codex_tui_core::app::AppEv;
use codex_tui_core::app::AppFx;
use codex_tui_core::app::Command;
use codex_tui_core::app::Ingress;
use codex_tui_core::ids::BackendId;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::status::Ev as StatusEv;
use codex_tui_core::screens::transcript::diff;
use codex_tui_rtwk::services::shell::*;
use rtwk::app::AppCx;
use rtwk_tokio::TokioApp;
use std::collections::BTreeMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;

/// The last argument the command must have, and what it answers.
type Response = (Option<&'static str>, Result<diff::Output, String>);

/// A scripted workspace: each command must be the next one expected
/// (`FakeRunner` of the oracle's `get_git_diff.rs` tests); it records the
/// directory every command ran in.
struct Scripted {
    responses: Mutex<VecDeque<Response>>,
    cwds: Mutex<Vec<PathBuf>>,
}

impl WorkspaceExec for Scripted {
    fn run(
        &self,
        call: diff::Call,
        cwd: &Path,
    ) -> Pin<Box<dyn Future<Output = Result<diff::Output, String>> + Send + '_>> {
        let cwd = cwd.to_path_buf();
        Box::pin(async move {
            self.cwds.lock().unwrap().push(cwd);
            let (last, response) = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .expect("a scripted response");
            if let Some(last) = last {
                assert_eq!(call.argv.last().map(String::as_str), Some(last));
            }
            response
        })
    }
}

fn output(exit_code: i32, stdout: &str) -> Result<diff::Output, String> {
    Ok(diff::Output {
        exit_code,
        stdout: stdout.to_owned(),
    })
}

#[tokio::test]
async fn the_runner_drives_the_sequence_in_the_given_directory() {
    let exec = Scripted {
        responses: Mutex::new(
            [
                (Some("--is-inside-work-tree"), output(0, "true\n")),
                (Some("core.fsmonitor"), output(1, "")),
                (Some("^filter\\..*\\.(clean|process)$"), output(1, "")),
                (Some("--color"), output(1, "tracked\n")),
                (Some("--exclude-standard"), output(0, "new.txt\n")),
                (Some("new.txt"), output(1, "untracked\n")),
            ]
            .into(),
        ),
        cwds: Mutex::new(Vec::new()),
    };
    let cwd = PathBuf::from("/workspace");

    let report = git_diff(&exec, &cwd).await;

    assert_eq!(
        report,
        Ok(diff::Report {
            in_repo: true,
            text: "tracked\nuntracked\n".to_owned()
        })
    );
    assert!(exec.responses.lock().unwrap().is_empty());
    assert_eq!(exec.cwds.lock().unwrap().as_slice(), [cwd.as_path(); 6]);
}

#[tokio::test]
async fn a_command_the_workspace_cannot_run_ends_the_diff_with_its_error() {
    let exec = Scripted {
        responses: Mutex::new([(None, Err("the app server went away".to_owned()))].into()),
        cwds: Mutex::new(Vec::new()),
    };

    let report = git_diff(&exec, Path::new("/workspace")).await;

    assert_eq!(
        report,
        Err(diff::Failure::Shell("the app server went away".to_owned()))
    );
}
