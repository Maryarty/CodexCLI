//! `settings`: the view half of S15 (settings dialogs): modal sheets over the
//! session page, each drawn without a frame at the bottom edge, as the
//! oracle draws its bottom-pane views (architecture §1, amendment
//! 2026-11-19b). `Screen` is the one widget the app opens, by the kind of
//! dialog a slash command names (`open`, `Args`); the feature builds the
//! dialog's catalogue from its facts (`Intent::Open`) and the sheet draws
//! it when it is there. What each dialog is before it is saved (its checks,
//! its order, its page) is its sheet's draft (`draft.rs`); the feature keeps
//! what it is offered and what it asked of the app (`Edit`, ADR-013). The
//! feature is `codex_tui_core::features::settings`.

use rtwk::overlay::{Dialog, DialogOptions, show_dialog};
use rtwk::provider::select;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::commands::SlashCommand;
use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::{Edit, ID, Intent, KIND, Settings, Which};

use super::FeatureView;
use crate::app::debug::seeded::Seeded;
use crate::app::debug::{DebugEntry, DebugOpen, Dir, EntryOpen};
use crate::features::verb;
use crate::pages::session::strings::Strings as SessionStrings;

pub mod draft;
pub mod features;
pub mod hooks;
pub mod items;
mod keys;
pub mod memories;
pub mod setup;
pub mod strings;
pub mod theme;

impl FeatureView for Settings {
    fn note_words(
        note: &codex_tui_core::features::settings::Note,
        _words: &SessionStrings,
    ) -> String {
        strings::note(note)
    }

    /// The slot of the dialogs in the debug menu: each dialog on a state worth opening.
    fn debug_dir() -> Option<Dir> {
        Some(Dir {
            name: <Settings as codex_tui_core::features::Feature>::NAME,
            id: ID,
            kind: KIND,
            title: strings::title,
            debug_entries,
            open_empty,
        })
    }
}

/// What opens a settings dialog: its kind, as the slash command names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Args {
    pub which: Which,
}

impl Args {
    /// The dialog `command` opens (`/theme`, `/memories`, `/statusline`, `/title`, `/skills`,
    /// `/config`, `/experimental`, `/keymap`; `/tui` is the configuration panel).
    pub fn of(command: SlashCommand) -> Args {
        Args {
            which: match command {
                SlashCommand::Theme => Which::Theme,
                SlashCommand::Memories => Which::Memories,
                SlashCommand::Statusline => Which::StatusLine,
                SlashCommand::Title => Which::Title,
                SlashCommand::Skills => Which::Skills,
                SlashCommand::Experimental => Which::Experimental,
                SlashCommand::Keymap => Which::Keymap,
                _ => Which::Config,
            },
        }
    }
}

