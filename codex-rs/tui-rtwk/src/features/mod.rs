//! The view half of the feature slices (ADR-012, ADR-013): what each feature of
//! `codex_tui_core::features` shows, one directory per feature (`src/features/<f>/`: its
//! `strings.rs` and views). The directories are the ones of core's `feature_list!`, whose other
//! lists this file invokes with its own macros (the `mod` lines are written out for rustfmt: a
//! feature of the list without its directory does not compile). A feature implements `FeatureView`
//! for its core marker type: the words of its notes and how its wishes are shown. What a feature
//! asks of the user is its own sheet, prompt or question (`open(cx, args)`, a wish's `shown`), built
//! of rtwk widgets over controllers and saying the user's decision as a verb (`verb`): core keeps
//! no menu, prompt or cursor of it.
//!
//! The generic places call in: `Strings::note` words a `NoteKey::Feature` through `note_words`,
//! the presenter shows a wish through `shown`, and the session page opens a feature's own sheet
//! (`Target::Feature`) through `open`.

use std::rc::Rc;

use rtwk::overlay::DialogOptions;
use rtwk::provider::read;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::Intent;
use codex_tui_core::commands::SlashCommand;
use codex_tui_core::feature_list;
use codex_tui_core::features::{
    Feature, FeatureIntent, FeatureInterest, FeatureNote, FeatureTarget, FeatureWish, Scope,
};
use codex_tui_core::ids::{Cwd, ThreadId};

use crate::app::debug::Dir;
use crate::app::view::Hs;
use crate::pages::session::strings::Strings as SessionStrings;

pub mod prompt;

// One line per directory of `feature_list!`, written out so that rustfmt (which follows `mod`
// lines, not macro expansions) formats the views too. A feature of the list without its
// directory does not compile: `view_dispatch!` needs its `FeatureView` impl.
pub mod copy;
pub mod export;
pub mod goal;
pub mod plugins;
pub mod rename;
pub mod settings;

/// How a wish of a feature is shown (ADR-013 Ж): a sheet that is open for the wish's scope
/// shows it from its own slice of the feature's state (the data of the wish is the feature's, a
/// sheet reads it like the rest), else the presenter opens a sheet that does. The presenter
/// (`app::presenters`) is the only one that says it was shown.
pub struct Shown {
    /// What the sheet that shows the wish is interested in (`Interest`) while it is open: the
    /// presenter finds an open sheet by the counter core keeps, so no registry of sheets is kept
    /// and a sheet open for another scope is not mistaken for this one. None for a question or a
    /// prompt that is no sheet of a scope: it opens once for each wish.
    pub held_by: Option<FeatureInterest>,
    /// The sheet that opens for the wish when none is open for its scope.
    pub open: Rc<dyn Fn() -> AnyWidget>,
    pub options: DialogOptions,
}

/// What the view of a feature says: the words of its notes, and how it shows its wishes.
pub trait FeatureView: Feature {
    /// How one of the feature's wishes is shown; none for a feature that has no wishes.
    fn shown(_wish: &Self::Wish, _scope: &Scope) -> Option<Shown> {
        None
    }

    /// The words of one of the feature's notes (the oracle's `add_info_message` text).
    fn note_words(note: &Self::Note, words: &SessionStrings) -> String;

    /// What opens the feature's own sheet: ids the page that opens it holds, never what was
    /// pressed (`Never<Self>` for a feature that has none).
    type Args: Clone + std::fmt::Debug + PartialEq + Eq;

    /// The arguments of the sheet a slash command named (`Feature::Target`), for the directory and
    /// the thread in use; a feature without a target has no command to answer.
    fn args(target: Self::Target, cwd: Cwd, thread: Option<ThreadId>) -> Self::Args;

    /// Opens the feature's own sheet from the widget that asks.
    fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Self::Args);

    /// The feature's slot in the debug menu (`--debug-screens`): its sheets on made-up data. None
    /// for a feature whose sheets have no fixture states.
    fn debug_dir() -> Option<Dir> {
        None
    }
}

