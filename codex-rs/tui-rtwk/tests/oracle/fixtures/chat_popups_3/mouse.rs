//! The mouse on `/plugins` (the oracle has none; the port's rule is that every selectable view
//! answers a tap, a double tap and the wheel): the browser's rows and tabs, the detail's actions,
//! the removal confirmation. A tap picks the row under it as the picker does everywhere; a double
//! tap is two taps, the second one on whatever the first brought up, so it must not act twice.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::features::plugins::{Location, Target, Want};

use super::{plugin, row_of, seed, select_tab};
use crate::kit::Screen;

fn two_plugins() -> Screen {
    let calendar = plugin(
        "plugin-calendar",
        "calendar",
        "Calendar",
        "Schedule management.",
        false,
    );
    let drive = plugin("plugin-drive", "drive", "Drive", "Document access.", true);
    let shelf = seed::shelf(vec![seed::curated(vec![calendar.clone(), drive.clone()])]);
    seed::with(&shelf, |held| {
        held.details.insert(
            "calendar".to_string(),
            seed::detail(calendar, Some("Schedules."), &[], &[], &[], &[]),
        );
        held.details.insert(
            "drive".to_string(),
            seed::detail(drive, Some("Documents."), &[], &[], &[], &[]),
        );
    });
    seed::open(100, 24, seed::config(), &shelf)
}

fn reads(s: &Screen) -> usize {
    seed::wants(s)
        .iter()
        .filter(|want| matches!(want, Want::Read(_)))
        .count()
}

#[test]
fn a_tap_on_a_plugin_row_opens_its_detail() {
    let mut s = two_plugins();
    s.click_text("Calendar");
    s.assert_contains("Back to plugins");
    s.assert_contains("Install plugin");
    assert_eq!(reads(&s), 1);
}

#[test]
fn a_double_tap_on_a_plugin_row_opens_one_detail_and_stays_in_it() {
    let mut s = two_plugins();
    s.double_click_text("Calendar");
    // the second tap lands on the detail the first one opened, not on a row that leaves it
    s.assert_contains("Install plugin");
    s.assert_absent("Type to search plugins");
    assert_eq!(reads(&s), 1);
    assert!(installs(&s).is_empty(), "{:?}", seed::wants(&s));
}

#[test]
fn a_tap_on_a_tab_shows_that_tab_and_a_tap_on_the_first_one_comes_back() {
    let mut s = two_plugins();
    s.assert_contains("Calendar");
    s.click_text("Installed (1)");
    s.assert_contains("Showing 1 installed plugins.");
    s.assert_absent("Calendar");
    s.click_text("All Plugins");
    s.assert_contains("Calendar");
    s.assert_contains("Drive");
}

#[test]
fn a_tap_on_the_add_marketplace_tab_and_its_row_opens_the_source_prompt() {
    let mut s = two_plugins();
    s.click_text("Add Marketplace");
    s.assert_contains("Add a marketplace from a Git repo or local root.");
    s.click_text("Add marketplace");
    s.assert_contains("owner/repo, git URL, or local marketplace path");
}

