//! The strings of `pets` (S24): a table of `&'static str` fields and typed
//! messages per locale, read by reference through `of(cx)`. `EN` is the
//! oracle's text byte for byte; each group of strings names the oracle file
//! it comes from. A language is one more `const` here, registered with
//! `with_ru(&RU)`-style on `TABLES`; a screen never changes for it. Nothing
//! else in this directory holds user-visible text (the rules test
//! `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::DependCx;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    // pets/picker.rs: `build_pet_picker_params`.
    pub picker_title: &'static str,
    pub picker_subtitle: &'static str,
    pub search_placeholder: &'static str,
    /// The disable row (`available_pet_entries`).
    pub disable_row: &'static str,
    /// pets/preview.rs: `PetPickerPreviewStatus::Loading`.
    pub preview_loading: &'static str,
    /// The two columns before a row of the pane: the cursor's `›` or blanks.
    pub cursor_marker: &'static str,
    pub indent: &'static str,
    /// The marker under a list with rows below the window.
    pub more_below: &'static str,
    /// The marker above a list with rows above the window.
    pub more_above: &'static str,
    /// The footer the oracle's snapshot shows.
    pub hint: &'static str,
    /// bottom_pane/list_selection_view.rs: the marker of the current row.
    pub current_marker: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "pets overlay",
    picker_title: "Select Pet",
    picker_subtitle: "Choose a pet to wake in the terminal.",
    search_placeholder: "Type to filter pets...",
    disable_row: "Disable terminal pets",
    preview_loading: "Loading preview...",
    cursor_marker: "\u{203a} ",
    indent: "  ",
    more_below: "\u{2193}",
    more_above: "\u{2191}",
    hint: "enter select \u{b7} esc back",
    current_marker: " (current)",
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
