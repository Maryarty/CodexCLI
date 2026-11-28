//! A workspace of real files for a test: the host checks a path a spoken answer names against
//! the disk (`Ask::WorkspaceFiles`), so a test that wants a link makes the file first. The
//! directory is the session's working directory (`Open::workspace`).

use tempfile::TempDir;

use super::Screen;
use super::seed::Open;

/// A directory that holds the files the test wrote, removed when the test drops it: bind it
/// before the screen that works in it, so the screen goes first.
pub struct Workspace {
    dir: TempDir,
}

/// A workspace with `files` (a path under the directory, its text).
pub fn workspace(files: &[(&str, &str)]) -> Workspace {
    let dir = tempfile::Builder::new()
        .prefix("codex-kit-workspace-")
        .tempdir()
        .expect("workspace directory");
    for (path, text) in files {
        let file = dir.path().join(path);
        std::fs::create_dir_all(file.parent().expect("a file has a directory")).expect("directory");
        std::fs::write(file, text).expect("write the file");
    }
    Workspace { dir }
}

impl Workspace {
    /// The directory.
    pub fn path(&self) -> String {
        self.dir.path().to_string_lossy().into_owned()
    }

    /// The `file:` URL of `file` as the host links it: of the canonical path.
    pub fn url(&self, file: &str) -> String {
        let canonical = self
            .dir
            .path()
            .join(file)
            .canonicalize()
            .expect("a file of the workspace");
        url::Url::from_file_path(canonical)
            .expect("a file URL")
            .to_string()
    }
}

impl Open {
    /// The session starts in the directory of `workspace`.
    pub fn workspace(self, workspace: &Workspace) -> Open {
        let path = workspace.path();
        self.thread_start(serde_json::json!({
            "thread": {
                "id": "thread-1", "environments": null, "extra": null, "sessionId": "thread-1",
                "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
                "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
                "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
                "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
                "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": path,
                "cliVersion": "0.0.0", "originator": null, "source": "cli",
                "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
                "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null,
                "turns": []
            },
            "model": "gpt-5", "modelProvider": "openai", "serviceTier": null,
            "cwd": path, "instructionSources": [],
            "approvalPolicy": "on-request", "approvalsReviewer": "user",
            "sandbox": {"type": "dangerFullAccess"}, "reasoningEffort": null
        }))
    }
}

impl Screen {
    /// Waits until the host answered the file check and the first `text` of the screen links to
    /// `url`: the link of a spoken answer follows the answer by one round of the host.
    pub fn wait_for_link(&mut self, text: &str, url: &str) -> &mut Screen {
        let (text, url) = (text.to_owned(), url.to_owned());
        let what = format!("`{text}` linked to {url}");
        self.wait_until(&what, move |grid| {
            grid.locate_cells(&text).is_some_and(|cells| {
                cells.iter().all(|&(x, y)| {
                    let cell = grid.cell(x, y);
                    cell.tail || cell.link.as_deref() == Some(url.as_str())
                })
            })
        })
    }
}
