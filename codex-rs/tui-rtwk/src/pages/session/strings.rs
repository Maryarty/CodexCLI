//! The strings of `chat` (S10): a table of `&'static str` fields and typed
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

use codex_tui_core::commands::{Notice, SlashCommand, UsageKey};
use codex_tui_core::screens::chat::{McpHeader, McpWarning, Retry, StatusHeader};
use codex_tui_core::screens::composer;
use codex_tui_core::threads::LocalWarning;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

pub(super) const INLINE_GAP: &str = " ";
pub(super) const MORE: &str = " …";
pub(super) const LINE_BREAK: &str = "\n";
const LIST_SEPARATOR: &str = ", ";
const ELLIPSIS: &str = "…";

/// The strings of this screen.
#[derive(Clone, Copy, Debug)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// What the composer starts its line with (`chat_composer.rs`).
    pub prompt: &'static str,
    /// The description of a command in the popup (`slash_command.rs`).
    pub description: fn(SlashCommand) -> &'static str,
    /// After the name of a command the popup lists but a side conversation cannot run, and
    /// before and after the reason in its description (`command_popup.rs`: `is_disabled`).
    pub command_disabled: &'static str,
    pub command_reason: [&'static str; 2],
    /// Why that command is disabled.
    pub command_unavailable: &'static str,
    /// `slash_dispatch.rs`: the messages a command line can end in.
    pub unrecognized: Msg<1>,
    pub disabled_during_task: Msg<1>,
    pub unavailable_in_side: Msg<1>,
    pub unavailable_in_review: Msg<1>,
    pub session_starting: Msg<1>,
    pub model_startup: &'static str,
    pub thread_name_empty: &'static str,
    pub side_rename: &'static str,
    /// `USER_SHELL_COMMAND_HELP_TITLE` and `_HINT` (`chatwidget.rs`): a bare `!`.
    pub shell_help: &'static str,
    /// `user_input_too_large_message` (`chat_composer.rs`): the limit and the size.
    pub input_too_large: Msg<2>,
    /// `submit_user_message`: the thread has no model to send with.
    pub thread_model_unavailable: &'static str,
    pub usage_needs_login: &'static str,
    pub usage_pwd: &'static str,
    pub usage_usage: &'static str,
    pub usage_raw: &'static str,
    pub usage_voice: &'static str,
    pub usage_keymap: &'static str,
    pub usage_mcp: &'static str,
    pub mcp_login_needs_session: &'static str,
    /// The part of a command that needs the session (session-core) is not
    /// built yet: the slice's own text, not the oracle's.
    pub not_available: Msg<1>,
    /// `slash_dispatch.rs`, `chatwidget.rs`: what a command that runs at once
    /// says (`add_info_message`).
    pub working_directory: Msg<1>,
    pub rollout: Msg<1>,
    pub rollout_missing: &'static str,
    pub desktop_opened: &'static str,
    pub stopping_terminals: &'static str,
    pub raw_output_on: &'static str,
    pub raw_output_off: &'static str,
    pub vim_on: &'static str,
    pub vim_off: &'static str,
    /// `add_app_server_stub_message`: the feature, then the message.
    pub stub: Msg<1>,
    pub stub_memory_maintenance: &'static str,
    /// `PARENT_OWNED_INPUT_MESSAGE`.
    pub parent_owned: &'static str,
    pub mode_busy: &'static str,
    pub image_inputs_unsupported: Msg<1>,
    pub clipboard_read_failed: Msg<1>,
    /// `plan_implementation.rs`: the question after a proposed plan.
    pub plan_title: &'static str,
    pub plan_yes: &'static str,
    pub plan_yes_note: &'static str,
    pub plan_clear: &'static str,
    pub plan_clear_note: &'static str,
    pub plan_clear_usage: Msg<1>,
    pub plan_no: &'static str,
    pub plan_no_note: &'static str,
    pub plan_used_percent: Msg<1>,
    pub plan_used_tokens: Msg<1>,
    /// `working_directory.rs`, `event_dispatch.rs`: why `/cd` did nothing.
    pub cd_not_started: &'static str,
    pub cd_not_idle: &'static str,
    pub cd_history_not_saved: &'static str,
    /// `resume_config.rs`: the folder's trust could not be read.
    pub cd_trust_check: Msg<1>,
    /// `event_dispatch.rs` `archive_current_thread`, `delete_current_thread`:
    /// why the thread was not removed (the failure carries the oracle's context
    /// message, `thread_archive`, `thread_delete`).
    pub archive_not_started: &'static str,
    pub delete_not_started: &'static str,
    pub archive_side: &'static str,
    pub delete_side: &'static str,
    pub archive_failed: &'static str,
    pub delete_failed: &'static str,
    /// `skill_popup.rs`, `mentions_v2/candidate.rs`: the tags of a row.
    pub tag_skill: &'static str,
    pub tag_app: &'static str,
    pub tag_plugin: &'static str,
    /// `chat_composer/warning_notice.rs` (`warning_labels`): the footer's
    /// `⚠ N warnings · f2 to view`.
    pub warning_badge: &'static str,
    /// `bottom_pane`/`chatwidget/rendering.rs`: the backend control above the
    /// composer, `Cloud  ⌃x b to cycle` (`BackendSessions::displayed_name`).
    pub backend_local: &'static str,
    pub backend_cloud: &'static str,
    pub backend_connection: Msg<1>,
    pub backend_key: &'static str,
    /// Oracle: `chatwidget/rendering.rs` (the external writer view): the
    /// session is open in another process and shown read-only.
    pub read_only_title: &'static str,
    pub header_prompt: &'static str,
    pub header_title: &'static str,
    pub header_version: Msg<1>,
    pub header_permissions: &'static str,
    pub header_yolo: &'static str,
    pub read_only_hint: &'static str,
    pub writer_retry_key: &'static str,
    pub writer_retry_detail: &'static str,
    pub writer_retry: &'static str,
    pub writer_fork: &'static str,
    pub writer_center: &'static str,
    pub writer_exit: &'static str,
    pub writer_transcript: &'static str,
    pub forking: &'static str,
    /// `startup_draft_renderable`: what the startup draft says while the named session opens.
    pub resuming_session: &'static str,
    /// The footer while the confirmed startup draft waits (`set_footer_hint_override`).
    pub startup_waiting: &'static str,
    pub startup_cancel_key: &'static str,
    pub startup_cancel: &'static str,
    /// The footer while the empty conversation moves to Cloud and when Cloud refused it
    /// (`cloud_switch_wait.rs` `show_cloud_switch_hint`).
    pub cloud_connecting: &'static str,
    pub cloud_failed: &'static str,
    pub cloud_failed_action: &'static str,
    /// `PendingChord::guide`, `ShortcutHud::render_overlay`: the block a held editor chord
    /// draws. The key that cancels, between a key and what it does, between the keys of one
    /// action, the rule's head and the cell before its tail, between the title and the cancel
    /// key of a compact block.
    pub chord_cancel: &'static str,
    pub hud_separator: &'static str,
    pub hud_key_separator: &'static str,
    pub chord_rule_lead: &'static str,
    pub chord_rule_gap: &'static str,
    pub chord_command_gap: &'static str,
    pub forking_session: &'static str,
    /// `chatwidget/provider_error.rs`: the notice that stands in place of the
    /// composer when the server refused the chat for its model provider.
    pub provider_lock: &'static str,
    pub provider_title_continue: &'static str,
    pub provider_title_mismatch: &'static str,
    pub provider_title_unavailable: &'static str,
    pub provider_title_policy: &'static str,
    provider_mismatch: Msg<2>,
    provider_not_configured: Msg<1>,
    provider_policy_provider: Msg<2>,
    pub provider_policy_settings: &'static str,
    provider_location: Msg<1>,
    pub provider_continue_key: &'static str,
    pub provider_continue: &'static str,
    pub provider_new_key: &'static str,
    pub provider_new: &'static str,
    pub provider_other_key: &'static str,
    pub provider_other: &'static str,
    pub provider_separator: &'static str,
    provider_continue_message: Msg<2>,
    provider_previous: &'static str,
    provider_named: Msg<1>,
    /// The display names of the providers the app ships with (`built_in_model_providers`);
    /// any other id is shown as it is.
    provider_names: &'static [(&'static str, &'static str)],
    /// The blank before a footer entry's label, and between two entries.
    pub writer_pad: &'static str,
    pub writer_gap: &'static str,
    pub backend_cycle: &'static str,
    /// `actionable_banner.rs` / `set_inline_banner`: the hint under the
    /// banner's actions, and under an information-only one.
    pub banner_choose: &'static str,
    pub banner_choose_dismiss: &'static str,
    pub banner_dismiss: &'static str,
    /// The Luna Reserve selector (`backend_banners.rs`): its local row, and the footer that
    /// names the keys (`accept_cancel_hint_line`).
    pub reserve_continue: &'static str,
    reserve_hint: Msg<2>,
    pub warning_singular: &'static str,
    pub warning_plural: &'static str,
    pub warning_key: &'static str,
    pub warning_view: &'static str,
    /// `mcp_startup.rs`: the status header of one server, of several (the
    /// answered, the total, the servers still starting), and the warnings.
    pub mcp_booting: Msg<1>,
    pub mcp_starting: Msg<3>,
    pub mcp_failed: Msg<1>,
    pub mcp_interrupted: Msg<1>,
    pub mcp_incomplete: Msg<1>,
    /// `protocol.rs`: a 429 reconnect reads as the server being busy.
    pub retry_busy: Msg<1>,
    /// `on_server_overloaded_error`: an overloaded server that said nothing.
    pub high_load: &'static str,
    /// `REVIEW_STEER_UNAVAILABLE_MESSAGE` (`interaction.rs`).
    pub review_steer_unavailable: &'static str,
    invalid_status_line_items: [Msg<1>; 2],
    invalid_title_items: [Msg<1>; 2],
    invalid_items_join: Msg<2>,
    /// The words of the picker sheets (the effort and mode names the notes
    /// repeat).
    pub picker: &'static crate::sheets::select::strings::Strings,
    // The notes of `/model`, `/permissions` and `/apps`
    // (`model_defaults.rs`, `config_persistence.rs`, `connectors.rs`).
    pub note_no_models: &'static str,
    pub note_models_not_loaded: &'static str,
    pub note_model_list_stale: &'static str,
    pub note_worktrees_unavailable: &'static str,
    pub note_worktree_list_failed: Msg<1>,
    pub note_worktree_removed: Msg<1>,
    pub note_worktree_remove_failed: Msg<2>,
    pub note_app_switch_failed: Msg<2>,
    pub note_model_provider_saving: &'static str,
    pub note_model_changed: Msg<1>,
    pub note_session_only: &'static str,
    pub note_default_effort: &'static str,
    pub note_permissions_updated: Msg<1>,
    pub note_permission_requested: Msg<1>,
    pub note_save_model_failed: Msg<1>,
    pub note_save_plan_failed: Msg<1>,
    pub note_save_profile_failed: Msg<1>,
    pub note_save_tier_failed: Msg<1>,
    pub note_tier_set: Msg<1>,
    pub note_reasoning_disabled: &'static str,
    pub note_reserve_unavailable: &'static str,
    pub note_reasoning_unavailable: Msg<1>,
    pub note_reasoning_lowest: Msg<1>,
    pub note_reasoning_highest: Msg<1>,
    pub note_ultra_available: Msg<1>,
    pub note_overridden: Msg<1>,
    pub note_setting_model: &'static str,
    pub note_setting_plan: &'static str,
    pub note_setting_permissions: &'static str,
    pub note_setting_tier: &'static str,
    pub note_ultra: Msg<2>,
    pub note_apps_disabled: &'static str,
    pub note_apps_hint: &'static str,
    pub note_no_apps: &'static str,
    /// The hint of the `/` popup is the framework's; this is the empty one.
    pub num: NumSyms,
}

