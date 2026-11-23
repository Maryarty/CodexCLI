//! `/apps` in the session page (oracle `chatwidget/connectors.rs`, `bottom_pane/app_link_view.rs`):
//! the directory of apps over the composer, the link dialog of the app picked from it, and what a
//! pick of an app without a page says. The directory is read from the app server (`app/list`) and
//! is the model's fact; the sheet is the nested navigator of `src/sheets/select/apps.rs`, its
//! pages (the directory, the link dialog) the sheet's own state. A pick that is a fact for the
//! app (the page opened in the browser, the switch written, the directory read again) is a
//! command the app gave the backend.

use codex_tui_core::app::Command;
use codex_tui_core::screens::ScreenCommand;
use codex_tui_core::screens::select;
use serde_json::{Value, json};

use crate::kit::{Owner, Screen};

const NOTION: &str = "https://example.test/notion";
const SLACK: &str = "https://example.test/slack";

fn chat_screen() -> Screen {
    Screen::build(Owner::DialogsPane).size(80, 24).live().open()
}

/// The `app/list` requests the app has made, oldest first.
fn asked(s: &Screen) -> Vec<Command> {
    s.commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Select(select::Command::Apps { .. }),
                    ..
                }
            )
        })
        .collect()
}

/// The switches the app wrote to the config, oldest first.
fn switched(s: &Screen) -> Vec<(String, bool)> {
    s.commands()
        .into_iter()
        .filter_map(|command| match command {
            Command::Screen {
                command: ScreenCommand::Select(select::Command::SetApp { id, enabled }),
                ..
            } => Some((id, enabled)),
            _ => None,
        })
        .collect()
}

/// An app of the directory (`AppInfo`), with a page when `url` is given.
fn app(id: &str, name: &str, about: &str, url: Option<&str>, installed: bool) -> Value {
    let mut app = json!({
        "id": id, "name": name, "description": about,
        "isAccessible": installed, "isEnabled": true, "pluginDisplayNames": []
    });
    if let Some(url) = url {
        app["installUrl"] = json!(url);
    }
    app
}

/// An `app/list` answer.
fn directory(apps: Vec<Value>) -> Value {
    json!({ "data": apps, "nextCursor": null })
}

/// Notion (installed) and Slack (to install), each with a page.
fn two() -> Vec<Value> {
    vec![
        app("notion", "Notion", "Workspace docs", Some(NOTION), true),
        app("slack", "Slack", "Team chat", Some(SLACK), false),
    ]
}

/// `/apps` typed and sent; the directory is asked of the app server and not yet answered.
fn opened() -> Screen {
    let mut s = chat_screen();
    s.type_text("/apps");
    s.keys("enter");
    s
}

/// The directory of `apps` on the screen.
fn loaded(apps: Vec<Value>) -> Screen {
    let mut s = opened();
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    s.answer_request(&asks[0], directory(apps));
    s
}

#[test]
fn a_bare_apps_asks_the_directory_once_and_shows_it_loading() {
    let s = opened();
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    s.assert_in_order(&[
        "Apps",
        "Loading installed and available apps...",
        "Loading apps...",
    ]);
}

#[test]
fn the_answer_fills_the_directory_with_its_counts_and_rows() {
    let s = loaded(two());
    s.assert_absent("Loading apps...");
    s.assert_in_order(&[
        "Apps",
        "Use $ to insert an installed app into your prompt.",
        "Installed 1 of 2 available apps.",
        "Type to search apps",
        "› Notion",
        "Slack",
        "enter select · esc back",
    ]);
}

#[test]
fn the_filter_narrows_the_directory_to_the_apps_that_match() {
    let mut s = loaded(two());
    s.type_text("sla");
    s.assert_absent("Notion");
    s.assert_contains("Slack");
    // the app is picked from what is left
    s.keys("enter");
    s.assert_contains("Install on ChatGPT");
}

#[test]
fn esc_in_the_directory_closes_the_sheet() {
    let mut s = loaded(two());
    s.keys("esc");
    s.assert_absent("Type to search apps");
    s.assert_absent("Notion");
}

#[test]
fn enter_on_an_app_with_a_page_opens_its_link_dialog_and_esc_closes_the_whole_sheet() {
    let mut s = loaded(two());
    s.keys("enter");
    s.assert_in_order(&[
        "Notion",
        "Workspace docs",
        "Use $ to insert this app into the prompt.",
        "› 1. Manage on ChatGPT",
        "2. Disable app",
        "3. Back",
    ]);
    // the oracle's view stack has nothing under the dialog: Esc ends it all
    s.keys("esc");
    s.assert_absent("Manage on ChatGPT");
    s.assert_absent("Type to search apps");
}

