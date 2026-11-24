//! The strings of `dialogs` (S23): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group of strings
//! names the oracle file it comes from. A language is one more `const` here,
//! registered with `with_ru(&RU)`-style on `TABLES`; a screen never changes
//! for it. Nothing else in this directory holds user-visible text (the rules
//! test `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use codex_tui_core::keymap::ActionId;
use codex_tui_core::keymap::actions::{
    CHAT_DECREASE_REASONING_EFFORT, CHAT_INCREASE_REASONING_EFFORT,
    COMPOSER_HISTORY_SEARCH_PREVIOUS, COMPOSER_QUEUE, EDITOR_INSERT_NEWLINE,
    GLOBAL_FIND_TRANSCRIPT, GLOBAL_FOCUS_ACTIVITY, GLOBAL_OPEN_AGENTS, GLOBAL_OPEN_EXTERNAL_EDITOR,
    GLOBAL_OPEN_TRANSCRIPT,
};
use codex_tui_core::keymap::fixed::{
    INTERRUPT_OR_QUIT, PASTE_IMAGE, SELECTION_START, SUBAGENT_NEXT, SUBAGENT_PREVIOUS,
    TRANSCRIPT_LATEST, TRANSCRIPT_LATEST_ALT, TRANSCRIPT_PAGE_DOWN, TRANSCRIPT_PAGE_UP,
    TRANSCRIPT_TOP, TRANSCRIPT_TOP_ALT,
};

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// One column of the shortcut overlay: its heading and its (key, label) rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Group {
    pub heading: &'static str,
    pub rows: &'static [(&'static str, &'static str)],
}

/// Where the keys of a row of the live overlay come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keys {
    /// Text no platform spells differently (`/`, `esc esc`).
    Plain(&'static str),
    /// A key the oracle fixes (a named action of `keymap::fixed`), spelt as the platform spells a
    /// hint (`\u{2303}v`).
    Fixed(ActionId),
    /// Two fixed keys, `a / b`.
    FixedPair(ActionId, ActionId),
    /// The first binding of the action in the user's keymap; the row is
    /// absent while the action is unbound (`Entry::hint`).
    Bound(ActionId),
}

/// One row of the live overlay (`shortcut_overlay.rs` `Entry`): its keys, the
/// action it names and the action while a turn runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub keys: Keys,
    pub label: &'static str,
    pub running: Option<&'static str>,
}

