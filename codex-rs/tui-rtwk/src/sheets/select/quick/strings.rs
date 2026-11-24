//! The words of `/presets` (oracle `chatwidget/quick_dials.rs`, `bottom_pane/custom_prompt_view`):
//! a table of `&'static str` fields per locale, read by reference through `of(cx)`. `EN` is the
//! oracle's text byte for byte. A language is one more `const` here, registered with
//! `with_ru(&RU)` on `TABLES`.

use rtwk::l10n::Arg;
use rtwk::l10n::DependCx;
use rtwk::l10n::Fmt;
use rtwk::l10n::Msg;
use rtwk::l10n::NumSyms;
use rtwk::l10n::msg;

use codex_tui_core::screens::select::quick_dials::{Action, Invalid, Source, Unavailable};

use crate::l10n::Tables;
use crate::l10n::lang;

/// The words of the slot pickers, the sources, the editor and the confirmation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Strings {
    // `show_quick_dials_picker`.
    pub save_title: &'static str,
    pub edit_title: &'static str,
    pub clear_title: &'static str,
    /// What an empty slot says in the place of its name.
    pub empty_slot: &'static str,
    // `open_quick_dial_prefills`.
    pub sources_title: &'static str,
    pub source_model: &'static str,
    pub source_permissions: &'static str,
    pub source_title: &'static str,
    pub source_status: &'static str,
    // `open_quick_dial_editor`.
    pub new_title: &'static str,
    pub edit_preset: Msg<1>,
    pub placeholder: &'static str,
    pub hint_slot: &'static str,
    pub hint_review: &'static str,
    /// The mark before the input (`›`).
    pub prompt_mark: &'static str,
    // `confirm_quick_dial`.
    pub replace: &'static str,
    pub delete: &'static str,
    pub save: &'static str,
    /// `{Replace|Delete|Save} preset {slot}?` over the question, the same without `?` on the row.
    pub confirm_title: Msg<2>,
    pub confirm_row: Msg<2>,
    /// Before the new input of a replacement.
    pub arrow: &'static str,
    // Why a source cannot be saved.
    pub model_unavailable: &'static str,
    pub effort_unavailable: &'static str,
    pub speed_unavailable: &'static str,
    pub no_profile: &'static str,
    pub approval_unavailable: &'static str,
    pub profile_name_unavailable: &'static str,
    pub title_items_unavailable: &'static str,
    pub status_items_unavailable: &'static str,
    // `quick_dial_hud`: the shortcut menu over the composer.
    pub hud_title: &'static str,
    pub hud_empty: &'static str,
    /// Between a key and what it does (` \u{b7} `).
    pub hud_separator: &'static str,
    /// The keys of the menu and what they do.
    pub hud_commands: [(&'static str, &'static str); 5],
    // `QuickDial::validate`.
    pub invalid_empty: &'static str,
    pub invalid_shell: &'static str,
    pub invalid_too_long: &'static str,
    pub num: NumSyms,
}

impl Strings {
    pub fn source(&self, source: Source) -> &'static str {
        match source {
            Source::Model => self.source_model,
            Source::Permissions => self.source_permissions,
            Source::TerminalTitle => self.source_title,
            Source::StatusLine => self.source_status,
        }
    }

    /// `Edit preset {slot}`.
    pub fn edit_preset(&self, slot: u8) -> String {
        Fmt::new(self.edit_preset, [Arg::Text(&slot.to_string())], &self.num).to_string()
    }

    fn action(&self, action: Action) -> &'static str {
        match action {
            Action::Replace => self.replace,
            Action::Delete => self.delete,
            Action::Save => self.save,
        }
    }

    /// `{action} preset {slot}?`.
    pub fn confirm_title(&self, action: Action, slot: u8) -> String {
        let args = [Arg::Text(self.action(action)), Arg::Text(&slot.to_string())];
        Fmt::new(self.confirm_title, args, &self.num).to_string()
    }

    /// `{action} preset {slot}`.
    pub fn confirm_row(&self, action: Action, slot: u8) -> String {
        let args = [Arg::Text(self.action(action)), Arg::Text(&slot.to_string())];
        Fmt::new(self.confirm_row, args, &self.num).to_string()
    }

    /// The reason a source row is disabled with.
    pub fn unavailable(&self, why: Unavailable) -> &'static str {
        match why {
            Unavailable::Model => self.model_unavailable,
            Unavailable::Effort => self.effort_unavailable,
            Unavailable::Speed => self.speed_unavailable,
            Unavailable::NoProfile => self.no_profile,
            Unavailable::ApprovalPolicy => self.approval_unavailable,
            Unavailable::ProfileName => self.profile_name_unavailable,
            Unavailable::TitleItems => self.title_items_unavailable,
            Unavailable::StatusItems => self.status_items_unavailable,
            Unavailable::Invalid(why) => self.invalid(why),
        }
    }

    pub fn invalid(&self, why: Invalid) -> &'static str {
        match why {
            Invalid::Empty => self.invalid_empty,
            Invalid::Shell => self.invalid_shell,
            Invalid::TooLong => self.invalid_too_long,
        }
    }
}

/// English: the oracle's text.
pub const EN: Strings = Strings {
    save_title: "Save preset to slot",
    edit_title: "Edit a preset",
    clear_title: "Delete a preset",
    empty_slot: "empty",
    sources_title: "Start preset from current settings",
    source_model: "Current model",
    source_permissions: "Current permissions",
    source_title: "Current terminal title",
    source_status: "Current status line",
    new_title: "New preset",
    edit_preset: msg!("Edit preset {0}"),
    placeholder: "Write a prompt or slash command",
    hint_slot: "enter choose slot \u{b7} esc back",
    hint_review: "enter review changes \u{b7} esc back",
    prompt_mark: "\u{203a}",
    replace: "Replace",
    delete: "Delete",
    save: "Save",
    confirm_title: msg!("{0} preset {1}?"),
    confirm_row: msg!("{0} preset {1}"),
    arrow: "\u{2192} ",
    model_unavailable: "The current model cannot be recalled with /model.",
    effort_unavailable: "Choose an available reasoning effort with /model before saving.",
    speed_unavailable: "Choose an available speed with /model before saving.",
    no_profile: "Select a named permission profile with /permissions first.",
    approval_unavailable: "The approval policy cannot be recalled.",
    profile_name_unavailable: "The permission profile name cannot be recalled.",
    title_items_unavailable: "Choose available terminal-title items with /title before saving.",
    status_items_unavailable: "Choose available status-line items with /statusline before saving.",
    hud_title: "Presets",
    hud_empty: "No presets saved. Press n to create one.",
    hud_separator: " \u{b7} ",
    hud_commands: [
        ("n", "new"),
        ("s", "from settings"),
        ("e", "edit"),
        ("d", "delete"),
        ("esc", "close"),
    ],
    invalid_empty: "Preset input cannot be empty.",
    invalid_shell: "Presets support prompts and slash commands, not shell commands.",
    invalid_too_long: "Preset input cannot exceed 8,000 UTF-8 bytes.",
    num: NumSyms::ENGLISH,
};

static TABLES: Tables<Strings> = Tables::english(&EN);

/// The table of the locale below `cx`.
pub fn of(cx: &mut impl DependCx) -> &'static Strings {
    TABLES.pick(lang(cx))
}
