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
    /// `slash_dispatch.rs`: the messages a command line can end in.
    pub unrecognized: Msg<1>,
    pub disabled_during_task: Msg<1>,
    pub unavailable_in_side: Msg<1>,
    pub unavailable_in_review: Msg<1>,
    pub session_starting: Msg<1>,
    pub model_startup: &'static str,
    /// `USER_SHELL_COMMAND_HELP_TITLE` and `_HINT` (`chatwidget.rs`): a bare `!`.
    pub shell_help: &'static str,
    /// `user_input_too_large_message` (`chat_composer.rs`): the limit and the size.
    pub input_too_large: Msg<2>,
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
    /// `add_app_server_stub_message`: the feature, then the message.
    pub stub: Msg<1>,
    pub stub_memory_maintenance: &'static str,
    /// `PARENT_OWNED_INPUT_MESSAGE`.
    pub parent_owned: &'static str,
    pub mode_busy: &'static str,
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
    /// The words of the picker sheets (the effort and mode names the notes
    /// repeat).
    pub picker: &'static crate::sheets::select::strings::Strings,
    // The notes of `/model`, `/permissions` and `/apps`
    // (`model_defaults.rs`, `config_persistence.rs`, `connectors.rs`).
    pub note_no_models: &'static str,
    pub note_models_not_loaded: &'static str,
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
    unrecognized: msg!("Unrecognized command '/{0}'. Type \"/\" for a list of supported commands."),
    disabled_during_task: msg!("'/{0}' is disabled while a task is in progress."),
    unavailable_in_side: msg!(
        "'/{0}' is unavailable in side conversations. Press Ctrl+C to return to the main thread first."
    ),
    unavailable_in_review: msg!("'/{0}' is unavailable while code review is running."),
    session_starting: msg!("Session is still starting; try /{0} again in a moment."),
    model_startup: "Model selection is disabled until startup completes.",
    shell_help: "Prefix a command with ! to run it locally\nExample: !ls",
    input_too_large: msg!("Message exceeds the maximum length of {0} characters ({1} provided)."),
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
    stub: msg!("{0}: Not available in TUI yet."),
    stub_memory_maintenance: "Memory maintenance",
    parent_owned: "This sub-agent is controlled by its parent. Direct input is disabled.",
    mode_busy: "Cannot switch collaboration mode while a turn is running.",
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
    picker: &crate::sheets::select::strings::EN,
    note_no_models: "No additional models are available right now.",
    note_models_not_loaded: "Could not load models. Try /model again.",
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
    pub fn local_warning(&self, warning: LocalWarning) -> &'static str {
        match warning {
            LocalWarning::ReviewSteerUnavailable => self.review_steer_unavailable,
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
        use codex_tui_core::threads::{CdRefusal, NoteKey, RemoveNote, Stub};
        let text = |msg: Msg<1>, arg: &str| Fmt::new(msg, [Arg::Text(arg)], &self.num).to_string();
        match key {
            NoteKey::Command(notice) => self.notice(notice),
            NoteKey::WorkingDirectory(dir) => text(self.working_directory, dir),
            NoteKey::Rollout(Some(path)) => text(self.rollout, path),
            NoteKey::Rollout(None) => self.rollout_missing.to_string(),
            NoteKey::DesktopOpened => self.desktop_opened.to_string(),
            NoteKey::StoppingTerminals => self.stopping_terminals.to_string(),
            NoteKey::Stub(Stub::MemoryMaintenance) => text(self.stub, self.stub_memory_maintenance),
            NoteKey::RawOutput(true) => self.raw_output_on.to_string(),
            NoteKey::RawOutput(false) => self.raw_output_off.to_string(),
            NoteKey::ParentOwned => self.parent_owned.to_string(),
            NoteKey::ModeBusy => self.mode_busy.to_string(),
            NoteKey::CdRefused(CdRefusal::NotStarted) => self.cd_not_started.to_string(),
            NoteKey::CdRefused(CdRefusal::NotIdle) => self.cd_not_idle.to_string(),
            NoteKey::CdRefused(CdRefusal::HistoryNotSaved) => self.cd_history_not_saved.to_string(),
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
        }
    }

    /// The words of what a picker said (`select::Note`).
    fn picker_note(&self, note: &codex_tui_core::screens::select::Note) -> String {
        use codex_tui_core::screens::select::permissions::{Label, Preset, Reviewer};
        use codex_tui_core::screens::select::{Note, Setting};
        let text = |msg: Msg<1>, arg: &str| Fmt::new(msg, [Arg::Text(arg)], &self.num).to_string();
        let int = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        let picker = self.picker;
        match note {
            Note::NoAdditionalModels => self.note_no_models.to_string(),
            Note::ModelsNotLoaded => self.note_models_not_loaded.to_string(),
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
            Note::SaveFailed { setting, error } => text(
                match setting {
                    Setting::Model => self.note_save_model_failed,
                    Setting::PlanEffort => self.note_save_plan_failed,
                    Setting::Permissions => self.note_save_profile_failed,
                    Setting::ServiceTier => self.note_save_tier_failed,
                },
                error,
            ),
            Note::Overridden(setting) => text(
                self.note_overridden,
                match setting {
                    Setting::Model => self.note_setting_model,
                    Setting::PlanEffort => self.note_setting_plan,
                    Setting::Permissions => self.note_setting_permissions,
                    Setting::ServiceTier => self.note_setting_tier,
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
