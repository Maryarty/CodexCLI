//! The strings of the footer under the composer: a table of `&'static str`
//! fields and typed messages (`Msg<N>`) per locale, read through `of(cx)`.
//! `EN` is the oracle's text byte for byte; each group names the oracle file
//! it comes from. Nothing else in this directory holds user-visible text
//! (rules test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::{Arg, DependCx, Fmt, Msg, NumSyms, msg};

use codex_tui_core::screens::chat::facts::{GoalIndicator, GoalUsage};

use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of the footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// `bottom_pane/footer.rs` `quit_shortcut_reminder_line`: the key as
    /// the oracle spells it (`⌃c`) and what follows it.
    pub key_ctrl_c: &'static str,
    pub quit_again: &'static str,
    /// `esc_hint_line`: the idle spelling (`esc esc to edit previous message`)
    /// and the primed one (`esc again to edit previous message`).
    pub esc_key: &'static str,
    pub esc_between: &'static str,
    pub esc_idle_rest: &'static str,
    pub esc_primed_rest: &'static str,
    /// `shortcut_overlay.rs` `close_hint`: what stands between the toggle's key
    /// and `esc`, and after `esc`.
    pub close_between: &'static str,
    pub close_rest: &'static str,
    /// `passive_hints.rs`: the queue hint's key and its two spellings.
    pub queue_key: &'static str,
    pub queue_long: &'static str,
    pub queue_short: &'static str,
    /// `context_window_line`: the percent left, the tokens used, and the
    /// line when nothing is known yet.
    pub context_left: Msg<1>,
    pub context_used: Msg<1>,
    pub context_unknown: &'static str,
    /// `warning_labels`: the badge, the count with its word, the key, the
    /// action after it.
    pub warning_badge: &'static str,
    pub warning_one: Msg<1>,
    pub warning_many: Msg<1>,
    pub warning_key: &'static str,
    pub warning_view: &'static str,
    /// `effort_status_line.rs` `EffortTier::label`: the letters the status
    /// line assembles into.
    pub effort_max: &'static str,
    pub effort_ultra: &'static str,
    /// `effort_ignition.rs` `EffortTier::prompt_glyph`: the composer's prompt
    /// while the tier is active.
    pub prompt_max: &'static str,
    pub prompt_ultra: &'static str,
    /// `footer.rs` `CollaborationModeIndicator::styled_line`: `Plan mode`, then
    /// ` (`, the cycle key, ` to cycle)` while the key works.
    pub plan_mode: &'static str,
    pub plan_cycle_open: &'static str,
    pub plan_cycle_key: &'static str,
    pub plan_cycle_close: &'static str,
    /// `footer.rs` `goal_status_indicator_line`: one line per status, the
    /// usage (`40K / 50K`) in the parentheses.
    pub goal_active: &'static str,
    pub goal_active_usage: Msg<1>,
    pub goal_paused: &'static str,
    pub goal_blocked: &'static str,
    pub goal_usage_limited: &'static str,
    pub goal_unmet: Msg<1>,
    pub goal_abandoned: &'static str,
    pub goal_achieved: &'static str,
    pub goal_achieved_usage: Msg<1>,
    /// The unit after the figure of a closed goal's token count.
    pub goal_tokens: &'static str,
    /// `textarea.rs` `vim_mode_indicator_span`.
    pub vim_normal: &'static str,
    pub vim_insert: &'static str,
    pub vim_replace: &'static str,
    /// `footer_hint_items_line`: the cell between a hint override's key and its label.
    pub key_gap: &'static str,
    /// `footer.rs` `side_conversation_context_line`: the word the label opens
    /// with (it is drawn bold) and the endings that mark a key hint, longest
    /// match first as the oracle tries them.
    pub side_word: &'static str,
    pub side_actions: [&'static str; 4],
    /// The separator the label's parts are split at.
    pub side_join: &'static str,
    pub num: NumSyms,
}

impl Strings {
    /// `N% context left`.
    pub fn left(&self, percent: i64) -> String {
        Fmt::new(
            self.context_left,
            [Arg::Text(&percent.clamp(0, 100).to_string())],
            &self.num,
        )
        .to_string()
    }

