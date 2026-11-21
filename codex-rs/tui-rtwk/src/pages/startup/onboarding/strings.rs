//! The strings of `onboarding` (S01-S04): a table of `&'static str` fields and typed
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

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// Oracle: `bottom_pane/picker_option.rs` `numbered_row`: the marks of
    /// the cursor's row and of the others, and the number before a label.
    pub cursor_mark: &'static str,
    pub other_mark: &'static str,
    numbered: Msg<2>,
    /// Oracle: `bottom_pane/picker_style.rs` `render_scroll_indicators`.
    pub more_above: &'static str,
    pub more_below: &'static str,
    /// Oracle: `onboarding/welcome.rs`: the line under the logo, its
    /// parts (the middle one bold).
    pub welcome: [&'static str; 3],
    /// Oracle: `update_prompt.rs`: the title, the separator, the versions.
    pub update_title: &'static str,
    pub update_dot: &'static str,
    update_versions: Msg<2>,
    pub release_notes: &'static str,
    pub release_notes_url: &'static str,
    update_now: Msg<1>,
    /// Oracle: `onboarding_screen.rs` `persist_selected_trust`: the failed
    /// write, the folder and the server's message.
    trust_failed: Msg<2>,
    pub skip: &'static str,
    pub skip_version: &'static str,
    /// The key hint: `enter`, ` continue · `, `esc`, ` skip`.
    pub update_hint: [&'static str; 4],
    /// Oracle: `onboarding/trust_directory.rs`.
    pub folder_access: &'static str,
    pub git_note: &'static str,
    pub disclosure_trust: &'static str,
    pub disclosure_restricted: &'static str,
    pub disclosure_existing_task: &'static str,
    pub trust_continue: &'static str,
    pub open_restricted: &'static str,
    pub open_existing_task: &'static str,
    /// The cancel choice: quit, back to the agents, keep the directory.
    pub cancel_choices: [&'static str; 3],
    /// The key hint: `enter`, ` continue`, ` continue and create sandbox`,
    /// ` · `, `esc`, then the cancel's word.
    pub trust_hint: [&'static str; 5],
    pub cancel_words: [&'static str; 3],
}

impl Strings {
    /// `1. label`.
    pub fn numbered(&self, number: usize, label: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.numbered,
            [Arg::Int(number as u64), Arg::Text(label)],
            num,
        )
        .to_string()
    }

    /// `0.0.0 → 9.9.9`.
    pub fn update_versions(&self, current: &str, latest: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.update_versions,
            [Arg::Text(current), Arg::Text(latest)],
            num,
        )
        .to_string()
    }

    /// `Failed to set trust for <folder>: <message>`.
    pub fn trust_failed(&self, target: &str, message: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.trust_failed,
            [Arg::Text(target), Arg::Text(message)],
            num,
        )
        .to_string()
    }

    /// `Update now (runs `<command>`)`.
    pub fn update_now(&self, command: &str, num: &NumSyms) -> String {
        Fmt::new(self.update_now, [Arg::Text(command)], num).to_string()
    }
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "startup page",
    cursor_mark: "\u{203a} ",
    other_mark: "  ",
    numbered: msg!("{0}. {1}"),
    more_above: "\u{2191}",
    more_below: "\u{2193}",
    welcome: [
        "  Welcome to ",
        "Codex",
        ", OpenAI's command-line coding agent",
    ],
    update_title: "Update available",
    update_dot: " \u{b7} ",
    update_versions: msg!("{0} \u{2192} {1}"),
    release_notes: "Release notes: ",
    release_notes_url: "https://github.com/openai/codex/releases/latest",
    update_now: msg!("Update now (runs `{0}`)"),
    trust_failed: msg!("Failed to set trust for {0}: {1}"),
    skip: "Skip",
    skip_version: "Skip until next version",
    update_hint: ["enter", " continue \u{b7} ", "esc", " skip"],
    folder_access: "Folder access",
    git_note: "Note: You\u{2019}re in a subdirectory of a Git project. Trusting will apply to the repository root:",
    disclosure_trust: "Trust this folder? Codex can read, edit, and run files here, subject to your permission settings. Folder settings can run code automatically, even without a model request. Continue only if you trust these files. Your trust decision will be saved.",
    disclosure_restricted: "Config, hooks, and exec policies from untrusted folders stay disabled. Trusted project folders can still contribute settings. Skills still load, and tools follow your permission settings. Opening will not change saved trust.",
    disclosure_existing_task: "This existing task may retain settings and history, including project configuration or hooks loaded while it was trusted. To use restricted settings, start a new task. The folder's trust setting will not change.",
    trust_continue: "Trust and continue",
    open_restricted: "Open restricted",
    open_existing_task: "Open existing task",
    cancel_choices: [
        "Quit",
        "Back to Agent Command Center",
        "Keep current directory",
    ],
    trust_hint: [
        "enter",
        " continue",
        " continue and create sandbox",
        " \u{b7} ",
        "esc",
    ],
    cancel_words: [" quit", " back", " cancel"],
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
