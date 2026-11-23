//! The strings of `transcript` (S10): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::DependCx;

use codex_tui_core::app::effects::CopyStatus;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// The follow control's labels, the widest first, a blank each side
    /// (`transcript_view/follow_control.rs`); the last that fits the row.
    pub follow_back: [&'static str; 5],
    /// The same while the transcript has news (`unseen_activity`).
    pub follow_new: [&'static str; 5],
    /// The footer's hint for the way back (`app/owned_transcript.rs`,
    /// `transcript_view/footer.rs`): the keys, then what they do.
    pub latest_keys_enter: &'static str,
    pub latest_keys: &'static str,
    pub latest_action: &'static str,
    /// The news in the footer, whole and short, and what joins it to the keys.
    pub new_activity: &'static str,
    pub new_short: &'static str,
    pub footer_joiner: &'static str,
    /// What the empty composer adds after the hint while it fits: the key
    /// that opens the shortcuts and its action (`render_transcript_footer`).
    pub shortcuts_key: &'static str,
    pub shortcuts_action: &'static str,
    /// The footer's hint while text is selected (`footer.rs` `selection_hint`): the widest
    /// first, each as pairs of keys and what they do; the last that fits the row.
    pub selection_hints: [&'static [(&'static str, &'static str)]; 4],
    /// The static pager's chrome (`pager_overlay.rs`): the header's slashes
    /// and the title's lead, the filler row, the rule and the percent.
    pub pager_slashes: &'static str,
    pub pager_title_lead: &'static str,
    pub pager_filler: &'static str,
    pub pager_rule: &'static str,
    /// What comes before and after the percent scrolled (` 100% `).
    pub pager_percent: [&'static str; 2],
    /// The hints under it: each pair of keys and what they do, joined by
    /// `hint_joiner`; the keys of a pair are joined by `hint_slash`.
    pub hint_gap: &'static str,
    pub hint_joiner: &'static str,
    pub hint_slash: &'static str,
    pub key_up: &'static str,
    pub key_down: &'static str,
    pub key_page_up: &'static str,
    pub key_page_down: &'static str,
    pub key_home: &'static str,
    pub key_end: &'static str,
    pub key_close: &'static str,
    pub key_enter: &'static str,
    pub hint_scroll: &'static str,
    pub hint_page: &'static str,
    pub hint_jump: &'static str,
    pub hint_close: &'static str,
    pub action_edit_configuration: &'static str,
    /// The transcript sheet (`pager_overlay/transcript.rs`): its title, the
    /// status row's select key and its action, what it says while older
    /// messages can still be loaded (`status_line_with_navigation`), and the
    /// keys and hints of the second row.
    pub sheet_title: &'static str,
    pub status_select_key: &'static str,
    pub status_select_action: &'static str,
    pub history_partial: &'static str,
    /// While an older page is on its way: the arrow, the text, its short spellings, the news
    /// after it, and the failure of a page with the keys that ask again
    /// (`status_line_with_navigation`, `footer`).
    pub loading_arrow: &'static str,
    pub history_loading: &'static str,
    pub loading_short: &'static str,
    pub loading_short_joined: &'static str,
    pub loading_ellipsis: &'static str,
    pub loading_ellipsis_joined: &'static str,
    pub loading_news: &'static str,
    pub history_retry: &'static str,
    pub history_retry_end: &'static str,
    pub retry_short: &'static str,
    pub retry_selected: &'static str,
    pub begin_keys: &'static str,
    pub retry_keys: &'static str,
    pub retry_action: &'static str,
    pub key_find: &'static str,
    pub key_esc: &'static str,
    pub key_right: &'static str,
    pub hint_find: &'static str,
    pub hint_browse: &'static str,
    pub hint_edit_next: &'static str,
    pub hint_edit_message: &'static str,
    /// The status row while prompts are browsed (`prompt_navigation_footer`):
    /// the label, whole and short, then the keys and what each does.
    pub browsing_label: [&'static str; 2],
    pub browsing_scroll_keys: &'static str,
    pub browsing_prompt_keys: &'static str,
    pub browsing_both_keys: &'static str,
    pub browsing_details_key: &'static str,
    pub browsing_rewind_key: &'static str,
    pub browsing_scroll: &'static str,
    pub browsing_prompts: &'static str,
    pub browsing_details: &'static str,
    pub browsing_rewind: &'static str,
    pub browsing_back: &'static str,
    /// The disclosure rows of a finished turn (`transcript_view/turn_details.rs`): the arrow of a
    /// closed, an open and a search group, what makes a call a search, and the label of the row
    /// that holds a turn's earlier messages (one message, many).
    pub fold_closed: &'static str,
    pub fold_open: &'static str,
    pub fold_search: &'static str,
    pub search_prefix: &'static str,
    pub previous_message: &'static str,
    pub previous_messages: &'static str,
    /// A group of calls: `{} tool calls · ` before the last call's label, `Ran {} commands`,
    /// `Explored files`, and the clipped label's mark.
    pub tool_calls: &'static str,
    pub ran_command: &'static str,
    pub ran_commands: &'static str,
    pub explored_files: &'static str,
    pub explored_ran_command: &'static str,
    pub explored_ran_commands: &'static str,
    pub label_more: &'static str,
    /// The label of one call (`tool_call_labels`): the verb running and finished, what follows.
    pub reading: &'static str,
    pub read: &'static str,
    pub searching_for: &'static str,
    pub searched_for: &'static str,
    pub searching_files_in: &'static str,
    pub searched_files_in: &'static str,
    pub listing_files_in: &'static str,
    pub listed_files_in: &'static str,
    pub running: &'static str,
    pub ran: &'static str,
    pub search_in: &'static str,
    pub exit_code: &'static str,
    pub edited_file: &'static str,
    pub edited_files: &'static str,
    pub patch_failed: &'static str,
    pub updated_plan: &'static str,
    pub viewed_image: &'static str,
    pub image_failed: &'static str,
    pub image_generated: &'static str,
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "transcript view",
    follow_back: [
        " ↓ Back to bottom · esc ",
        " ↓ Back to bottom ",
        " ↓ Bottom · esc ",
        " ↓ Bottom ",
        " ↓ ",
    ],
    follow_new: [
        " New activity · ↓ Back to bottom · esc ",
        " New activity · ↓ Bottom ",
        " New · ↓ Bottom ",
        " ↓ Bottom ",
        " ↓ ",
    ],
    latest_keys_enter: "enter/esc",
    latest_keys: "esc",
    latest_action: " latest",
    new_activity: "New activity",
    new_short: "New",
    footer_joiner: " · ",
    shortcuts_key: "?",
    shortcuts_action: " shortcuts",
    selection_hints: [
        &[
            ("⌃c", " copy"),
            ("enter", " copy & follow"),
            ("esc", " clear"),
        ],
        &[("enter", " copy & follow"), ("esc", " clear")],
        &[("enter", " copy+↓"), ("esc", "")],
        &[("esc", " clear")],
    ],
    // a run longer than any row, clipped by the row (`"/ ".repeat(width / 2)`)
    pager_slashes: "/ / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / / ",
    pager_title_lead: "/ ",
    pager_filler: "~",
    pager_rule: "─",
    pager_percent: [" ", "% "],
    hint_gap: " ",
    hint_joiner: " · ",
    hint_slash: "/",
    key_up: "↑",
    key_down: "↓",
    key_page_up: "pgup",
    key_page_down: "pgdn",
    key_home: "home",
    key_end: "end",
    key_close: "q",
    key_enter: "enter",
    hint_scroll: "to scroll",
    hint_page: "to page",
    hint_jump: "to jump",
    hint_close: "close",
    action_edit_configuration: "Edit configuration",
    sheet_title: "T R A N S C R I P T",
    status_select_key: "⌃space",
    status_select_action: " select",
    history_partial: "Earlier messages available.  ",
    loading_arrow: "↑ ",
    history_loading: "Loading earlier messages…",
    loading_short: "Loading…",
    loading_short_joined: "Loading… · ",
    loading_ellipsis: "…",
    loading_ellipsis_joined: "… ",
    loading_news: " · New activity",
    history_retry: "Retry history: ",
    history_retry_end: ".  ",
    retry_short: "Retry: ",
    retry_selected: " · Retry: ",
    begin_keys: "⌥</⌃home",
    retry_keys: "⌃home",
    retry_action: " retry",
    key_find: "f3",
    key_esc: "esc",
    key_right: "→",
    hint_find: "find",
    hint_browse: "browse prompts",
    hint_edit_next: "to edit next",
    hint_edit_message: "to edit message",
    browsing_label: ["Browsing transcript", "Browsing"],
    browsing_scroll_keys: "↑↓/jk",
    browsing_prompt_keys: "←→/hl",
    browsing_both_keys: "↑↓/jk ←→/hl",
    browsing_details_key: "⌃t",
    browsing_rewind_key: "↵",
    browsing_scroll: "scroll",
    browsing_prompts: "prompts",
    browsing_details: "details",
    browsing_rewind: "rewind",
    browsing_back: "back",
    fold_closed: "▸",
    fold_open: "▾",
    fold_search: "⌕",
    search_prefix: "Search",
    previous_message: "1 previous message",
    previous_messages: "{} previous messages",
    tool_calls: "{} tool calls · ",
    ran_command: "Ran {} command",
    ran_commands: "Ran {} commands",
    explored_files: "Explored files",
    explored_ran_command: "Explored files · ran {} command",
    explored_ran_commands: "Explored files · ran {} commands",
    label_more: " …",
    reading: "Reading",
    read: "Read",
    searching_for: "Searching for",
    searched_for: "Searched for",
    searching_files_in: "Searching files in",
    searched_files_in: "Searched files in",
    listing_files_in: "Listing files in",
    listed_files_in: "Listed files in",
    running: "Running",
    ran: "Ran",
    search_in: " in ",
    exit_code: " (exit {})",
    edited_file: "Edited {}",
    edited_files: "Edited files",
    patch_failed: "Failed to apply patch",
    updated_plan: "Updated plan",
    viewed_image: "Viewed image {}",
    image_failed: "Image generation failed",
    image_generated: "Generated image",
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

/// The three spellings of how the copy of `characters` characters went, longest first, the
/// composer gap shows the longest that fits (oracle `composer_gap.rs` `CopyFeedback::line`; the
/// oracle's English until the catalog has the entries: a typed const table, ADR-010).
pub fn copy_feedback(result: Result<CopyStatus, ()>, characters: usize) -> [String; 3] {
    match result {
        Ok(CopyStatus::Confirmed) => [
            format!("Copied {characters} chars to host clipboard"),
            format!("Copied {characters} chars"),
            "Copied".into(),
        ],
        Ok(CopyStatus::Pending(_)) => [
            format!("Copying {characters} chars…"),
            "Copying…".into(),
            "Copying…".into(),
        ],
        Ok(CopyStatus::Busy) => [
            "Copy already in progress".into(),
            "Copy in progress".into(),
            "Copy busy".into(),
        ],
        Ok(CopyStatus::Unconfirmed) => [
            "Copy sent to terminal · paste to verify".into(),
            "Copy sent · verify paste".into(),
            "Copy unconfirmed".into(),
        ],
        Err(()) => [
            "Copy failed · try again".into(),
            "Copy failed".into(),
            "Copy failed".into(),
        ],
    }
}