    /// `3 warnings`, `1 warning`.
    pub fn warnings(&self, count: usize) -> String {
        let table = if count == 1 {
            self.warning_one
        } else {
            self.warning_many
        };
        Fmt::new(table, [Arg::Text(&count.to_string())], &self.num).to_string()
    }

    /// `Pursuing goal (40K / 50K)` and the other lines of `indicator`.
    pub fn goal(&self, indicator: &GoalIndicator) -> String {
        let with = |table: Msg<1>, usage: &GoalUsage| {
            let figure = match usage.tokens {
                true => format!("{}{}", usage.figure, self.goal_tokens),
                false => usage.figure.clone(),
            };
            Fmt::new(table, [Arg::Text(&figure)], &self.num).to_string()
        };
        match indicator {
            GoalIndicator::Active { usage: Some(usage) } => with(self.goal_active_usage, usage),
            GoalIndicator::Active { usage: None } => self.goal_active.to_owned(),
            GoalIndicator::Paused => self.goal_paused.to_owned(),
            GoalIndicator::Blocked => self.goal_blocked.to_owned(),
            GoalIndicator::UsageLimited => self.goal_usage_limited.to_owned(),
            GoalIndicator::BudgetLimited { usage: Some(usage) } => with(self.goal_unmet, usage),
            GoalIndicator::BudgetLimited { usage: None } => self.goal_abandoned.to_owned(),
            GoalIndicator::Complete { usage: Some(usage) } => with(self.goal_achieved_usage, usage),
            GoalIndicator::Complete { usage: None } => self.goal_achieved.to_owned(),
        }
    }

    /// `123K used` (`used` is the compact count).
    pub fn used(&self, compact: &str) -> String {
        Fmt::new(self.context_used, [Arg::Text(compact)], &self.num).to_string()
    }
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    key_ctrl_c: "\u{2303}c",
    quit_again: " again to quit",
    esc_key: "esc",
    esc_between: " ",
    esc_idle_rest: " to edit previous message",
    esc_primed_rest: " again to edit previous message",
    close_between: " / ",
    close_rest: " close",
    queue_key: "tab",
    queue_long: " to queue message",
    queue_short: " to queue",
    context_left: msg!("{0}% context left"),
    context_used: msg!("{0} used"),
    context_unknown: "100% context left",
    warning_badge: "\u{26a0} ",
    warning_one: msg!("{0} warning"),
    warning_many: msg!("{0} warnings"),
    warning_key: "f2",
    warning_view: " to view",
    effort_max: "MAX",
    effort_ultra: "ULTRA",
    prompt_max: "\u{203a}",
    prompt_ultra: "\u{00bb}",
    plan_mode: "Plan mode",
    plan_cycle_open: " (",
    plan_cycle_key: "\u{21e7}tab",
    plan_cycle_close: " to cycle)",
    goal_active: "Pursuing goal",
    goal_active_usage: msg!("Pursuing goal ({0})"),
    goal_paused: "Goal paused (/goal resume)",
    goal_blocked: "Goal stalled (/goal resume)",
    goal_usage_limited: "Goal hit usage limits (/goal resume)",
    goal_unmet: msg!("Goal unmet ({0})"),
    goal_abandoned: "Goal abandoned",
    goal_achieved: "Goal achieved",
    goal_achieved_usage: msg!("Goal achieved ({0})"),
    goal_tokens: " tokens",
    vim_normal: "Vim: Normal",
    vim_insert: "Vim: Insert",
    vim_replace: "Vim: Replace",
    key_gap: " ",
    side_word: "Side",
    side_actions: [" to switch", " to close", " to archive", " for side"],
    side_join: " \u{b7} ",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The text of the debug entries and tests of the effort animations: the
/// status line of the oracle's snapshots and a draft.
pub(super) mod sample {
    pub const MODEL: &str = "gpt-5.4";
    pub const BRANCH: &str = " \u{b7} main";
    pub const LINK_A: &str = "a ";
    pub const LINK_B: &str = "b ";
    pub const DRAFT: &str = "  > keep my draft exactly as typed";
}
