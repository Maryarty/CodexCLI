//! The strings of the picker: a table of `&'static str` fields per locale,
//! read by reference through `of(cx)`. `EN` is the oracle's text byte for
//! byte (bottom_pane/selection_popup_common.rs, selection_tabs.rs). A language
//! is one more `const` here, registered with `with_ru(&RU)` on `TABLES`.

use rtwk::l10n::DependCx;

use crate::l10n::Tables;
use crate::l10n::lang;

/// The words the picker adds to the rows a caller hands it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The mark after the name of the row in use (`is_current`).
    pub current: &'static str,
    /// The empty list, the scroll indicators and the mark after a disabled
    /// row's name.
    pub no_matches: &'static str,
    pub more_above: &'static str,
    pub more_below: &'static str,
    pub disabled_mark: &'static str,
    /// The cell before a row: the cursor's mark, or blank.
    pub mark_on: &'static str,
    pub mark_off: &'static str,
    /// The tab strip's marks for tabs out of room.
    pub tab_left: &'static str,
    pub tab_right: &'static str,
    /// A disabled row's reason after its description, or alone, and the
    /// closing of the first.
    pub reason_after: &'static str,
    pub reason_alone: &'static str,
    pub reason_close: &'static str,
    /// The rule row of a section break (multi-select picker).
    pub section_break: &'static str,
    /// Between the names of the preview line.
    pub preview_join: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    current: " (current)",
    no_matches: "no matches",
    more_above: "\u{2191}",
    more_below: "\u{2193}",
    disabled_mark: " (disabled)",
    mark_on: "\u{203a}",
    mark_off: " ",
    tab_left: "\u{2039}",
    tab_right: "\u{203a}",
    reason_after: " (disabled: ",
    reason_alone: "disabled: ",
    reason_close: ")",
    section_break: "  ───────────────────────",
    preview_join: " · ",
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
