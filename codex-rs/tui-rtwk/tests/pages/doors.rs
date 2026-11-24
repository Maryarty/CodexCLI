// test root: pages
//! The doors of the W2 round-1 "needs first" task (doors-r1): one test per door, each the
//! smallest use a package makes of it. The features that use them have their own tests.

use rtwk::focus::KeyPress;
use rtwk_core::input::{KeyEvent, Mods};

use codex_tui_rtwk::keymap::press;

/// A press as the terminal reports it under a Cyrillic layout: the character typed, the key it
/// is on a US layout.
fn layout_press(typed: &str, base: char, mods: Mods) -> KeyPress {
    let event = KeyEvent::char(typed).with_mods(mods).with_base_char(base);
    KeyPress::from_event(&event).expect("a character key is a press")
}

#[test]
fn ctrl_c_is_the_key_on_a_us_layout_too() {
    assert!(press::is_ctrl_c(&layout_press("c", 'c', Mods::CTRL)));
    // a Cyrillic layout's Ctrl+с is Ctrl+C (oracle #35599)
    assert!(press::is_ctrl_c(&layout_press("с", 'c', Mods::CTRL)));
    assert!(!press::is_ctrl_c(&layout_press("с", 'c', Mods::NONE)));
    assert!(!press::is_ctrl_c(&layout_press("в", 'd', Mods::CTRL)));
}

mod presets_chord {
    use codex_tui_core::app::AppEv;

    use crate::common::start;

    const KEYS: &str = "n · new   s · from settings   e · edit   d · delete   esc · close";

    /// `Ctrl+X p`, the default of `open_presets` (oracle `leader_defaults`), opens the menu of
    /// `/presets` as the command does, over a draft too.
    #[test]
    fn ctrl_x_p_opens_the_presets_menu() {
        let mut h = start();
        h.wait_idle();
        h.inject([AppEv::Presets(Vec::new())]);
        h.t.type_text("draft");
        h.press("ctrl+x");
        h.press("p");
        h.wait_for("the menu", |s| s.contains(KEYS));
        assert!(h.t.screen().contains("draft"), "{}", h.t.screen());
    }

    /// A key that is not the chord's completion is the composer's.
    #[test]
    fn ctrl_x_then_another_key_leaves_the_menu_closed() {
        let mut h = start();
        h.wait_idle();
        h.press("ctrl+x");
        h.press("q");
        h.wait_idle();
        assert!(!h.t.screen().contains(KEYS), "{}", h.t.screen());
    }
}
