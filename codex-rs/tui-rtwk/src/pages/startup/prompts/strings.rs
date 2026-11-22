//! The strings of `startup` (S05): a table of `&'static str` fields and typed
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
    /// Oracle: `bottom_pane/picker_option.rs` `numbered_row`: the number
    /// before a label.
    numbered: Msg<2>,
    /// Oracle: `bottom_pane/picker_style.rs` `render_scroll_indicators`.
    pub more_above: &'static str,
    pub more_below: &'static str,
    /// Oracle: `cwd_prompt.rs`.
    cwd_title: Msg<1>,
    cwd_notes: Msg<1>,
    pub cwd_current_note: &'static str,
    pub cwd_verbs: [&'static str; 2],
    pub cwd_past: [&'static str; 2],
    cwd_session: Msg<1>,
    cwd_current: Msg<1>,
    pub cwd_always_session: &'static str,
    pub cwd_always_current: &'static str,
    cwd_always_current_in: Msg<1>,
    /// `enter`, ` continue · `, `esc`, ` use session · `, `⌃c`, ` quit`.
    pub cwd_hint: [&'static str; 6],
    /// `persist_remembered_cwd_selection`: the history line of a failed save.
    pub cwd_not_saved: &'static str,
    /// Oracle: `model_migration.rs`.
    migration_heading: Msg<1>,
    migration_recommend: Msg<2>,
    migration_fallback: Msg<1>,
    migration_with_link: Msg<2>,
    migration_keep: Msg<1>,
    pub migration_choices: [&'static str; 2],
    /// `enter/esc`, ` confirm · ` (or ` continue · `), `⌃c`, ` quit`.
    pub migration_keys: &'static str,
    pub migration_confirm: &'static str,
    pub migration_continue: &'static str,
    pub migration_ctrl_c: &'static str,
    pub migration_quit: &'static str,
    /// Oracle: `startup_hooks_review.rs`.
    pub hooks_title: &'static str,
    hooks_count: Msg<1>,
    pub hooks_one: &'static str,
    pub hooks_sandbox: &'static str,
    hooks_failed: Msg<1>,
    pub hooks_trusting: &'static str,
    pub hooks_choices: [&'static str; 3],
    /// `enter`, ` confirm`, ` · `, `esc`, ` skip`.
    pub hooks_hint: [&'static str; 5],
    /// Oracle: `unarchive_prompt.rs`.
    pub unarchive_title: &'static str,
    unarchive_choice: Msg<1>,
    pub unarchive_cancel: &'static str,
    pub unarchive_verbs: [&'static str; 2],
    /// `enter`, ` continue · `, `esc`, ` cancel`.
    pub unarchive_hint: [&'static str; 4],
    /// Oracle: `oss_selection.rs`.
    pub oss_title: &'static str,
    pub oss_subtitle: &'static str,
    pub oss_providers: [&'static str; 2],
    pub oss_descriptions: [&'static str; 2],
    pub oss_statuses: [&'static str; 3],
    pub oss_separator: &'static str,
    /// `↑/↓`, ` choose · `, `enter`, ` select · `, `esc`, ` LM Studio · `,
    /// `⌃c`, ` exit`.
    pub oss_hint: [&'static str; 8],
    /// Oracle: `daemon_recovery.rs`.
    pub daemon_incompatible: &'static str,
    pub daemon_cannot: &'static str,
    pub daemon_persist: &'static str,
    pub daemon_features: &'static str,
    daemon_feature: Msg<2>,
    daemon_more: Msg<1>,
    pub daemon_choices: [&'static str; 4],
    pub daemon_disabled_suffix: &'static str,
    pub daemon_unmanaged: &'static str,
    pub daemon_unresolvable: &'static str,
    pub true_word: &'static str,
    pub false_word: &'static str,
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

    /// `Working directory · resume`.
    pub fn cwd_title(&self, verb: &str, num: &NumSyms) -> String {
        Fmt::new(self.cwd_title, [Arg::Text(verb)], num).to_string()
    }

    /// `Session = latest cwd recorded in the resumed session`.
    pub fn cwd_session_note(&self, past: &str, num: &NumSyms) -> String {
        Fmt::new(self.cwd_notes, [Arg::Text(past)], num).to_string()
    }

    pub fn cwd_use_session(&self, path: &str, num: &NumSyms) -> String {
        Fmt::new(self.cwd_session, [Arg::Text(path)], num).to_string()
    }

    pub fn cwd_use_current(&self, path: &str, num: &NumSyms) -> String {
        Fmt::new(self.cwd_current, [Arg::Text(path)], num).to_string()
    }

    pub fn cwd_always_current_in(&self, path: &str, num: &NumSyms) -> String {
        Fmt::new(self.cwd_always_current_in, [Arg::Text(path)], num).to_string()
    }

    pub fn migration_heading(&self, name: &str, num: &NumSyms) -> String {
        Fmt::new(self.migration_heading, [Arg::Text(name)], num).to_string()
    }

    pub fn migration_recommend(&self, from: &str, to: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.migration_recommend,
            [Arg::Text(from), Arg::Text(to)],
            num,
        )
        .to_string()
    }

    pub fn migration_fallback(&self, name: &str, num: &NumSyms) -> String {
        Fmt::new(self.migration_fallback, [Arg::Text(name)], num).to_string()
    }

    /// `<description> Learn more about <name> at ` (the link follows).
    pub fn migration_with_link(&self, description: &str, name: &str, num: &NumSyms) -> String {
        Fmt::new(
            self.migration_with_link,
            [Arg::Text(description), Arg::Text(name)],
            num,
        )
        .to_string()
    }

    pub fn migration_keep(&self, model: &str, num: &NumSyms) -> String {
        Fmt::new(self.migration_keep, [Arg::Text(model)], num).to_string()
    }

    /// `2 hooks are new or changed.` (`1 hook is new or changed.`).
    pub fn hooks_count(&self, count: usize, num: &NumSyms) -> String {
        if count == 1 {
            return self.hooks_one.to_owned();
        }
        Fmt::new(self.hooks_count, [Arg::Int(count as u64)], num).to_string()
    }

    /// `Failed to trust hooks: <detail>`.
    pub fn hooks_failed(&self, detail: &str, num: &NumSyms) -> String {
        Fmt::new(self.hooks_failed, [Arg::Text(detail)], num).to_string()
    }

    pub fn unarchive_choice(&self, verb: &str, num: &NumSyms) -> String {
        Fmt::new(self.unarchive_choice, [Arg::Text(verb)], num).to_string()
    }

    /// `  name = true`.
    pub fn daemon_feature(&self, name: &str, enabled: bool, num: &NumSyms) -> String {
        let value = if enabled {
            self.true_word
        } else {
            self.false_word
        };
        Fmt::new(
            self.daemon_feature,
            [Arg::Text(name), Arg::Text(value)],
            num,
        )
        .to_string()
    }

    /// `  and 1 more`.
    pub fn daemon_more(&self, count: usize, num: &NumSyms) -> String {
        Fmt::new(self.daemon_more, [Arg::Int(count as u64)], num).to_string()
    }
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "startup prompts",
    numbered: msg!("{0}. {1}"),
    more_above: "\u{2191}",
    more_below: "\u{2193}",
    cwd_title: msg!("Working directory \u{b7} {0}"),
    cwd_notes: msg!("Session = latest cwd recorded in the {0} session"),
    cwd_current_note: "Current = your current working directory",
    cwd_verbs: ["resume", "fork"],
    cwd_past: ["resumed", "forked"],
    cwd_session: msg!("Use session directory ({0})"),
    cwd_current: msg!("Use current directory ({0})"),
    cwd_always_session: "Always use session directory",
    cwd_always_current: "Always use current directory",
    cwd_always_current_in: msg!("Always use current directory ({0})"),
    cwd_not_saved: "Failed to save working directory preference.",
    cwd_hint: [
        "enter",
        " continue \u{b7} ",
        "esc",
        " use session \u{b7} ",
        "\u{2303}c",
        " quit",
    ],
    migration_heading: msg!("Codex just got an upgrade. Introducing {0}."),
    migration_recommend: msg!("We recommend switching from {0} to {1}."),
    migration_fallback: msg!("{0} is recommended for better performance and reliability."),
    migration_with_link: msg!("{0} Learn more about {1} at "),
    migration_keep: msg!("You can continue using {0} if you prefer."),
    migration_choices: ["Try new model", "Use existing model"],
    migration_keys: "enter/esc",
    migration_confirm: " confirm \u{b7} ",
    migration_continue: " continue \u{b7} ",
    migration_ctrl_c: "\u{2303}c",
    migration_quit: " quit",
    hooks_title: "Hooks need review",
    hooks_count: msg!("{0} hooks are new or changed."),
    hooks_one: "1 hook is new or changed.",
    hooks_sandbox: "Hooks can run outside the sandbox after you trust them.",
    hooks_failed: msg!("Failed to trust hooks: {0}"),
    hooks_trusting: "Trusting hooks...",
    hooks_choices: [
        "Review hooks",
        "Trust all and continue",
        "Continue without trusting (hooks won't run)",
    ],
    hooks_hint: ["enter", " confirm", " \u{b7} ", "esc", " skip"],
    unarchive_title: "This conversation is archived",
    unarchive_choice: msg!("Unarchive and {0}"),
    unarchive_cancel: "Cancel",
    unarchive_verbs: ["resume", "fork"],
    unarchive_hint: ["enter", " continue \u{b7} ", "esc", " cancel"],
    oss_title: "Select an open-source provider",
    oss_subtitle: "Choose a local AI server for this session.",
    oss_providers: ["LM Studio", "Ollama"],
    oss_descriptions: [
        "Local LM Studio server (default port 1234)",
        "Local Ollama server (Responses API, default port 11434)",
    ],
    oss_statuses: ["\u{25cf} Running", "\u{25cb} Not running", "? Unknown"],
    oss_separator: " \u{b7} ",
    oss_hint: [
        "\u{2191}/\u{2193}",
        " choose \u{b7} ",
        "enter",
        " select \u{b7} ",
        "esc",
        " LM Studio \u{b7} ",
        "\u{2303}c",
        " exit",
    ],
    daemon_incompatible: "Background server has incompatible feature settings",
    daemon_cannot: "Cannot use the background server",
    daemon_persist: "These settings persist and can disable functionality for other clients. Restart may interrupt active or queued work.",
    daemon_features: "Restart will use these shared feature settings:",
    daemon_feature: msg!("  {0} = {1}"),
    daemon_more: msg!("  and {0} more"),
    daemon_choices: [
        "Run without daemon this time",
        "Restart with these settings (recommended)",
        "Restart with these settings",
        "Cancel",
    ],
    daemon_disabled_suffix: " (disabled)",
    daemon_unmanaged: "This server is not managed by Codex.",
    daemon_unresolvable: "Restart cannot resolve this compatibility check.",
    true_word: "true",
    false_word: "false",
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
