//! The strings of `select` (S14): a table of `&'static str` fields per locale,
//! read by reference through `of(cx)`. `EN` is the oracle's text byte for
//! byte; each group names the oracle file it comes from. A language is one
//! more `const` here, registered with `with_ru(&RU)` on `TABLES`; a screen
//! never changes for it. The words every picker adds to its rows are the
//! picker's (`widgets/picker/strings.rs`). Nothing else in this directory
//! holds user-visible text (the rules test
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
    /// bottom_pane/list_selection_view.rs (model picker test).
    pub model_title: &'static str,
    /// chatwidget/permissions.rs: the `/permissions` popup.
    pub permissions_title: &'static str,
    pub permissions_hint: &'static str,
    pub access_ask: &'static str,
    pub access_ask_description: &'static str,
    pub access_full: &'static str,
    pub access_full_description: &'static str,
    /// chatwidget/slash_dispatch.rs: the confirmations of `/archive`, `/delete`.
    pub archive_title: &'static str,
    pub archive_subtitle: &'static str,
    pub archive_subtitle_running: &'static str,
    pub archive_no: &'static str,
    pub archive_no_description: &'static str,
    pub archive_yes: &'static str,
    pub archive_yes_description: &'static str,
    pub delete_title: &'static str,
    pub delete_subtitle: &'static str,
    pub delete_no: &'static str,
    pub delete_no_description: &'static str,
    pub delete_yes: &'static str,
    pub delete_yes_description: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "generic selection dialog",
    model_title: "Select Model and Effort",
    permissions_title: "Update Model Permissions",
    permissions_hint: "enter select · esc back",
    access_ask: "Ask for approval",
    access_ask_description: "Read and edit workspace files and run commands, with approval required for internet access or edits outside the workspace",
    access_full: "Full Access",
    access_full_description: "Use with caution: Codex can edit files outside this workspace and access the internet without approval",
    archive_title: "Archive this session?",
    archive_subtitle: "Are you sure? This will archive the current session",
    archive_subtitle_running: "This will stop the current turn and archive the session.",
    archive_no: "No, don't archive",
    archive_no_description: "Return to the current session",
    archive_yes: "Yes, archive",
    archive_yes_description: "Archive this session now",
    delete_title: "Delete this session?",
    delete_subtitle: "Also deletes subagents and saved side chats. Cannot be undone.",
    delete_no: "No, keep this session",
    delete_no_description: "Return to the current session",
    delete_yes: "Yes, delete and return to command center",
    delete_yes_description: "Permanently delete this session now",
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
