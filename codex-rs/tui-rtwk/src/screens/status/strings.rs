//! The strings of `status` (S19): a table of `&'static str` fields and typed
//! messages per locale, read by reference through `of(cx)`. `EN` is the
//! oracle's text byte for byte (`status/card.rs`, `status/helpers.rs`,
//! `status/rate_limits.rs`). A language is one more `const` here, registered
//! with `with_ru(&RU)` on `TABLES`; a screen never changes for it. Nothing
//! else in this directory holds user-visible text (the rules test
//! `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    pub num: NumSyms,
    /// The command the card answers, the dialog's title.
    pub command: &'static str,
    pub header: Msg<1>,
    pub usage_link_1: &'static str,
    pub usage_link_2: &'static str,
    pub model: &'static str,
    pub directory: &'static str,
    pub permissions: &'static str,
    pub agents: &'static str,
    pub token_usage: &'static str,
    pub context_window: &'static str,
    pub agents_none: &'static str,
    pub model_value: Msg<2>,
    pub reasoning: Msg<1>,
    pub summaries: Msg<1>,
    pub details_separator: &'static str,
    pub profile_read_only: &'static str,
    pub profile_workspace: &'static str,
    pub profile_full: &'static str,
    pub approval_ask: &'static str,
    pub approval_auto: &'static str,
    pub approval_never: &'static str,
    pub approval_on_failure: &'static str,
    pub approval_untrusted: &'static str,
    pub permissions_value: Msg<2>,
    pub tokens_total: Msg<3>,
    pub context_value: Msg<3>,
    pub limit_hours: Msg<1>,
    pub limit_weekly: &'static str,
    pub limit_value: Msg<3>,
    pub am: &'static str,
    pub pm: &'static str,
    pub resets: Msg<3>,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "/status card",
    num: NumSyms::ENGLISH,
    command: "/status",
    header: msg!(">_ OpenAI Codex (v{0})"),
    usage_link_1: "Visit https://chatgpt.com/settings/usage for up-to-date",
    usage_link_2: "information on rate limits and credits",
    model: "Model:",
    directory: "Directory:",
    permissions: "Permissions:",
    agents: "Agents.md:",
    token_usage: "Token usage:",
    context_window: "Context window:",
    agents_none: "<none>",
    model_value: msg!("{0} ({1})"),
    reasoning: msg!("reasoning {0}"),
    summaries: msg!("summaries {0}"),
    details_separator: ", ",
    profile_read_only: "Read Only",
    profile_workspace: "Workspace",
    profile_full: "Full Access",
    approval_ask: "Ask for approval",
    approval_auto: "Approve for me",
    approval_never: "never ask",
    approval_on_failure: "on failure",
    approval_untrusted: "untrusted commands",
    permissions_value: msg!("{0} ({1})"),
    tokens_total: msg!("{0} total  ({1} input + {2} output)"),
    context_value: msg!("{0}% left ({1} used / {2})"),
    limit_hours: msg!("{0}h limit:"),
    limit_weekly: "Weekly limit:",
    limit_value: msg!("{0} {1}% left {2}"),
    am: "AM",
    pm: "PM",
    resets: msg!("(resets {0}:{1} {2})"),
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

/// The title of the screen in `lang`, for code with no tree (the debug
/// menu's rows).
pub fn title(lang: Lang) -> &'static str {
    TABLES.pick(lang).title
}

impl Strings {
    pub fn header<'a>(&'a self, version: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.header, [Arg::Text(version)], &self.num)
    }

    pub fn model_value<'a>(&'a self, model: &'a str, details: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.model_value,
            [Arg::Text(model), Arg::Text(details)],
            &self.num,
        )
    }

    pub fn reasoning<'a>(&'a self, effort: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.reasoning, [Arg::Text(effort)], &self.num)
    }

    pub fn summaries<'a>(&'a self, summary: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.summaries, [Arg::Text(summary)], &self.num)
    }

    pub fn permissions_value<'a>(&'a self, profile: &'a str, approval: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.permissions_value,
            [Arg::Text(profile), Arg::Text(approval)],
            &self.num,
        )
    }

    pub fn tokens_total<'a>(&'a self, total: &'a str, input: &'a str, output: &'a str) -> Fmt<'a, 3> {
        Fmt::new(
            self.tokens_total,
            [Arg::Text(total), Arg::Text(input), Arg::Text(output)],
            &self.num,
        )
    }

    pub fn context_value<'a>(&'a self, percent: i64, used: &'a str, window: &'a str) -> Fmt<'a, 3> {
        Fmt::new(
            self.context_value,
            [
                Arg::Int(u64::try_from(percent).unwrap_or(0)),
                Arg::Text(used),
                Arg::Text(window),
            ],
            &self.num,
        )
    }

    pub fn limit_hours(&self, hours: i64) -> Fmt<'_, 1> {
        Fmt::new(
            self.limit_hours,
            [Arg::Int(u64::try_from(hours).unwrap_or(0))],
            &self.num,
        )
    }

    pub fn limit_value<'a>(&'a self, bar: &'a str, percent_left: i64, resets: &'a str) -> Fmt<'a, 3> {
        Fmt::new(
            self.limit_value,
            [
                Arg::Text(bar),
                Arg::Int(u64::try_from(percent_left).unwrap_or(0)),
                Arg::Text(resets),
            ],
            &self.num,
        )
    }

    pub fn resets<'a>(&'a self, hour: &'a str, minute: &'a str, meridiem: &'a str) -> Fmt<'a, 3> {
        Fmt::new(
            self.resets,
            [Arg::Text(hour), Arg::Text(minute), Arg::Text(meridiem)],
            &self.num,
        )
    }
}