/// One column of the live overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    pub heading: &'static str,
    pub rows: &'static [Entry],
}

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu.
    pub title: &'static str,
    pub num: NumSyms,
    // bottom_pane/shortcut_overlay.rs: the overlay's two pages.
    pub shortcuts_title: &'static str,
    pub shortcuts_list_title: &'static str,
    pub shortcuts_keyboard_title: &'static str,
    pub shortcut_groups: [Group; 3],
    /// The live overlay's columns (the bottom pane's `?`), keys from the keymap.
    pub shortcut_sections: [Section; 3],
    /// Between the two keys of a pair (`pgup / pgdn`).
    pub shortcut_either: &'static str,
    pub keymap_customize: &'static str,
    pub shortcuts_close: &'static str,
    /// bottom_pane/shortcut_overlay.rs `render`: shown over `/keymap customize` when the room is short.
    pub shortcuts_resize: &'static str,
    /// bottom_pane/shortcut_overlay/keyboard.rs: the base layout, drawn as is.
    pub keyboard: [&'static str; 15],
    // bottom_pane/warnings_view_render.rs.
    pub warnings_title: Msg<3>,
    pub warnings_title_empty: &'static str,
    pub no_warnings: &'static str,
    pub hint_keep: &'static str,
    pub hint_dismiss: &'static str,
    pub hint_copy: &'static str,
    pub hint_pages: &'static str,
    pub hint_separator: &'static str,
    // bottom_pane/feedback_view.rs: `feedback_selection_params`.
    pub feedback_title: &'static str,
    pub feedback_categories: [(&'static str, &'static str); 5],
    // bottom_pane/feedback_view.rs: `feedback_consent_params`.
    pub consent_title: &'static str,
    pub consent_disclosure_save: &'static str,
    pub consent_disclosure: &'static str,
    pub consent_bullet: &'static str,
    pub consent_files: [&'static str; 5],
    pub consent_windows_log: &'static str,
    pub consent_yes: &'static str,
    pub consent_yes_description: &'static str,
    pub consent_no: &'static str,
    pub consent_no_description: &'static str,
    /// bottom_pane/popup_consts.rs: `standard_popup_hint_line`.
    pub popup_hint: &'static str,
    // bottom_pane/custom_prompt_view.rs: the text prompt (`/rename`, `/export`'s file name).
    pub prompt_hint: &'static str,
    pub prompt_mark: &'static str,
    /// `with_text_suggestion`: the line while the suggestion is awaited, and once it is in.
    pub prompt_suggestion_loading: &'static str,
    pub prompt_suggestion_ready: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "small dialogs",
    num: NumSyms::ENGLISH,
    shortcuts_title: "Keyboard shortcuts",
    shortcuts_list_title: "Keyboard shortcuts  ·  Tab: Keyboard",
    shortcuts_keyboard_title: "Keyboard shortcuts  Base  ·  Tab: List",
    shortcut_groups: [
        Group {
            heading: "Compose",
            rows: &[
                ("/", "Commands"),
                ("@", "Mention files"),
                ("!", "Shell command"),
                ("⇧enter", "New line"),
                ("⌃v", "Paste image"),
                ("⌃g", "External editor"),
                ("⌃r", "Search history"),
                ("esc", "Edit last message"),
            ],
        },
        Group {
            heading: "Session",
            rows: &[
                ("tab", "Send message"),
                ("⌥,", "Less reasoning"),
                ("⌥.", "More reasoning"),
                ("⌥← / ⌥→", "Subagents (empty prompt)"),
                ("f4", "Inspect activity"),
                ("⌃c", "Quit"),
            ],
        },
        Group {
            heading: "Transcript (open first)",
            rows: &[
                ("⌃t", "Open transcript"),
                ("f3", "Find text"),
                ("pgup / pgdn", "Scroll"),
                ("⌃space", "Start selection"),
                ("⌃home / ⌃end", "Top / latest"),
                ("⇧⌥, / ⇧⌥.", "Top / latest"),
            ],
        },
    ],
    shortcut_sections: [
        Section {
            heading: "Compose",
            rows: &[
                Entry {
                    keys: Keys::Plain("/"),
                    label: "Commands",
                    running: None,
                },
                Entry {
                    keys: Keys::Plain("@"),
                    label: "Mention files",
                    running: None,
                },
                Entry {
                    keys: Keys::Plain("!"),
                    label: "Shell command",
                    running: None,
                },
                Entry {
                    keys: Keys::Bound(EDITOR_INSERT_NEWLINE),
                    label: "New line",
                    running: None,
                },
                Entry {
                    keys: Keys::Fixed(PASTE_IMAGE),
                    label: "Paste image",
                    running: None,
                },
                Entry {
                    keys: Keys::Bound(GLOBAL_OPEN_EXTERNAL_EDITOR),
                    label: "External editor",
                    running: None,
                },
                Entry {
                    keys: Keys::Bound(COMPOSER_HISTORY_SEARCH_PREVIOUS),
                    label: "Search history",
                    running: None,
                },
                Entry {
                    keys: Keys::Plain("esc esc"),
                    label: "Edit last message",
                    running: None,
                },
            ],
        },
        Section {
            heading: "Session",
            rows: &[
                Entry {
                    keys: Keys::Bound(COMPOSER_QUEUE),
                    label: "Send message",
                    running: Some("Queue message"),
                },
                Entry {
                    keys: Keys::Bound(CHAT_DECREASE_REASONING_EFFORT),
                    label: "Less reasoning",
                    running: None,
                },
                Entry {
                    keys: Keys::Bound(CHAT_INCREASE_REASONING_EFFORT),
                    label: "More reasoning",
                    running: None,
                },
                Entry {
                    keys: Keys::Bound(GLOBAL_OPEN_AGENTS),
                    label: "Agents (empty prompt)",
                    running: None,
                },
                Entry {
                    keys: Keys::FixedPair(SUBAGENT_PREVIOUS, SUBAGENT_NEXT),
                    label: "Subagents (empty prompt)",
                    running: None,
                },
                Entry {
                    keys: Keys::Bound(GLOBAL_FOCUS_ACTIVITY),
                    label: "Inspect activity",
                    running: None,
                },
                Entry {
                    keys: Keys::Fixed(INTERRUPT_OR_QUIT),
                    label: "Quit",
                    running: Some("Interrupt"),
                },
            ],
        },
        Section {
            heading: "Transcript (open first)",
            rows: &[
                Entry {
                    keys: Keys::Bound(GLOBAL_OPEN_TRANSCRIPT),
                    label: "Open transcript",
                    running: None,
                },
                Entry {
                    keys: Keys::Bound(GLOBAL_FIND_TRANSCRIPT),
                    label: "Find text",
                    running: None,
                },
                Entry {
                    keys: Keys::FixedPair(TRANSCRIPT_PAGE_UP, TRANSCRIPT_PAGE_DOWN),
                    label: "Scroll",
                    running: None,
                },
                Entry {
                    keys: Keys::Fixed(SELECTION_START),
                    label: "Start selection",
                    running: None,
                },
                Entry {
                    keys: Keys::FixedPair(TRANSCRIPT_TOP, TRANSCRIPT_LATEST),
                    label: "Top / latest",
                    running: None,
                },
                Entry {
                    keys: Keys::FixedPair(TRANSCRIPT_TOP_ALT, TRANSCRIPT_LATEST_ALT),
                    label: "Top / latest",
                    running: None,
                },
            ],
        },
    ],
    shortcut_either: " / ",
    keymap_customize: "/keymap customize",
    shortcuts_close: "? / esc close",
    shortcuts_resize: "\u{2026} resize to see all",
    keyboard: [
        "┌──────────────────────────────────────────────────────────────────────┐",
        "│ Esc     F1 F2 F3 F4   F5 F6 F7 F8  F9 F10 F11 F12   PrtSc ScrLk Pau  │",
        "│                                                                      │",
        "├──────────────────────────────────────────────────────────────────────┤",
        "│ `  1  2  3  4  5  6  7  8  9  0  -  =    Bksp       Ins  Home  PgU   │",
        "│                                                                      │",
        "│  Tab  Q  W  E  R  T  Y  U  I  O  P  [  ]    \\       Del   End  PgD   │",
        "│                                                                      │",
        "│  Caps  A  S  D  F  G  H  J  K  L  ;  '   Enter                       │",
        "│                                                                      │",
        "│  Shift  Z  X  C  V  B  N  M  ,  .  /    Shift              ↑         │",
        "│                                                                      │",
        "│ Ctrl Super Alt    Space    Alt  Super Menu Ctrl      ←     ↓    →    │",
        "│                                                                      │",
        "└──────────────────────────────────────────────────────────────────────┘",
    ],
    warnings_title: msg!("Warnings · {0} of {1} · {2}"),
    warnings_title_empty: "Warnings",
    no_warnings: "No warnings",
    hint_keep: "k keep & next",
    hint_dismiss: "esc dismiss & close",
    hint_copy: "⌃o copy",
    hint_pages: "←/→ warning",
    hint_separator: " · ",
    feedback_title: "How was this?",
    feedback_categories: [
        ("bug", "Crash, error message, hang, or broken UI/behavior"),
        (
            "bad result",
            "Output was off-target, incorrect, incomplete, or unhelpful",
        ),
        (
            "good result",
            "Helpful, correct, high\u{2011}quality, or delightful result worth celebrating",
        ),
        (
            "safety check",
            "Benign usage blocked due to safety checks or refusals",
        ),
        (
            "other",
            "Slowness, feature suggestion, UX feedback, or anything else",
        ),
    ],
    consent_title: "Include logs and session history?",
    consent_disclosure_save: "Saving a ZIP does not submit feedback to OpenAI.\nFiles may contain prompts, commands, and tool output:",
    consent_disclosure: "Files may contain prompts, commands, and tool output:",
    consent_bullet: "  \u{2022} ",
    consent_files: [
        "codex-logs.log",
        "client-logs.txt (if available)",
        "codex-doctor-report.json",
        "codex-apps-tools-cache.json (if available)",
        "codex-app-directory-cache.json (if available)",
    ],
    consent_windows_log: "windows-sandbox.log",
    consent_yes: "Yes",
    consent_yes_description: "Include session logs and diagnostics in the report",
    consent_no: "No",
    consent_no_description: "Keep only your note and report metadata",
    popup_hint: "enter select \u{b7} esc back",
    prompt_hint: "enter submit \u{b7} esc back",
    prompt_mark: "\u{203a}",
    prompt_suggestion_loading: "Generating a title suggestion\u{2026}",
    prompt_suggestion_ready: "Suggested from this conversation",
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

impl Strings {
    /// "Warnings · 1 of 2 · MCP · example".
    pub fn warnings_title<'a>(&'a self, page: usize, total: usize, source: &'a str) -> Fmt<'a, 3> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(
            self.warnings_title,
            [int(page), int(total), Arg::Text(source)],
            &self.num,
        )
    }
}
