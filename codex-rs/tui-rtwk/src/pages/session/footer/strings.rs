//! The strings of the footer under the composer: a table of `&'static str`
//! fields and typed messages (`Msg<N>`) per locale, read through `of(cx)`.
//! `EN` is the oracle's text byte for byte; each group names the oracle file
//! it comes from. Nothing else in this directory holds user-visible text
//! (rules test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::{Arg, DependCx, Fmt, Msg, NumSyms, msg};

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
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
