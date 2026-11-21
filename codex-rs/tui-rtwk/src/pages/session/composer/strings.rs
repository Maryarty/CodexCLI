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
    pub shell_mode: &'static str,
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
    shell_mode: "Shell mode",
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
