//! The view half of `/usage` (ADR-013): a bare `/usage` is `Target::Feature(UsageMenu)`, whose
//! `open` says the verb `Intent::Menu` to the usage screen (a count known to be zero is read
//! again) and shows the sheet: the menu with `View analytics` and `Redeem reset`, the credits
//! behind the second and the confirmation of a credit (`sheet`). The state is the usage screen's
//! (`codex_tui_core::screens::usage::reset`); the feature itself is
//! `codex_tui_core::features::usage_menu` (the slash command's route).

pub mod sheet;
pub mod strings;

use rtwk::Cx;
use rtwk::overlay::show_dialog;

use codex_tui_core::features::Feature;
use codex_tui_core::features::usage_menu::{Sheet, UsageMenu};
use codex_tui_core::ids::{Cwd, ThreadId};
use codex_tui_core::screens::usage::Intent;

use super::{FeatureSheet, FeatureView};
use crate::pages::session::strings::Strings as SessionStrings;

impl FeatureView for UsageMenu {
    fn note_words(note: &<UsageMenu as Feature>::Note, _words: &SessionStrings) -> String {
        note.absurd()
    }
}

impl FeatureSheet for UsageMenu {
    type Args = Args;

    fn args(_target: Sheet, _cwd: Cwd, _thread: Option<ThreadId>) -> Args {
        Args
    }

    fn open<S: 'static>(cx: &mut Cx<'_, S>, _args: Args) {
        sheet::say(cx, Intent::Menu);
        drop(show_dialog::<(), S>(
            cx,
            |_| sheet::sheet(),
            sheet::options(),
        ));
    }
}

/// What opens `/usage`: nothing, the verb needs no ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Args;
