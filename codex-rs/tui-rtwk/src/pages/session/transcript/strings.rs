//! The strings of `transcript` (S10): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::DependCx;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// The follow control's labels, the widest first, a blank each side
    /// (`transcript_view/follow_control.rs`); the last that fits the row.
    pub follow_back: [&'static str; 5],
    /// The same while the transcript has news (`unseen_activity`).
    pub follow_new: [&'static str; 5],
    /// The footer's hint for the way back (`app/owned_transcript.rs`,
    /// `transcript_view/footer.rs`): the keys, then what they do.
    pub latest_keys_enter: &'static str,
    pub latest_keys: &'static str,
    pub latest_action: &'static str,
    /// The news in the footer, whole and short, and what joins it to the keys.
    pub new_activity: &'static str,
    pub new_short: &'static str,
    pub footer_joiner: &'static str,
    /// What the empty composer adds after the hint while it fits: the key
    /// that opens the shortcuts and its action (`render_transcript_footer`).
    pub shortcuts_key: &'static str,
    pub shortcuts_action: &'static str,
    /// The static pager's chrome (`pager_overlay.rs`): the header's slashes
    /// and the title's lead, the filler row, the rule and the percent.
    pub pager_slashes: &'static str,
    pub pager_title_lead: &'static str,
    pub pager_filler: &'static str,
    pub pager_rule: &'static str,
    /// What comes before and after the percent scrolled (` 100% `).
    pub pager_percent: [&'static str; 2],
    /// The hints under it: each pair of keys and what they do, joined by
    /// `hint_joiner`; the keys of a pair are joined by `hint_slash`.
    pub hint_joiner: &'static str,
    pub hint_slash: &'static str,
    pub key_up: &'static str,
    pub key_down: &'static str,
    pub key_page_up: &'static str,
    pub key_page_down: &'static str,
    pub key_home: &'static str,
    pub key_end: &'static str,
    pub key_close: &'static str,
    pub key_enter: &'static str,
    pub hint_scroll: &'static str,
    pub hint_page: &'static str,
    pub hint_jump: &'static str,
    pub hint_close: &'static str,
    pub action_edit_configuration: &'static str,
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "transcript view",
    follow_back: [
        " ↓ Back to bottom · esc ",
        " ↓ Back to bottom ",
        " ↓ Bottom · esc ",
        " ↓ Bottom ",
        " ↓ ",
    ],
    follow_new: [
        " New activity · ↓ Back to bottom · esc ",
        " New activity · ↓ Bottom ",
        " New · ↓ Bottom ",
        " ↓ Bottom ",
        " ↓ ",
    ],
    latest_keys_enter: "enter/esc",
    latest_keys: "esc",
    latest_action: " latest",
    new_activity: "New activity",
    new_short: "New",
    footer_joiner: " · ",
    shortcuts_key: "?",
    shortcuts_action: " shortcuts",
    pager_slashes: "/ ",
    pager_title_lead: "/ ",
    pager_filler: "~",
    pager_rule: "─",
    pager_percent: [" ", "% "],
    hint_joiner: " · ",
    hint_slash: "/",
    key_up: "↑",
    key_down: "↓",
    key_page_up: "pgup",
    key_page_down: "pgdn",
    key_home: "home",
    key_end: "end",
    key_close: "q",
    key_enter: "enter",
    hint_scroll: "to scroll",
    hint_page: "to page",
    hint_jump: "to jump",
    hint_close: "close",
    action_edit_configuration: "Edit configuration",
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