/// English: the oracle's text (the slice names the oracle file per group).
pub const EN: Strings = Strings {
    title: "main chat screen",
    prompt: "› ",
    description: description_en,
    command_disabled: " (disabled)",
    command_reason: [" (disabled: ", ")"],
    command_unavailable: "not available in a side conversation",
    unrecognized: msg!("Unrecognized command '/{0}'. Type \"/\" for a list of supported commands."),
    disabled_during_task: msg!("'/{0}' is disabled while a task is in progress."),
    unavailable_in_side: msg!(
        "'/{0}' is unavailable in side conversations. Press Ctrl+C to return to the main thread first."
    ),
    unavailable_in_review: msg!("'/{0}' is unavailable while code review is running."),
    session_starting: msg!("Session is still starting; try /{0} again in a moment."),
    model_startup: "Model selection is disabled until startup completes.",
    thread_name_empty: "Thread name cannot be empty.",
    side_rename: "Side conversations cannot be renamed.",
    shell_help: "Prefix a command with ! to run it locally\nExample: !ls",
    input_too_large: msg!("Message exceeds the maximum length of {0} characters ({1} provided)."),
    thread_model_unavailable: "Thread model is unavailable. Wait for the thread to finish syncing or choose a model before sending input.",
    usage_needs_login: "Sign in with ChatGPT to use /usage.",
    usage_pwd: "Usage: /pwd",
    usage_usage: "Usage: /usage [daily|weekly|cumulative]",
    usage_raw: "Usage: /raw [on|off]",
    usage_voice: "Usage: /voice [settings|mute|stop]",
    usage_keymap: "Usage: /keymap [debug]",
    usage_mcp: "Usage: /mcp [verbose | login <name>]",
    mcp_login_needs_session: "MCP sign-in requires an active session.",
    not_available: msg!("'/{0}' is not available yet."),
    working_directory: msg!("Current working directory: {0}"),
    rollout: msg!("Current rollout path: {0}"),
    rollout_missing: "Rollout path is not available yet.",
    desktop_opened: "Opened this session in the Desktop app.",
    stopping_terminals: "Stopping all background terminals.",
    raw_output_on: "Raw output mode on: transcript text is shown for clean terminal selection.",
    raw_output_off: "Raw output mode off: rich transcript rendering restored.",
    vim_on: "Vim mode enabled.",
    vim_off: "Vim mode disabled.",
    stub: msg!("{0}: Not available in TUI yet."),
    stub_memory_maintenance: "Memory maintenance",
    parent_owned: "This sub-agent is controlled by its parent. Direct input is disabled.",
    mode_busy: "Cannot switch collaboration mode while a turn is running.",
    image_inputs_unsupported: msg!(
        "Model {0} does not support image inputs. Remove images or switch models."
    ),
    clipboard_read_failed: msg!("Failed to paste image: {0}"),
    plan_title: "Implement this plan?",
    plan_yes: "Yes, implement this plan",
    plan_yes_note: "Switch to Default and start coding",
    plan_clear: "Yes, clear context and implement",
    plan_clear_note: "Fresh thread with this plan",
    plan_clear_usage: msg!("Start a fresh thread (current context: {0})"),
    plan_no: "No, stay in Plan mode",
    plan_no_note: "Continue planning with the model",
    plan_used_percent: msg!("{0}% used"),
    plan_used_tokens: msg!("{0} used"),
    cd_not_started: "The session must start before you can change its working directory.",
    cd_not_idle: "Changing directories requires an idle primary session without queued input.",
    cd_history_not_saved: "Conversation history is not saved.",
    cd_trust_check: msg!("Unable to check folder trust: {0}"),
    archive_not_started: "A thread must start before it can be archived.",
    delete_not_started: "A thread must start before it can be deleted.",
    archive_side: "'/archive' is unavailable in side conversations. Press Ctrl+C to return to the main thread first.",
    delete_side: "'/delete' is unavailable in side conversations. Press Ctrl+C to return to the main thread first.",
    archive_failed: "Failed to archive current thread: failed to archive session",
    delete_failed: "Failed to delete current thread: failed to delete session",
    tag_skill: "[Skill]",
    tag_app: "[App]",
    tag_plugin: "Plugin",
    warning_badge: "\u{26a0} ",
    backend_local: "Local",
    backend_cloud: "Cloud",
    backend_connection: msg!("Connection {0}"),
    backend_key: "\u{2303}x b",
    read_only_title: "This conversation is open in another app",
    header_prompt: ">_ ",
    header_title: "OpenAI Codex",
    header_version: msg!(" (v{0})"),
    header_permissions: "permissions: ",
    header_yolo: "YOLO mode",
    read_only_hint: "Close it there and press R to continue here.",
    writer_retry_key: "r",
    writer_retry_detail: " to retry",
    writer_retry: "retry",
    writer_fork: "fork",
    writer_center: "command center",
    writer_exit: "exit",
    writer_transcript: "transcript",
    forking: "Forking conversation…",
    resuming_session: "  Resuming session…",
    startup_waiting: "Waiting for startup",
    startup_cancel_key: "esc",
    startup_cancel: "cancel",
    cloud_connecting: "Connecting to Cloud…",
    cloud_failed: "Cloud conversation did not open",
    cloud_failed_action: "draft kept",
    chord_cancel: "Esc cancel",
    hud_separator: " \u{b7} ",
    hud_key_separator: " / ",
    chord_rule_lead: "\u{2500} ",
    chord_rule_gap: " ",
    chord_command_gap: "   ",
    forking_session: "  Forking session…",
    provider_lock: "\u{1f512}",
    provider_title_continue: "Continue in a new chat",
    provider_title_mismatch: "Model provider mismatch",
    provider_title_unavailable: "Model provider unavailable",
    provider_title_policy: "Model provider blocked by policy",
    provider_mismatch: msg!(
        "This chat uses {0}, but Codex now uses {1}, so this chat is read-only."
    ),
    provider_not_configured: msg!(
        "This chat uses model provider \u{201c}{0}\u{201d}, which is no longer configured. Restore this provider\u{2019}s configuration to continue."
    ),
    provider_policy_provider: msg!(
        "This chat uses model provider \u{201c}{0}\u{201d}, but your organization requires \u{201c}{1}\u{201d}. Contact your administrator or start a new chat using the required provider."
    ),
    provider_policy_settings: "This chat\u{2019}s model provider settings no longer match your organization\u{2019}s requirements. Contact your administrator or start a new chat using the current provider settings.",
    provider_location: msg!("Configuration on {0}"),
    provider_continue_key: "c",
    provider_continue: " continue in new chat",
    provider_new_key: "n",
    provider_new: " start fresh",
    provider_other_key: "esc",
    provider_other: " other chats",
    provider_separator: "\u{b7} ",
    provider_continue_message: msg!(
        "Read {0} and let\u{2019}s continue where we left off.\n\nSaved conversation: {1}"
    ),
    provider_previous: "our previous conversation",
    provider_named: msg!("\u{201c}{0}\u{201d}"),
    provider_names: &[
        ("openai", "OpenAI"),
        ("amazon-bedrock", "Amazon Bedrock"),
        ("amazon-bedrock-runtime", "Amazon Bedrock Runtime"),
        ("gcp", "Google Cloud Vertex AI"),
        ("ollama", "gpt-oss"),
        ("lmstudio", "gpt-oss"),
    ],
    writer_pad: " ",
    writer_gap: "   ",
    backend_cycle: " to cycle",
    banner_choose: "Press a number to choose",
    banner_choose_dismiss: "Press a number to choose \u{b7} esc to dismiss \u{b7} type to continue",
    banner_dismiss: "esc to dismiss \u{b7} type to continue",
    reserve_continue: "Continue with Luna Reserve",
    reserve_hint: msg!("Press {0} to confirm or {1} to continue working"),
    warning_singular: "warning",
    warning_plural: "warnings",
    warning_key: "f2",
    warning_view: " to view",
    mcp_booting: msg!("Booting MCP server: {0}"),
    mcp_starting: msg!("Starting MCP servers ({0}/{1}): {2}"),
    mcp_failed: msg!("MCP client for `{0}` failed to start"),
    mcp_interrupted: msg!(
        "MCP startup interrupted. The following servers were not initialized: {0}"
    ),
    mcp_incomplete: msg!("MCP startup incomplete (failed: {0})"),
    retry_busy: msg!("Server is busy, reconnecting {0}"),
    high_load: "Codex is currently experiencing high load.",
    review_steer_unavailable: "Steer messages aren't supported during /review. Press Ctrl+C now to cancel the review.",
    invalid_status_line_items: [
        msg!("Ignored invalid status line item: {0}."),
        msg!("Ignored invalid status line items: {0}."),
    ],
    invalid_title_items: [
        msg!("Ignored invalid terminal title item: {0}."),
        msg!("Ignored invalid terminal title items: {0}."),
    ],
    invalid_items_join: msg!("{0} and {1}"),
    picker: &crate::sheets::select::strings::EN,
    note_no_models: "No additional models are available right now.",
    note_models_not_loaded: "Could not load models. Try /model again.",
    note_worktrees_unavailable: "Managed worktrees require a local Git repository.",
    note_worktree_list_failed: msg!("Cannot list managed worktrees: {0}"),
    note_worktree_removed: msg!("Removed worktree at {0}. Thread history was kept."),
    note_worktree_remove_failed: msg!("Could not remove worktree at {0}: {1}"),
    note_app_switch_failed: msg!("Failed to update app config for {0}: {1}"),
    note_model_list_stale: "The model list may be out of date. Restart the app server and TUI to refresh /model.",
    note_model_provider_saving: "Saving the default provider. Try /model after the save finishes.",
    note_model_changed: msg!("Model changed to {0}"),
    note_session_only: " for this session only",
    note_default_effort: "default",
    note_permissions_updated: msg!("Permissions updated to {0}"),
    note_permission_requested: msg!("Permission selection requested: {0}"),
    note_save_model_failed: msg!("Failed to save default model: {0}"),
    note_save_plan_failed: msg!("Failed to save Plan mode reasoning effort: {0}"),
    note_save_profile_failed: msg!("Failed to set permission profile: {0}"),
    note_save_tier_failed: msg!(
        "Failed to save default service tier: {0}\nYou can continue this task. To save the default, resolve the error above, then switch to a different tier and back to the desired tier."
    ),
    note_tier_set: msg!("Service tier set to {0}"),
    note_reasoning_disabled: "Reasoning shortcuts are disabled until startup completes.",
    note_reserve_unavailable: "Luna model settings are unavailable; please try /model again in a moment.",
    note_reasoning_unavailable: msg!("Reasoning shortcuts are unavailable for {0}."),
    note_reasoning_lowest: msg!("Reasoning is already at the lowest level ({0})."),
    note_reasoning_highest: msg!("Reasoning is already at the highest level ({0})."),
    note_ultra_available: msg!("Ultra is available under /model → {0} → More reasoning…"),
    note_overridden: msg!(
        "Saved {0}, but a higher-priority configuration layer overrides the saved value."
    ),
    note_setting_model: "default model and reasoning effort",
    note_setting_plan: "Plan mode reasoning effort",
    note_setting_permissions: "permissions",
    note_setting_tier: "default service tier",
    note_ultra: msg!(
        "Ultra reasoning may proactively use multiple agents. This session is configured for {0} concurrent threads with up to {1} subagents which can increase usage quickly. Consider setting features.multi_agent_v2.max_concurrent_threads_per_session below 8."
    ),
    note_apps_disabled: "Apps are disabled.",
    note_apps_hint: "Enable the apps feature to use $ or /apps.",
    note_no_apps: "No apps available.",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}

