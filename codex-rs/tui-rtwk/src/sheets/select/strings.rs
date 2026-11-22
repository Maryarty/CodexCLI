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

use codex_tui_core::screens::select::models::Effort;

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
    /// chatwidget/permissions.rs: the `/permissions` popup.
    pub permissions_title: &'static str,
    pub permissions_hint: &'static str,
    pub access_ask: &'static str,
    pub access_ask_description: &'static str,
    pub access_full: &'static str,
    pub access_full_description: &'static str,
    pub num: NumSyms,
    // chatwidget/model_popups.rs: `open_model_popup_with_presets` and
    // `open_all_models_popup_with_view_id`.
    pub quick_title: &'static str,
    pub quick_subtitle: &'static str,
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
    permissions_hint: "enter select · esc back",
    access_ask: "Ask for approval",
    access_ask_description: "Read and edit workspace files and run commands, with approval required for internet access or edits outside the workspace",
    access_full: "Full Access",
    access_full_description: "Use with caution: Codex can edit files outside this workspace and access the internet without approval",
    num: NumSyms::ENGLISH,
    quick_title: "Select Model",
    quick_subtitle: "Pick a quick auto mode or browse all models.",
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