/// What the debug menu lists for this directory: each dialog on a state
/// worth opening, made by a fixture script.
pub fn debug_entries() -> Vec<DebugEntry> {
    let entries = [
        (
            "status_line_preview",
            "the status line setup with three items and runtime values",
            include_str!("fixtures/status_line.jsonl"),
        ),
        (
            "status_line_thread_usage",
            "the status line setup with the thread credits and cost",
            include_str!("fixtures/status_line_thread_usage.jsonl"),
        ),
        (
            "status_line_hostname",
            "the status line setup with the hostname",
            include_str!("fixtures/status_line_hostname.jsonl"),
        ),
        (
            "terminal_title",
            "the terminal title setup",
            include_str!("fixtures/title.jsonl"),
        ),
        (
            "theme_picker",
            "the syntax theme picker over six themes, the codex home under ~",
            include_str!("fixtures/theme_picker.jsonl"),
        ),
        (
            "skills_basic",
            "the skills toggle: two skills, the first on",
            include_str!("fixtures/skills_basic.jsonl"),
        ),
        (
            "skills_long_names",
            "the skills toggle with two long display names",
            include_str!("fixtures/skills_long_names.jsonl"),
        ),
        (
            "skills_empty",
            "the skills toggle with no skills",
            include_str!("fixtures/skills_empty.jsonl"),
        ),
        (
            "experimental_features_two",
            "the experimental features with two features, the first on",
            include_str!("fixtures/experimental_two.jsonl"),
        ),
        (
            "experimental_features_discovery",
            "the experimental features after the server's catalogue arrived: three on",
            include_str!("fixtures/experimental_discovery.jsonl"),
        ),
        (
            "experimental_features_save_failed",
            "the experimental features after a write that did not confirm",
            include_str!("fixtures/experimental_save_failed.jsonl"),
        ),
        (
            "experimental_features_empty",
            "the experimental features, the catalogue came empty",
            include_str!("fixtures/experimental_empty.jsonl"),
        ),
        (
            "experimental_features_unavailable",
            "the experimental features, the catalogue request failed",
            include_str!("fixtures/experimental_unavailable.jsonl"),
        ),
        (
            "experimental_features_changed_default",
            "the experimental features with one server feature",
            include_str!("fixtures/experimental_changed_default.jsonl"),
        ),
        (
            "memories_settings",
            "the memories dialog: use on, generate off",
            include_str!("fixtures/memories.jsonl"),
        ),
        (
            "memories_reset_confirmation",
            "the memories reset confirmation",
            include_str!("fixtures/memories_reset_confirmation.jsonl"),
        ),
        (
            "hooks_events",
            "the hooks browser: three hooks, the events page",
            include_str!("fixtures/hooks_events.jsonl"),
        ),
        (
            "hooks_events_review_column",
            "the hooks browser with one untrusted hook: the review column",
            include_str!("fixtures/hooks_events_review_column.jsonl"),
        ),
        (
            "hooks_events_issues",
            "the hooks browser with no hooks, a skipped matcher and a parse error",
            include_str!("fixtures/hooks_events_issues.jsonl"),
        ),
        (
            "hooks_status_messages",
            "the hooks browser: two status messages, one blank",
            include_str!("fixtures/hooks_status_messages.jsonl"),
        ),
        (
            "hooks_mcp_tool",
            "the hooks browser: one plugin hook with an MCP tool handler",
            include_str!("fixtures/hooks_mcp_tool.jsonl"),
        ),
        (
            "hooks_context_limit",
            "the hooks browser: one hook with an unlimited additional context",
            include_str!("fixtures/hooks_context_limit.jsonl"),
        ),
        (
            "hooks_untrusted_enabled",
            "the hooks browser: one untrusted, enabled, async hook",
            include_str!("fixtures/hooks_untrusted_enabled.jsonl"),
        ),
        (
            "hooks_trusted_and_review",
            "the hooks browser: a trusted hook and an untrusted one",
            include_str!("fixtures/hooks_trusted_and_review.jsonl"),
        ),
        (
            "hooks_two_managed",
            "the hooks browser: two system hooks of PreToolUse",
            include_str!("fixtures/hooks_two_managed.jsonl"),
        ),
        (
            "hooks_many",
            "the hooks browser: eleven hooks of PreToolUse, every second one named",
            include_str!("fixtures/hooks_many.jsonl"),
        ),
        (
            "hooks_long_command",
            "the hooks browser: one hook with a command of twenty words and a title",
            include_str!("fixtures/hooks_long_command.jsonl"),
        ),
        (
            "hooks_empty",
            "the hooks browser with no hooks at all",
            include_str!("fixtures/hooks_empty.jsonl"),
        ),
        (
            "hooks_review_needed",
            "the hooks browser: one untrusted, disabled hook",
            include_str!("fixtures/hooks_review_needed.jsonl"),
        ),
        (
            "hooks_managed_handler",
            "the hooks browser: one managed hook of PreToolUse",
            include_str!("fixtures/hooks_managed_handler.jsonl"),
        ),
        (
            "hooks_modified_handler",
            "the hooks browser: one modified, disabled hook",
            include_str!("fixtures/hooks_modified_handler.jsonl"),
        ),
        (
            "hooks_events_review_mixed",
            "the hooks browser: an untrusted, a modified and a trusted hook",
            include_str!("fixtures/hooks_events_review_mixed.jsonl"),
        ),
        (
            "hooks_wrapped_url",
            "the hooks browser: a command holding a URL longer than the details column",
            include_str!("fixtures/hooks_wrapped_url.jsonl"),
        ),
        (
            "theme_picker_all",
            "the syntax theme picker over the built-in themes and a custom one",
            include_str!("fixtures/theme_picker_all.jsonl"),
        ),
    ];
    entries
        .into_iter()
        .map(|(name, description, script)| DebugEntry {
            name,
            description,
            script,
            open: if name == "memories_reset_confirmation" {
                // The dialog opens on the confirmation: the draft's start.
                EntryOpen::Dialog(|seeded| debug_screen(seeded, true))
            } else {
                EntryOpen::Dialog(|seeded| debug_screen(seeded, false))
            },
        })
        .collect()
}