impl Strings {
    /// The usage label of the plan prompt's clear-context row (`89% used`, `123K used`).
    pub fn plan_usage(&self, used: codex_tui_core::screens::chat::plan_mode::Used) -> String {
        use codex_tui_core::screens::chat::plan_mode::Used;
        let (msg, text) = match used {
            Used::Percent(percent) => (self.plan_used_percent, percent.to_string()),
            Used::Tokens(tokens) => (
                self.plan_used_tokens,
                codex_tui_core::format::format_tokens_compact(tokens),
            ),
        };
        Fmt::new(msg, [Arg::Text(&text)], &self.num).to_string()
    }

    /// The description of the clear-context row.
    pub fn plan_clear_description(&self, label: Option<&str>) -> String {
        match label {
            Some(label) => {
                Fmt::new(self.plan_clear_usage, [Arg::Text(label)], &self.num).to_string()
            }
            None => self.plan_clear_note.to_string(),
        }
    }
}

/// The title of the screen in `lang`, for code with no tree (the debug
/// menu's rows).
pub fn title(lang: Lang) -> &'static str {
    TABLES.pick(lang).title
}

/// The description of a command in the popup (`slash_command.rs`).
pub fn description(lang: Lang, command: SlashCommand) -> &'static str {
    (TABLES.pick(lang).description)(command)
}

