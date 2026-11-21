//! The strings of `select` (S14): a table of `&'static str` fields per locale,
//! read by reference through `of(cx)`. `EN` is the oracle's text byte for
//! byte; each group names the oracle file it comes from. A language is one
//! more `const` here, registered with `with_ru(&RU)` on `TABLES`; a screen
//! never changes for it. Nothing else in this directory holds user-visible
//! text (the rules test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::DependCx;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// bottom_pane/list_selection_view.rs (model picker test).
    pub model_title: &'static str,
    /// The mark after the name of the row in use (`is_current`).
    pub current: &'static str,
    /// chatwidget/permissions.rs: the `/permissions` popup.
    pub permissions_title: &'static str,
    pub permissions_hint: &'static str,
    pub access_ask: &'static str,
    pub access_ask_description: &'static str,
    pub access_full: &'static str,
    pub access_full_description: &'static str,
    /// bottom_pane/selection_popup_common.rs: the empty list, the scroll
    /// indicators and the mark after a disabled row's name.
    pub no_matches: &'static str,
    pub more_above: &'static str,
    pub more_below: &'static str,
    pub disabled_mark: &'static str,
    /// A disabled row's reason after its description, or alone, and the
    /// closing of the first.
    pub reason_after: &'static str,
    pub reason_alone: &'static str,
    pub reason_close: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "generic selection dialog",
    model_title: "Select Model and Effort",
    current: " (current)",
    permissions_title: "Update Model Permissions",
    permissions_hint: "enter select · esc back",
    access_ask: "Ask for approval",
    access_ask_description: "Read and edit workspace files and run commands, with approval required for internet access or edits outside the workspace",
    access_full: "Full Access",
    access_full_description: "Use with caution: Codex can edit files outside this workspace and access the internet without approval",
    no_matches: "no matches",
    more_above: "\u{2191}",
    more_below: "\u{2193}",
    disabled_mark: " (disabled)",
    reason_after: " (disabled: ",
    reason_alone: "disabled: ",
    reason_close: ")",
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
