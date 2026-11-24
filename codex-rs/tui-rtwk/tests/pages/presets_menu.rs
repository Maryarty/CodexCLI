// test root: pages
//! The shortcut menu of `/presets` over the composer (oracle `chatwidget/quick_dials.rs`:
//! `open_quick_dial_menu`, `handle_quick_dial_menu`; the block of `bottom_pane/shortcut_hud.rs`):
//! the command opens it, its keys lead on, any other key closes it and reaches the composer. The
//! pages a key opens are `quick_dials.rs`'s; the rows and the columns are the oracle's snapshots
//! (`quick_dial_shortcuts*`, `tests/oracle/chat_popups_4.rs`).

use codex_tui_core::app::AppEv;
use codex_tui_core::screens::select::quick_dials::{Dial, Origin, Slot};

use crate::common::{Harness, start};

fn preset(slot: u8, input: &str, label: Option<&str>) -> Slot {
    Slot {
        slot,
        dial: Dial {
            input: input.to_owned(),
            label: label.map(str::to_owned),
        },
        origin: Origin::User,
    }
}

const KEYS: &str = "n · new   s · from settings   e · edit   d · delete   esc · close";

/// A chat that has the presets `slots` and ran `/presets`.
fn menu(slots: Vec<Slot>) -> Harness {
    let mut h = start();
    h.wait_idle();
    h.inject([AppEv::Presets(slots)]);
    h.t.type_text("/presets");
    h.press("enter");
    h.wait_for("the menu", |s| s.contains(KEYS));
    h
}

#[test]
fn the_menu_lists_the_saved_presets_by_slot_and_the_keys_under_them() {
    let h = menu(vec![
        preset(2, "/status", None),
        preset(1, "/model gpt-5.5", Some("coding")),
        preset(0, "Plan the change\nin two steps", None),
    ]);
    let screen = h.t.screen();
    let at = |words: &str| {
        screen
            .find(words)
            .unwrap_or_else(|| panic!("{words}\n{screen}"))
    };
    assert!(at("Presets") < at("1 · coding"), "{screen}");
    assert!(at("1 · coding") < at("2 · /status"), "{screen}");
    assert!(at("2 · /status") < at("0 · Plan the change"), "{screen}");
    assert!(at("0 · Plan the change") < at(KEYS), "{screen}");
    assert!(!screen.contains("in two steps"), "{screen}");
}

#[test]
fn an_empty_list_says_how_to_start() {
    let h = menu(Vec::new());
    assert!(
        h.t.screen()
            .contains("No presets saved. Press n to create one."),
        "{}",
        h.t.screen()
    );
}

#[test]
fn esc_closes_the_menu_and_the_next_key_is_the_composers() {
    let mut h = menu(Vec::new());
    h.press("esc");
    h.settle();
    assert!(!h.t.screen().contains(KEYS), "{}", h.t.screen());
    h.t.type_text("x");
    h.settle();
    assert!(h.t.screen().contains("› x"), "{}", h.t.screen());
}

#[test]
fn another_key_closes_the_menu_and_goes_on_to_the_composer() {
    let mut h = menu(Vec::new());
    h.t.type_text("x");
    h.settle();
    assert!(!h.t.screen().contains(KEYS), "{}", h.t.screen());
    assert!(h.t.screen().contains("› x"), "{}", h.t.screen());
}

#[test]
fn enter_leaves_the_menu_as_it_is() {
    let mut h = menu(Vec::new());
    h.press("enter");
    h.settle();
    assert!(h.t.screen().contains(KEYS), "{}", h.t.screen());
}

#[test]
fn n_opens_the_editor_and_the_menu_is_gone() {
    let mut h = menu(Vec::new());
    h.t.type_text("n");
    h.wait_for("the editor", |s| s.contains("New preset"));
    assert!(!h.t.screen().contains(KEYS), "{}", h.t.screen());
}