impl Strings {
    /// The hint under a backend banner.
    pub(super) fn banner_hint(
        &self,
        hint: crate::pages::session::banners::BannerHint,
    ) -> Option<&'static str> {
        use crate::pages::session::banners::BannerHint;
        match hint {
            BannerHint::Choose => Some(self.banner_choose),
            BannerHint::ChooseOrDismiss => Some(self.banner_choose_dismiss),
            BannerHint::Dismiss => Some(self.banner_dismiss),
            BannerHint::None => None,
        }
    }

    /// The Luna selector's footer, with the keys the list's keymap names.
    pub(super) fn reserve_hint(&self, accept: &str, cancel: &str) -> String {
        Fmt::new(
            self.reserve_hint,
            [Arg::Text(accept), Arg::Text(cancel)],
            &self.num,
        )
        .to_string()
    }

    pub(super) fn transcript_label(&self, message: Msg<1>, arg: &str) -> String {
        self.one(message, arg)
    }

    fn one(&self, message: Msg<1>, arg: &str) -> String {
        Fmt::new(message, [Arg::Text(arg)], &self.num).to_string()
    }

    /// The words under the notice's title (`provider_error_message`).
    pub(super) fn provider_message(
        &self,
        lock: &super::live::provider_error::Lock,
        configured: &[codex_tui_core::screens::chat::ModelProviderName],
    ) -> String {
        use super::live::provider_error::Lock;
        let two = |message: Msg<2>, a: &str, b: &str| {
            Fmt::new(message, [Arg::Text(a), Arg::Text(b)], &self.num).to_string()
        };
        let provider_name = |id: &str| match configured
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.name.trim())
        {
            Some("") => id.to_owned(),
            Some(name) => name.to_owned(),
            None => self.provider_name(id),
        };
        match lock {
            Lock::Mismatch { provider, current } => two(
                self.provider_mismatch,
                &provider_name(provider),
                &provider_name(current),
            ),
            Lock::NotConfigured { provider } => {
                self.one(self.provider_not_configured, &provider_name(provider))
            }
            Lock::PolicyProvider { provider, required } => two(
                self.provider_policy_provider,
                &provider_name(provider),
                &provider_name(required),
            ),
            Lock::PolicySettings => self.provider_policy_settings.to_owned(),
        }
    }

    pub(super) fn provider_title(
        &self,
        lock: &super::live::provider_error::Lock,
        can_continue: bool,
    ) -> &'static str {
        use super::live::provider_error::Lock;
        match lock {
            Lock::Mismatch { .. } if can_continue => self.provider_title_continue,
            Lock::Mismatch { .. } => self.provider_title_mismatch,
            Lock::NotConfigured { .. } => self.provider_title_unavailable,
            Lock::PolicyProvider { .. } | Lock::PolicySettings => self.provider_title_policy,
        }
    }

    fn provider_name(&self, id: &str) -> String {
        self.provider_names
            .iter()
            .find(|(known, _)| *known == id)
            .map_or_else(|| id.to_owned(), |(_, name)| (*name).to_owned())
    }

    /// Where the remote server's configuration is, under the message.
    pub(super) fn provider_location(&self, address: &str) -> String {
        self.one(self.provider_location, address)
    }

    /// What the continue key puts in the new chat's composer.
    pub(super) fn provider_continue_message(&self, name: Option<&str>, path: &str) -> String {
        let name = match name.filter(|name| !name.trim().is_empty()) {
            Some(name) => self.one(self.provider_named, name),
            None => self.provider_previous.to_owned(),
        };
        Fmt::new(
            self.provider_continue_message,
            [Arg::Text(&name), Arg::Text(path)],
            &self.num,
        )
        .to_string()
    }

    /// The status row's header when a retry or MCP startup owns it.
    pub fn status_header(&self, header: &StatusHeader) -> String {
        match header {
            StatusHeader::Retry(Retry::Server(message)) => message.clone(),
            StatusHeader::Retry(Retry::Busy { attempt }) => self.one(self.retry_busy, attempt),
            StatusHeader::Mcp(McpHeader::Booting { server }) => self.one(self.mcp_booting, server),
            StatusHeader::Mcp(McpHeader::Starting {
                completed,
                total,
                servers,
                more,
            }) => {
                let mut named = servers.join(LIST_SEPARATOR);
                if *more {
                    named.push_str(LIST_SEPARATOR);
                    named.push_str(ELLIPSIS);
                }
                Fmt::new(
                    self.mcp_starting,
                    [
                        Arg::Text(&completed.to_string()),
                        Arg::Text(&total.to_string()),
                        Arg::Text(&named),
                    ],
                    &self.num,
                )
                .to_string()
            }
        }
    }

    /// The words of a warning the client raised itself.
    pub fn local_warning(&self, warning: &LocalWarning) -> String {
        match warning {
            LocalWarning::ReviewSteerUnavailable => self.review_steer_unavailable.to_owned(),
            LocalWarning::InvalidStatusLineItems(items) => {
                self.invalid_items(self.invalid_status_line_items, items)
            }
            LocalWarning::InvalidTitleItems(items) => {
                self.invalid_items(self.invalid_title_items, items)
            }
        }
    }

    fn invalid_items(&self, words: [Msg<1>; 2], items: &[String]) -> String {
        let ids: Vec<_> = items.iter().map(|id| format!(r#""{id}""#)).collect();
        if let Some((last, rest)) = ids.split_last().filter(|(_, rest)| !rest.is_empty()) {
            let front = rest.join(LIST_SEPARATOR);
            let joined = Fmt::new(
                self.invalid_items_join,
                [Arg::Text(&front), Arg::Text(last)],
                &self.num,
            )
            .to_string();
            self.one(words[1], &joined)
        } else {
            self.one(words[0], ids.first().map_or("", String::as_str))
        }
    }

    /// The words of a warning MCP startup added to the history.
    pub fn mcp_warning(&self, warning: &McpWarning) -> String {
        match warning {
            McpWarning::Failed { server, error } => match error {
                Some(error) => error.clone(),
                None => self.one(self.mcp_failed, server),
            },
            McpWarning::Interrupted { servers } => {
                self.one(self.mcp_interrupted, &servers.join(LIST_SEPARATOR))
            }
            McpWarning::Incomplete { servers } => {
                self.one(self.mcp_incomplete, &servers.join(LIST_SEPARATOR))
            }
        }
    }

    /// The name of a backend on the control.
    pub fn backend(&self, label: codex_tui_core::app::BackendLabel) -> String {
        use codex_tui_core::app::BackendLabel;
        match label {
            BackendLabel::Local => self.backend_local.to_owned(),
            BackendLabel::Cloud => self.backend_cloud.to_owned(),
            BackendLabel::Connection(n) => Fmt::new(
                self.backend_connection,
                [Arg::Text(&n.to_string())],
                &self.num,
            )
            .to_string(),
        }
    }

    /// The words of a note of the transcript (a command line's answer).
    pub fn note(&self, key: &codex_tui_core::threads::NoteKey) -> String {
        use codex_tui_core::screens::agents::Action;
        use codex_tui_core::threads::{CdRefusal, NoteKey, RemoveNote, Stub, Toggle};
        let text = |msg: Msg<1>, arg: &str| Fmt::new(msg, [Arg::Text(arg)], &self.num).to_string();
        match key {
            NoteKey::Command(notice) => self.notice(notice),
            NoteKey::Server(message) | NoteKey::ServerDone(message) => message.clone(),
            NoteKey::WorkingDirectory(dir) => text(self.working_directory, dir),
            NoteKey::Rollout(Some(path)) => text(self.rollout, path),
            NoteKey::Rollout(None) => self.rollout_missing.to_string(),
            NoteKey::DesktopOpened => self.desktop_opened.to_string(),
            NoteKey::StoppingTerminals => self.stopping_terminals.to_string(),
            NoteKey::Stub(Stub::MemoryMaintenance) => text(self.stub, self.stub_memory_maintenance),
            NoteKey::Toggled(Toggle::RawOutput, true) => self.raw_output_on.to_string(),
            NoteKey::Toggled(Toggle::RawOutput, false) => self.raw_output_off.to_string(),
            NoteKey::Toggled(Toggle::Vim, true) => self.vim_on.to_string(),
            NoteKey::Toggled(Toggle::Vim, false) => self.vim_off.to_string(),
            NoteKey::ParentOwned => self.parent_owned.to_string(),
            NoteKey::ModeBusy => self.mode_busy.to_string(),
            NoteKey::CdRefused(CdRefusal::NotStarted) => self.cd_not_started.to_string(),
            NoteKey::CdRefused(CdRefusal::NotIdle) => self.cd_not_idle.to_string(),
            NoteKey::CdRefused(CdRefusal::HistoryNotSaved) => self.cd_history_not_saved.to_string(),
            NoteKey::CdRefused(CdRefusal::TrustCheck(error)) => text(self.cd_trust_check, error),
            NoteKey::Remove(Action::Archive, RemoveNote::NotStarted) => {
                self.archive_not_started.to_string()
            }
            NoteKey::Remove(Action::Delete, RemoveNote::NotStarted) => {
                self.delete_not_started.to_string()
            }
            NoteKey::Remove(Action::Archive, RemoveNote::SideConversation) => {
                self.archive_side.to_string()
            }
            NoteKey::Remove(Action::Delete, RemoveNote::SideConversation) => {
                self.delete_side.to_string()
            }
            NoteKey::Remove(Action::Archive, RemoveNote::Failed) => self.archive_failed.to_string(),
            NoteKey::Remove(Action::Delete, RemoveNote::Failed) => self.delete_failed.to_string(),
            NoteKey::Select(note) => self.picker_note(note),
            NoteKey::Feature(note) => crate::features::note_words(note, self),
            // The words of the mode notes are the view task's (chunk-popups-plan); the oracle's
            // English stands until its catalog entries exist.
            NoteKey::Mode(note) => mode_note(note),
        }
    }

    /// The words of what a picker said (`select::Note`).
    fn picker_note(&self, note: &codex_tui_core::screens::select::Note) -> String {
        use codex_tui_core::screens::select::permissions::{Label, Preset, Reviewer};
        use codex_tui_core::screens::select::{Note, Provider, Setting};
        let text = |msg: Msg<1>, arg: &str| Fmt::new(msg, [Arg::Text(arg)], &self.num).to_string();
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        let picker = self.picker;
        match note {
            Note::NoAdditionalModels => self.note_no_models.to_string(),
            // The words of the preset notes are the view task's (chunk-popups-quick-dials); the
            // oracle's English stands until its catalog entries exist.
            Note::PresetsUnavailable => "Presets are unavailable in this conversation.".to_string(),
            Note::PresetsStarting => "Presets are available after startup completes.".to_string(),
            Note::PresetImages => {
                "Images cannot be copied into a preset. Starting with empty text.".to_string()
            }
            Note::PresetMentions => {
                "Linked mentions cannot be copied into a preset. Starting with empty text."
                    .to_string()
            }
            Note::PresetInvalid(reason) => preset_reason(*reason).to_string(),
            Note::PresetBroken { slot, reason } => {
                format!("Preset {slot}: {}", preset_reason(*reason))
            }
            Note::PresetOutside(slot) => format!(
                "Preset {slot} is defined outside your user config. Edit its source configuration instead."
            ),
            Note::PresetFailed { slot, error } => {
                format!("Failed to update preset {slot}: {error}")
            }
            Note::PresetSaved(slot) => format!("Saved preset {slot}."),
            Note::PresetDeleted(slot) => format!("Deleted preset {slot}."),
            Note::ModelsNotLoaded => self.note_models_not_loaded.to_string(),
            Note::Worktree(
                codex_tui_core::screens::select::worktree::WorktreeLine::Unavailable,
            ) => self.note_worktrees_unavailable.to_string(),
            Note::Worktree(
                codex_tui_core::screens::select::worktree::WorktreeLine::ListFailed(error),
            ) => text(self.note_worktree_list_failed, error),
            // `start_managed_worktree`, `finish_managed_worktree` (English only until the table
            // of the session strings gets their keys: UI LEFT).
            Note::Worktree(codex_tui_core::screens::select::worktree::WorktreeLine::Refused(
                why,
            )) => match why {
                codex_tui_core::screens::select::worktree::Refusal::NotIdle => {
                    "Creating a worktree requires an idle primary session without queued input."
                }
                codex_tui_core::screens::select::worktree::Refusal::Pending => {
                    "A worktree is already being created."
                }
                codex_tui_core::screens::select::worktree::Refusal::HistoryNotSaved => {
                    "Conversation history is not saved."
                }
            }
            .to_string(),
            Note::Worktree(codex_tui_core::screens::select::worktree::WorktreeLine::Failed(
                error,
            )) => error.clone(),
            Note::Worktree(codex_tui_core::screens::select::worktree::WorktreeLine::Retained {
                root,
                reason,
            }) => {
                use codex_tui_core::app::worktree::{BindFailure, RetainedReason};
                let reason = match reason {
                    RetainedReason::Start(codex_tui_core::rpc::RpcError::Remote(error)) => {
                        format!("Cannot start the conversation in the new worktree: {error}")
                    }
                    RetainedReason::Start(_) => {
                        "The server did not start a conversation in the new worktree.".to_owned()
                    }
                    RetainedReason::Bind(BindFailure::Workspace(error)) => error.clone(),
                    RetainedReason::Bind(BindFailure::Interrupted) => {
                        "Binding the conversation to the new worktree was interrupted.".to_owned()
                    }
                };
                format!(
                    "{reason} A checkout was retained at {root}; remove it with `git worktree remove <checkout-path>` from the source repository if it is no longer needed."
                )
            }
            Note::Worktree(codex_tui_core::screens::select::worktree::WorktreeLine::Removed {
                root,
            }) => text(self.note_worktree_removed, root),
            Note::Worktree(
                codex_tui_core::screens::select::worktree::WorktreeLine::RemoveFailed {
                    root,
                    error,
                },
            ) => Fmt::new(
                self.note_worktree_remove_failed,
                [Arg::Text(root), Arg::Text(error)],
                &self.num,
            )
            .to_string(),
            Note::ModelPickerUnavailable(Provider::Saving) => {
                self.note_model_provider_saving.to_string()
            }
            Note::ModelPickerUnavailable(_) => self.note_model_list_stale.to_string(),
            Note::ModelChanged {
                model,
                effort,
                session,
            } => {
                let mut line = text(self.note_model_changed, model);
                if !model.starts_with("codex-auto-") {
                    line.push(' ');
                    match effort {
                        None | Some(codex_protocol::openai_models::ReasoningEffort::None) => {
                            line.push_str(self.note_default_effort);
                        }
                        Some(effort) => line.push_str(effort.as_str()),
                    }
                }
                if *session {
                    line.push_str(self.note_session_only);
                }
                line
            }
            Note::ServiceTierSet(tier) => text(self.note_tier_set, tier),
            Note::ReasoningDisabled => self.note_reasoning_disabled.to_string(),
            Note::ReserveUnavailable => self.note_reserve_unavailable.to_string(),
            Note::ReasoningUnavailable(model) => text(self.note_reasoning_unavailable, model),
            Note::ReasoningBound { direction, effort } => {
                use codex_protocol::openai_models::ReasoningEffort as E;
                use codex_tui_core::screens::select::reasoning::Direction;
                let label = match effort {
                    E::XHigh => "extra high".to_string(),
                    E::Custom(value) => value.clone(),
                    other => other.as_str().to_string(),
                };
                let message = match direction {
                    Direction::Lower => self.note_reasoning_lowest,
                    Direction::Raise => self.note_reasoning_highest,
                };
                text(message, &label)
            }
            Note::UltraAvailable(model) => {
                let path = if model.starts_with("codex-auto-") {
                    model.clone()
                } else {
                    format!("All models → {model}")
                };
                text(self.note_ultra_available, &path)
            }
            Note::UltraConcurrency(threads) => Fmt::new(
                self.note_ultra,
                [int(*threads), int(threads.saturating_sub(1))],
                &self.num,
            )
            .to_string(),
            Note::PermissionsUpdated(label) | Note::PermissionSelectionRequested(label) => {
                let name = match label {
                    Label::Mode {
                        preset: Preset::Auto,
                        reviewer: Reviewer::AutoReview,
                    } => picker.mode_approve.to_string(),
                    Label::Mode {
                        preset: Preset::Auto,
                        ..
                    } => picker.mode_ask.to_string(),
                    Label::Mode {
                        preset: Preset::FullAccess,
                        ..
                    } => picker.mode_full.to_string(),
                    Label::Mode {
                        preset: Preset::ReadOnly,
                        ..
                    } => picker.mode_read.to_string(),
                    Label::Profile(id) => id.clone(),
                };
                let message = match note {
                    Note::PermissionSelectionRequested(_) => self.note_permission_requested,
                    _ => self.note_permissions_updated,
                };
                text(message, &name)
            }
            Note::SaveFailed { setting, error } => match setting {
                Setting::Model => text(self.note_save_model_failed, error),
                Setting::PlanEffort => text(self.note_save_plan_failed, error),
                Setting::Permissions => text(self.note_save_profile_failed, error),
                Setting::ServiceTier => text(self.note_save_tier_failed, error),
                // `SetAppEnabled` refused: the link dialog's switch was not saved.
                Setting::App(id) => Fmt::new(
                    self.note_app_switch_failed,
                    [Arg::Text(id), Arg::Text(error)],
                    &self.num,
                )
                .to_string(),
            },
            Note::Overridden(setting) => text(
                self.note_overridden,
                match setting {
                    Setting::Model => self.note_setting_model,
                    Setting::PlanEffort => self.note_setting_plan,
                    Setting::Permissions => self.note_setting_permissions,
                    Setting::ServiceTier => self.note_setting_tier,
                    Setting::App(id) => id,
                },
            ),
            Note::AppsDisabled => format!(
                "{}{INLINE_GAP}{}",
                self.note_apps_disabled, self.note_apps_hint
            ),
            Note::NoApps => self.note_no_apps.to_string(),
            Note::AppsFailed(error) => error.clone(),
            Note::AppLinkUnavailable(status) => {
                picker.app_status_line(picker.apps_no_link, *status)
            }
            // The words of the `/permissions` arguments and shortcut notes are the view task's
            // (chunk-popups-permission-args); the oracle's English stands until its catalog entries
            // exist.
            Note::PermissionArgs(error) => permission_args_error(error),
            Note::PermissionRefused(refusal) => permission_refusal(picker, refusal),
            Note::NoOtherPermissionModes => "No other permission modes are available.".to_string(),
        }
    }

    /// The words of a notice a command line ended in.
    pub fn notice(&self, notice: &Notice) -> String {
        let text =
            |msg: Msg<1>, name: &str| Fmt::new(msg, [Arg::Text(name)], &self.num).to_string();
        match notice {
            Notice::Unrecognized(name) => text(self.unrecognized, name),
            Notice::DisabledDuringTask(c) => text(self.disabled_during_task, c.command()),
            Notice::UnavailableInSide(c) => text(self.unavailable_in_side, c.command()),
            Notice::TierUnavailableInSide(name) => text(self.unavailable_in_side, name),
            Notice::UnavailableInReview(c) => text(self.unavailable_in_review, c.command()),
            Notice::SessionStarting(c) => text(self.session_starting, c.command()),
            Notice::NotAvailable(c) => text(self.not_available, c.command()),
            Notice::ModelStartup => self.model_startup.to_string(),
            Notice::ShellHelp => self.shell_help.to_string(),
            Notice::ThreadModelUnavailable => self.thread_model_unavailable.to_string(),
            Notice::ImageInputsUnsupported(model) => text(self.image_inputs_unsupported, model),
            Notice::ClipboardReadFailed(error) => text(self.clipboard_read_failed, error),
            Notice::InputTooLarge { provided } => Fmt::new(
                self.input_too_large,
                [
                    Arg::Text(&composer::MAX_INPUT_CHARS.to_string()),
                    Arg::Text(&provided.to_string()),
                ],
                &self.num,
            )
            .to_string(),
            Notice::UsageNeedsLogin => self.usage_needs_login.to_string(),
            Notice::McpLoginNeedsSession => self.mcp_login_needs_session.to_string(),
            Notice::Usage(key) => match key {
                UsageKey::Pwd => self.usage_pwd,
                UsageKey::Usage => self.usage_usage,
                UsageKey::Raw => self.usage_raw,
                UsageKey::Voice => self.usage_voice,
                UsageKey::Keymap => self.usage_keymap,
                UsageKey::Mcp => self.usage_mcp,
            }
            .to_string(),
        }
    }
}

/// The oracle's `SlashCommand::description` (`slash_command.rs`), exhaustive
/// so a new command cannot go without a description.
fn description_en(command: SlashCommand) -> &'static str {
    match command {
        SlashCommand::Config => "configure Codex preferences",
        SlashCommand::Feedback => "send logs to maintainers or save a ZIP",
        SlashCommand::New => "start a new chat during a conversation",
        SlashCommand::Init => "create an AGENTS.md file with instructions for Codex",
        SlashCommand::Compact => "summarize conversation to prevent hitting the context limit",
        SlashCommand::Recap => "summarize the current conversation now",
        SlashCommand::Review => "review my current changes and find issues",
        SlashCommand::Rename => "rename the current thread",
        SlashCommand::Resume => "resume a saved chat",
        SlashCommand::Archive => "archive this session",
        SlashCommand::Delete => "permanently delete this session",
        SlashCommand::Clear => "clear the terminal and start a new chat",
        SlashCommand::Fork => "fork the current chat",
        SlashCommand::Worktree => "start or continue a conversation in a new worktree",
        SlashCommand::App => "continue this session in the Desktop app",
        SlashCommand::Quit | SlashCommand::Exit => "exit Codex",
        SlashCommand::Copy => "copy the last response or part of it",
        SlashCommand::Export => "export the conversation as markdown",
        SlashCommand::Raw => "toggle raw scrollback mode for copy-friendly terminal selection",
        SlashCommand::Tui => "choose the TUI mode for the next launch",
        SlashCommand::Diff => "show git diff (including untracked files)",
        SlashCommand::Mention => "mention a file",
        SlashCommand::Skills => "use skills to improve how Codex performs specific tasks",
        SlashCommand::Import => "import setup, this project, and recent chats from Claude Code",
        SlashCommand::Hooks => "view and manage lifecycle hooks",
        SlashCommand::Daemon => "Manage the local background server",
        SlashCommand::Warnings => "view retained warnings and diagnostic details",
        SlashCommand::Status => "show current session configuration and token usage",
        SlashCommand::Cd => "change the current working directory",
        SlashCommand::Pwd => "show the current working directory",
        SlashCommand::Usage => "view account usage or use a usage limit reset",
        SlashCommand::DebugConfig => "show config layers and requirement sources for debugging",
        SlashCommand::Title => "configure which items appear in the terminal title",
        SlashCommand::Statusline => "configure which items appear in the status line",
        SlashCommand::Theme => "choose a syntax highlighting theme",
        SlashCommand::Pets => "choose or hide the terminal pet",
        SlashCommand::Ps => "list background terminals",
        SlashCommand::Stop => "stop all background terminals",
        SlashCommand::MemoryDrop => "DO NOT USE",
        SlashCommand::MemoryUpdate => "DO NOT USE",
        SlashCommand::Model => "choose what model and reasoning effort to use",
        SlashCommand::Presets => "run or save reusable prompts and slash commands",
        SlashCommand::Daybreak => "turn Daybreak on or off",
        SlashCommand::Ide => {
            "include current selection, open files, and other context from your IDE"
        }
        SlashCommand::Plan => "switch to Plan mode",
        SlashCommand::Voice => "start or stop voice; use /voice settings to choose a voice",
        SlashCommand::Goal => "set or view the goal for a long-running task",
        SlashCommand::Agents => "open the agent command center",
        SlashCommand::MultiAgents => "switch between this session's subagents",
        SlashCommand::Side | SlashCommand::Btw => "start a side conversation",
        SlashCommand::Permissions => "choose what Codex is allowed to do",
        SlashCommand::Keymap => "remap TUI shortcuts",
        SlashCommand::Vim => "toggle Vim mode for the composer",
        SlashCommand::ElevateSandbox => "set up elevated agent sandbox",
        SlashCommand::Experimental => "toggle experimental features",
        SlashCommand::AutoReview => "approve one retry of a recent auto-review denial",
        SlashCommand::Memories => "configure memory use and generation",
        SlashCommand::Mcp => "list MCP tools; use /mcp verbose or /mcp login <name>",
        SlashCommand::Apps => "manage apps",
        SlashCommand::Plugins => "browse plugins",
        SlashCommand::Logout => "log out of Codex",
        SlashCommand::Rollout => "print the rollout file path",
        SlashCommand::TestApproval => "test approval request",
    }
}

