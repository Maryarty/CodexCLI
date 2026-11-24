//! The words and rows of the usage menu and the reset flow behind it (oracle
//! `chatwidget/usage.rs`, `chatwidget/reset_credits.rs`): the menu, the picker of the credits,
//! its message pages, the confirmation. Core says what each state is
//! (`screens::usage::reset`), the rows are made here. The wording is the oracle's English until
//! the catalog has these entries.

use chrono::{DateTime, FixedOffset, Utc};

use codex_tui_core::screens::cells::{ClockFormat, ViewerClock};
use codex_tui_core::screens::usage::reset::{
    CreditOption, Expiry, Listing, MenuEntry, Progress, Redemption, Settled,
};

use crate::widgets::picker::list::{Columns, Item, List};

/// The row ids of the usage menu.
pub const ANALYTICS: &str = "usage.analytics";
pub const REDEEM: &str = "usage.redeem";
/// The row ids of the reset pages: `credit.{n}` is the n-th credit of the picker.
pub const CREDIT: &str = "reset.credit.";
pub const CANCEL: &str = "reset.cancel";
pub const CLOSE: &str = "reset.close";
/// `Try again` of a list that failed to load, or of a credit that was gone.
pub const RELOAD: &str = "reset.reload";
/// `Try again` of a redemption that failed.
pub const RETRY: &str = "reset.retry";
/// The rows of the confirmation.
pub const YES: &str = "confirm.yes";
pub const NO: &str = "confirm.no";

const RESETS_TITLE: &str = "Usage limit resets";
const DEFAULT_TITLE: &str = "Full reset";
const DEFAULT_DESCRIPTION: &str = "Reset your current usage limits";

/// `usage limit reset` or `usage limit resets` (oracle `reset_label`).
fn label(count: i64) -> &'static str {
    if count == 1 {
        "usage limit reset"
    } else {
        "usage limit resets"
    }
}

fn row(id: &str, name: &str, description: Option<String>) -> Item {
    Item {
        id: id.to_owned(),
        name: name.to_owned(),
        description,
        ..Item::default()
    }
}

/// A list of the panel picker (`SelectionViewParams::picker`: columns from all rows, the
/// description hidden below 24 cells).
fn picker(title: &str, subtitle: String, items: Vec<Item>, hint: Option<String>) -> List {
    List {
        title: Some(title.to_owned()),
        subtitle: Some(subtitle),
        footer_hint: hint,
        items,
        columns: Columns::AutoAllRows,
        hide_description_below: Some(24),
        panel: true,
        ..List::default()
    }
}

/// The usage menu (oracle `usage_menu_params`); `hint` is `enter open · esc back` in the
/// keymap's keys.
pub fn menu(entry: MenuEntry, hint: String) -> List {
    let description = match entry {
        MenuEntry::Available(count) => format!("{count} available"),
        MenuEntry::Check => "Check availability".to_owned(),
        MenuEntry::NoneAvailable => "None available".to_owned(),
    };
    let mut redeem = row(REDEEM, "Redeem reset", Some(description));
    redeem.disabled = !entry.enabled();
    picker(
        "Usage",
        "Account usage and resets.".to_owned(),
        vec![
            row(ANALYTICS, "View analytics", Some("Usage history".to_owned())),
            redeem,
        ],
        Some(hint),
    )
}

/// When a credit stops being usable, as the picker and the confirmation say it (oracle
/// `reset_credit_options`): `Expires 09:39 on 18 Jun 2026` in the viewer's clock and zone.
fn expiry(expiry: Expiry, clock: ViewerClock, zone_secs: i32) -> String {
    const UNAVAILABLE: &str = "Expiration unavailable";
    let at = match expiry {
        Expiry::Never => return "Does not expire".to_owned(),
        Expiry::Unavailable => return UNAVAILABLE.to_owned(),
        Expiry::At(at) => at,
    };
    let local = FixedOffset::east_opt(clock.utc_offset.unwrap_or(zone_secs))
        .zip(DateTime::<Utc>::from_timestamp(at, 0))
        .map(|(zone, at)| at.with_timezone(&zone));
    let time = match clock.format {
        ClockFormat::TwelveHour => "%-I:%M %p",
        ClockFormat::TwentyFourHour => "%H:%M",
    };
    match local {
        Some(at) => format!("Expires {} on {}", at.format(time), at.format("%-d %b %Y")),
        None => UNAVAILABLE.to_owned(),
    }
}

/// What the confirmation of one credit says (oracle `show_rate_limit_reset_confirmation`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chosen {
    /// The credit to redeem; none lets the backend choose.
    pub credit_id: Option<String>,
    pub title: String,
    pub detail: Option<String>,
    pub description: String,
}

