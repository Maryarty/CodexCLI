//! The words of `/review` (oracle `chatwidget/review_popups.rs`, `bottom_pane/custom_prompt_view`):
//! a table of `&'static str` fields per locale, read by reference through `of(cx)`. `EN` is the
//! oracle's text byte for byte. A language is one more `const` here, registered with
//! `with_ru(&RU)` on `TABLES`.

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use crate::l10n::Tables;
use crate::l10n::lang;

/// The words of the review menus and of the instructions prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    // `open_review_popup`: the four presets.
    pub presets_title: &'static str,
    pub base_name: &'static str,
    pub base_description: &'static str,
    pub uncommitted_name: &'static str,
    pub commit_name: &'static str,
    pub custom_name: &'static str,
    // `show_review_branch_picker`.
    pub branches_title: &'static str,
    pub current_branch: Msg<1>,
    pub detached_head: &'static str,
    pub branches_search: &'static str,
    // `show_review_commits`.
    pub commits_title: &'static str,
    pub commits_search: &'static str,
    // `show_review_custom_prompt`, and the prompt's layout (`picker.rs`).
    pub instructions_title: &'static str,
    pub instructions_placeholder: &'static str,
    /// The mark before the input (`›`).
    pub num: NumSyms,
}

impl Strings {
    /// `Current branch: {name}`, `(detached HEAD)` on none.
    pub fn current_branch(&self, current: Option<&str>) -> String {
        let name = current.unwrap_or(self.detached_head);
        Fmt::new(self.current_branch, [Arg::Text(name)], &self.num).to_string()
    }
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    presets_title: "Select a review preset",
    base_name: "Review against a base branch",
    base_description: "(PR Style)",
    uncommitted_name: "Review uncommitted changes",
    commit_name: "Review a commit",
    custom_name: "Custom review instructions",
    branches_title: "Select a base branch",
    current_branch: msg!("Current branch: {0}"),
    detached_head: "(detached HEAD)",
    branches_search: "Type to search branches",
    commits_title: "Select a commit to review",
    commits_search: "Type to search commits",
    instructions_title: "Custom review instructions",
    instructions_placeholder: "Type instructions and press Enter",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
