//! The words of `/diff` (oracle `chatwidget/slash_dispatch.rs`, `app/event_dispatch.rs`
//! `DiffResult`): what the pager says in place of a diff. The wording is the oracle's English
//! until the catalog has these entries.

use codex_tui_core::features::diff::PagerNote;
use codex_tui_core::screens::transcript::diff::Failure;

/// The title of the pager (`Overlay::new_static_with_lines(.., "D I F F")`).
pub const TITLE: &str = "D I F F";

/// The pager's text for a document with no diff (`DiffResult`): `No changes detected.` in
/// italics (as an ANSI text, the pager's own language), the other two as plain words, backticks
/// and underscores as they are (the oracle's lines are not Markdown).
pub fn note(note: &PagerNote) -> String {
    match note {
        PagerNote::NoChanges => "\u{1b}[3mNo changes detected.\u{1b}[0m".to_owned(),
        PagerNote::NotRepo => "`/diff` — _not inside a git repository_".to_owned(),
        PagerNote::Failed(failure) => format!("Failed to compute diff: {}", failure_text(failure)),
    }
}

fn failure_text(failure: &Failure) -> String {
    match failure {
        Failure::Shell(text) => text.clone(),
        Failure::Git { args, status } => {
            format!("git {} exited with status {status}", args.join(" "))
        }
        Failure::NoAnswer => "the shell answered too few commands".to_owned(),
        Failure::Finished => "the diff had already ended".to_owned(),
    }
}
