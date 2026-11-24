//! `status`: the view half of S19 (the `/status` card): the session's
//! configuration and usage as labelled rows over the card the model holds.
//! Oracle: `codex-rs/tui/src/status/`.
//!
//! `/status` is a cell of the thread's transcript (`Item::Status`, ADR-003):
//! `StatusCell` draws the card it holds. The debug entries open the same cell on
//! the card a fixture script put into the thread.

mod bar;
mod cell;

pub use cell::StatusCell;

use rtwk::AnyWidget;

use super::dispatch::{CellEnv, Place, below};
use crate::app::debug::DebugEntry;
use crate::app::debug::DebugOpen;
use crate::app::debug::EntryOpen;
use crate::app::debug::seeded::Seeded;
use codex_tui_core::threads::Item;

pub mod strings;

/// The cell of the card a script put into the shown thread (`status_card`), as the
/// page of a route; nothing when no card was seeded.
fn cell(seeded: &Seeded<'_>) -> AnyWidget {
    let thread = seeded.core.active_thread();
    match thread.transcript().last_status() {
        Some(card) => StatusCell { card: card.clone() }.into(),
        None => AnyWidget::default(),
    }
}

/// One entry of the cell on a fixture of the same name.
macro_rules! cell_entry {
    ($name:literal, $description:literal) => {
        DebugEntry {
            name: $name,
            description: $description,
            script: include_str!(concat!("fixtures/", $name, ".jsonl")),
            open: EntryOpen::Route(cell),
        }
    };
}

/// What the debug menu lists for this directory: one entry per state worth
/// opening, on the model a fixture script made.
pub fn debug_entries() -> Vec<DebugEntry> {
    vec![
        cell_entry!(
            "reasoning_details",
            "the cell: reasoning details, two limits (12-hour clock)"
        ),
        cell_entry!(
            "chatgpt_plan_without_email",
            "the cell: a ChatGPT account with a plan and no email"
        ),
        cell_entry!(
            "active_user_defined_profile",
            "the cell: a user-defined profile"
        ),
        cell_entry!(
            "server_auth_not_required",
            "the cell: a provider with no OpenAI auth"
        ),
        cell_entry!(
            "server_auth_required",
            "the cell: a provider with OpenAI auth (the link)"
        ),
        cell_entry!(
            "auto_review_permissions",
            "the cell: the auto-review reviewer"
        ),
        cell_entry!("forked_from", "the cell: a forked session"),
        cell_entry!("monthly_limit", "the cell: a monthly window"),
        cell_entry!(
            "enterprise_monthly_credit",
            "the cell: a workspace's monthly credit limit"
        ),
        cell_entry!(
            "generic_limit_labels",
            "the cell: windows of no known length"
        ),
        cell_entry!("unlimited_credits", "the cell: unlimited credits"),
        cell_entry!("positive_credits", "the cell: a credit balance"),
        cell_entry!(
            "credits_available_none",
            "the cell: credits with no balance (none)"
        ),
        cell_entry!(
            "credits_available_empty",
            "the cell: credits with no balance (empty)"
        ),
        cell_entry!(
            "credits_available_zero",
            "the cell: credits with no balance (zero)"
        ),
        cell_entry!(
            "credits_available_not_a_number",
            "the cell: credits with no balance (not a number)"
        ),
        cell_entry!(
            "credits_available_inf",
            "the cell: credits with no balance (inf)"
        ),
        cell_entry!(
            "unlimited_without_flag",
            "the cell: unlimited credits without the has-credits flag"
        ),
        cell_entry!("cached_tokens", "the cell: cached input tokens"),
        cell_entry!(
            "long_path",
            "the cell: a long wide-character path, a session id"
        ),
        cell_entry!(
            "narrow_terminal",
            "the cell: custom permissions, one window"
        ),
        cell_entry!(
            "halfwidth_kana_narrow",
            "the cell: half-width kana at 42 columns"
        ),
        cell_entry!("missing_limits", "the cell: no limits read yet"),
        cell_entry!("default_reasoning", "the cell: the default reasoning"),
        cell_entry!("remote_server", "the cell: a remote server"),
        cell_entry!(
            "local_background_server",
            "the cell: the local background server"
        ),
        cell_entry!("refreshing_limits", "the cell: limits while a refresh runs"),
        cell_entry!(
            "credits_and_limits",
            "the cell: a Pro account: windows and credits"
        ),
        cell_entry!("unavailable_limits", "the cell: limits not available"),
        cell_entry!(
            "refreshing_empty_limits",
            "the cell: empty limits while a refresh runs"
        ),
        cell_entry!("stale_limits", "the cell: stale limits and the warning"),
        cell_entry!(
            "cached_limits_hide_credits",
            "the cell: stale limits with a hidden balance"
        ),
        cell_entry!(
            "context_last_usage",
            "the cell: the context window of the last usage"
        ),
    ]
}

/// Opens the empty form of this directory (debug menu): nothing seeded, nothing
/// to show.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Route(|_| AnyWidget::default())
}

/// The card of `/status`.
/// The cell of `item` when it is this file's (`cell_list!`).
pub fn view(item: &Item, _env: &CellEnv<'_>, place: &Place) -> AnyWidget {
    let Item::Status { card, .. } = item else {
        return AnyWidget::default();
    };
    below(place.separator, StatusCell { card: card.clone() }.into())
}
