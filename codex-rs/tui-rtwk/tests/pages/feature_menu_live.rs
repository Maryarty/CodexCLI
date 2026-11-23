//! A menu of a feature slice (`Menu::Feature`) reads its rows from the feature's state at every
//! build (`FeatureView::menu_list`), and the state is provided to the views like the screens':
//! a step of any feature rebuilds the open menu under the same picker, so the cursor stays.
//! `/goal`'s `Replace goal?` is the feature menu the real list has; its rows do not change, but
//! the rebuild that a changed state brings is the same one a live menu gets.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::features::goal;
use codex_tui_core::features::{FeatureIntent, FeatureMenu};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::select::{Ev, Menu};

use crate::chat::chat_with_history;
use crate::common::{Harness, ms, start};

/// The row the cursor is on in the open confirmation.
fn cursor_row(screen: &str) -> String {
    screen
        .lines()
        .find(|line| line.trim_start().starts_with('›') && line.contains(". "))
        .unwrap_or_else(|| panic!("no row with the cursor on:\n{screen}"))
        .trim()
        .to_owned()
}

fn open_replace_goal(h: &mut Harness) {
    // the select sheet is opened by a command that targets it; the model then holds the menu
    h.t.type_text("/model");
    h.press("enter");
    h.t.pump_for(ms(400));
    let menu = Menu::Feature(FeatureMenu::from(goal::Menu::Replace {
        thread: ThreadId::new("thread-1"),
        objective: "ship it".into(),
    }));
    h.inject([AppEv::Screen(ScreenEv::Select(Ev::Opened(menu)))]);
    h.wait_for("the confirmation", |s| s.contains("Replace goal?"));
}

#[test]
fn a_feature_menu_keeps_its_cursor_when_a_feature_step_rebuilds_it() {
    let mut h = start();
    chat_with_history(&mut h);
    open_replace_goal(&mut h);
    assert!(
        cursor_row(&h.t.screen()).contains("Replace current goal"),
        "{}",
        h.t.screen()
    );
    h.press("down");
    h.settle();
    assert!(
        cursor_row(&h.t.screen()).contains("Cancel"),
        "{}",
        h.t.screen()
    );
    // a step of a feature changes the state the menu reads from (the shared value is a new one)
    h.inject([AppEv::Intent(Intent::Feature(FeatureIntent::from(
        goal::Intent::Command {
            source: None,
            args: None,
        },
    )))]);
    h.settle();
    let screen = h.t.screen();
    assert!(screen.contains("Replace goal?"), "{screen}");
    assert!(
        cursor_row(&screen).contains("Cancel"),
        "the rebuilt menu keeps the cursor:\n{screen}"
    );
}

/// `/export` (a slice with a menu of its own): the bare command opens the destination picker the
/// feature built, with the oracle's header and rows, and the cursor on the first.
#[test]
fn the_bare_export_command_opens_the_feature_destination_picker() {
    let mut h = start();
    chat_with_history(&mut h);
    h.t.type_text("/export");
    h.press("enter");
    h.wait_for("the destination picker", |s| s.contains("Export conversation"));
    let screen = h.t.screen();
    assert!(
        screen.contains("Save the complete conversation as Markdown"),
        "{screen}"
    );
    assert!(
        cursor_row(&screen).contains("Copy to clipboard"),
        "the cursor starts on the first row:\n{screen}"
    );
    assert!(screen.contains("Save to file"), "{screen}");
}
