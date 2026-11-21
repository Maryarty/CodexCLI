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
    // history_cell/hook_cell.rs
    pub hook_provenance: &'static str,
    pub hook_bullet: &'static str,
    pub hook_after_bullet: &'static str,
    pub hook_completed: &'static str,
    pub hook_failed: &'static str,
    pub hook_blocked: &'static str,
    pub hook_stopped: &'static str,
    pub hook_branch: &'static str,
    pub hook_body_indent: &'static str,
    // history_cell/notices.rs
    pub error_line: &'static str,
    pub warning_mark: &'static str,
    // multi_agents.rs
    pub collab_bullet: &'static str,
    pub collab_spawned: &'static str,
    pub collab_spawn_failed: &'static str,
    pub collab_sent_input: &'static str,
    pub collab_waiting_for: &'static str,
    pub collab_waiting_agents: &'static str,
    pub collab_waiting_count: &'static str,
    pub collab_finished_waiting: &'static str,
    pub collab_none_completed: &'static str,
    pub collab_closed: &'static str,
    pub collab_resuming: &'static str,
    pub collab_resumed: &'static str,
    pub collab_resume_failed: &'static str,
    pub collab_started: &'static str,
    pub collab_interacted: &'static str,
    pub collab_interrupted: &'static str,
    pub collab_completed: &'static str,
    pub collab_state_separator: &'static str,
    pub collab_message_separator: &'static str,
    pub collab_pending_init: &'static str,
    pub collab_running: &'static str,
    pub collab_interrupted_state: &'static str,
    pub collab_completed_state: &'static str,
    pub collab_errored: &'static str,
    pub collab_error: &'static str,
    pub collab_shutdown: &'static str,
    pub collab_not_found: &'static str,
    pub collab_tick: &'static str,
    // history_cell/delegated_message.rs, patches.rs
    pub delegated_attribution: &'static str,
    pub generated_heading: &'static str,
    pub generated_failed: &'static str,
    pub generated_cross: &'static str,
    pub generated_saved_to: &'static str,
    pub collab_role_open: &'static str,
    pub collab_role_close: &'static str,
    pub collab_request: &'static str,
    pub collab_request_effort: &'static str,
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
    hook_provenance: "↳ Hook · ",
    hook_bullet: "•",
    hook_after_bullet: " ",
    hook_completed: "Hook completed",
    hook_failed: "Hook failed",
    hook_blocked: "Blocked by hook",
    hook_stopped: "Hook stopped",
    hook_branch: "  └ ",
    hook_body_indent: "    ",
    error_line: "■ {}",
    warning_mark: "⚠ ",
    collab_bullet: "• ",
    collab_spawned: "Spawned",
    collab_spawn_failed: "Agent spawn failed",
    collab_sent_input: "Sent input to",
    collab_waiting_for: "Waiting for",
    collab_waiting_agents: "Waiting for agents",
    collab_waiting_count: "Waiting for {} agents",
    collab_finished_waiting: "Finished waiting",
    collab_none_completed: "No agents completed yet",
    collab_closed: "Closed",
    collab_resuming: "Resuming",
    collab_resumed: "Resumed",
    collab_resume_failed: "Agent resume failed",
    collab_started: "Started ",
    collab_interacted: "Interacted with ",
    collab_interrupted: "Interrupted ",
    collab_completed: "Completed ",
    collab_state_separator: ": ",
    collab_message_separator: " - ",
    collab_pending_init: "Pending init",
    collab_running: "Running",
    collab_interrupted_state: "Interrupted",
    collab_completed_state: "Completed",
    collab_errored: "Agent errored",
    collab_error: "Error",
    collab_shutdown: "Shutdown",
    collab_not_found: "Not found",
    collab_tick: "`",
    delegated_attribution: "Sent by Codex from task {}",
    generated_heading: "Generated Image:",
    generated_failed: "Image generation failed",
    generated_cross: "✗ ",
    generated_saved_to: "Saved to: ",
    collab_role_open: "[",
    collab_role_close: "]",
    collab_request: "({model} {effort})",
    collab_request_effort: "({})",
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
