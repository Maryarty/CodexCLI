//! The strings of `cells` (S11): a table of `&'static str` fields and typed
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
    /// The mark before every cell's first row (history_cell `"• ".dim()`).
    pub bullet: &'static str,
    /// The indent of a cell's later rows.
    pub indent: &'static str,
    // history_cell/search.rs WebSearchCell::summary
    pub opened_page: &'static str,
    pub opened: &'static str,
    pub opening_page: &'static str,
    pub opening: &'static str,
    pub searched_for: &'static str,
    pub searched_page: &'static str,
    pub searching_for: &'static str,
    pub searching_page: &'static str,
    pub browsing: &'static str,
    pub searched_web: &'static str,
    pub searching_web: &'static str,
    pub web_for: &'static str,
    pub gap: &'static str,
    // history_cell/patches.rs
    pub viewed_image: &'static str,
    pub patch_failed: &'static str,
    pub error_unavailable: &'static str,
    // transcript_view/layout.rs activity_layout
    pub show_details: &'static str,
    pub hide_details: &'static str,
    // diff_render.rs render_changes_block
    pub added: &'static str,
    pub deleted: &'static str,
    pub edited: &'static str,
    pub file: &'static str,
    pub files: &'static str,
    pub moved_to: &'static str,
    pub file_branch: &'static str,
    pub diff_indent: &'static str,
    pub hunk_gap: &'static str,
    pub count_open: &'static str,
    pub plus: &'static str,
    pub minus: &'static str,
    pub count_close: &'static str,
    // history_cell/plans.rs ProposedPlanCell
    pub proposed_plan: &'static str,
    pub empty_plan: &'static str,
    // history_cell/plans.rs PlanUpdateCell
    pub updated_plan: &'static str,
    /// Before the steps done and the steps in all: ` · `, `/`, ` complete`.
    pub plan_progress: (&'static str, &'static str, &'static str),
    pub plan_branch: &'static str,
    pub plan_indent: &'static str,
    pub no_steps: &'static str,
    // thread_transcript/other_items.rs
    pub context_compacted: &'static str,
    pub review_started: (&'static str, &'static str),
    pub review_finished: (&'static str, &'static str),
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "streaming and history cells",
    bullet: "• ",
    indent: "  ",
    opened_page: "Opened page",
    opened: "Opened",
    opening_page: "Opening page",
    opening: "Opening",
    searched_for: "Searched for",
    searched_page: "Searched page",
    searching_for: "Searching for",
    searching_page: "Searching page",
    browsing: "Browsing the web",
    searched_web: "Searched the web",
    searching_web: "Searching the web",
    web_for: " for ",
    gap: " ",
    viewed_image: "Viewed image ",
    patch_failed: "✘ Failed to apply patch",
    error_unavailable: "(error details unavailable)",
    show_details: "+ Show details",
    hide_details: "− Hide details",
    added: "Added",
    deleted: "Deleted",
    edited: "Edited",
    file: "file",
    files: "files",
    moved_to: " → ",
    file_branch: "  └ ",
    diff_indent: "    ",
    hunk_gap: "⋮",
    count_open: "(",
    plus: "+",
    minus: "-",
    count_close: ")",
    proposed_plan: "Proposed Plan",
    empty_plan: "(empty)",
    updated_plan: "Updated Plan",
    plan_progress: (" · ", "/", " complete"),
    plan_branch: "  └ ",
    plan_indent: "    ",
    no_steps: "(no steps provided)",
    context_compacted: "Context compacted",
    review_started: (">> Code review started: ", " <<"),
    review_finished: ("<< Code review finished: ", " >>"),
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
