// test root: pages
//! `/plugins` as a feature's sheet (ADR-013 pilot): the browser is the first route of a nested
//! `Navigator` in the sheet and the detail, the prompts and the confirmations are routes over it.
//! What the page tests pin is the route model: the `Back to plugins` row pops one route and the
//! browser under it still has its cursor, tab and query, because it was never rebuilt. Esc follows
//! the oracle page by page: it closes the sheet on the browser and on the detail (the oracle's
//! detail has no `on_cancel`) and pops only where the oracle goes back. The oracle's screens of the same pages are the shard
//! `chat_popups_3`; the feature's own rules (the ops, the late reply, the interest) are
//! `core/tests/plugins.rs`.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_tui_core::features::plugins::Want;

#[path = "../oracle/fixtures/chat_popups_3/seed.rs"]
#[allow(dead_code)]
mod seed;

use crate::kit::Screen;

fn plugin(id: &str, name: &str, display: &str) -> serde_json::Value {
    seed::summary(
        id,
        name,
        Some(display),
        Some("Does a thing."),
        false,
        true,
        "AVAILABLE",
    )
}

/// `/plugins` over three plugins of the OpenAI Curated marketplace, each with a detail.
fn three() -> Screen {
    let plugins = vec![
        plugin("plugin-alpha", "alpha", "Alpha"),
        plugin("plugin-bravo", "bravo", "Bravo"),
        plugin("plugin-charlie", "charlie", "Charlie"),
    ];
    let shelf = seed::shelf(vec![seed::curated(plugins.clone())]);
    seed::with(&shelf, |held| {
        for summary in plugins {
            let name = summary["name"].as_str().unwrap_or_default().to_string();
            held.details.insert(
                name,
                seed::detail(summary, Some("What it does."), &[], &[], &[], &[]),
            );
        }
    });
    seed::open(100, 24, seed::config(), &shelf)
}

/// The row the cursor is on (the row that starts with the arrow).
fn cursor(s: &Screen) -> String {
    s.rows()
        .into_iter()
        .find(|row| row.trim_start().starts_with('›'))
        .unwrap_or_default()
}

fn reads(s: &Screen) -> usize {
    seed::wants(s)
        .iter()
        .filter(|want| matches!(want, Want::Read(_)))
        .count()
}

#[test]
fn back_in_the_detail_returns_to_the_browser_with_the_cursor_where_it_was() {
    let mut s = three();
    s.keys("down").keys("down");
    assert!(cursor(&s).contains("Charlie"), "{}", cursor(&s));
    s.keys("enter");
    s.assert_contains("Back to plugins");
    s.click_text("Back to plugins");
    s.assert_absent("Back to plugins");
    s.assert_contains("Type to search plugins");
    assert!(cursor(&s).contains("Charlie"), "{}", cursor(&s));
}

#[test]
fn back_in_the_detail_keeps_the_tab_and_the_query_of_the_browser() {
    let mut s = three();
    s.keys("right").keys("right");
    s.assert_contains("OpenAI Curated marketplace.");
    s.type_text("brav");
    s.assert_absent("Alpha");
    s.keys("enter");
    s.assert_contains("Back to plugins");
    s.click_text("Back to plugins");
    s.assert_contains("OpenAI Curated marketplace.");
    s.assert_contains("brav");
    s.assert_contains("Bravo");
    s.assert_absent("Alpha");
}

#[test]
fn esc_on_the_browser_closes_the_sheet() {
    let mut s = three();
    s.assert_contains("Type to search plugins");
    s.keys("esc");
    s.assert_absent("Type to search plugins");
    s.assert_absent("Bravo");
}

#[test]
fn esc_on_the_detail_closes_the_sheet_as_the_oracle_does() {
    let mut s = three();
    s.click_text("Bravo");
    s.assert_contains("Back to plugins");
    s.keys("esc");
    s.assert_absent("Back to plugins");
    s.assert_absent("Type to search plugins");
}

