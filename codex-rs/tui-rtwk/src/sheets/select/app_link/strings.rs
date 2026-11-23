//! The words of the app link dialog (oracle `bottom_pane/app_link_view.rs`, the instructions of
//! `chatwidget/connectors.rs`): a table of `&'static str` fields per locale, read by reference
//! through `of(cx)`. `EN` is the oracle's text byte for byte. A language is one more `const`
//! here, registered with `with_ru(&RU)` on `TABLES`.

use rtwk::l10n::DependCx;

use crate::l10n::Tables;
use crate::l10n::lang;

/// The words of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    // The actions.
    pub manage: &'static str,
    pub install: &'static str,
    pub disable: &'static str,
    pub enable: &'static str,
    pub back: &'static str,
    pub installed_it: &'static str,
    // The link screen's text.
    pub instructions_manage: &'static str,
    pub instructions_install: &'static str,
    pub use_mention: &'static str,
    pub appears_later: &'static str,
    pub use_after_install: &'static str,
    // The confirmation screen.
    pub finish_title: &'static str,
    pub finish_open: &'static str,
    pub finish_then: &'static str,
    pub setup_url: &'static str,
    // The hint under the actions (`Tab / ↑ ↓ move · enter select · esc close`).
    pub hint_tab: &'static str,
    pub hint_or: &'static str,
    pub hint_space: &'static str,
    pub hint_move: &'static str,
    pub hint_join: &'static str,
    pub hint_select: &'static str,
    pub hint_close: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    manage: "Manage on ChatGPT",
    install: "Install on ChatGPT",
    disable: "Disable app",
    enable: "Enable app",
    back: "Back",
    installed_it: "I already Installed it",
    instructions_manage: "Manage this app in your browser.",
    instructions_install: "Install this app in your browser, then reload Codex.",
    use_mention: "Use $ to insert this app into the prompt.",
    appears_later: "Newly installed apps can take a few minutes to appear in /apps.",
    use_after_install: "After installed, use $ to insert this app into the prompt.",
    finish_title: "Finish App Setup",
    finish_open: "Complete app setup on ChatGPT in the browser window that just opened.",
    finish_then: "Sign in there if needed, then return here and select \"I already Installed it\".",
    setup_url: "Setup URL:",
    hint_tab: "tab",
    hint_or: " / ",
    hint_space: " ",
    hint_move: " move",
    hint_join: " · ",
    hint_select: " select",
    hint_close: " close",
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