/// `parse_permission_profile_args` and `slash_command_args::parse` failures.
fn permission_args_error(
    error: &codex_tui_core::screens::select::permission_args::Error,
) -> String {
    use codex_tui_core::commands::args::ArgError;
    use codex_tui_core::screens::select::permission_args::Error;
    match error {
        Error::Usage => "Usage: /permissions <profile-id> [--approval-policy=<policy>] [--reviewer=user|auto_review]".to_string(),
        Error::Unexpected(argument) => format!("Unexpected permission argument `{argument}`."),
        Error::Approval(value) => format!(
            "Unknown approval policy `{value}`. Expected `untrusted`, `on-request`, `never`, or a granular policy."
        ),
        Error::Reviewer(value) => {
            format!("Unknown permission reviewer `{value}`. Expected `user` or `auto_review`.")
        }
        Error::Args(ArgError::Quoting) => "Invalid command arguments. Check the quoting.".to_string(),
        Error::Args(ArgError::NeedsValue(name)) => {
            format!("Command setting `{name}` requires a value.")
        }
        Error::Args(ArgError::TakesNoValue(name)) => {
            format!("Command setting `{name}` does not take a value.")
        }
        Error::Args(ArgError::OnlyOne(name)) => format!("Specify only one of --{name} or --no-{name}."),
        Error::Args(ArgError::Repeated(name)) => {
            format!("Command setting `{name}` was provided more than once.")
        }
        Error::Args(ArgError::Unknown(name)) => format!("Unknown command setting `{name}`."),
    }
}

