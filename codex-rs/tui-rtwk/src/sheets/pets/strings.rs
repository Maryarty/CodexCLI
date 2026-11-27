//! The strings of `pets` (S24): a table of `&'static str` fields and typed
//! messages per locale, read by reference through `of(cx)`. `EN` is the
//! oracle's text byte for byte; each group of strings names the oracle file
//! it comes from. A language is one more `const` here, registered with
//! `with_ru(&RU)`-style on `TABLES`; a screen never changes for it. Nothing
//! else in this directory holds user-visible text (the rules test
//! `adr010_no_user_visible_literal_outside_the_string_tables`).

use codex_tui_core::screens::pets::Unavailable;
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
    pub preview_unavailable: &'static str,
    pub preview_disabled: &'static str,
    pub preview_disabled_detail: &'static str,
    pub catalog_loading: &'static str,
    pub unavailable_title: &'static str,
    pub unavailable_terminal: &'static str,
    pub unavailable_tmux: &'static str,
    pub unavailable_zellij: &'static str,
    pub unavailable_iterm: &'static str,
    pub apply_title: &'static str,
    pub apply_detail: &'static str,
    pub close: &'static str,
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
    preview_unavailable: "Preview unavailable",
    preview_disabled: "Terminal pets disabled",
    preview_disabled_detail: "No pet will be shown.",
    catalog_loading: "Loading terminal pets...",
    unavailable_title: "Pets unavailable",
    unavailable_terminal: "Pets aren’t available in this terminal. Terminal pets need image support, and this terminal environment doesn’t expose a supported image protocol. Try a terminal with Kitty graphics or Sixel support, or run Codex outside tmux.",
    unavailable_tmux: "Pets are unavailable in this tmux pane. Use fullscreen Codex with allow-passthrough all, or run Codex outside tmux.",
    unavailable_zellij: "Pets are disabled in Zellij. Terminal images don’t stay reliably pane-local in Zellij. Run Codex outside Zellij to use pets.",
    unavailable_iterm: "Pets require iTerm2 3.6 or newer. Upgrade iTerm2 to use terminal pets.",
    apply_title: "Loading Pet",
    apply_detail: "Preparing the terminal pet.",
    close: "1. Close",
    cursor_marker: "\u{203a} ",
    indent: "  ",
    more_below: "\u{2193}",
    more_above: "\u{2191}",
    hint: "enter select \u{b7} esc back",
    current_marker: " (current)",
};

impl Strings {
    pub fn unavailable(&self, reason: Unavailable) -> &'static str {
        match reason {
            Unavailable::Terminal => self.unavailable_terminal,
            Unavailable::Tmux => self.unavailable_tmux,
            Unavailable::Zellij => self.unavailable_zellij,
            Unavailable::OldIterm => self.unavailable_iterm,
        }
    }
}

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
