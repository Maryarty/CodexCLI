//! The view half of the feature slices (ADR-012, ADR-013): what each feature of
//! `codex_tui_core::features` shows, one directory per feature (`src/features/<f>/`: its
//! `strings.rs` and views). The directories are the ones of core's `feature_list!`, whose other
//! lists this file invokes with its own macros (the `mod` lines are written out for rustfmt: a
//! feature of the list without its directory does not compile). A feature implements `FeatureView`
//! for its core marker type: the words of its notes and how its wishes are shown; a feature that a
//! slash command opens a sheet of (`Feature::OPENS`) implements `FeatureSheet` too, the only one
//! to have `args` and `open`. What a feature asks of the user is its own sheet, prompt or question
//! (`FeatureSheet::open`, a wish's `shown`), built of rtwk widgets over controllers and saying the
//! user's decision as a verb (`verb`): core keeps no menu, prompt or cursor of it.
//!
//! The generic places call in: `Strings::note` words a `NoteKey::Feature` through `note_words`,
//! the transcript draws it as the feature's own `cell` when words are not enough, the presenter
//! shows a wish through `shown`, the session page opens a feature's own sheet
//! (`Target::Feature`) through `open`, the settings dialogs through `settings::open`, binds the keys a feature owns (`actions`) and the surfaces
//! that have room for a line of a feature's (`Slot`) ask `slot`.

use std::rc::Rc;

use rtwk::overlay::DialogOptions;
use rtwk::provider::read;
use rtwk::style::Span;
use rtwk::{AnyWidget, Cx};

use codex_tui_core::app::Intent;
use codex_tui_core::commands::SlashCommand;
use codex_tui_core::feature_list;
use codex_tui_core::features::{
    Feature, FeatureIntent, FeatureInterest, FeatureNote, FeatureTarget, FeatureWish, Scope,
};
use codex_tui_core::ids::{Cwd, ThreadId};
use codex_tui_core::keymap::ActionId;

use crate::app::debug::Dir;
use crate::app::view::Hs;
use crate::pages::session::strings::Strings as SessionStrings;

pub mod prompt;

// The `pub mod` line of every slice's view half and `sheet_list!` are written by `build.rs` from
// core's slices (`test-mods/registry.rs`): a slice of core without its view directory (or
// `<f>.rs`) does not build, and one with it needs no line here. rustfmt does not follow the
// generated modules: `tools/rtwk-port/registry/fmt.sh` formats them.
include!(concat!(env!("OUT_DIR"), "/features.rs"));

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

/// A surface that has room for a line of a feature's, besides what the surface itself shows
/// (pillar 25): a feature adds the line from its own file (`FeatureView::slot`) and the surface
/// asks `slots`, so neither names a feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// A line of the chat's footer (a state of a feature the user should keep in view).
    FooterLine,
    /// A line of the usage notice (the rate-limit and credit hints under the composer).
    UsageNotice,
    /// A line of the header of a compaction (what the cell of `ContextCompacted` says first).
    CompactionHeader,
}

/// The line a feature adds to a [`Slot`]: styled words, the surface lays them out.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotLine {
    pub spans: Vec<Span>,
}

/// A key a feature owns (`Ctrl+G` of the external editor): the keymap action whose keys do it and
/// what it says when pressed. The page binds the keys of every feature's actions
/// (`Action::Feature`) and hands `run` the thread the chat shows and the composer's text; the
/// action says a verb of its feature (`KeyCx::say`), never acts on the page.
#[derive(Clone, Copy, Debug)]
pub struct FeatureAction {
    pub id: ActionId,
    /// The other keymap actions whose keys do it too (the hardwired forms of a key).
    pub also: &'static [ActionId],
    pub run: fn(&mut KeyCx<'_>),
}

/// What a key of a feature may read and say: the thread the chat shows and what the composer
/// would submit now, and the door the verb goes through (`Intent::Feature`).
pub struct KeyCx<'a> {
    pub thread: &'a ThreadId,
    pub draft: &'a str,
    say: &'a mut dyn FnMut(FeatureIntent),
}

