//! The strings of `composer` (S10): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::{Arg, DependCx, Fmt, Msg, NumSyms, msg};

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// What a large paste shows in the draft (`chat_composer.rs`,
    /// `next_large_paste_placeholder`).
    pub pasted_content: Msg<1>,
    /// What an attached image shows in the draft
    /// (`codex_protocol::models::local_image_label_text`).
    pub image_label: Msg<1>,
    /// Shell mode's prompt and footer label (`chat_composer.rs`
    /// `shell_mode_footer_line`).
    pub shell_prompt: &'static str,
    /// The footer's flash of a plain-text composer that refused an over-long
    /// message (`chat_composer.rs`, `show_footer_flash`): the limit.
    pub message_too_long: Msg<1>,
    pub shell_mode: &'static str,
    /// The placeholder of a thread its parent controls (`set_parent_owned_thread`).
    pub parent_placeholder: &'static str,
    /// The placeholder while the app closes (`show_shutdown_in_progress`).
    pub shutting_down: &'static str,
    /// The placeholder of a disabled composer with no word of its own.
    pub input_disabled: &'static str,
    /// The footer while reverse history search is open
    /// (`history_search.rs`, `history_search_footer_line`): the label, the
    /// state after the query, and the two actions of a match, each key and
    /// its word.
    pub search_label: &'static str,
    pub search_gap: &'static str,
    pub search_no_match: &'static str,
    pub search_searching: &'static str,
    pub search_accept_key: &'static str,
    pub search_accept: &'static str,
    pub search_cancel_key: &'static str,
    pub search_cancel: &'static str,
    pub search_between: &'static str,
    pub num: NumSyms,
}

impl Strings {
    /// The placeholder of a paste of `chars` characters, the count in
    /// plain digits as the oracle prints it.
    pub fn pasted(&self, chars: usize) -> String {
        Fmt::new(
            self.pasted_content,
            [Arg::Text(&chars.to_string())],
            &self.num,
        )
        .to_string()
    }

    /// The flash of a message over `limit` characters.
    pub fn too_long(&self, limit: usize) -> String {
        Fmt::new(
            self.message_too_long,
            [Arg::Text(&limit.to_string())],
            &self.num,
        )
        .to_string()
    }

    /// The placeholder of the `n`-th attached image.
    pub fn image(&self, n: usize) -> String {
        Fmt::new(self.image_label, [Arg::Text(&n.to_string())], &self.num).to_string()
    }
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "composer",
    pasted_content: msg!("[Pasted Content {0} chars]"),
    image_label: msg!("[Image #{0}]"),
    shell_prompt: "!",
    message_too_long: msg!("Message too long; limit {0} characters"),
    shell_mode: "Shell mode",
    parent_placeholder: "Viewing sub-agent \u{2014} direct input is disabled",
    shutting_down: "Shutting down...",
    input_disabled: "Input disabled.",
    search_label: "reverse-i-search: ",
    search_gap: "  ",
    search_no_match: "  no match",
    search_searching: "  searching",
    search_accept_key: "enter",
    search_accept: " accept",
    search_cancel_key: "esc",
    search_cancel: " cancel",
    search_between: " \u{b7} ",
    num: NumSyms::ENGLISH,
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
