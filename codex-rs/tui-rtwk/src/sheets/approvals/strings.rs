//! The strings of `approvals` (S13): a table of `&'static str` fields and typed
//! messages (`Msg<N>` behind a method) per locale, read by reference through
//! `of(cx)`. `EN` is the oracle's text byte for byte; each group names the
//! oracle file it comes from. A language is one more `const` here, registered
//! with `with_ru(&RU)` on `TABLES`; a screen never changes for it. Nothing
//! else in this directory holds user-visible text (the rules test
//! `adr010_no_user_visible_literal_outside_the_string_tables`).

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
    pub num: NumSyms,
    // bottom_pane/approval_overlay.rs: `build_options` (titles), `build_header`.
    pub run_command: &'static str,
    pub network_title: Msg<1>,
    pub grant_permissions: &'static str,
    pub make_edits: &'static str,
    pub reason: &'static str,
    pub thread: &'static str,
    pub input: &'static str,
    pub environment: &'static str,
    pub permission_rule: &'static str,
    pub description: &'static str,
    pub default_description: &'static str,
    pub destination: &'static str,
    pub unavailable: &'static str,
    /// Words of the permission rule line (`format_additional_permissions_rule`).
    pub rule_network: &'static str,
    pub rule_read: &'static str,
    pub rule_write: &'static str,
    // approval_overlay.rs: `exec_options`, `patch_options`, `permissions_options`.
    pub accept: &'static str,
    pub accept_network: &'static str,
    pub accept_prefix: Msg<1>,
    pub accept_session: &'static str,
    pub accept_session_network: &'static str,
    pub accept_session_permissions: &'static str,
    pub block_host: &'static str,
    pub allow_host: &'static str,
    pub decline: &'static str,
    pub cancel: &'static str,
    pub patch_session: &'static str,
    pub grant_turn: &'static str,
    pub grant_turn_strict: &'static str,
    pub grant_session: &'static str,
    pub deny_permissions: &'static str,
    /// The key shown after an option, `esc` for the cancel key.
    pub key_esc: &'static str,
    /// `approval_footer_hint`: "Press enter to confirm or esc to cancel".
    /// The shortcut after a label: ` (y)`.
    pub key_open_paren: &'static str,
    pub key_close_paren: &'static str,
    /// The prompt before a command (`$ echo hi`).
    pub shell_prompt: &'static str,
    /// The hint line in its pieces (`accept_cancel_hint_line`): the keys are
    /// bold, the words dim.
    pub hint_press: &'static str,
    pub key_enter: &'static str,
    pub hint_confirm: &'static str,
    pub hint_cancel: &'static str,
    pub hint_or: &'static str,
    pub key_open: &'static str,
    pub hint_open: &'static str,
    // bottom_pane/mcp_server_elicitation.rs: the approval form.
    pub field_progress: &'static str,
    pub elicit_allow: &'static str,
    pub elicit_allow_description: &'static str,
    pub elicit_cancel: &'static str,
    pub elicit_cancel_description: &'static str,
    pub elicit_hint: &'static str,
    // bottom_pane/request_user_input: the options form.
    pub question_progress: Msg<3>,
    pub none_of_the_above: &'static str,
    pub none_description: &'static str,
    pub user_input_hint: &'static str,
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "approvals and requests",
    num: NumSyms::ENGLISH,
    run_command: "Would you like to run the following command?",
    network_title: msg!("Do you want to approve network access to \"{0}\"?"),
    grant_permissions: "Would you like to grant these permissions?",
    make_edits: "Would you like to make the following edits?",
    reason: "Reason: ",
    thread: "Thread: ",
    input: "Input: ",
    environment: "Environment: ",
    permission_rule: "Permission rule: ",
    description: "Description: ",
    default_description: "Apply proposed file edits",
    destination: "Destination: ",
    unavailable: "unavailable",
    rule_network: "network",
    rule_read: "read",
    rule_write: "write",
    accept: "Yes, proceed",
    accept_network: "Yes, just this once",
    accept_prefix: msg!("Yes, and don't ask again for commands that start with `{0}`"),
    accept_session: "Yes, and don't ask again for this command in this session",
    accept_session_network: "Yes, and allow this host for this conversation",
    accept_session_permissions: "Yes, and allow these permissions for this session",
    block_host: "No, and block this host in the future",
    allow_host: "Yes, and allow this host in the future",
    decline: "No, continue without running it",
    cancel: "No, and tell Codex what to do differently",
    patch_session: "Yes, and don't ask again for these files",
    grant_turn: "Yes, grant these permissions for this turn",
    grant_turn_strict: "Yes, grant for this turn with strict auto review",
    grant_session: "Yes, grant these permissions for this session",
    deny_permissions: "No, continue without permissions",
    key_esc: "esc",
    key_open_paren: " (",
    key_close_paren: ")",
    shell_prompt: "$ ",
    hint_press: "Press ",
    key_enter: "enter",
    hint_confirm: " to confirm or ",
    hint_cancel: " to cancel",
    hint_or: " or ",
    key_open: "o",
    hint_open: " to open thread",
    field_progress: "Field 1/1",
    elicit_allow: "Allow",
    elicit_allow_description: "Run the tool and continue",
    elicit_cancel: "Cancel",
    elicit_cancel_description: "Cancel this tool call",
    elicit_hint: "enter to submit | esc to cancel",
    question_progress: msg!("Question {0}/{1} ({2} unanswered)"),
    none_of_the_above: "None of the above",
    none_description: "Optionally, add details in notes (tab)",
    user_input_hint: "tab to add notes | enter to submit answer | esc to interrupt",
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
    pub fn network_title<'a>(&'a self, host: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.network_title, [Arg::Text(host)], &self.num)
    }

    pub fn accept_prefix<'a>(&'a self, prefix: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.accept_prefix, [Arg::Text(prefix)], &self.num)
    }

    pub fn question_progress(&self, index: usize, total: usize, unanswered: usize) -> Fmt<'_, 3> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(
            self.question_progress,
            [int(index), int(total), int(unanswered)],
            &self.num,
        )
    }
}
