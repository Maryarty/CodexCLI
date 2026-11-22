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
    /// Between a key and what it does in the tips.
    pub key_gap: &'static str,
    pub key_open: &'static str,
    pub hint_open: &'static str,
    // approval_overlay.rs: the plain approval of an elicitation that has no form
    // (`elicitation_options`, `build_header`).
    pub elicit_title: Msg<1>,
    pub server: &'static str,
    pub elicit_accept: &'static str,
    pub elicit_decline: &'static str,
    pub elicit_cancel: &'static str,
    // bottom_pane/user_verification.rs
    pub verify_accept: &'static str,
    pub verify_waiting: &'static str,
    pub hint_cancel_request: &'static str,
    // bottom_pane/mcp_server_elicitation.rs: the form (`render`, `footer_tips`).
    pub field_progress: Msg<2>,
    pub required_unanswered: Msg<1>,
    pub no_fields: &'static str,
    pub answer_placeholder: &'static str,
    pub optional_answer_placeholder: &'static str,
    pub answer_required: &'static str,
    pub tip_submit: &'static str,
    pub tip_submit_all: &'static str,
    pub tip_submit_answer: &'static str,
    pub tip_arrows: &'static str,
    pub tip_navigate: &'static str,
    pub tip_change_field_keys: &'static str,
    pub tip_change_field: &'static str,
    pub tip_cancel: &'static str,
    pub tip_separator: &'static str,
    pub composer_prompt: &'static str,
    // bottom_pane/pending_thread_approvals.rs
    pub banner_mark: &'static str,
    pub banner_needed: Msg<1>,
    pub banner_more: &'static str,
    pub banner_command: &'static str,
    pub banner_review: &'static str,
    pub too_long: Msg<2>,
    /// The choices of the approval fallback (`approval_choice_options`, `tool_approval_options`) and
    /// of a boolean field.
    pub act_allow: &'static str,
    pub act_allow_session: &'static str,
    pub act_always_allow: &'static str,
    pub act_deny: &'static str,
    pub act_cancel: &'static str,
    pub allow_tool: &'static str,
    pub allow_request: &'static str,
    pub session_tool: &'static str,
    pub session_request: &'static str,
    pub always_tool: &'static str,
    pub always_request: &'static str,
    pub deny_request: &'static str,
    pub cancel_tool: &'static str,
    pub cancel_request: &'static str,
    pub bool_true: &'static str,
    pub bool_false: &'static str,
    pub param_line: Msg<2>,
    // bottom_pane/request_user_input: the options form.
    pub question_progress: Msg<2>,
    pub question_unanswered: Msg<1>,
    pub none_of_the_above: &'static str,
    pub none_description: &'static str,
    pub notes_placeholder: &'static str,
    pub select_option_placeholder: &'static str,
    pub key_tab: &'static str,
    pub tip_add_notes: &'static str,
    pub tip_clear_notes: &'static str,
    pub tip_navigate_questions: &'static str,
    pub tip_change_question: &'static str,
    pub tip_interrupt: &'static str,
    /// The confirmation of a submit with unanswered questions.
    pub confirm_title: &'static str,
    pub confirm_subtitle_one: &'static str,
    pub confirm_subtitle: Msg<1>,
    pub confirm_proceed: &'static str,
    pub confirm_proceed_one: &'static str,
    pub confirm_proceed_many: Msg<1>,
    pub confirm_go_back: &'static str,
    pub confirm_go_back_desc: &'static str,
    pub hint_go_back: &'static str,
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
    key_gap: " ",
    key_open: "o",
    hint_open: " to open thread",
    elicit_title: msg!("{0} needs your approval."),
    server: "Server: ",
    elicit_accept: "Yes, provide the requested info",
    elicit_decline: "No, but continue without it",
    elicit_cancel: "Cancel this request",
    verify_accept: "Verify and approve",
    verify_waiting: "Waiting for verification…",
    hint_cancel_request: " to cancel this request",
    field_progress: msg!("Field {0}/{1}"),
    required_unanswered: msg!(" ({0} required unanswered)"),
    no_fields: "No fields",
    answer_placeholder: "Type your answer",
    optional_answer_placeholder: "Type your answer (optional)",
    answer_required: "Answer required fields before submitting.",
    tip_submit: "to submit",
    tip_submit_all: "to submit all",
    tip_submit_answer: "to submit answer",
    tip_arrows: "←/→",
    tip_navigate: "to navigate fields",
    tip_change_field_keys: "⌃p / ⌃n",
    tip_change_field: "change field",
    tip_cancel: "to cancel",
    tip_separator: " | ",
    composer_prompt: "›",
    banner_mark: "!",
    banner_needed: msg!("Approval needed in {0}"),
    banner_more: "...",
    banner_command: "/subagents",
    banner_review: " to review requests",
    too_long: msg!("Message exceeds the maximum length of {0} characters ({1} provided)."),
    act_allow: "Allow",
    act_allow_session: "Allow for this session",
    act_always_allow: "Always allow",
    act_deny: "Deny",
    act_cancel: "Cancel",
    allow_tool: "Run the tool and continue",
    allow_request: "Allow this request and continue",
    session_tool: "Run the tool and remember this choice for this session",
    session_request: "Allow this request and remember this choice for this session",
    always_tool: "Run the tool and remember this choice for future tool calls",
    always_request: "Allow this request and remember this choice for future requests",
    deny_request: "Decline this request and continue",
    cancel_tool: "Cancel this tool call",
    cancel_request: "Cancel this request",
    bool_true: "True",
    bool_false: "False",
    param_line: msg!("{0}: {1}"),
    question_progress: msg!("Question {0}/{1}"),
    question_unanswered: msg!(" ({0} unanswered)"),
    none_of_the_above: "None of the above",
    none_description: "Optionally, add details in notes (tab)",
    notes_placeholder: "Add notes",
    select_option_placeholder: "Select an option to add notes",
    key_tab: "tab",
    tip_add_notes: "to add notes",
    tip_clear_notes: "to clear notes",
    tip_navigate_questions: "to navigate questions",
    tip_change_question: "change question",
    tip_interrupt: "to interrupt",
    confirm_title: "Submit with unanswered questions?",
    confirm_subtitle_one: "1 unanswered question",
    confirm_subtitle: msg!("{0} unanswered questions"),
    confirm_proceed: "Proceed",
    confirm_proceed_one: "Submit with 1 unanswered question",
    confirm_proceed_many: msg!("Submit with {0} unanswered questions"),
    confirm_go_back: "Go back",
    confirm_go_back_desc: "Return to the first unanswered question",
    hint_go_back: " to go back",
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

    pub fn banner_needed<'a>(&'a self, thread: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.banner_needed, [Arg::Text(thread)], &self.num)
    }

    pub fn accept_prefix<'a>(&'a self, prefix: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.accept_prefix, [Arg::Text(prefix)], &self.num)
    }

    pub fn elicit_title<'a>(&'a self, server: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.elicit_title, [Arg::Text(server)], &self.num)
    }

    pub fn field_progress(&self, index: usize, total: usize) -> Fmt<'_, 2> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(self.field_progress, [int(index), int(total)], &self.num)
    }

    pub fn required_unanswered(&self, count: usize) -> Fmt<'_, 1> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(self.required_unanswered, [int(count)], &self.num)
    }

    pub fn too_long(&self, limit: usize, actual: usize) -> Fmt<'_, 2> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(self.too_long, [int(limit), int(actual)], &self.num)
    }

    pub fn param_line<'a>(&'a self, name: &'a str, value: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.param_line,
            [Arg::Text(name), Arg::Text(value)],
            &self.num,
        )
    }

    pub fn question_progress(&self, index: usize, total: usize) -> Fmt<'_, 2> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(self.question_progress, [int(index), int(total)], &self.num)
    }

    pub fn question_unanswered(&self, count: usize) -> Fmt<'_, 1> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(self.question_unanswered, [int(count)], &self.num)
    }

    pub fn confirm_subtitle(&self, count: usize) -> Fmt<'_, 1> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(self.confirm_subtitle, [int(count)], &self.num)
    }

    pub fn confirm_proceed_many(&self, count: usize) -> Fmt<'_, 1> {
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(self.confirm_proceed_many, [int(count)], &self.num)
    }
}