/// `try_select_permission_profile`'s error lines.
fn permission_refusal(
    picker: &crate::sheets::select::strings::Strings,
    refusal: &codex_tui_core::screens::select::permission_args::Refusal,
) -> String {
    use codex_tui_core::screens::select::permission_args::Refusal;
    use codex_tui_core::screens::select::permissions::Disabled;
    match refusal {
        Refusal::ApproveUnavailable => {
            "Approve for me is unavailable. Run `/permissions` to see available profiles."
                .to_string()
        }
        Refusal::Unknown(id) => format!(
            "Unknown permission profile `{id}`. Run `/permissions` to see available profiles."
        ),
        Refusal::Unavailable { profile, why } => {
            let reason = match why {
                Disabled::NotOnServer => picker.not_on_server.to_string(),
                Disabled::Requirements => picker.disabled_by_requirements.to_string(),
                Disabled::Reason(text) => text.clone(),
                Disabled::Idle => String::new(),
            };
            format!("Permission profile `{profile}` is unavailable: {reason}.")
        }
    }
}

/// `QuickDial::validate`'s reasons.
fn preset_reason(reason: codex_tui_core::screens::select::quick_dials::Invalid) -> &'static str {
    use codex_tui_core::screens::select::quick_dials::Invalid;
    match reason {
        Invalid::Empty => "Preset input cannot be empty.",
        Invalid::Shell => "Presets support prompts and slash commands, not shell commands.",
        Invalid::TooLong => "Preset input cannot exceed 8,000 UTF-8 bytes.",
    }
}