/// The dialog a seeded entry shows: the kind the script offered (a script offers one).
fn debug_screen(seeded: &Seeded<'_>, confirm_reset: bool) -> AnyWidget {
    let which = seeded
        .core
        .features()
        .settings
        .offers
        .keys()
        .next()
        .copied();
    Screen {
        which,
        confirm_reset,
    }
    .into()
}

/// Opens the dialog of `args` from the widget that asks (`show_dialog`): the feature is told to
/// build its catalogue (`Intent::Open`), the sheet draws it when it is there.
pub fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Args) {
    verb(cx, Intent::Open(args.which));
    drop(show_dialog::<(), S>(
        cx,
        move |_| {
            Screen {
                which: Some(args.which),
                confirm_reset: false,
            }
            .into()
        },
        // Esc is the dialog's own key when one is up; the barrier closes the dialog (the
        // theme picker restores its preview as it goes), or the unoffered slot.
        DialogOptions {
            barrier_dismissible: true,
            ..crate::sheets::sheet_options()
        },
    ));
}

/// Opens the sheet with no seeded model (debug menu): the empty slot.
pub fn open_empty() -> DebugOpen {
    DebugOpen::Dialog(|_| Screen::default().into())
}

/// What a dialog asks of the app: a request the feature records (`Intent::Request`) and, when
/// it is a write of the config file, makes.
fn request<S: 'static>(cx: &mut Cx<'_, S>, edit: Edit) {
    verb(cx, Intent::Request(edit));
}

/// Which sheet the dialog asked for is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shown {
    Setup(Which),
    Theme,
    Features,
    Memories,
    Hooks,
    Other,
}

impl Shown {
    fn of(which: Which) -> Self {
        match which {
            Which::StatusLine | Which::Title | Which::Skills => Shown::Setup(which),
            Which::Theme => Shown::Theme,
            Which::Experimental => Shown::Features,
            Which::Memories => Shown::Memories,
            Which::Hooks => Shown::Hooks,
            Which::Config | Which::Keymap => Shown::Other,
        }
    }
}

/// The sheet of this slot: the dialog of the kind that was asked for, drawn once the feature
/// offers its catalogue. The sheets close themselves when they are done.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Screen {
    /// The kind of dialog; none: the slot is opened with no dialog (the debug menu's row).
    pub which: Option<Which>,
    /// The memories dialog opens on its reset confirmation (a debug entry's start).
    pub confirm_reset: bool,
}

/// Widget state: whether the feature offers the dialog yet.
pub struct ScreenState {
    offered: bool,
}

impl StatefulWidget for Screen {
    type State = ScreenState;

    fn create_state(&self) -> ScreenState {
        ScreenState { offered: false }
    }
}

impl_widget!(stateful eq Screen);

impl State for ScreenState {
    type Widget = Screen;
    type Msg = ();

    fn did_change_dependencies(&mut self, w: &Screen, cx: &mut Cx<'_, Self>) {
        self.offered = w.which.is_some_and(|which| {
            select::<FeatureStates, bool, _>(cx, move |states| {
                states.settings.offers.contains_key(&which)
            })
        });
    }

    fn build(&self, w: &Screen, cx: &mut Cx<'_, Self>) -> AnyWidget {
        match w.which.filter(|_| self.offered).map(Shown::of) {
            Some(Shown::Setup(which)) => setup::SetupSheet { which }.into(),
            Some(Shown::Theme) => theme::ThemeSheet.into(),
            Some(Shown::Features) => features::FeaturesSheet.into(),
            Some(Shown::Memories) => memories::MemoriesSheet {
                confirming: w.confirm_reset,
            }
            .into(),
            Some(Shown::Hooks) => hooks::HooksSheet.into(),
            // No dialog offered (the debug menu's row, a dialog nothing builds yet): the
            // skeleton's empty-slot line.
            None | Some(Shown::Other) => {
                let shared = crate::l10n::strings::of(cx);
                Dialog::bare(Text::new(
                    shared
                        .empty_slot(<Settings as codex_tui_core::features::Feature>::NAME, ID)
                        .to_string(),
                ))
                .into()
            }
        }
    }
}
