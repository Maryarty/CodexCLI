//! The chat's key actions, one file each (registry 1 of the UI hubs, ADR-012 for the keys): a
//! key action is a file of this directory with its keymap action (`KeyAction::ID`) and what it
//! does (`KeyAction::run`), and nothing else: `build.rs` writes `action_list!` and the `pub mod`
//! lines from the files (`test-mods/registry.rs`, by file name). The list generates `keys::Action`,
//! its keymap ids, its presses and its dispatch (`keys.rs`); neither the page's `update` nor the
//! key binding of the page names an action. The keys of a feature are the feature's own
//! (`FeatureView::actions`), reached through `Action::Feature`, never a file here.
//!
//! Only the actions of the chat's hub role are files here: those that open a screen, start
//! history search or switch a mode of the page. An action acts through what the page hands it
//! (`Keyed`: the door to the app and the thread's controllers); it never reaches into the page's
//! own state.

use rtwk::Cx;

use codex_tui_core::keymap::ActionId;

use crate::app::view::Hs;
use crate::pages::session::session_page::ChatState;
use crate::widgets::thread_ui::ThreadUi;

include!(concat!(env!("OUT_DIR"), "/key_actions.rs"));

/// What the page hands the action that a key ran: the door to the app and the controllers of the
/// thread the page shows (its modes, its composer).
pub struct Keyed<'a> {
    pub hs: &'a Hs,
    pub ui: &'a ThreadUi,
}

/// A key action of the chat: the marker type of a file of this directory.
pub trait KeyAction {
    /// The keymap action whose keys do it.
    const ID: ActionId;

    /// The other keymap actions whose keys do it too (the hardwired forms of a key: Ctrl+/ comes
    /// as a C0 byte without the kitty protocol).
    const ALSO: &'static [ActionId] = &[];

    /// What the key does, from the page that bound it.
    fn run(cx: &mut Cx<'_, ChatState>, keyed: &Keyed<'_>);
}
