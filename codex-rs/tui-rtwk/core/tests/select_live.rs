#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The app's half of `/model` and `/permissions` (S14): the model of the
//! screen says which request its answer must belong to, the app gives it a
//! ticket, sends it as a screen command with no thread and reads the answer
//! into the screen's events: `model/list` for the catalog, and
//! `configRequirements/read` followed by the pages of `permissionProfile/list`
//! (at most ten) for the permission catalog.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::Ticket;
use codex_tui_core::rpc::{Reply, RpcError};
use codex_tui_core::screens::select::models::{Current as ModelCurrent, Preset};
use codex_tui_core::screens::select::permissions::{
    Current as PermsCurrent, Failure, PermissionsMenu, Profile,
};
use codex_tui_core::screens::select::{self, Intent as SelectIntent, Menu};
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, ScreenReply};
use pretty_assertions::assert_eq;
use serde_json::json;

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn open(app: &mut CodexApp, intent: SelectIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))),
    )
}

fn answer(app: &mut CodexApp, ticket: Ticket, reply: select::Reply) -> AppFx {
    run(
        app,
        AppEv::Reply(ticket, Reply::screen(ScreenReply::Select(reply))),
    )
}

/// The one select command in `fx`, with its ticket; no thread rides with it.
fn asked(fx: &AppFx) -> (Ticket, select::Command) {
    let [
        Command::Screen {
            ticket,
            thread,
            command: ScreenCommand::Select(command),
            ..
        },
    ] = fx.commands()
    else {
        panic!("expected one select command, got {:?}", fx.commands());
    };
    assert_eq!(thread.as_str(), "", "an app-scoped ask has no thread");
    (*ticket, command.clone())
}

fn profile(id: &str) -> Profile {
    serde_json::from_value(json!({"id": id})).expect("profile")
}

fn model_menu() -> PermsCurrent {
    PermsCurrent {
        discover: true,
        ..PermsCurrent::default()
    }
}

fn permissions_menu(app: &CodexApp) -> PermissionsMenu {
    match app.screens().select.stack.last() {
        Some(Menu::Permissions(menu)) => (**menu).clone(),
        other => panic!("expected a permissions menu, got {other:?}"),
    }
}

fn presets() -> Vec<Preset> {
    serde_json::from_value(json!([
        {"model": "gpt-5.5", "name": "GPT-5.5", "default_effort": "medium",
         "efforts": [{"effort": "low"}, {"effort": "medium"}]},
        {"model": "gpt-sol", "name": "GPT-Sol", "default_effort": "high",
         "efforts": [{"effort": "high"}]},
    ]))
    .expect("presets")
}

/// `/model` asks for the catalog; the answer fills it and the menu shows.
#[test]
fn model_opens_and_asks_the_server_for_the_catalog() {
    let mut app = CodexApp::new();
    let fx = open(
        &mut app,
        SelectIntent::OpenModels(ModelCurrent::new("gpt-5.5")),
    );
    let (ticket, command) = asked(&fx);
    let select::Command::Models { request } = command else {
        panic!("expected model/list");
    };
    assert_eq!(app.screens().select.catalog.pending, Some(request));
    answer(&mut app, ticket, select::Reply::Models(presets()));
    let select = &app.screens().select;
    assert_eq!(select.catalog.pending, None);
    assert_eq!(select.catalog.presets.len(), 2);
}

/// The answer of an older ask, once the menu was opened again, is dropped:
/// the second ask is the one the screen waits for.
#[test]
fn an_older_model_answer_is_dropped() {
    let mut app = CodexApp::new();
    let first = open(
        &mut app,
        SelectIntent::OpenModels(ModelCurrent::new("gpt-5.5")),
    );
    let (old, _) = asked(&first);
    let second = open(
        &mut app,
        SelectIntent::OpenModels(ModelCurrent::new("gpt-5.5")),
    );
    let (new, command) = asked(&second);
    let select::Command::Models { request } = command else {
        panic!("expected model/list");
    };
    answer(&mut app, old, select::Reply::Models(presets()));
    assert_eq!(app.screens().select.catalog.pending, Some(request));
    assert!(app.screens().select.catalog.presets.is_empty());
    answer(&mut app, new, select::Reply::Models(presets()));
    assert_eq!(app.screens().select.catalog.pending, None);
}

/// A failed `model/list` leaves the catalog as it was and the screen free.
#[test]
fn a_failed_model_list_clears_the_wait() {
    let mut app = CodexApp::new();
    let fx = open(
        &mut app,
        SelectIntent::OpenModels(ModelCurrent::new("gpt-5.5")),
    );
    let (ticket, _) = asked(&fx);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote("down".into()))),
    );
    assert_eq!(app.screens().select.catalog.pending, None);
}