impl<'a> KeyCx<'a> {
    pub fn new(
        thread: &'a ThreadId,
        draft: &'a str,
        say: &'a mut dyn FnMut(FeatureIntent),
    ) -> KeyCx<'a> {
        KeyCx { thread, draft, say }
    }

    /// Says a verb of the feature (ids and text it holds, never what was pressed).
    pub fn say(&mut self, intent: impl Into<FeatureIntent>) {
        (self.say)(intent.into());
    }
}

/// What the view of a feature says: the words of its notes, and how it shows its wishes.
pub trait FeatureView: Feature {
    /// How one of the feature's wishes is shown; none for a feature that has no wishes.
    fn shown(_wish: &Self::Wish, _scope: &Scope) -> Option<Shown> {
        None
    }

    /// The words of one of the feature's notes (the oracle's `add_info_message` text).
    fn note_words(note: &Self::Note, words: &SessionStrings) -> String;

    /// The cell of one of the feature's notes when its words are not enough (a link, a snapshot
    /// of rows, a styled block): the transcript draws it in place of the one row of words, which
    /// stay the text of the raw mode and of a selection. None: the words.
    fn cell(_note: &Self::Note) -> Option<AnyWidget> {
        None
    }

    /// What the feature adds to a surface's slot, read from the app through `cx`. None: nothing.
    fn slot<S: 'static>(_slot: Slot, _cx: &mut Cx<'_, S>) -> Option<SlotLine> {
        None
    }

    /// A text the feature hands the composer of `thread` (the external editor's edit): numbered,
    /// read from the app through `cx`. None: nothing waits.
    fn handed<S: 'static>(_thread: &ThreadId, _cx: &mut Cx<'_, S>) -> Option<(u64, String)> {
        None
    }

    /// The composer took the text numbered `number` (`handed`): the feature forgets it.
    fn taken<S: 'static>(_number: u64, _hs: &Hs, _cx: &mut Cx<'_, S>) {}

    /// The keys the feature owns, bound by the chat page.
    fn actions() -> Vec<FeatureAction> {
        Vec::new()
    }

    /// The feature's slot in the debug menu (`--debug-screens`): its sheets on made-up data. None
    /// for a feature whose sheets have no fixture states.
    fn debug_dir() -> Option<Dir> {
        None
    }
}

/// What a feature that a slash command opens a sheet of adds to its view (`Feature::OPENS`, the
/// generated `FeatureTarget`): the arguments of the sheet and how it opens. Only a slice that
/// declares `OPENS` implements it and is listed in `sheet_list!` (the rule
/// `feature_sheet_only_with_opens` keeps the two equal): a slice without a command has no `Args`
/// and no `open` to stub, and `FeatureTarget` of it is uninhabited, so no arm names it.
pub trait FeatureSheet: FeatureView {
    /// What opens the feature's own sheet: ids the page that opens it holds, never what was
    /// pressed.
    type Args: Clone + std::fmt::Debug + PartialEq + Eq;

    /// The arguments of the sheet the slash command named (`Feature::Target`), for the directory
    /// and the thread in use.
    fn args(target: Self::Target, cwd: Cwd, thread: Option<ThreadId>) -> Self::Args;

    /// Opens the feature's own sheet from the widget that asks.
    fn open<S: 'static>(cx: &mut Cx<'_, S>, args: Self::Args);
}

impl OpenArgs {
    /// The arguments of the sheet `target` names, for the directory and the thread in use.
    pub fn of(target: FeatureTarget, cwd: Cwd, thread: Option<ThreadId>) -> OpenArgs {
        target_args(target, cwd, thread)
    }
}

/// Opens the settings dialog the slash command names (`Target::Settings`). The dialogs are no
/// `FeatureSheet`: no feature target names them, the command's own target does.
pub fn open_settings<S: 'static>(cx: &mut Cx<'_, S>, command: SlashCommand) {
    settings::open(cx, settings::Args::of(command));
}