#[test]
fn a_tap_on_an_app_opens_its_link_dialog() {
    let mut s = loaded(two());
    s.click_text("Slack");
    s.assert_in_order(&[
        "Slack",
        "Team chat",
        "Install this app in your browser, then reload Codex.",
        "› 1. Install on ChatGPT",
        "2. Back",
    ]);
    // Back is the dialog's own close: a tap moves the cursor to it, a second one does it
    s.click_text("Back");
    s.wait(500);
    s.click_text("Back");
    s.assert_absent("Install on ChatGPT");
    s.assert_absent("Type to search apps");
}

#[test]
fn an_app_without_a_page_says_so_in_the_history_and_the_sheet_closes() {
    let mut s = loaded(vec![app("lost", "Lost App", "No page", None, true)]);
    // the row says what a pick does
    s.assert_contains("Installed · App link unavailable");
    s.keys("enter");
    s.assert_absent("Type to search apps");
    // the note is what is left of it
    s.assert_contains("App link unavailable");
    assert!(s.opened_urls().is_empty(), "{:?}", s.opened_urls());
}

#[test]
fn a_failed_read_says_so_and_retry_asks_the_directory_again() {
    let mut s = opened();
    let asks = asked(&s);
    assert_eq!(asks.len(), 1, "{asks:?}");
    s.answer_error(&asks[0], "app/list failed: 403 Forbidden");
    s.assert_in_order(&[
        "Apps",
        "Failed to load apps.",
        "App directory unavailable",
        "Retry",
    ]);
    s.assert_absent("Loading apps...");
    // the cursor skips the row that cannot be picked: Enter is Retry
    s.keys("enter");
    let asks = asked(&s);
    assert_eq!(asks.len(), 2, "{asks:?}");
    s.assert_contains("Loading apps...");
    // the sheet stayed: the second answer fills it
    s.answer_request(&asks[1], directory(two()));
    s.assert_contains("Installed 1 of 2 available apps.");
}

#[test]
fn the_page_of_an_installed_app_opens_in_the_browser_and_the_dialog_stays() {
    let mut s = loaded(two());
    s.keys("enter");
    s.keys("enter");
    assert_eq!(s.opened_urls(), [NOTION]);
    s.assert_contains("› 1. Manage on ChatGPT");
}

#[test]
fn the_install_page_goes_on_to_the_confirmation_and_i_already_installed_it_reads_the_directory() {
    let mut s = loaded(two());
    s.keys("down enter");
    s.assert_contains("› 1. Install on ChatGPT");
    s.keys("enter");
    assert_eq!(s.opened_urls(), [SLACK]);
    s.assert_in_order(&[
        "Finish App Setup",
        "Setup URL:",
        SLACK,
        "› 1. I already Installed it",
        "2. Back",
    ]);
    assert_eq!(asked(&s).len(), 1);
    s.keys("enter");
    // the directory is read again, past the caches, and the sheet is gone
    let asks = asked(&s);
    assert_eq!(asks.len(), 2, "{asks:?}");
    s.assert_absent("Finish App Setup");
    s.assert_absent("Type to search apps");
}

#[test]
fn back_on_the_confirmation_returns_to_the_install_screen_and_asks_nothing() {
    let mut s = loaded(two());
    s.keys("down enter enter");
    s.assert_contains("Finish App Setup");
    s.keys("2");
    s.assert_absent("Finish App Setup");
    s.assert_contains("› 1. Install on ChatGPT");
    assert_eq!(asked(&s).len(), 1);
}

#[test]
fn the_switch_of_an_installed_app_shows_at_once_and_is_written_to_the_config() {
    let mut s = loaded(two());
    s.keys("enter");
    s.keys("2");
    s.assert_contains("2. Enable app");
    s.assert_absent("Disable app");
    assert_eq!(switched(&s), [("notion".to_string(), false)]);
    // and back on
    s.keys("2");
    s.assert_contains("2. Disable app");
    assert_eq!(
        switched(&s),
        [("notion".to_string(), false), ("notion".to_string(), true)]
    );
}

#[test]
fn a_tap_on_the_switch_moves_the_cursor_and_a_second_tap_writes_it() {
    let mut s = loaded(two());
    s.keys("enter");
    s.click_text("Disable app");
    s.assert_contains("› 2. Disable app");
    assert!(switched(&s).is_empty());
    s.wait(500);
    s.click_text("Disable app");
    s.assert_contains("2. Enable app");
    assert_eq!(switched(&s), [("notion".to_string(), false)]);
}
