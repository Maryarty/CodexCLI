//! The small commands' local IDE transport.

mod ipc;
mod prompt;
#[cfg(windows)]
mod windows_pipe;

use std::path::PathBuf;

use codex_tui_core::features::small_cmds::IdeFailure;
use serde::Deserialize;

pub(crate) const OPEN_IDE_HINT: &str =
    "Open this project in VS Code or Cursor with the Codex extension active.";

/// One read of the local IDE's active file and tabs. The caller has already
/// checked that the owning session uses a local workspace.
pub(crate) async fn read_ide_context(
    workspace: PathBuf,
    codex_home: PathBuf,
) -> Result<Option<String>, IdeFailure> {
    tokio::task::spawn_blocking(move || {
        if !workspace.is_absolute() || !workspace.is_dir() {
            return Err(IdeFailure {
                hint: OPEN_IDE_HINT.to_owned(),
                skip_hint: OPEN_IDE_HINT.to_owned(),
            });
        }
        ipc::fetch_ide_context(&workspace, &codex_home)
            .map(|context| prompt::render_prompt_context(&context))
            .map_err(|error| IdeFailure {
                hint: error.user_facing_hint(),
                skip_hint: error.prompt_skip_hint(),
            })
    })
    .await
    .map_err(|_| IdeFailure {
        hint: "Codex could not read IDE context. Try /ide again.".to_owned(),
        skip_hint: "Codex could not read IDE context. Codex will keep trying on future messages."
            .to_owned(),
    })?
}

// The wire shape is carried from `tui/src/ide_context.rs`. Only the
// extension fields needed for the prompt are decoded.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContext {
    active_file: Option<ActiveFile>,
    #[serde(default)]
    open_tabs: Vec<FileDescriptor>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveFile {
    #[serde(flatten)]
    descriptor: FileDescriptor,
    selection: Range,
    #[serde(default)]
    active_selection_content: String,
    #[serde(default)]
    selections: Vec<Range>,
}

#[derive(Debug, Deserialize)]
struct FileDescriptor {
    label: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct Range {
    start: Position,
    end: Position,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Position {
    line: u32,
    character: u32,
}
