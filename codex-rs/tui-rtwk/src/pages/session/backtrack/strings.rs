//! The strings of `backtrack` (S17): a table of `&'static str` fields and typed
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
    /// `restore_backtrack_prompt_after_revert_error` (`app_backtrack.rs`): before the
    /// server's own message.
    pub failed: &'static str,
    /// `SIDE_EDIT_PREVIOUS_UNAVAILABLE_MESSAGE` (`app_backtrack.rs`).
    pub side: &'static str,
    /// The revert was not confirmed in time (`event_dispatch.rs`, the error after a
    /// revert the server did not answer).
    pub not_confirmed: &'static str,
}

impl Strings {
    /// The error line of a failed edit, with the server's message.
    pub fn failed_with(&self, message: &str) -> String {
        [self.failed, message].concat()
    }
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "backtrack (Esc Esc)",
    failed: "Failed to edit the selected prompt: ",
    side: "Editing previous prompts is unavailable in side conversations.",
    not_confirmed: "prompt edit could not be confirmed; resume this session to reload its history",
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