#[test]
fn the_wheel_scrolls_a_long_plugin_list_and_a_tap_opens_a_row_it_brought_in() {
    let plugins: Vec<_> = (0..40)
        .map(|n| {
            plugin(
                &format!("plugin-{n:02}"),
                &format!("plugin{n:02}"),
                &format!("Plugin number {n:02}"),
                "Does a thing.",
                false,
            )
        })
        .collect();
    let shelf = seed::shelf(vec![seed::curated(plugins.clone())]);
    seed::with(&shelf, |held| {
        held.details.insert(
            "*".to_string(),
            seed::detail(
                plugins[35].clone(),
                Some("Does a thing."),
                &[],
                &[],
                &[],
                &[],
            ),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.assert_contains("Plugin number 00");
    s.assert_absent("Plugin number 35");
    let row = row_of(&s, "Plugin number 00");
    s.wheel_down(10, u16::try_from(row).unwrap_or(14), 40);
    s.assert_absent("Plugin number 00");
    s.assert_contains("Plugin number 35");
    s.wheel_up(10, u16::try_from(row).unwrap_or(14), 40);
    s.assert_contains("Plugin number 00");
    s.wheel_down(10, u16::try_from(row).unwrap_or(14), 40);
    s.click_text("Plugin number 35");
    assert_eq!(reads(&s), 1);
    s.assert_contains("Back to plugins");
}

fn calendar_detail() -> (Screen, seed::Shelf) {
    let calendar = plugin(
        "plugin-calendar",
        "calendar",
        "Calendar",
        "Schedule management.",
        false,
    );
    let shelf = seed::shelf(vec![seed::curated(vec![calendar.clone()])]);
    seed::with(&shelf, |held| {
        held.held.insert("plugin/install");
        held.details.insert(
            "calendar".to_string(),
            seed::detail(calendar, Some("Schedules."), &[], &[], &[], &[]),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.click_text("Calendar");
    s.assert_contains("Install plugin");
    (s, shelf)
}

fn installs(s: &Screen) -> Vec<Target> {
    seed::wants(s)
        .into_iter()
        .filter_map(|want| match want {
            Want::Install(target) => Some(target),
            _ => None,
        })
        .collect()
}

#[test]
fn a_tap_on_install_asks_the_install_and_a_double_tap_asks_it_once() {
    let (mut s, _) = calendar_detail();
    s.click_text("Install plugin");
    assert_eq!(installs(&s).len(), 1, "{:?}", seed::wants(&s));
    s.assert_contains("Installing");

    let (mut s, _) = calendar_detail();
    s.double_click_text("Install plugin");
    assert_eq!(installs(&s).len(), 1, "{:?}", seed::wants(&s));
}

#[test]
fn a_tap_on_back_to_plugins_returns_to_the_list() {
    let (mut s, _) = calendar_detail();
    s.click_text("Back to plugins");
    s.assert_contains("Type to search plugins");
    s.assert_contains("Calendar");
}

fn confirmation() -> (Screen, seed::Shelf) {
    let shelf = seed::shelf(vec![
        seed::curated(Vec::new()),
        seed::repo(vec![plugin(
            "plugin-debug",
            "debug",
            "Debug Plugin",
            "Debug.",
            false,
        )]),
    ]);
    seed::with(&shelf, |held| {
        held.held.insert("marketplace/remove");
    });
    let mut config = seed::config();
    config.configured = vec![seed::git("repo")];
    let mut s = seed::open(100, 24, config, &shelf);
    select_tab(&mut s, "Repo Marketplace.");
    s.keys("ctrl+r");
    s.assert_contains("Remove Repo Marketplace marketplace?");
    (s, shelf)
}

fn removals(s: &Screen) -> usize {
    seed::wants(s)
        .iter()
        .filter(|want| matches!(want, Want::Remove { .. }))
        .count()
}

#[test]
fn a_tap_on_remove_marketplace_removes_it_once_even_as_a_double_tap() {
    let (mut s, _) = confirmation();
    s.double_click_text("Remove marketplace");
    assert_eq!(removals(&s), 1);
    s.assert_contains("Removing Repo Marketplace...");
}

#[test]
fn a_tap_on_back_to_plugins_in_the_confirmation_keeps_the_marketplace() {
    let (mut s, _) = confirmation();
    s.click_text("Back to plugins");
    assert_eq!(removals(&s), 0);
    s.assert_contains("Repo Marketplace.");
    s.assert_contains("Type to search plugins");
}

#[test]
fn a_tap_on_a_remote_row_asks_its_detail_from_the_remote_marketplace() {
    let shelf = seed::shelf(Vec::new());
    seed::with(&shelf, |held| {
        held.held.insert("plugin/read");
        held.remote = vec![seed::marketplace(
            "workspace-directory",
            None,
            Some("Workspace"),
            vec![seed::remote_summary(
                "plugins~Plugin_calendar",
                "calendar",
                Some("Calendar"),
                Some("Schedules."),
                false,
            )],
        )];
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.wait_for("Calendar");
    s.click_text("Calendar");
    let target = Target {
        location: Location::Remote {
            marketplace: "workspace-directory".to_string(),
        },
        plugin: "plugins~Plugin_calendar".to_string(),
    };
    assert!(seed::wants(&s).contains(&Want::Read(target)));
}
