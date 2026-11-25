#![allow(clippy::expect_used)]

//! The apps directory notification and the scope boundary (core-pass for chunk-popups-apps-rest):
//! `app/list/updated` with a thread on display reads the directory again and drops the apps a
//! user cannot call from the mentions; an account change or another displayed thread forgets the
//! directory and ends the sheet that shows it (the scope the sheet opened under is another one:
//! `Apps::scope`; oracle `refresh_connector_directory_after_notification`,
//! `invalidate_connector_scope`, `connectors_for_mentions`).

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::select::apps::Cache;
use serde_json::json;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn mentions() -> String {
    format!(
        r#"{{"step":"screen","dir":"popovers","ev":{}}}"#,
        json!({"mentions": [
            {"kind":"skill","name":"Skill","description":"","insert":"skill","path":"/s/SKILL.md"},
            {"kind":"app","name":"Notion","description":"","insert":"notion","path":"app://notion"},
            {"kind":"app","name":"Linear","description":"","insert":"linear","path":"app://linear"},
        ]})
    )
}

fn listed(apps: &[(&str, bool)]) -> String {
    let data: Vec<_> = apps
        .iter()
        .map(|(id, accessible)| {
            json!({"id": id, "name": id, "isAccessible": accessible, "isEnabled": true,
                   "pluginDisplayNames": []})
        })
        .collect();
    format!(
        r#"{{"step":"notify","message":{}}}"#,
        json!({"method": "app/list/updated", "params": {"data": data}})
    )
}

fn listed_enabled(apps: &[(&str, bool)]) -> String {
    let data: Vec<_> = apps
        .iter()
        .map(|(id, enabled)| {
            json!({"id": id, "name": id, "isAccessible": true, "isEnabled": enabled,
                   "pluginDisplayNames": []})
        })
        .collect();
    format!(
        r#"{{"step":"notify","message":{}}}"#,
        json!({"method": "app/list/updated", "params": {"data": data}})
    )
}

fn names(sandbox: &Sandbox) -> Vec<String> {
    sandbox
        .app()
        .screens()
        .popovers
        .mentions
        .iter()
        .map(|mention| mention.name.clone())
        .collect()
}

#[test]
fn a_directory_notification_drops_the_apps_a_user_cannot_call_from_the_mentions() {
    let script = [
        SESSION.to_string(),
        mentions(),
        listed(&[("notion", true), ("linear", false)]),
    ]
    .join("\n");
    let sandbox = Sandbox::play(&script).expect("plays");
    assert_eq!(names(&sandbox), vec!["Skill", "Notion"]);
}

#[test]
fn a_directory_notification_drops_an_accessible_but_disabled_app_from_the_mentions() {
    let script = [
        SESSION.to_string(),
        mentions(),
        listed_enabled(&[("notion", true), ("linear", false)]),
    ]
    .join("\n");
    let sandbox = Sandbox::play(&script).expect("plays");
    assert_eq!(names(&sandbox), vec!["Skill", "Notion"]);
}

#[test]
fn a_directory_notification_reads_the_directory_again() {
    let script = [SESSION.to_string(), listed(&[("notion", true)])].join("\n");
    let sandbox = Sandbox::play(&script).expect("plays");
    let apps = &sandbox.app().screens().select.apps;
    assert!(
        apps.pending.is_some(),
        "the notification's directory is never taken as it is: app/list follows"
    );
    assert_eq!(apps.cache, Cache::Loading);
}

#[test]
fn an_account_change_forgets_the_directory_and_ends_the_apps_sheet() {
    let script = [
        SESSION.to_string(),
        listed(&[("notion", true)]),
        r#"{"step":"notify","message":{"method":"account/updated","params":{"authMode":null,"planType":null}}}"#.to_string(),
    ]
    .join("\n");
    let sandbox = Sandbox::play(&script).expect("plays");
    let select = &sandbox.app().screens().select;
    assert_eq!(select.apps.cache, Cache::Uninitialized);
    assert_eq!(select.apps.pending, None);
    assert_eq!(
        select.apps.scope(),
        1,
        "the sheet over the directory is of another scope now"
    );
}

fn list_apps(sandbox: &mut Sandbox) {
    sandbox.send(AppEv::Intent(codex_tui_core::app::Intent::Screen(
        codex_tui_core::screens::ScreenIntent::Popovers(
            codex_tui_core::screens::popovers::Intent::ListApps,
        ),
    )));
}

/// The first `$` popup reads the directory once (the oracle reads it eagerly): a second one while
/// it is read or held asks nothing more.
#[test]
fn the_first_dollar_popup_reads_the_directory_once() {
    let mut sandbox = Sandbox::play(SESSION).expect("plays");
    assert!(sandbox.app().screens().select.apps.pending.is_none());
    list_apps(&mut sandbox);
    let asked = sandbox.app().screens().select.apps.pending;
    assert!(asked.is_some(), "app/list is asked");
    list_apps(&mut sandbox);
    assert_eq!(sandbox.app().screens().select.apps.pending, asked);
}

/// With the apps feature off nothing is asked.
#[test]
fn the_dollar_popup_asks_no_directory_without_the_apps_feature() {
    let script = [
        SESSION.to_string(),
        r#"{"step":"screen","dir":"chat","ev":{"flags":{"connectors_enabled":false}}}"#.to_string(),
    ]
    .join("\n");
    let mut sandbox = Sandbox::play(&script).expect("plays");
    list_apps(&mut sandbox);
    assert!(sandbox.app().screens().select.apps.pending.is_none());
}

/// A new scope forgets the directory: the next `$` reads it again.
#[test]
fn the_dollar_popup_reads_the_directory_again_after_a_scope_change() {
    let script = [
        SESSION.to_string(),
        r#"{"step":"notify","message":{"method":"account/updated","params":{"authMode":null,"planType":null}}}"#.to_string(),
    ]
    .join("\n");
    let mut sandbox = Sandbox::play(&script).expect("plays");
    assert_eq!(
        sandbox.app().screens().select.apps.cache,
        Cache::Uninitialized
    );
    list_apps(&mut sandbox);
    assert!(sandbox.app().screens().select.apps.pending.is_some());
}