/// `/permissions` reads the requirements, then page after page of the
/// profiles, and only then shows the menu.
#[test]
fn permissions_read_the_requirements_then_every_page() {
    let mut app = CodexApp::new();
    let fx = open(&mut app, SelectIntent::OpenPermissions(model_menu()));
    let (ticket, command) = asked(&fx);
    assert!(matches!(command, select::Command::Requirements { .. }));
    assert!(matches!(
        permissions_menu(&app),
        PermissionsMenu::Loading(_)
    ));

    let fx = answer(&mut app, ticket, select::Reply::Requirements(None));
    let (ticket, command) = asked(&fx);
    let select::Command::Profiles { cursor, .. } = command else {
        panic!("expected the first page");
    };
    assert_eq!(cursor, None);

    let fx = answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: vec![profile(":read-only")],
            next: Some("c2".into()),
        },
    );
    let (ticket, command) = asked(&fx);
    let select::Command::Profiles { cursor, .. } = command else {
        panic!("expected the second page");
    };
    assert_eq!(cursor.as_deref(), Some("c2"));
    assert!(matches!(
        permissions_menu(&app),
        PermissionsMenu::Loading(_)
    ));

    let fx = answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: vec![profile(":workspace")],
            next: None,
        },
    );
    assert!(fx.commands().is_empty());
    assert!(matches!(
        permissions_menu(&app),
        PermissionsMenu::Profiles(_)
    ));
    let found = app
        .screens()
        .select
        .profiles
        .found
        .clone()
        .expect("a catalog");
    assert_eq!(
        found
            .profiles
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        [":read-only", ":workspace"]
    );
}

/// A server whose pages never end: after ten, the menu says so.
#[test]
fn pages_that_never_end_stop_after_ten() {
    let mut app = CodexApp::new();
    let fx = open(&mut app, SelectIntent::OpenPermissions(model_menu()));
    let (ticket, _) = asked(&fx);
    let mut fx = answer(&mut app, ticket, select::Reply::Requirements(None));
    for n in 0..select::MAX_PAGES {
        let (ticket, _) = asked(&fx);
        fx = answer(
            &mut app,
            ticket,
            select::Reply::Profiles {
                profiles: vec![profile(&format!("p{n}"))],
                next: Some(format!("c{n}")),
            },
        );
    }
    assert!(fx.commands().is_empty(), "no eleventh page is asked");
    let PermissionsMenu::Failed { failure, .. } = permissions_menu(&app) else {
        panic!("expected the failure menu");
    };
    assert_eq!(failure, Failure::Pagination);
}

/// A failed read shows the error's own text, and `Retry` asks again.
#[test]
fn a_failed_read_shows_the_error_and_retry_asks_again() {
    let mut app = CodexApp::new();
    let fx = open(&mut app, SelectIntent::OpenPermissions(model_menu()));
    let (ticket, _) = asked(&fx);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote("no route".into()))),
    );
    let PermissionsMenu::Failed { failure, .. } = permissions_menu(&app) else {
        panic!("expected the failure menu");
    };
    assert_eq!(failure, Failure::Other("no route".into()));
    let fx = open(
        &mut app,
        SelectIntent::Choose(select::permissions::RETRY.into()),
    );
    let (_, command) = asked(&fx);
    assert!(matches!(command, select::Command::Requirements { .. }));
    assert!(matches!(
        permissions_menu(&app),
        PermissionsMenu::Loading(_)
    ));
}

/// The opener reads the thread's facts: the status line's model and effort
/// become the `/model` menu's, and a policy and sandbox that equal a
/// built-in preset select it in `/permissions`.
#[test]
fn the_openers_read_the_status_facts() {
    use codex_tui_core::screens::chat::StatusFacts;
    use codex_tui_core::screens::select::permissions::Preset as Built;
    use codex_tui_core::screens::status::Profile as Sandbox;
    let facts = StatusFacts {
        model: Some("gpt-5.5".into()),
        effort: Some("high".into()),
        approval_policy: Some("on-request".into()),
        profile: Sandbox::Workspace,
        auto_review: true,
        ..StatusFacts::default()
    };
    let current = ModelCurrent::from_facts(&facts);
    assert_eq!(current.model, "gpt-5.5");
    assert_eq!(
        current.effort,
        Some(codex_protocol::openai_models::ReasoningEffort::High)
    );
    let perms = PermsCurrent::from_facts(&facts);
    assert_eq!(perms.matches, Some(Built::Auto));
    assert_eq!(perms.profile.as_deref(), Some(":workspace"));
    assert_eq!(perms.reviewer, select::permissions::Reviewer::AutoReview);
    assert!(perms.discover);
    let custom = StatusFacts {
        approval_policy: Some("never".into()),
        profile: Sandbox::Workspace,
        ..StatusFacts::default()
    };
    assert_eq!(PermsCurrent::from_facts(&custom).matches, None);
}