/// Enter on a pet of the `/pets` picker: `[tui].pet` is written (the settings own the config
/// writes, and the words of a refusal; `Edit::PetSelected`).
pub fn pick_pet<S: 'static>(cx: &mut Cx<'_, S>, selector: String) {
    use codex_tui_core::features::settings::{Edit, Intent};
    verb(cx, Intent::Request(Edit::PetSelected(selector)));
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
        /// The typed arguments of a feature's own sheet (`Target::Feature`): one variant per
        /// feature that opens one (`sheet_list!`).
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum OpenArgs {
            $($variant(<codex_tui_core::features::$name::$variant as FeatureSheet>::Args),)*
        }

        $(
        impl From<<codex_tui_core::features::$name::$variant as FeatureSheet>::Args> for OpenArgs {
            fn from(args: <codex_tui_core::features::$name::$variant as FeatureSheet>::Args) -> Self {
                Self::$variant(args)
            }
        }
        )*

        /// The arguments of the sheet the slash command named, for the directory and thread in use.
        /// A feature without a sheet names no target (`Never`), so no arm is written for it.
        #[allow(unused_variables)]
        fn target_args(target: FeatureTarget, cwd: Cwd, thread: Option<ThreadId>) -> OpenArgs {
            match target {
                $(FeatureTarget::$variant(target) => OpenArgs::$variant(
                    <codex_tui_core::features::$name::$variant as FeatureSheet>::args(
                        target, cwd, thread,
                    ),
                ),)*
            }
        }

        /// Opens a feature's own sheet from the widget that asks.
        pub fn open<S: 'static>(cx: &mut Cx<'_, S>, args: OpenArgs) {
            match args {
                $(OpenArgs::$variant(args) => {
                    <codex_tui_core::features::$name::$variant as FeatureSheet>::open(cx, args)
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

macro_rules! view_cells {
    ($($name:ident => $variant:ident;)*) => {
        /// The cell of a feature's note, when the feature draws one of its own.
        #[allow(unused_variables)]
        pub fn cell(note: &FeatureNote) -> Option<AnyWidget> {
            match *note {
                $(FeatureNote::$variant(ref note) => {
                    <codex_tui_core::features::$name::$variant as FeatureView>::cell(note)
                })*
            }
        }
    };
}

macro_rules! view_slots {
    ($($name:ident => $variant:ident;)*) => {
        /// The lines the features add to `slot`, in list order.
        #[allow(unused_variables)]
        pub fn slots<S: 'static>(slot: Slot, cx: &mut Cx<'_, S>) -> Vec<SlotLine> {
            [$(<codex_tui_core::features::$name::$variant as FeatureView>::slot(slot, cx),)*]
                .into_iter()
                .flatten()
                .collect()
        }
    };
}

macro_rules! view_handed {
    ($($name:ident => $variant:ident;)*) => {
        /// The text a feature hands the composer of `thread`, the first in list order.
        #[allow(unused_variables)]
        pub fn handed<S: 'static>(thread: &ThreadId, cx: &mut Cx<'_, S>) -> Option<(u64, String)> {
            let mut text = None;
            $(
            let own = <codex_tui_core::features::$name::$variant as FeatureView>::handed(thread, cx);
            text = text.or(own);
            )*
            text
        }

        /// The composer took the text numbered `number`: every feature that handed one forgets
        /// it.
        #[allow(unused_variables)]
        pub fn taken<S: 'static>(number: u64, hs: &Hs, cx: &mut Cx<'_, S>) {
            $(<codex_tui_core::features::$name::$variant as FeatureView>::taken(number, hs, cx);)*
        }
    };
}

macro_rules! view_actions {
    ($($name:ident => $variant:ident;)*) => {
        /// The keys the features own, in list order.
        pub fn actions() -> Vec<FeatureAction> {
            let mut all = Vec::new();
            $(all.extend(<codex_tui_core::features::$name::$variant as FeatureView>::actions());)*
            all
        }
    };
}

/// Runs the feature action whose keymap action is `id` (`Action::Feature`); false when no
/// feature owns it.
pub fn run_action(id: ActionId, keyed: &mut KeyCx<'_>) -> bool {
    match actions().into_iter().find(|action| action.id == id) {
        Some(action) => {
            (action.run)(keyed);
            true
        }
        None => false,
    }
}

feature_list!(view_dispatch);
feature_list!(view_wishes);
feature_list!(view_debug);
sheet_list!(view_open);
feature_list!(view_cells);
feature_list!(view_slots);
feature_list!(view_handed);
feature_list!(view_actions);