impl OpenArgs {
    /// The arguments of the settings dialog `command` opens (`Target::Settings`).
    pub fn settings(command: SlashCommand) -> OpenArgs {
        OpenArgs::Settings(settings::Args::of(command))
    }

    /// The arguments of the sheet `target` names, for the directory and the thread in use.
    pub fn of(target: FeatureTarget, cwd: Cwd, thread: Option<ThreadId>) -> OpenArgs {
        target_args(target, cwd, thread)
    }
}

/// A verb of a feature, from the widget that has the user's decision: the app applies it
/// (`Intent::Feature`). The arguments are ids (a directory, a plugin's row), never what was
/// pressed.
pub fn verb<S: 'static>(cx: &mut Cx<'_, S>, intent: impl Into<FeatureIntent>) {
    let hs = read::<Hs, _>(cx);
    verb_by(cx, &hs, intent);
}

/// `verb` through the door a widget kept from above: one that says its verb as it leaves the
/// tree (`dispose_with`) cannot read the door then.
pub fn verb_by<S: 'static>(cx: &mut Cx<'_, S>, hs: &Hs, intent: impl Into<FeatureIntent>) {
    cx.emit(&hs.intent, Intent::Feature(intent.into()));
}

macro_rules! view_dispatch {
    ($($name:ident => $variant:ident;)*) => {
        /// The words of a feature's note.
        #[allow(unused_variables)]
        pub fn note_words(note: &FeatureNote, words: &SessionStrings) -> String {
            match *note {
                $(FeatureNote::$variant(ref note) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::note_words(note, words)
                })*
            }
        }
    };
}

macro_rules! view_open {
    ($($name:ident => $variant:ident;)*) => {
        /// The typed arguments of a feature's own sheet (`Target::Feature`, the settings
        /// dialogs): one variant per feature, uninhabited for a feature without a sheet.
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum OpenArgs {
            $($variant(<codex_tui_core::features::$name::$variant as FeatureView>::Args),)*
        }

        $(
        impl From<<codex_tui_core::features::$name::$variant as FeatureView>::Args> for OpenArgs {
            fn from(args: <codex_tui_core::features::$name::$variant as FeatureView>::Args) -> Self {
                Self::$variant(args)
            }
        }
        )*

        /// The arguments of the sheet the slash command named, for the directory and thread in use.
        #[allow(unused_variables)]
        fn target_args(target: FeatureTarget, cwd: Cwd, thread: Option<ThreadId>) -> OpenArgs {
            match target {
                $(FeatureTarget::$variant(target) => OpenArgs::$variant(
                    <codex_tui_core::features::$name::$variant as FeatureView>::args(
                        target, cwd, thread,
                    ),
                ),)*
            }
        }

        /// Opens a feature's own sheet from the widget that asks.
        pub fn open<S: 'static>(cx: &mut Cx<'_, S>, args: OpenArgs) {
            match args {
                $(OpenArgs::$variant(args) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::open(cx, args)
                })*
            }
        }
    };
}

macro_rules! view_debug {
    ($($name:ident => $variant:ident;)*) => {
        /// The slots of the features in the debug menu, in list order.
        pub fn debug_dirs() -> Vec<Dir> {
            [$(<codex_tui_core::features::$name::$variant as FeatureView>::debug_dir(),)*]
                .into_iter()
                .flatten()
                .collect()
        }
    };
}

macro_rules! view_wishes {
    ($($name:ident => $variant:ident;)*) => {
        /// How a wish of a feature is shown: what an open sheet of it is interested in, and the sheet
        /// that opens for it.
        #[allow(unused_variables)]
        pub fn shown(wish: &FeatureWish, scope: &Scope) -> Option<Shown> {
            match *wish {
                $(FeatureWish::$variant(ref wish) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::shown(wish, scope)
                })*
            }
        }
    };
}

feature_list!(view_dispatch);
feature_list!(view_wishes);
feature_list!(view_debug);
feature_list!(view_open);
