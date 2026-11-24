//! The sheet of the slot: which dialog the `Screen` draws (`Shown`), once the feature offers it.

use rtwk::overlay::Dialog;
use rtwk::provider::select;
use rtwk::text::Text;
use rtwk::{AnyWidget, Cx, State, StatefulWidget, impl_widget};

use codex_tui_core::features::FeatureStates;
use codex_tui_core::features::settings::{ID, Settings, Which};

use super::{config_panel, features, hooks, keymap_editor, memories, setup, theme};

/// Which sheet the dialog asked for is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shown {
    Setup(Which),
    Theme,
    Features,
    Memories,
    Hooks,
    Config,
    Keymap,
}

impl Shown {
    fn of(which: Which) -> Self {
        match which {
            Which::StatusLine | Which::Title | Which::Skills => Shown::Setup(which),
            Which::Theme => Shown::Theme,
            Which::Experimental => Shown::Features,
            Which::Memories => Shown::Memories,
            Which::Hooks => Shown::Hooks,
            Which::Config => Shown::Config,
            Which::Keymap => Shown::Keymap,
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
    /// The keymap editor opens on the keypress inspector (`/keymap debug`).
    pub inspect: bool,
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
            Some(Shown::Config) => config_panel::ConfigSheet.into(),
            Some(Shown::Keymap) => keymap_editor::KeymapSheet {
                inspect: w.inspect,
            }
            .into(),
            // No dialog offered (the debug menu's row): the skeleton's empty-slot line.
            None => {
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
