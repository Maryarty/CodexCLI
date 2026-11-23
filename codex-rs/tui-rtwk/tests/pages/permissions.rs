//! `/permissions` in the session page (oracle `chatwidget/permission_popups.rs`,
//! `permission_discovery.rs`, `permissions_menu.rs`): the sheet that waits behind
//! `Loading permission profiles…` until the server's catalog is read, lists the built-in modes and
//! the server's profiles, asks `Enable full access?` before a pick of full access, and shows the
//! `Retry` page of a read that failed. A pick says one verb and the sheet is gone; the inline
//! `/permissions <profile>` opens the same sheet while the catalog is read and takes the pick
//! once it came.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use codex_tui_core::app::Command;
use codex_tui_core::screens::{ScreenCommand, select};

use crate::kit::{Owner, Screen};

fn chat_screen() -> Screen {
    Screen::build(Owner::DialogsPane).size(80, 24).live().open()
}

/// `/permissions` typed: the sheet waits for the catalog.
fn loading() -> Screen {
    let mut s = chat_screen();
    s.type_text("/permissions");
    s.keys("enter");
    s.assert_contains("Update Model Permissions");
    s.assert_contains("Loading permission profiles");
    s
}

/// The asks the select screen made of the app, oldest first.
fn asked(s: &Screen, wanted: impl Fn(&select::Command) -> bool) -> Vec<Command> {
    s.commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Select(asked),
                    ..
                } if wanted(asked)
            )
        })
        .collect()
}

fn requirements(s: &Screen) -> Vec<Command> {
    asked(s, |c| matches!(c, select::Command::Requirements { .. }))
}

fn profile_pages(s: &Screen) -> Vec<Command> {
    asked(s, |c| matches!(c, select::Command::Profiles { .. }))
}

/// The permission profile ids the thread was asked to change to, oldest first.
fn updates(s: &Screen) -> Vec<Option<String>> {
    s.commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Select(select::Command::Settings { patch, .. }),
                ..
            } => Some(patch.permissions),
            _ => None,
        })
        .collect()
}

/// The server answers the requirements read and the one page of profiles that follows it.
fn read(s: &mut Screen) {
    let asks = requirements(s);
    let ask = asks.last().expect("the requirements were asked");
    s.answer_request(ask, serde_json::json!({ "requirements": null }));
    let pages = profile_pages(s);
    let page = pages.last().expect("a page of profiles was asked");
    s.answer_request(
        page,
        serde_json::json!({
            "data": [{ "id": ":read-only" }, { "id": ":workspace" }, { "id": ":danger-full-access" }],
            "nextCursor": null,
        }),
    );
}

/// The menu with the catalog read.
fn listed() -> Screen {
    let mut s = loading();
    read(&mut s);
    s.assert_absent("Loading permission profiles");
    s
}

#[test]
fn the_sheet_waits_for_the_catalog_and_lists_the_modes_in_the_oracles_order() {
    let mut s = loading();
    assert_eq!(requirements(&s).len(), 1);
    s.assert_contains("enter select · esc back");
    read(&mut s);
    s.assert_in_order(&[
        "Update Model Permissions",
        "1. Ask for approval",
        "2. Full Access",
        "3. Read Only",
        "enter select · esc back",
    ]);
    s.assert_absent("Loading permission profiles");
}

#[test]
fn enter_on_the_loading_row_asks_nothing_and_the_sheet_stays() {
    let mut s = loading();
    s.keys("enter");
    s.assert_contains("Loading permission profiles");
    assert!(updates(&s).is_empty());
}

#[test]
fn enter_on_a_mode_asks_the_server_to_update_the_thread_and_closes_the_sheet() {
    let mut s = listed();
    s.keys("enter");
    assert_eq!(updates(&s), [Some(":workspace".to_string())]);
    s.assert_absent("Update Model Permissions");
}

#[test]
fn a_tap_on_a_row_picks_it() {
    let mut s = listed();
    s.click_text("Read Only");
    assert_eq!(updates(&s), [Some(":read-only".to_string())]);
    s.assert_absent("Update Model Permissions");
}

#[test]
fn esc_closes_the_sheet_and_asks_nothing_of_the_thread() {
    let mut s = listed();
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty());
}

#[test]
fn esc_while_loading_closes_the_sheet_and_the_catalog_that_comes_opens_nothing() {
    let mut s = loading();
    s.keys("esc");
    s.assert_absent("Update Model Permissions");
    read(&mut s);
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty());
}

#[test]
fn the_catalog_is_kept_so_the_next_open_lists_the_modes_without_a_new_read() {
    let mut s = listed();
    s.keys("esc");
    s.type_text("/permissions");
    s.keys("enter");
    s.assert_in_order(&["Update Model Permissions", "1. Ask for approval"]);
    s.assert_absent("Loading permission profiles");
    assert_eq!(requirements(&s).len(), 1, "the catalog is not read twice");
}

#[test]
fn full_access_asks_first_and_applies_only_when_confirmed() {
    let mut s = listed();
    s.keys("down enter");
    s.assert_in_order(&[
        "Enable full access?",
        "When Codex runs with full access",
        "1. Yes, continue anyway",
        "2. Cancel",
    ]);
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty(), "nothing before the answer");
    s.keys("enter");
    assert_eq!(updates(&s), [Some(":danger-full-access".to_string())]);
    s.assert_absent("Enable full access?");
}

#[test]
fn cancel_on_the_confirmation_opens_the_permissions_menu_again() {
    let mut s = listed();
    s.keys("down enter");
    s.keys("down enter");
    s.assert_absent("Enable full access?");
    s.assert_in_order(&["Update Model Permissions", "1. Ask for approval"]);
    assert!(updates(&s).is_empty());
}

#[test]
fn esc_on_the_confirmation_closes_the_sheet_and_applies_nothing() {
    let mut s = listed();
    s.keys("down enter");
    s.keys("esc");
    s.assert_absent("Enable full access?");
    s.assert_absent("Update Model Permissions");
    assert!(updates(&s).is_empty());
}

#[test]
fn a_failed_read_says_why_and_retry_reads_again() {
    let mut s = loading();
    let asks = requirements(&s);
    s.answer_error(&asks[0], "no route");
    s.assert_in_order(&[
        "Update Model Permissions",
        "Failed to load permissions: no route",
        "Retry",
    ]);
    s.assert_absent("Loading permission profiles");
    s.keys("enter");
    s.assert_contains("Loading permission profiles");
    assert_eq!(requirements(&s).len(), 2, "the retry reads again");
    read(&mut s);
    s.assert_in_order(&["Update Model Permissions", "1. Ask for approval"]);
}

#[test]
fn an_inline_profile_waits_for_the_catalog_and_the_sheet_goes_when_it_came() {
    let mut s = chat_screen();
    s.type_text("/permissions :read-only");
    s.keys("enter");
    s.assert_contains("Loading permission profiles");
    assert!(updates(&s).is_empty());
    read(&mut s);
    s.assert_absent("Update Model Permissions");
    assert_eq!(updates(&s), [Some(":read-only".to_string())]);
}

#[test]
fn an_inline_full_access_asks_the_confirmation_once_the_catalog_came() {
    let mut s = chat_screen();
    s.type_text("/permissions :danger-full-access");
    s.keys("enter");
    s.assert_contains("Loading permission profiles");
    read(&mut s);
    s.assert_contains("Enable full access?");
    assert!(updates(&s).is_empty(), "nothing before the answer");
    s.keys("enter");
    assert_eq!(updates(&s), [Some(":danger-full-access".to_string())]);
    s.assert_absent("Enable full access?");
}
