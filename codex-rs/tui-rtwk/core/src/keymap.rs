//! The keymap: Codex's `tui.keymap.*` config as a [`Keymap`] of bindings per
//! action and context, with chords and a leader key.
//!
//! Carried from the oracle's `tui/src/keymap.rs` (`RuntimeKeymap`),
//! `keymap/{bindings,chords,leader,vim_search}.rs` and the config crate's
//! `tui_keymap.rs`. No rtwk and no IO: the host reads the config into a
//! [`KeymapSpec`], the app holds the resolved `Keymap` (`CodexApp::keymap`),
//! and the view turns a binding into rtwk shortcuts (`src/keymap`).

pub mod actions;
pub mod catalogue;
pub mod fixed;
mod key;
mod resolve;
mod spec;

pub use catalogue::{ActionId, Context};
pub use key::{Binding, Key, Mods, Stroke};
pub use resolve::{Keymap, KeymapError};
pub use spec::KeymapSpec;
