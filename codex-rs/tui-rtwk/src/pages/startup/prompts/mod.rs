//! `startup`: the view half of S05 (the startup prompts: working directory,
//! model migration, hooks review, unarchive, provider picker, daemon
//! recovery). Its model part, events and intents are core's
//! `codex_tui_core::screens::startup`.
//!
//! In Flutter each prompt is a `Navigator.push` of a full-screen page that
//! returns what the user chose; here the prompt is data in the model
//! (`Intent::Ask`), the page is `Panel` over it, and the answer is the
//! model's (`startup.answers`), which the flow that asked reads.

use rtwk::Cx;
use rtwk::navigation::{Navigator, PageRoute};
use rtwk::provider::read;

use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::startup::{Intent, Prompt};

use crate::app::debug::{DebugEntry, DebugOpen, EntryOpen};
use crate::app::view::Hs;

mod keys;
mod panel;
pub mod strings;

pub use panel::Panel;

/// The route of the prompt the model has open.
pub fn route() -> PageRoute<()> {
    PageRoute::new(|_| Panel.into())
}

/// Puts `prompt` on screen and pushes its route: what the flow calls to ask.
/// The answer lands in `ScreenStates.startup.answers`.
pub fn ask<S>(cx: &mut Cx<'_, S>, prompt: Prompt) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.screen, ScreenIntent::Startup(Intent::Ask(prompt)));
    drop(Navigator::of(cx).push::<()>(route()));
}

/// What the debug menu lists: one entry per state of the oracle's tests.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries: [(&'static str, &'static str, &'static str); 21] = [
        (
            "cwd_resume",
            "working directory on resume",
            include_str!("fixtures/cwd_resume.jsonl"),
        ),
        (
            "cwd_fork",
            "working directory on fork",
            include_str!("fixtures/cwd_fork.jsonl"),
        ),
        (
            "cwd_remote_exec",
            "working directory, remembering current not allowed",
            include_str!("fixtures/cwd_remote_exec.jsonl"),
        ),
        (
            "cwd_remembered_current",
            "working directory, remembered current differs",
            include_str!("fixtures/cwd_remembered_current.jsonl"),
        ),
        (
            "cwd_long_paths",
            "working directory with long paths",
            include_str!("fixtures/cwd_long_paths.jsonl"),
        ),
        (
            "migration_opt_out",
            "model migration with the choices",
            include_str!("fixtures/migration_opt_out.jsonl"),
        ),
        (
            "migration_gpt5_family",
            "model migration with a link and no choices",
            include_str!("fixtures/migration_gpt5_family.jsonl"),
        ),
        (
            "migration_markdown",
            "model migration from a markdown template",
            include_str!("fixtures/migration_markdown.jsonl"),
        ),
        (
            "migration_markdown_url",
            "model migration, markdown with a long URL",
            include_str!("fixtures/migration_markdown_url.jsonl"),
        ),
        (
            "hooks_review",
            "startup hooks review",
            include_str!("fixtures/hooks_review.jsonl"),
        ),
        (
            "hooks_review_error",
            "hooks review after a failed trust",
            include_str!("fixtures/hooks_review_error.jsonl"),
        ),
        (
            "hooks_trusting",
            "hooks review while the trust is written",
            include_str!("fixtures/hooks_trusting.jsonl"),
        ),
        (
            "unarchive_resume",
            "archived conversation, resume",
            include_str!("fixtures/unarchive_resume.jsonl"),
        ),
        (
            "unarchive_fork",
            "archived conversation, fork",
            include_str!("fixtures/unarchive_fork.jsonl"),
        ),
        (
            "oss_stopped",
            "provider picker, both stopped",
            include_str!("fixtures/oss_stopped.jsonl"),
        ),
        (
            "oss_running",
            "provider picker, both running",
            include_str!("fixtures/oss_running.jsonl"),
        ),
        (
            "oss_unknown",
            "provider picker, status unknown",
            include_str!("fixtures/oss_unknown.jsonl"),
        ),
        (
            "oss_mixed",
            "provider picker, LM Studio stopped, Ollama running",
            include_str!("fixtures/oss_mixed.jsonl"),
        ),
        (
            "daemon_menu",
            "daemon recovery, managed",
            include_str!("fixtures/daemon_menu.jsonl"),
        ),
        (
            "daemon_unmanaged",
            "daemon recovery, not managed by Codex",
            include_str!("fixtures/daemon_unmanaged.jsonl"),
        ),
        (
            "daemon_non_restartable",
            "daemon recovery, a restart cannot help",
            include_str!("fixtures/daemon_non_restartable.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: EntryOpen::Route(|_| Panel.into()),
        })
        .collect()
}

/// Opens the empty form of this slot (debug menu): no prompt is open.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| Panel.into())
}
