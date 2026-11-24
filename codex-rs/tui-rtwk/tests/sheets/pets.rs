//! Screen tests of S24 (the `/pets` picker). Expected text: the oracle's
//! snapshot (`dialogs::oracle`), `›` marker kept. Oracle tests:
//! `chatwidget/tests/slash_commands.rs` (`slash_pets_picker`),
//! `pets/picker.rs` tests.

use crate::common::{Harness, ms, start_driven, start_sized};
use crate::dialogs::oracle::assert_rows;
use codex_tui_core::app::effects::Ask;
use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::pets::{Ev, Pet, Picker};

fn open(entry: &str) -> Harness {
    let mut h = start_sized(true, 80, 24);
    h.open_debug_row("pets", Some(entry));
    h.t.pump_for(ms(800));
    h
}

// chatwidget/tests/slash_commands.rs: slash_pets_picker
#[test]
fn slash_pets_picker() {
    let h = open("picker");
    assert_rows(
        &h.t.screen(),
        "chatwidget/snapshots/codex_tui__chatwidget__tests__slash_pets_picker.snap",
    );
}

#[test]
fn the_disabled_row_is_marked_current() {
    let h = open("picker_disabled");
    assert!(h.t.screen().contains("(current)"), "{}", h.t.screen());
}

#[test]
fn typing_filters_the_list() {
    let mut h = open("picker");
    for key in "dew".chars() {
        h.press(&key.to_string());
    }
    let screen = h.t.screen();
    assert!(screen.contains("Dewey"), "{screen}");
    assert!(!screen.contains("Fireball"), "{screen}");
}

#[test]
fn enter_picks_the_pet_and_closes() {
    let mut h = open("picker");
    h.press("enter");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Select Pet"), "{}", h.t.screen());
}

/// `/pets` on a driven harness with the catalog the host listed.
fn pets_command() -> Harness {
    let pet = |selector: &str, name: &str| Pet {
        selector: selector.into(),
        name: name.into(),
        description: None,
        legacy_selector: None,
    };
    let mut h = start_driven(false, 80, 24);
    h.wait_idle();
    h.inject([AppEv::Screen(ScreenEv::Pets(Ev::Listed(Picker::build(
        None,
        vec![pet("codex", "Codex"), pet("dewey", "Dewey")],
    ))))]);
    h.t.type_text("/pets");
    h.press("enter");
    h.wait_for("the picker", |screen| screen.contains("Select Pet"));
    h.advance(600);
    h
}

/// The pick is saved: Enter writes `tui.pet` through the host (the oracle's `PetSelected`).
#[test]
fn enter_writes_the_pick_to_the_config() {
    let mut h = pets_command();
    h.press("down");
    h.press("enter");
    h.wait_until("the write", |h| !edits(h).is_empty());
    assert_eq!(edits(&h), vec![("tui.pet".to_owned(), "dewey".into())]);
    assert!(!h.t.screen().contains("Select Pet"), "{}", h.t.screen());
}

/// The disable row is saved as the selector the oracle's `PetDisabled` writes.
#[test]
fn enter_on_the_disable_row_writes_the_disabled_pet() {
    let mut h = pets_command();
    h.press("up");
    h.press("enter");
    h.wait_until("the write", |h| !edits(h).is_empty());
    assert_eq!(edits(&h), vec![("tui.pet".to_owned(), "disabled".into())]);
}

/// The config edits the app has asked of the host.
fn edits(h: &Harness) -> Vec<(String, serde_json::Value)> {
    h.fake
        .commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Effect {
                ask: Ask::ConfigWrite { edits },
                ..
            } => Some(edits),
            _ => None,
        })
        .flatten()
        .map(|edit| (edit.key_path, edit.value))
        .collect()
}

#[test]
fn the_arrows_move_the_cursor_with_the_search_field_focused() {
    let mut h = open("picker");
    h.press("down");
    assert!(h.t.screen().contains("› Dewey"), "{}", h.t.screen());
    h.press("up");
    h.press("up");
    assert!(h.t.screen().contains("› BSOD"), "{}", h.t.screen());
}

#[test]
fn esc_clears_the_search_then_leaves() {
    let mut h = open("picker");
    for key in "dew".chars() {
        h.press(&key.to_string());
    }
    h.press("esc");
    let screen = h.t.screen();
    assert!(screen.contains("Fireball"), "{screen}");
    h.press("esc");
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Select Pet"), "{}", h.t.screen());
}

/// The oracle scrolls the window to the cursor row (`ScrollState`), also when
/// the search field has the focus and the arrows are the screen's keys.
/// The list is a `ListView` with a `ListController` (G35 not a gap): the
/// screen's key moves the cursor and calls `ensure_index_visible`.
#[test]
fn the_cursor_row_stays_in_the_window_when_the_arrows_pass_its_end() {
    let mut h = open("picker");
    // The picker opens on Codex; six downs reach the ninth row, past the window of 8.
    for _ in 0..6 {
        h.press("down");
    }
    let screen = h.t.screen();
    let cursor = screen.lines().filter(|l| l.contains('›')).count();
    assert_eq!(cursor, 1, "{screen}");
}

/// ADR-011 (beyond the oracle): a click on a pet puts the cursor there, a
/// click on the pet the cursor is on picks it and closes.
#[test]
fn a_click_selects_a_pet_and_a_second_click_picks_it() {
    let mut h = open("picker");
    let screen = h.t.screen();
    let (y, line) = screen
        .lines()
        .enumerate()
        .find(|(_, line)| line.contains("Dewey"))
        .expect("Dewey is listed");
    let x = u16::try_from(line.find("Dewey").unwrap()).unwrap();
    let y = u16::try_from(y).unwrap();
    h.t.click(x, y);
    h.settle();
    assert!(h.t.screen().contains("› Dewey"), "{}", h.t.screen());
    h.t.click(x, y);
    h.settle();
    h.t.pump_for(ms(800));
    assert!(!h.t.screen().contains("Select Pet"), "{}", h.t.screen());
}
