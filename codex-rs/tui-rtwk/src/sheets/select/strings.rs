//! The strings of `select` (S14): a table of `&'static str` fields per locale,
//! read by reference through `of(cx)`. `EN` is the oracle's text byte for
//! byte; each group names the oracle file it comes from. A language is one
//! more `const` here, registered with `with_ru(&RU)` on `TABLES`; a screen
//! never changes for it. The words every picker adds to its rows are the
//! picker's (`widgets/picker/strings.rs`). Nothing else in this directory
//! holds user-visible text (the rules test
//! `adr010_no_user_visible_literal_outside_the_string_tables`).

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use codex_tui_core::screens::select::apps::Status;
use codex_tui_core::screens::select::models::Effort;
use codex_tui_core::screens::select::permissions::Failure;

use crate::l10n::Lang;
use crate::l10n::Tables;
use crate::l10n::lang;

/// The strings of this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    /// The name of this screen in the debug menu and on its slot dialog.
    pub title: &'static str,
    /// bottom_pane/list_selection_view.rs (model picker test).
    pub model_title: &'static str,
    // chatwidget/permissions_menu.rs, permission_popups.rs and
    // permission_discovery.rs: the `/permissions` menus.
    pub permissions_title: &'static str,
    pub permissions_no_profiles: &'static str,
    pub permissions_loading: &'static str,
    pub permissions_retry: &'static str,
    pub permissions_failed: Msg<1>,
    // chatwidget/connectors.rs: the `/apps` directory.
    pub apps_title: &'static str,
    pub apps_loading_subtitle: &'static str,
    pub apps_loading: &'static str,
    pub apps_loading_description: &'static str,
    pub apps_failed_subtitle: &'static str,
    pub apps_unavailable: &'static str,
    pub apps_unavailable_description: &'static str,
    pub apps_retry: &'static str,
    pub apps_retry_description: &'static str,
    pub apps_mention_hint: &'static str,
    pub apps_counts: Msg<2>,
    pub apps_search: &'static str,
    pub apps_installed: &'static str,
    /// `/presets`: the same, its overlay is not built.
    pub presets_action: &'static str,
    pub apps_disabled: &'static str,
    pub apps_installable: &'static str,
    pub apps_brief: Msg<2>,
    pub apps_open_installed: Msg<1>,
    pub apps_open_install: Msg<1>,
    pub apps_no_link: Msg<1>,
    /// chatwidget/slash_dispatch.rs: the confirmations of `/archive`, `/delete`.
    pub archive_title: &'static str,
    pub archive_subtitle: &'static str,
    pub archive_subtitle_running: &'static str,
    pub archive_no: &'static str,
    pub archive_no_description: &'static str,
    pub archive_yes: &'static str,
    pub archive_yes_description: &'static str,
    pub delete_title: &'static str,
    pub delete_subtitle: &'static str,
    pub delete_no: &'static str,
    pub delete_no_description: &'static str,
    pub delete_yes: &'static str,
    pub delete_yes_description: &'static str,
    pub failure_duplicate: &'static str,
    pub failure_cloud: &'static str,
    pub failure_unsupported: &'static str,
    pub failure_pagination: &'static str,
    pub failure_timeout: &'static str,
    pub mode_ask: &'static str,
    pub mode_ask_description: &'static str,
    /// `Ask for approval` while the non-admin Windows sandbox is on, and the note that points at
    /// the admin setup (`windows_degraded_sandbox_enabled`, `show_elevate_sandbox_hint`).
    pub mode_ask_non_admin: &'static str,
    pub elevate_hint: &'static str,
    pub mode_approve: &'static str,
    pub mode_approve_description: &'static str,
    pub mode_full: &'static str,
    pub mode_full_description: &'static str,
    pub mode_read: &'static str,
    pub mode_read_description: &'static str,
    pub profile_configured: &'static str,
    pub profile_unlisted: &'static str,
    pub not_on_server: &'static str,
    pub disabled_by_requirements: &'static str,
    // `open_full_access_confirmation`.
    pub full_title: &'static str,
    pub full_warning: &'static str,
    pub full_caution: &'static str,
    pub cyber_risk: &'static str,
    pub cyber_safeguards: &'static str,
    pub cyber_recommend_auto: &'static str,
    pub cyber_recommend_ask: &'static str,
    pub full_continue: &'static str,
    pub full_continue_description: &'static str,
    pub full_cancel: &'static str,
    pub full_cancel_description: &'static str,
    pub num: NumSyms,
    // chatwidget/model_popups.rs: `open_model_popup_with_presets` and
    // `open_all_models_popup_with_view_id`.
    pub quick_title: &'static str,
    pub quick_subtitle: &'static str,
    pub reserve_subtitle: &'static str,
    pub loading: &'static str,
    pub all_models: &'static str,
    pub all_models_description: Msg<1>,
    // `open_reasoning_popup`, `open_advanced_reasoning_popup`.
    pub reasoning_title: Msg<1>,
    pub default_mark: &'static str,
    pub more_reasoning: &'static str,
    pub more_description: Msg<2>,
    pub more_join: &'static str,
    pub more_verb_one: &'static str,
    pub more_verb_many: &'static str,
    pub rate_limit_warning: Msg<1>,
    pub advanced_title: &'static str,
    pub advanced_warning: &'static str,
    pub max_description: &'static str,
    pub ultra_description: &'static str,
    // `reasoning_effort_label`.
    pub effort_none: &'static str,
    pub effort_minimal: &'static str,
    pub effort_low: &'static str,
    pub effort_medium: &'static str,
    pub effort_high: &'static str,
    pub effort_xhigh: &'static str,
    pub effort_max: &'static str,
    pub effort_ultra: &'static str,
    pub effort_persistent: &'static str,
    // `open_plan_reasoning_scope_prompt`.
    pub scope_title: &'static str,
    pub scope_subtitle: Msg<1>,
    pub scope_phrase: Msg<1>,
    pub scope_no_reasoning: &'static str,
    pub scope_selected_reasoning: &'static str,
    pub scope_plan_only: &'static str,
    pub scope_all_modes: &'static str,
    pub scope_plan_only_description: Msg<1>,
    pub scope_all_modes_description: Msg<1>,
    pub source_override: Msg<1>,
    pub source_default: Msg<1>,
    pub source_default_none: &'static str,
    pub source_unknown: &'static str,
    // `session_model_selection_action`: the words of the footer's keys.
    pub word_default: &'static str,
    pub word_apply: &'static str,
    pub word_session: &'static str,
}

