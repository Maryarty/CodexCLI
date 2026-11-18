//! The strings of `pets` (S24): a table of `&'static str` fields and typed
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
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings { title: "pets overlay" };

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
