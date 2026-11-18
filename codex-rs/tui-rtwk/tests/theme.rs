//! The theme: one rtwk `ThemeData` at the root, the skeleton's style tokens
//! taken from it, never a literal colour.

use std::rc::Rc;

use codex_tui_rtwk::run::{app_view, root};
use codex_tui_rtwk::theme::Tokens;
use rtwk::prelude::ThemeData;

use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::shell::Backend;

#[test]
fn the_tokens_come_from_the_theme_data() {
    let data = Rc::new(ThemeData::DARK);
    let tokens = Tokens::new(data.clone());
    assert_eq!(tokens.accent(), data.accent);
    assert_eq!(tokens.error(), data.error);
    assert_eq!(tokens.dim(), data.dim());
}

#[test]
fn another_theme_gives_other_tokens() {
    let dark = Tokens::new(Rc::new(ThemeData::DARK));
    let light = Tokens::new(Rc::new(ThemeData::LIGHT));
    assert_ne!(dark.error(), light.error());
}

#[test]
fn the_root_carries_a_theme() {
    let app = UiApp::new(Backend::None);
    assert!(root(app_view(app)).theme.is_some());
}