#[test]
fn a_tap_on_back_in_the_detail_goes_back_and_a_following_esc_closes_the_sheet() {
    let mut s = three();
    s.click_text("Bravo");
    s.click_text("Back to plugins");
    s.assert_absent("Back to plugins");
    assert!(cursor(&s).contains("Bravo"), "{}", cursor(&s));
    s.keys("esc");
    s.assert_absent("Type to search plugins");
}

#[test]
fn a_second_visit_to_the_same_detail_shows_it_again() {
    let mut s = three();
    s.keys("enter");
    s.click_text("Back to plugins");
    s.keys("enter");
    s.assert_contains("Back to plugins");
    // `Intent::Read` is the feature's verb: the page says it each time, the feature decides what
    // to send
    assert!(reads(&s) >= 1, "{:?}", seed::wants(&s));
}

#[test]
fn the_sheet_loads_the_list_when_it_opens_and_not_again_while_it_stays() {
    let mut s = three();
    let lists = |s: &Screen| {
        seed::wants(s)
            .iter()
            .filter(|want| matches!(want, Want::List))
            .count()
    };
    let first = lists(&s);
    assert!(first >= 1, "{:?}", seed::wants(&s));
    s.keys("enter");
    s.click_text("Back to plugins");
    s.keys("right").keys("left");
    assert_eq!(lists(&s), first, "{:?}", seed::wants(&s));
}

/// `/plugins` over a plugin whose install needs the setup of the app `arabica`; the server's
/// directory holds that app as `accessible` and switched off.
fn setup_of_arabica(accessible: bool) -> Screen {
    let linear = seed::set(
        seed::remote_summary(
            "plugins~Plugin_linear",
            "plugin",
            Some("Plugin"),
            Some("Issue tracking."),
            false,
        ),
        &[("installPolicy", serde_json::json!("INSTALLED_BY_DEFAULT"))],
    );
    let apps = serde_json::json!([{
        "id": "arabica", "name": "Arabica", "description": "A cafe",
        "installUrl": "https://example.test/arabica",
        "isAccessible": accessible, "isEnabled": false, "pluginDisplayNames": [],
    }]);
    let shelf = seed::shelf(Vec::new());
    seed::with(&shelf, |held| {
        held.remote = vec![seed::marketplace(
            "workspace-shared-with-me-private",
            None,
            Some("Shared with me"),
            vec![linear.clone()],
        )];
        held.details.insert(
            "plugins~Plugin_linear".to_string(),
            seed::remote_detail("workspace-shared-with-me-private", linear, None),
        );
        held.replies
            .insert("app/list", Ok(serde_json::json!({ "data": apps.clone() })));
        held.replies.insert(
            "plugin/install",
            Ok(serde_json::json!({
                "authPolicy": "ON_INSTALL",
                "appsNeedingAuth": [{ "id": "arabica", "name": "Arabica" }],
            })),
        );
    });
    let mut s = seed::start(80, 24, seed::config(), &shelf);
    s.feed(
        &serde_json::json!({
            "step": "notify",
            "message": { "method": "app/list/updated", "params": { "data": apps } },
        })
        .to_string(),
    );
    s.type_text("/plugins").keys("enter");
    s.keys("enter");
    s.keys("down enter");
    s
}

#[test]
fn the_auth_step_of_an_accessible_but_disabled_app_says_it_is_installed_and_continues() {
    let s = setup_of_arabica(true);
    s.assert_contains("Plugin plugin installed.");
    s.assert_contains("App setup 1/1: Arabica");
    s.assert_contains("Already installed in this session.");
    s.assert_contains("Continue");
    s.assert_contains("Skip remaining app setup");
}

#[test]
fn the_auth_step_of_an_app_the_directory_does_not_hold_as_accessible_is_not_installed() {
    let s = setup_of_arabica(false);
    s.assert_contains("App setup 1/1: Arabica");
    s.assert_absent("Already installed");
}
