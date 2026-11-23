//! The words of the worktree pickers (oracle `chatwidget/worktree_picker.rs`): a table of
//! `&'static str` fields per locale, read by reference through `of(cx)`. `EN` is the oracle's text
//! byte for byte. A language is one more `const` here, registered with `with_ru(&RU)` on `TABLES`.

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use codex_tui_core::screens::select::worktree::{Ago, Entry, Owner};

use crate::l10n::Tables;
use crate::l10n::lang;

/// The words of the checkout choice, `/worktree`, the browser, a worktree's actions and the delete
/// confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    // `open_session_checkout_picker`.
    pub checkout_new_title: &'static str,
    pub checkout_fork_title: &'static str,
    pub current_name: &'static str,
    pub current_description: &'static str,
    pub create_name: &'static str,
    pub create_description: &'static str,
    // `show_managed_worktree_picker`.
    pub worktrees_title: &'static str,
    pub continue_name: &'static str,
    pub continue_description: &'static str,
    pub start_name: &'static str,
    pub start_description: &'static str,
    pub browse_name: &'static str,
    pub browse_description: &'static str,
    // `request_managed_worktrees`, `on_managed_worktrees_loaded`.
    pub browser_title: &'static str,
    pub loading: &'static str,
    pub browser_subtitle: &'static str,
    pub browser_empty: &'static str,
    pub status_none: Msg<1>,
    pub status_unavailable: Msg<1>,
    pub updated: Msg<2>,
    pub archived_updated: Msg<2>,
    pub just_now: &'static str,
    pub minutes_ago: Msg<1>,
    pub hours_ago: Msg<1>,
    pub days_ago: Msg<1>,
    // `show_managed_worktree_actions`.
    pub actions_title: &'static str,
    pub actions_title_of: Msg<1>,
    pub resume: &'static str,
    pub copy: &'static str,
    pub delete: &'static str,
    pub delete_blocked: &'static str,
    // `confirm_managed_worktree_removal`.
    pub confirm_title: &'static str,
    pub confirm_cancel: &'static str,
    pub confirm_delete: &'static str,
    pub confirm_delete_description: &'static str,
    pub num: NumSyms,
}

impl Strings {
    /// The browser's description of a worktree: its owner's state, how long ago it was updated and
    /// where it is (`on_managed_worktrees_loaded`).
    pub fn describe(&self, entry: &Entry, now: i64) -> String {
        let cwd = Arg::Text(&entry.cwd);
        match &entry.owner {
            Owner::None => Fmt::new(self.status_none, [cwd], &self.num).to_string(),
            Owner::Unavailable(_) => {
                Fmt::new(self.status_unavailable, [cwd], &self.num).to_string()
            }
            Owner::Archived(thread) => Fmt::new(
                self.archived_updated,
                [Arg::Text(&self.ago(thread.updated_at, now)), cwd],
                &self.num,
            )
            .to_string(),
            Owner::Resumable(thread) => Fmt::new(
                self.updated,
                [Arg::Text(&self.ago(thread.updated_at, now)), cwd],
                &self.num,
            )
            .to_string(),
        }
    }

    /// `just now`, `5m ago`, `2h ago`, `3d ago` (`worktree_updated_ago`).
    fn ago(&self, updated_at: i64, now: i64) -> String {
        let (msg, count) = match Ago::of(updated_at, now) {
            Ago::JustNow => return self.just_now.to_string(),
            Ago::Minutes(count) => (self.minutes_ago, count),
            Ago::Hours(count) => (self.hours_ago, count),
            Ago::Days(count) => (self.days_ago, count),
        };
        Fmt::new(msg, [Arg::Text(&count.to_string())], &self.num).to_string()
    }

    /// `Worktree: {title}`, or `Worktree` for one no thread owns.
    pub fn actions_heading(&self, entry: &Entry) -> String {
        match &entry.owner {
            Owner::Archived(thread) | Owner::Resumable(thread) => {
                Fmt::new(self.actions_title_of, [Arg::Text(&thread.title)], &self.num).to_string()
            }
            Owner::None | Owner::Unavailable(_) => self.actions_title.to_string(),
        }
    }
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    checkout_new_title: "Where should the new conversation run?",
    checkout_fork_title: "Where should the forked conversation run?",
    current_name: "Use current Git worktree",
    current_description: "Keep using the current working directory",
    create_name: "Create new Git worktree",
    create_description: "Create a separate checkout of this repository in another directory",
    worktrees_title: "Worktrees",
    continue_name: "Continue current conversation",
    continue_description: "Preserve this conversation in the new checkout",
    start_name: "Start new conversation",
    start_description: "Open a fresh conversation in the new checkout",
    browse_name: "Browse worktrees",
    browse_description: "Resume an owner thread or copy a working directory",
    browser_title: "Managed worktrees",
    loading: "Loading worktrees\u{2026}",
    browser_subtitle: "Select a worktree to resume, copy its path, or delete it",
    browser_empty: "No worktrees in this repository's configured pool",
    status_none: msg!("No attached thread \u{b7} {0}"),
    status_unavailable: msg!("Owner thread unavailable \u{b7} {0}"),
    updated: msg!("updated {0} \u{b7} {1}"),
    archived_updated: msg!("Archived \u{b7} updated {0} \u{b7} {1}"),
    just_now: "just now",
    minutes_ago: msg!("{0}m ago"),
    hours_ago: msg!("{0}h ago"),
    days_ago: msg!("{0}d ago"),
    actions_title: "Worktree",
    actions_title_of: msg!("Worktree: {0}"),
    resume: "Resume owner thread",
    copy: "Copy working directory",
    delete: "Delete worktree",
    delete_blocked: "Switch to another checkout before deleting this one",
    confirm_title: "Delete this worktree?",
    confirm_cancel: "Cancel",
    confirm_delete: "Delete worktree",
    confirm_delete_description: "Keeps thread history; may disrupt other sessions",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