impl Strings {
    /// `reasoning_effort_label`.
    pub fn effort(&self, effort: &Effort) -> String {
        match effort {
            Effort::None => self.effort_none,
            Effort::Minimal => self.effort_minimal,
            Effort::Low => self.effort_low,
            Effort::Medium => self.effort_medium,
            Effort::High => self.effort_high,
            Effort::XHigh => self.effort_xhigh,
            Effort::Max => self.effort_max,
            Effort::Ultra => self.effort_ultra,
            Effort::Persistent => self.effort_persistent,
            Effort::Custom(value) => return value.clone(),
        }
        .to_string()
    }

    /// `reasoning_effort_sentence_label`: the label in a sentence.
    pub fn effort_sentence(&self, effort: &Effort) -> String {
        match effort {
            Effort::Custom(value) => value.clone(),
            effort => self.effort(effort).to_lowercase(),
        }
    }

    /// `the selected reasoning`, `no reasoning` or `high reasoning`.
    pub fn reasoning_phrase(&self, effort: Option<&Effort>) -> String {
        match effort {
            Some(Effort::None) => self.scope_no_reasoning.to_string(),
            Some(effort) => {
                let label = self.effort_sentence(effort);
                Fmt::new(self.scope_phrase, [Arg::Text(&label)], &self.num).to_string()
            }
            None => self.scope_selected_reasoning.to_string(),
        }
    }

    /// The subtitle of the menu that could not read the catalog
    /// (`discovery_error` and the errors of `request_permission_profiles`).
    pub fn failure(&self, failure: &Failure) -> String {
        match failure {
            Failure::Duplicate => self.failure_duplicate.to_string(),
            Failure::Cloud => self.failure_cloud.to_string(),
            Failure::Unsupported => self.failure_unsupported.to_string(),
            Failure::Pagination => self.failure_pagination.to_string(),
            Failure::Timeout => self.failure_timeout.to_string(),
            Failure::Other(error) => {
                Fmt::new(self.permissions_failed, [Arg::Text(error)], &self.num).to_string()
            }
        }
    }