/// The oracle's words of the `/goal` notes (`goal_display.rs`, `goal_menu.rs`,
/// `thread_goal_actions.rs`), until the catalog has them.
fn mode_note(note: &codex_tui_core::threads::ModeNote) -> String {
    use codex_tui_core::threads::ModeNote;
    match note {
        ModeNote::Disabled => {
            "Collaboration modes are disabled.\nEnable collaboration modes to use /plan.".to_owned()
        }
        ModeNote::PlanUnavailable => "Plan mode unavailable right now.".to_owned(),
        ModeNote::ModelChanged(change) => {
            let mode = match change.mode {
                codex_protocol::config_types::ModeKind::Plan => "Plan",
                codex_protocol::config_types::ModeKind::Default => "Default",
            };
            match &change.effort {
                Some(effort) => format!(
                    "Model changed to {} {effort} for {mode} mode.",
                    change.model
                ),
                None => format!("Model changed to {} for {mode} mode.", change.model),
            }
        }
        ModeNote::AutomaticSwitch(switch) => {
            use codex_tui_core::screens::chat::SwitchReason;
            let (prefix, suffix) = match switch.reason {
                SwitchReason::UsageLimit => ("Automatically switched to", " due to usage limits."),
                SwitchReason::UsageRecovered => (
                    "Automatically switched back to",
                    " because ordinary usage is available again.",
                ),
            };
            let name = if switch.model.eq_ignore_ascii_case("gpt-reserve") {
                "Luna Reserve"
            } else {
                switch.model.as_str()
            };
            match &switch.effort {
                Some(effort) => format!("{prefix} {name} {effort}{suffix}"),
                None => format!("{prefix} {name}{suffix}"),
            }
        }
    }
}