/// The credit of the picker's n-th row, worded.
pub fn chosen(option: &CreditOption, clock: ViewerClock, zone_secs: i32) -> Chosen {
    Chosen {
        credit_id: option.credit_id.clone(),
        title: option
            .title
            .clone()
            .unwrap_or_else(|| DEFAULT_TITLE.to_owned()),
        detail: option.expiry.map(|at| expiry(at, clock, zone_secs)),
        description: option
            .description
            .clone()
            .unwrap_or_else(|| DEFAULT_DESCRIPTION.to_owned()),
    }
}

/// The picker of the credits (oracle `rate_limit_reset_picker_params`): a row each, and `Cancel`.
pub fn credits(
    count: i64,
    options: &[CreditOption],
    clock: ViewerClock,
    zone_secs: i32,
    hint: String,
) -> List {
    let mut items: Vec<Item> = options
        .iter()
        .enumerate()
        .map(|(n, option)| {
            let chosen = chosen(option, clock, zone_secs);
            row(
                &format!("{CREDIT}{n}"),
                &chosen.title,
                Some(chosen.detail.unwrap_or(chosen.description)),
            )
        })
        .collect();
    items.push(row(CANCEL, "Cancel", None));
    picker(
        RESETS_TITLE,
        format!("{count} {} available.", label(count)),
        items,
        Some(hint),
    )
}

/// The confirmation (oracle `show_rate_limit_reset_confirmation`): `No, go back` is the row the
/// cursor starts on.
pub fn confirmation(chosen: &Chosen, hint: String) -> List {
    let subtitle = match &chosen.detail {
        Some(detail) => format!("{} · {detail}", chosen.title),
        None => chosen.title.clone(),
    };
    let mut list = picker(
        "Use this reset?",
        subtitle,
        vec![
            row(YES, "Yes, use reset", Some(chosen.description.clone())),
            row(
                NO,
                "No, go back",
                Some("Choose a different reset".to_owned()),
            ),
        ],
        Some(hint),
    );
    list.initial = Some(1);
    list
}

/// A page that waits and cannot be left (`Loading...` can: it keeps the standard hint).
fn waiting(subtitle: &str, name: &str, hint: Option<String>) -> List {
    let mut wait = row("", name, None);
    wait.disabled = true;
    picker(RESETS_TITLE, subtitle.to_owned(), vec![wait], hint)
}

/// A message and `Close` (oracle `rate_limit_reset_message_params`).
fn message(text: &str, hint: String) -> List {
    picker(
        RESETS_TITLE,
        text.to_owned(),
        vec![row(CLOSE, "Close", None)],
        Some(hint),
    )
}

/// A message, `Try again` and `Close` (oracle `reset_refresh_params`): `again` is the row id of
/// the retry.
fn again(text: &str, again: &str, hint: String) -> List {
    picker(
        RESETS_TITLE,
        text.to_owned(),
        vec![row(again, "Try again", None), row(CLOSE, "Close", None)],
        Some(hint),
    )
}

/// The reset page for what core says: a redemption's step, or how the list read stands.
/// `hint` is `enter select · esc back` in the keymap's keys.
pub fn resets(
    listing: Listing,
    redemption: Option<&Redemption>,
    count: i64,
    options: &[CreditOption],
    clock: ViewerClock,
    zone_secs: i32,
    hint: &dyn Fn(&str) -> String,
) -> List {
    if let Some(redemption) = redemption {
        return match redemption.step {
            Progress::Asking => waiting("Resetting your usage...", "Using a reset...", None),
            Progress::Refreshing => waiting(
                "Usage reset. Checking your remaining resets...",
                "Refreshing...",
                None,
            ),
            Progress::Settled(Settled::Reset { left: Some(left) }) => message(
                &format!("Usage reset. You have {left} {} left.", label(left)),
                hint("select"),
            ),
            Progress::Settled(Settled::Reset { left: None }) => {
                message("Usage reset.", hint("select"))
            }
            Progress::Settled(Settled::NothingToReset) => message(
                "Your usage does not need a reset right now.",
                hint("select"),
            ),
            Progress::Settled(Settled::NoCredit) => {
                message("No usage limit resets are available.", hint("select"))
            }
            Progress::Settled(Settled::Stale) => again(
                "That reset is no longer available. Refresh to see your current resets.",
                RELOAD,
                hint("select"),
            ),
            Progress::Settled(Settled::Failed) => again(
                "Couldn't reset usage. Please try again.",
                RETRY,
                hint("select"),
            ),
        };
    }
    match listing {
        Listing::Idle | Listing::Checking(_) => waiting(
            "Checking your available resets...",
            "Loading...",
            Some(hint("select")),
        ),
        Listing::Ready => credits(count, options, clock, zone_secs, hint("choose")),
        Listing::Empty => message(
            "You don't have any usage limit resets available.",
            hint("select"),
        ),
        Listing::Failed => again(
            "Couldn't load usage limit resets. Please try again.",
            RELOAD,
            hint("select"),
        ),
    }
}
