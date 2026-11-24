//! What the small commands read of the host (`features/small_cmds` facts): whether an IDE answers.
//! The IDE's ipc socket is not carried yet, so none does: the oracle's hint for an IDE that is
//! not connected (`IdeContextError::Connect`).

pub(crate) const OPEN_IDE_HINT: &str =
    "Open this project in VS Code or Cursor with the Codex extension active.";