    pub fn app_status(&self, status: Status) -> &'static str {
        match status {
            Status::Installed => self.apps_installed,
            Status::Disabled => self.apps_disabled,
            Status::Installable => self.apps_installable,
        }
    }

    pub fn apps_counts(&self, installed: usize, total: usize) -> String {
        let count = |n: usize| Arg::Int(u64::try_from(n).unwrap_or(u64::MAX));
        Fmt::new(
            self.apps_counts,
            [count(installed), count(total)],
            &self.num,
        )
        .to_string()
    }

    /// `{status} · {description}` (`connector_brief_description`).
    pub fn app_brief(&self, status: Status, description: &str) -> String {
        Fmt::new(
            self.apps_brief,
            [Arg::Text(self.app_status(status)), Arg::Text(description)],
            &self.num,
        )
        .to_string()
    }

    /// The text of a message of one argument, the app's status label.
    pub fn app_status_line(&self, msg: Msg<1>, status: Status) -> String {
        Fmt::new(msg, [Arg::Text(self.app_status(status))], &self.num).to_string()
    }

    pub fn all_models_description(&self, current: &str) -> String {
        Fmt::new(self.all_models_description, [Arg::Text(current)], &self.num).to_string()
    }

    pub fn reasoning_title(&self, model: &str) -> String {
        Fmt::new(self.reasoning_title, [Arg::Text(model)], &self.num).to_string()
    }

    pub fn more_description(&self, efforts: &[Effort]) -> String {
        let label = efforts
            .iter()
            .map(|effort| self.effort(effort))
            .collect::<Vec<_>>()
            .join(self.more_join);
        let verb = if efforts.len() == 1 {
            self.more_verb_one
        } else {
            self.more_verb_many
        };
        Fmt::new(
            self.more_description,
            [Arg::Text(&label), Arg::Text(verb)],
            &self.num,
        )
        .to_string()
    }

    pub fn rate_limit_warning(&self, effort: &Effort) -> String {
        let label = self.effort(effort);
        Fmt::new(self.rate_limit_warning, [Arg::Text(&label)], &self.num).to_string()
    }

    pub fn scope_subtitle(&self, phrase: &str) -> String {
        Fmt::new(self.scope_subtitle, [Arg::Text(phrase)], &self.num).to_string()
    }

    pub fn scope_plan_only_description(&self, phrase: &str) -> String {
        Fmt::new(
            self.scope_plan_only_description,
            [Arg::Text(phrase)],
            &self.num,
        )
        .to_string()
    }

    pub fn scope_all_modes_description(&self, source: &str) -> String {
        Fmt::new(
            self.scope_all_modes_description,
            [Arg::Text(source)],
            &self.num,
        )
        .to_string()
    }

    pub fn source_override(&self, effort: &str) -> String {
        Fmt::new(self.source_override, [Arg::Text(effort)], &self.num).to_string()
    }

    pub fn source_default(&self, effort: &str) -> String {
        Fmt::new(self.source_default, [Arg::Text(effort)], &self.num).to_string()
    }
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    title: "generic selection dialog",
    model_title: "Select Model and Effort",
    permissions_title: "Update Model Permissions",
    permissions_no_profiles: "No permission profiles returned by the server.",
    permissions_loading: "Loading permission profiles…",
    permissions_retry: "Retry",
    permissions_failed: msg!("Failed to load permissions: {0}"),
    apps_title: "Apps",
    apps_loading_subtitle: "Loading installed and available apps...",
    apps_loading: "Loading apps...",
    apps_loading_description: "This updates when the full list is ready",
    apps_failed_subtitle: "Failed to load apps.",
    apps_unavailable: "App directory unavailable",
    apps_unavailable_description: "Retry the failed app directory request, or press Esc to continue",
    apps_retry: "Retry",
    apps_retry_description: "Reload installed and available apps",
    apps_mention_hint: "Use $ to insert an installed app into your prompt.",
    apps_counts: msg!("Installed {0} of {1} available apps."),
    apps_search: "Type to search apps",
    apps_installed: "Installed",
    presets_action: "presets menu",
    apps_disabled: "Installed · Disabled",
    apps_installable: "Can be installed",
    apps_brief: msg!("{0} · {1}"),
    apps_open_installed: msg!(
        "{0} · Press Enter to open the app page to install, manage, or enable/disable this app"
    ),
    apps_open_install: msg!("{0} · Press Enter to open the app page to install this app"),
    apps_no_link: msg!("{0} · App link unavailable"),
    archive_title: "Archive this session?",
    archive_subtitle: "Are you sure? This will archive the current session",
    archive_subtitle_running: "This will stop the current turn and archive the session.",
    archive_no: "No, don't archive",
    archive_no_description: "Return to the current session",
    archive_yes: "Yes, archive",
    archive_yes_description: "Archive this session now",
    delete_title: "Delete this session?",
    delete_subtitle: "Also deletes subagents and saved side chats. Cannot be undone.",
    delete_no: "No, keep this session",
    delete_no_description: "Return to the current session",
    delete_yes: "Yes, delete and return to command center",
    delete_yes_description: "Permanently delete this session now",
    failure_duplicate: "The server returned duplicate permission profiles.",
    failure_cloud: "Cloud conversations use server-selected permissions.",
    failure_unsupported: "This server does not support permission discovery. Upgrade the Codex server to use this menu.",
    failure_pagination: "Permission discovery exceeded its pagination limit. Try /permissions again.",
    failure_timeout: "Permission discovery timed out. Try /permissions again.",
    mode_ask: "Ask for approval",
    mode_ask_description: "Read and edit workspace files and run commands, with approval required for internet access or edits outside the workspace",
    mode_ask_non_admin: "Ask for approval (non-admin sandbox)",
    elevate_hint: "The non-admin sandbox protects your files and prevents network access under most circumstances. However, it carries greater risk if prompt injected. To upgrade to the default sandbox, run /setup-default-sandbox.",
    mode_approve: "Approve for me",
    mode_approve_description: "Only ask for actions detected as potentially unsafe",
    mode_full: "Full Access",
    mode_full_description: "Use with caution: Codex can edit files outside this workspace and access the internet without approval",
    mode_read: "Read Only",
    mode_read_description: "Read workspace files, with approval required for edits or internet access",
    profile_configured: "Configured permission profile",
    profile_unlisted: "Current permission profile",
    not_on_server: "Not available on this server",
    disabled_by_requirements: "Disabled by requirements",
    full_title: "Enable full access?",
    full_warning: "When Codex runs with full access, it can edit any file on your computer and run commands with network, without your approval. ",
    full_caution: "Exercise caution when enabling full access. This significantly increases the risk of data loss, leaks, or unexpected behavior.",
    cyber_risk: "Cyber models carry a higher risk of dangerous actions.",
    cyber_safeguards: " Ensure proper safeguards are in place before granting full access. ",
    cyber_recommend_auto: "We strongly recommend selecting \"Approve for me\" instead, and customizing the reviewer policy for your use case.",
    cyber_recommend_ask: "We strongly recommend selecting \"Ask for approval\" instead.",
    full_continue: "Yes, continue anyway",
    full_continue_description: "Apply full access for this session",
    full_cancel: "Cancel",
    full_cancel_description: "Go back without enabling full access",
    num: NumSyms::ENGLISH,
    quick_title: "Select Model",
    quick_subtitle: "Pick a quick auto mode or browse all models.",
    reserve_subtitle: "Other models return when ordinary usage is available again.",
    loading: "Loading models…",
    all_models: "All models",
    all_models_description: msg!("Choose a specific model and reasoning level (current: {0})"),
    reasoning_title: msg!("Select Reasoning Level for {0}"),
    default_mark: " (default)",
    more_reasoning: "More reasoning…",
    more_description: msg!("{0} {1} usage limits faster"),
    more_join: " and ",
    more_verb_one: "consumes",
    more_verb_many: "consume",
    rate_limit_warning: msg!("⚠ {0} reasoning effort can quickly consume Plus plan rate limits"),
    advanced_title: "Advanced Reasoning",
    advanced_warning: "⚠ Consumes usage limits faster",
    max_description: "For difficult problems when quality matters more than speed · higher usage",
    ultra_description: "For demanding work using multiple agents · highest usage",
    effort_none: "None",
    effort_minimal: "Minimal",
    effort_low: "Low",
    effort_medium: "Medium",
    effort_high: "High",
    effort_xhigh: "Extra high",
    effort_max: "Max",
    effort_ultra: "Ultra",
    effort_persistent: "Persistent",
    scope_title: "Apply reasoning change",
    scope_subtitle: msg!("Choose where to apply {0}."),
    scope_phrase: msg!("{0} reasoning"),
    scope_no_reasoning: "no reasoning",
    scope_selected_reasoning: "the selected reasoning",
    scope_plan_only: "Apply to Plan mode override",
    scope_all_modes: "Apply to global default and Plan mode override",
    scope_plan_only_description: msg!("Always use {0} in Plan mode"),
    scope_all_modes_description: msg!(
        "Set the global default reasoning level and Plan mode override, replacing the current {0}"
    ),
    source_override: msg!("user-chosen Plan override ({0})"),
    source_default: msg!("built-in Plan default ({0})"),
    source_default_none: "built-in Plan default (no reasoning)",
    source_unknown: "built-in Plan default",
    word_default: "default",
    word_apply: "apply",
    word_session: "session",
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
