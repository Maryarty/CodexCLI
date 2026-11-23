//! The view half of the feature slices (ADR-012, ADR-013): what each feature of
//! `codex_tui_core::features` shows, one directory per feature (`src/features/<f>/`: its
//! `strings.rs` and views). The directories are the ones of core's `feature_list!`, which this file
//! invokes with its own macros, so the two lists cannot differ. A feature implements `FeatureView`
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
use codex_tui_core::feature_list;
use codex_tui_core::features::{
    Feature, FeatureIntent, FeatureInterest, FeatureNote, FeatureTarget, FeatureWish, Scope,
};
use codex_tui_core::ids::{Cwd, ThreadId};

use crate::app::view::Hs;
use crate::pages::session::strings::Strings as SessionStrings;

pub mod prompt;

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
}

/// The typed arguments of a feature's own sheet (`Target::Feature`): ids the page that opens it
/// holds, never what was pressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpenArgs {
    Plugins(plugins::Args),
    Export(export::Args),
}

impl OpenArgs {
    /// The arguments of the sheet `target` names, for the directory and the thread in use.
    pub fn of(target: FeatureTarget, cwd: Cwd, thread: Option<ThreadId>) -> OpenArgs {
        match target {
            FeatureTarget::Plugins => OpenArgs::Plugins(plugins::Args { cwd }),
            FeatureTarget::Export => OpenArgs::Export(export::Args { thread }),
        }
    }
}

/// Opens a feature's own sheet from the widget that asks (`Target::Feature`).
pub fn open<S: 'static>(cx: &mut Cx<'_, S>, args: OpenArgs) {
    match args {
        OpenArgs::Plugins(args) => plugins::open(cx, args),
        OpenArgs::Export(args) => export::open(cx, args),
    }
}

/// A verb of a feature, from the widget that has the user's decision: the app applies it
/// (`Intent::Feature`). The arguments are ids (a directory, a plugin's row), never what was
/// pressed.
pub fn verb<S: 'static>(cx: &mut Cx<'_, S>, intent: impl Into<FeatureIntent>) {
    let hs = read::<Hs, _>(cx);
    cx.emit(&hs.intent, Intent::Feature(intent.into()));
}

macro_rules! view_mods {
    ($($name:ident => $variant:ident;)*) => {
        $(pub mod $name;)*
    };
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

feature_list!(view_mods);
feature_list!(view_dispatch);
feature_list!(view_wishes);
