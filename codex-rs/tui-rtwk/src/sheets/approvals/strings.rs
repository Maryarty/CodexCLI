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

use codex_tui_core::screens::approvals::verify::Cause;

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
    pub send_input_title: Msg<1>,
    pub send_input_existing: &'static str,
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
    /// Between the progress and the countdown of a request that answers itself.
    pub countdown_separator: &'static str,
    pub auto_resolves_in: Msg<1>,
    /// The row that stands for the header rows that did not fit (`[… 7 lines] ⌃a view all`).
    pub clipped_lines: Msg<1>,
    pub view_all_key: &'static str,
    pub view_all: &'static str,
    /// The marks of a decision cell and who decided (`history_cell/approvals.rs`).
    pub approved_mark: &'static str,
    pub refused_mark: &'static str,
    /// The bullet of an info line and the mark of an error line of the review lines
    /// (`new_info_event`, `new_error_event`).
    pub info_mark: &'static str,
    /// Between an info line's message and its dim hint.
    pub info_gap: &'static str,
    pub error_mark: &'static str,
    pub you: &'static str,
    pub auto_reviewer: &'static str,
    pub verb_approved: &'static str,
    pub verb_persisted: &'static str,
    pub verb_denied: &'static str,
    pub verb_did_not_approve: &'static str,
    pub verb_canceled: &'static str,
    pub verb_timed_out: &'static str,
    pub cell_request: &'static str,
    pub cell_review: &'static str,
    pub cell_codex_to_run: &'static str,
    pub cell_this_request: &'static str,
    pub cell_this_time: &'static str,
    pub cell_every_time: &'static str,
    pub cell_always_run: &'static str,
    pub cell_network_to: &'static str,
    pub cell_persisted_network_to: &'static str,
    pub cell_saved_rule: &'static str,
    pub cell_canceled_run: &'static str,
    pub cell_canceled_network: &'static str,
    pub cell_denied_run: &'static str,
    pub cell_timed_out_run: &'static str,
    pub cell_timed_out_request: &'static str,
    pub cell_timed_out_access: &'static str,
    /// A command of several lines shows its first, and these dots; one over eighty graphemes ends in the others.
    pub cell_more_lines: &'static str,
    pub cell_cut: &'static str,
    /// What the auto-reviewer judged that was not a command (`new_guardian_denied_*_request`,
    /// `new_guardian_timed_out_*_request`, `on_guardian_assessment`).
    pub cell_denied_for: &'static str,
    pub cell_timed_out_before: &'static str,
    pub cell_denied_apply: &'static str,
    pub cell_timed_out_apply: &'static str,
    pub cell_patch_touching: &'static str,
    pub cell_files: &'static str,
    pub codex_to: &'static str,
    pub codex_could: &'static str,
    pub act_send_input: Msg<2>,
    pub act_call_mcp: Msg<2>,
    pub act_access: Msg<1>,
    pub act_request_permissions: &'static str,
    pub with_reason: Msg<2>,
    /// A review as the status line and the `/auto-review` rows say it (`action_summary`).
    pub sum_patch_one: Msg<1>,
    pub sum_patch_many: Msg<1>,
    pub sum_network: Msg<1>,
    pub sum_mcp: Msg<2>,
    pub sum_permission: &'static str,
    /// The status line while reviews run (`PendingGuardianReviewStatus`).
    pub review_one: &'static str,
    pub review_many: Msg<1>,
    pub review_bullet: Msg<1>,
    pub review_more: Msg<1>,
    /// The `/auto-review` list (`open_auto_review_denials_popup`).
    pub denials_title: &'static str,
    pub denials_subtitle: &'static str,
    pub denials_action: &'static str,
    pub denials_rationale: &'static str,
    pub denials_no_rationale: &'static str,
    pub denials_hint: &'static str,
    /// What the history says of the list and of a pick (`add_info_message`, `add_error_message`).
    pub no_denials: &'static str,
    pub no_denials_hint: &'static str,
    pub retry_approved: &'static str,
    pub retry_approved_hint: &'static str,
    pub denial_gone: &'static str,
    pub unlocalized: Msg<1>,
    /// What the history warns when a user verification did not complete (oracle
    /// `user_verification_errors.rs`, safe messages only, never the provider's diagnostics).
    pub verify_unreachable: &'static str,
    pub verify_invalid_params: &'static str,
    pub verify_credential_missing: &'static str,
    pub verify_biometrics_unavailable: &'static str,
    pub verify_provider_unavailable: &'static str,
    pub verify_user_cancelled: &'static str,
    pub verify_interrupted: &'static str,
    pub verify_authentication_failed: &'static str,
    pub verify_timeout: &'static str,
    pub verify_service: &'static str,
    /// The warning of a user verification on a remote workspace.
    pub verify_remote: &'static str,
    /// What a permissions answer says (`handle_permissions_decision`).
    pub granted_nothing: &'static str,
    pub granted_turn: &'static str,
    pub granted_strict: &'static str,
    pub granted_session: &'static str,
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
    send_input_title: msg!("Would you like to send input to terminal {0}?"),
    send_input_existing: "Would you like to send input to the existing terminal?",
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
    countdown_separator: " \u{b7} ",
    auto_resolves_in: msg!("auto-resolves in {0}"),
    clipped_lines: msg!("[\u{2026} {0} lines]"),
    view_all_key: "\u{2303}a",
    view_all: " view all",
    // The marks of a decision cell and who decided (`history_cell/approvals.rs`).
    approved_mark: "\u{2714} ",
    refused_mark: "\u{2717} ",
    info_mark: "\u{2022} ",
    info_gap: " ",
    error_mark: "\u{25a0} ",
    you: "You ",
    auto_reviewer: "Auto-reviewer ",
    verb_approved: "approved",
    verb_persisted: "persisted",
    verb_denied: "denied",
    verb_did_not_approve: "did not approve",
    verb_canceled: "canceled",
    verb_timed_out: "timed out",
    cell_request: "Request ",
    cell_review: "Review ",
    cell_codex_to_run: " codex to run ",
    cell_this_request: " this request",
    cell_this_time: " this time",
    cell_every_time: " every time this session",
    cell_always_run: " codex to always run commands that start with ",
    cell_network_to: " codex network access to ",
    cell_persisted_network_to: " Codex network access to ",
    cell_saved_rule: " and saved that rule",
    cell_canceled_run: " the request to run ",
    cell_canceled_network: " the request for codex network access to ",
    cell_denied_run: " for codex to run ",
    cell_timed_out_run: " before codex could run ",
    cell_timed_out_request: " before this request could be approved",
    cell_timed_out_access: " before codex could access ",
    // A command of several lines shows its first, and these dots; one over eighty graphemes ends in the others.
    cell_more_lines: " ...",
    cell_cut: "...",
    // What the auto-reviewer judged that was not a command.
    cell_denied_for: " for ",
    cell_timed_out_before: " before ",
    cell_denied_apply: " for codex to apply ",
    cell_timed_out_apply: " before codex could apply ",
    cell_patch_touching: "a patch touching ",
    cell_files: " files",
    codex_to: "codex to ",
    codex_could: "codex could ",
    act_send_input: msg!("send input to terminal {0}: {1}"),
    act_call_mcp: msg!("call MCP tool {0}.{1}"),
    act_access: msg!("access {0}"),
    act_request_permissions: "request permissions",
    with_reason: msg!("{0}: {1}"),
    sum_patch_one: msg!("apply_patch touching {0}"),
    sum_patch_many: msg!("apply_patch touching {0} files"),
    sum_network: msg!("network access to {0}"),
    sum_mcp: msg!("MCP {0} on {1}"),
    sum_permission: "permission request",
    review_one: "Reviewing approval request",
    review_many: msg!("Reviewing {0} approval requests"),
    review_bullet: msg!("• {0}"),
    review_more: msg!("+{0} more"),
    denials_title: "Auto-review Denials",
    denials_subtitle: "Select a denied action to approve.",
    denials_action: "Action",
    denials_rationale: "Rationale",
    denials_no_rationale: "Auto-review did not include a rationale",
    denials_hint: "enter select · esc back",
    no_denials: "No recent auto-review denials in this thread.",
    no_denials_hint: "Denials are recorded after auto-review rejects an action.",
    retry_approved: "Approval recorded for one retry of the selected auto-review denial.",
    retry_approved_hint: "The model will see the approval context; the retry still goes through auto-review.",
    denial_gone: "That auto-review denial is no longer available.",
    unlocalized: msg!("failed to localize guardian filesystem paths: {0}"),
    verify_unreachable: "Could not complete user verification with the local Codex binary.",
    verify_invalid_params: "The local Codex binary could not verify this request.",
    verify_credential_missing: "No user-verification credential is available in the local Codex binary.",
    verify_biometrics_unavailable: "Biometric verification is currently unavailable on this device.",
    verify_provider_unavailable: "User verification is unavailable for this request.",
    verify_user_cancelled: "User verification was cancelled.",
    verify_interrupted: "User verification was interrupted.",
    verify_authentication_failed: "Biometric verification did not succeed. The request was cancelled.",
    verify_timeout: "User verification timed out. The request was cancelled.",
    verify_service: "The local Codex binary could not complete user verification.",
    verify_remote: "User verification is unavailable for remote workspaces.",
    // What a permissions answer says (`handle_permissions_decision`).
    granted_nothing: "You did not grant additional permissions",
    granted_turn: "You granted additional permissions",
    granted_strict: "You granted additional permissions with strict auto review",
    granted_session: "You granted additional permissions for this session",
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
    pub fn send_input_title<'a>(&'a self, process: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.send_input_title, [Arg::Text(process)], &self.num)
    }

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

    pub fn auto_resolves_in<'a>(&'a self, remaining: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.auto_resolves_in, [Arg::Text(remaining)], &self.num)
    }

    pub fn header_cut(&self, rows: usize) -> Fmt<'_, 1> {
        Fmt::new(
            self.clipped_lines,
            [Arg::Int(u64::try_from(rows).unwrap_or(u64::MAX))],
            &self.num,
        )
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

    pub fn act_send_input<'a>(&'a self, process: &'a str, input: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.act_send_input,
            [Arg::Text(process), Arg::Text(input)],
            &self.num,
        )
    }

    pub fn act_call_mcp<'a>(&'a self, server: &'a str, tool: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.act_call_mcp,
            [Arg::Text(server), Arg::Text(tool)],
            &self.num,
        )
    }

    pub fn act_access<'a>(&'a self, target: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.act_access, [Arg::Text(target)], &self.num)
    }

    pub fn with_reason<'a>(&'a self, subject: &'a str, reason: &'a str) -> Fmt<'a, 2> {
        Fmt::new(
            self.with_reason,
            [Arg::Text(subject), Arg::Text(reason)],
            &self.num,
        )
    }

    pub fn sum_patch_one<'a>(&'a self, file: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.sum_patch_one, [Arg::Text(file)], &self.num)
    }

    pub fn sum_patch_many(&self, files: usize) -> Fmt<'_, 1> {
        let count = Arg::Int(u64::try_from(files).unwrap_or(u64::MAX));
        Fmt::new(self.sum_patch_many, [count], &self.num)
    }

    pub fn sum_network<'a>(&'a self, target: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.sum_network, [Arg::Text(target)], &self.num)
    }

    pub fn sum_mcp<'a>(&'a self, tool: &'a str, label: &'a str) -> Fmt<'a, 2> {
        Fmt::new(self.sum_mcp, [Arg::Text(tool), Arg::Text(label)], &self.num)
    }

    pub fn review_many(&self, reviews: usize) -> Fmt<'_, 1> {
        let count = Arg::Int(u64::try_from(reviews).unwrap_or(u64::MAX));
        Fmt::new(self.review_many, [count], &self.num)
    }

    pub fn review_bullet<'a>(&'a self, detail: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.review_bullet, [Arg::Text(detail)], &self.num)
    }

    pub fn review_more(&self, more: usize) -> Fmt<'_, 1> {
        let count = Arg::Int(u64::try_from(more).unwrap_or(u64::MAX));
        Fmt::new(self.review_more, [count], &self.num)
    }

    /// The safe message of a failed verification (`verify::Failure::cause`).
    pub fn verification_failure(&self, cause: Cause) -> &'static str {
        match cause {
            Cause::Unreachable => self.verify_unreachable,
            Cause::InvalidParams => self.verify_invalid_params,
            Cause::CredentialMissing => self.verify_credential_missing,
            Cause::BiometricsUnavailable => self.verify_biometrics_unavailable,
            Cause::ProviderUnavailable => self.verify_provider_unavailable,
            Cause::UserCancelled => self.verify_user_cancelled,
            Cause::Interrupted => self.verify_interrupted,
            Cause::AuthenticationFailed => self.verify_authentication_failed,
            Cause::Timeout => self.verify_timeout,
            Cause::Service => self.verify_service,
        }
    }

    pub fn unlocalized<'a>(&'a self, error: &'a str) -> Fmt<'a, 1> {
        Fmt::new(self.unlocalized, [Arg::Text(error)], &self.num)
    }
}
