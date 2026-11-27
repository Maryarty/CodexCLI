//! The status-line and terminal-title configuration names and their legacy aliases.
//! The session surfaces and the settings dialogs use the same catalogues.

/// `StatusLineItem`, in `iter()` order.
pub const STATUS_LINE: [&str; 33] = [
    "model",
    "model-with-reasoning",
    "reasoning",
    "current-dir",
    "project-name",
    "hostname",
    "git-branch",
    "pull-request-number",
    "branch-changes",
    "run-state",
    "permissions",
    "approval-mode",
    "context-remaining",
    "context-used",
    "five-hour-limit",
    "weekly-limit",
    "codex-version",
    "context-window-size",
    "used-tokens",
    "total-input-tokens",
    "total-output-tokens",
    "thread-credits",
    "estimated-thread-cost",
    "thread-id",
    "fast-mode",
    "daybreak",
    "raw-output",
    "thread-name",
    "thread-title",
    "workspace-headline",
    "task-progress",
    "profile",
    "provider",
];

/// `TerminalTitleItem`, in `iter()` order.
pub const TERMINAL_TITLE: [&str; 25] = [
    "app-name",
    "project-name",
    "current-dir",
    "activity",
    "run-state",
    "thread-name",
    "thread-title",
    "git-branch",
    "context-remaining",
    "context-used",
    "five-hour-limit",
    "weekly-limit",
    "codex-version",
    "used-tokens",
    "total-input-tokens",
    "total-output-tokens",
    "thread-credits",
    "estimated-thread-cost",
    "thread-id",
    "fast-mode",
    "daybreak",
    "model",
    "model-with-reasoning",
    "reasoning",
    "task-progress",
];

/// `DEFAULT_TERMINAL_TITLE_ITEMS`.
pub const DEFAULT_TITLE: [&str; 3] = ["activity", "thread-name", "project-name"];

/// The names an item is written with, for `id` read as the config may spell it (a serialize
/// alias): the name the dialog lists it under.
fn canonical(id: &str) -> &str {
    match id {
        "model-name" => "model",
        "project" | "project-root" => "project-name",
        "status" => "run-state",
        "approval" => "approval-mode",
        "context-usage" => "context-used",
        "session-id" => "thread-id",
        "model-provider" => "provider",
        other => other,
    }
}

/// A configured status-line id that the oracle recognizes, in its canonical spelling.
pub fn status_item(id: &str) -> Option<&str> {
    let id = canonical(id);
    STATUS_LINE.contains(&id).then_some(id)
}

/// A configured terminal-title id that the oracle recognizes, in its canonical spelling.
pub fn title_item(id: &str) -> Option<&str> {
    let id = match id {
        "spinner" => "activity",
        "thread" => "thread-title",
        "project-root" => return None,
        other => canonical(other),
    };
    TERMINAL_TITLE.contains(&id).then_some(id)
}
