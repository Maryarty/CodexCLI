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
use codex_tui_core::screens::select::apps::{App, Cache, Status};
use codex_tui_core::screens::select::models::{
    Answer, Current as ModelCurrent, ModelMenu, Onward, Preset,
};
use codex_tui_core::screens::select::permissions::{
    Chosen, Current as PermsCurrent, Failure, Heading, PermissionsMenu, Phase, Profile, Recommend,
};
use codex_tui_core::screens::select::{self, Intent as SelectIntent, Opening};
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
/// The `model/list` that opening `/permissions` warms the catalog with (for
/// the cyber warning of full access) is not counted.
fn asked(fx: &AppFx) -> (Ticket, select::Command) {
    let warmed = |command: &&Command| {
        matches!(
            command,
            Command::Screen {
                command: ScreenCommand::Select(select::Command::Models { .. }),
                ..
            }
        )
    };
    let all: Vec<_> = fx.commands().iter().collect();
    let asks: Vec<_> = if all.len() > 1 {
        all.into_iter().filter(|c| !warmed(c)).collect()
    } else {
        all
    };
    let [
        Command::Screen {
            ticket,
            thread,
            command: ScreenCommand::Select(command),
            ..
        },
    ] = asks.as_slice()
    else {
        panic!("expected one select command, got {:?}", fx.commands());
    };
    assert_eq!(thread.as_str(), "", "an app-scoped ask has no thread");
    (*ticket, command.clone())
}

fn profile(id: &str) -> Profile {
    serde_json::from_value(json!({"id": id})).expect("profile")
}

/// What the pickers said, as the transcript of the thread in use has it
/// (the screen's notes are drained into it).
fn said(app: &CodexApp) -> Vec<select::Note> {
    use codex_tui_core::threads::{Item, NoteKey};
    let mut said: Vec<_> = app
        .active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Select(note) => Some(note.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert!(
        app.screens().select.notes.is_empty(),
        "the screen's notes are told at once"
    );
    said.shrink_to_fit();
    said
}

fn model_menu() -> PermsCurrent {
    PermsCurrent {
        discover: true,
        ..PermsCurrent::default()
    }
}

/// The menu of the last permissions sheet the screen asked for (no presenter runs in these
/// tests: the shows wait).
fn permissions_menu(app: &CodexApp) -> PermissionsMenu {
    let asked = app
        .screens()
        .select
        .shows
        .pending()
        .filter_map(|show| match &show.opening {
            Opening::Permissions(menu) => Some(menu.clone()),
            _ => None,
        })
        .last();
    asked.expect("a permissions sheet was asked for")
}

/// What the menu is showing over what the model holds.
fn phase(app: &CodexApp) -> Phase {
    permissions_menu(app).phase(&app.screens().select.profiles)
}

/// What a sheet on that menu says to the model for the row `id`.
fn verb(app: &CodexApp, id: &str) -> SelectIntent {
    let select = &app.screens().select;
    match permissions_menu(app)
        .choose(&select.profiles, id)
        .expect("a row that can be chosen")
    {
        Chosen::Done(selection) => SelectIntent::SetPermissions(selection),
        Chosen::Retry => SelectIntent::LoadProfiles,
        Chosen::Menu(menu) => panic!("the pick asks first: {menu:?}"),
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
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
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
    // the loading sheet reads its answer under the request it carries
    assert_eq!(select.catalog.answered, Some((request, Answer::Listed)));
}

/// The answer of an older ask, once the menu was opened again, is dropped:
/// the second ask is the one the screen waits for.
#[test]
fn an_older_model_answer_is_dropped() {
    let mut app = CodexApp::new();
    let first = open(
        &mut app,
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
    );
    let (old, _) = asked(&first);
    let second = open(
        &mut app,
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
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
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
    );
    let (ticket, command) = asked(&fx);
    let select::Command::Models { request } = command else {
        panic!("expected model/list");
    };
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote("down".into()))),
    );
    assert_eq!(app.screens().select.catalog.pending, None);
    assert_eq!(
        app.screens().select.catalog.answered,
        Some((request, Answer::Failed))
    );
}

/// The full-access confirmation is the cyber one when the catalog says the
/// thread's model is a cyber model: `/permissions` asks `model/list` once,
/// out of sight, for it.
#[test]
fn the_model_catalog_is_warmed_for_the_cyber_warning() {
    let mut app = CodexApp::new();
    let current = PermsCurrent {
        model: Some("gpt-cyber".into()),
        ..model_menu()
    };
    let fx = open(&mut app, SelectIntent::OpenPermissions(current));
    let models: Vec<_> = all(&fx)
        .into_iter()
        .filter_map(|(ticket, command)| {
            matches!(command, select::Command::Models { .. }).then_some(ticket)
        })
        .collect();
    assert_eq!(models.len(), 1, "one model/list: {:?}", fx.commands());
    let cyber: Preset = serde_json::from_value(json!({
        "model": "gpt-cyber", "name": "Cyber", "default_effort": "high",
        "efforts": [{"effort": "high"}], "specialty": "cyber"
    }))
    .expect("preset");
    answer(&mut app, models[0], select::Reply::Models(vec![cyber]));
    assert_eq!(
        phase(&app),
        Phase::Loading,
        "the answer is not for a picker"
    );
    let (ticket, _) = asked(&fx);
    let fx = answer(&mut app, ticket, select::Reply::Requirements(None));
    let (ticket, _) = asked(&fx);
    answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: vec![profile(":danger-full-access")],
            next: None,
        },
    );
    // the pick asks first; the confirmation reads the model catalog for its warning
    let select = &app.screens().select;
    let Some(Chosen::Menu(confirm)) =
        permissions_menu(&app).choose(&select.profiles, ":danger-full-access")
    else {
        panic!("expected the confirmation");
    };
    assert!(matches!(confirm, PermissionsMenu::FullAccess { .. }));
    assert_eq!(
        confirm.page(&select.profiles, &select.catalog).heading,
        Heading::FullAccess {
            cyber: Some(Recommend::Ask)
        }
    );
}

/// `/permissions` reads the requirements, then page after page of the
/// profiles, and only then shows the menu.
#[test]
fn permissions_read_the_requirements_then_every_page() {
    let mut app = CodexApp::new();
    let fx = open(&mut app, SelectIntent::OpenPermissions(model_menu()));
    let (ticket, command) = asked(&fx);
    assert!(matches!(command, select::Command::Requirements { .. }));
    assert_eq!(phase(&app), Phase::Loading);

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
    assert_eq!(phase(&app), Phase::Loading);

    let fx = answer(
        &mut app,
        ticket,
        select::Reply::Profiles {
            profiles: vec![profile(":workspace")],
            next: None,
        },
    );
    assert!(fx.commands().is_empty());
    assert_eq!(phase(&app), Phase::Listed);
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
    assert_eq!(phase(&app), Phase::Failed);
    assert_eq!(
        app.screens().select.profiles.failure,
        Some(Failure::Pagination)
    );
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
    assert_eq!(phase(&app), Phase::Failed);
    assert_eq!(
        app.screens().select.profiles.failure,
        Some(Failure::Other("no route".into()))
    );
    let verb = verb(&app, select::permissions::RETRY);
    assert_eq!(verb, SelectIntent::LoadProfiles);
    let fx = open(&mut app, verb);
    let (_, command) = asked(&fx);
    assert!(matches!(command, select::Command::Requirements { .. }));
    assert_eq!(phase(&app), Phase::Loading);
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
        agent_threads: Some(8),
        ..StatusFacts::default()
    };
    let current = ModelCurrent::from_facts(&facts);
    assert_eq!(current.model, "gpt-5.5");
    // the config's thread limit rides along: an Ultra pick warns from 8 on
    assert_eq!(current.threads, Some(8));
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

/// A model menu opened on `presets()` in an app with thread `thread-1` active.
fn threaded() -> CodexApp {
    use codex_tui_core::fixtures::script::build;
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    let mut app = build(&format!("{script}\n")).expect("plays").app;
    let fx = open(
        &mut app,
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
    );
    let (ticket, _) = asked(&fx);
    answer(&mut app, ticket, select::Reply::Models(presets()));
    app
}

/// A final pick of a row of the quick `/model` page, as the sheet makes it: the model leads from
/// the row to the `Onward` (`ModelMenu::choose`), the page says the pick (`Intent::Settle`).
fn settle(app: &mut CodexApp, id: &str, session: bool) -> AppFx {
    let menu = ModelMenu::Models(ModelCurrent::new("gpt-5.5"));
    let Onward::Done(selection) = menu.choose(&app.screens().select.catalog, id, session) else {
        panic!("a final pick of {id}");
    };
    open(app, SelectIntent::settle(selection))
}

/// The `/model` sheets the screen asked for and nobody showed yet.
fn model_shows(app: &CodexApp) -> Vec<select::Opening> {
    app.screens()
        .select
        .shows
        .pending()
        .filter(|show| matches!(show.opening, select::Opening::Models(_)))
        .map(|show| show.opening.clone())
        .collect()
}

/// Every select command in `fx`, in order.
fn all(fx: &AppFx) -> Vec<(Ticket, select::Command)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Select(command),
                ..
            } => Some((*ticket, command.clone())),
            _ => None,
        })
        .collect()
}

/// A default pick updates the thread, saves the defaults, and says
/// `Model changed to ..` only once the write was saved.
#[test]
fn a_default_model_pick_updates_the_thread_saves_it_and_then_says_so() {
    let mut app = threaded();
    // `gpt-sol` has one effort: the pick settles at once.
    let fx = settle(&mut app, "gpt-sol", false);
    let asks = all(&fx);
    let [
        (_, select::Command::Settings { thread, patch }),
        (write, select::Command::Write { edits }),
    ] = &asks[..]
    else {
        panic!("expected the settings update and the write, got {asks:?}");
    };
    assert_eq!(thread, "thread-1");
    assert_eq!(patch.model.as_deref(), Some("gpt-sol"));
    let keys: Vec<_> = edits.iter().map(|e| (e.key, e.value.as_deref())).collect();
    assert_eq!(
        keys,
        vec![
            ("model", Some("gpt-sol")),
            ("model_reasoning_effort", Some("high"))
        ]
    );
    assert!(said(&app).is_empty(), "nothing is said before the write");
    answer(
        &mut app,
        *write,
        select::Reply::Written { overridden: false },
    );
    let notes = said(&app);
    assert!(matches!(
        notes.as_slice(),
        [select::Note::ModelChanged { model, session: false, .. }] if model == "gpt-sol"
    ));
}

/// The `s` key picks for the session: the thread updates, nothing is written.
#[test]
fn a_session_pick_updates_the_thread_and_writes_nothing() {
    let mut app = threaded();
    let fx = settle(&mut app, "gpt-sol", true);
    let asks = all(&fx);
    let [(_, select::Command::Settings { patch, .. })] = &asks[..] else {
        panic!("expected only the settings update, got {asks:?}");
    };
    assert_eq!(patch.model.as_deref(), Some("gpt-sol"));
    assert!(matches!(
        said(&app).as_slice(),
        [select::Note::ModelChanged { session: true, .. }]
    ));
}

/// A write a layer above overrides says so, and a failed one says why.
#[test]
fn a_write_that_is_overridden_or_failed_is_reported() {
    let mut app = threaded();
    let fx = settle(&mut app, "gpt-sol", false);
    let (write, _) = all(&fx).pop().expect("a write");
    answer(&mut app, write, select::Reply::Written { overridden: true });
    // The warning is for the detailed transcript only (`add_warning_message`).
    assert!(matches!(
        said(&app).as_slice(),
        [select::Note::ModelChanged { .. }]
    ));
    assert!(!select::Note::Overridden(select::Setting::Model).in_transcript());
    let mut app = threaded();
    let fx = settle(&mut app, "gpt-sol", false);
    let (write, _) = all(&fx).pop().expect("a write");
    run(
        &mut app,
        AppEv::Reply(write, Reply::error(RpcError::Remote("read-only".into()))),
    );
    assert_eq!(
        said(&app),
        vec![select::Note::SaveFailed {
            setting: select::Setting::Model,
            error: "read-only".into()
        }]
    );
}

/// A permission pick changes the thread's profile, policy and reviewer for
/// its next turns and says `Permissions updated to ..`.
#[test]
fn a_permission_pick_updates_the_thread_and_says_so() {
    use codex_tui_core::fixtures::script::build;
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    let mut app = build(&format!("{script}\n")).expect("plays").app;
    open(
        &mut app,
        SelectIntent::OpenPermissions(PermsCurrent::default()),
    );
    let verb = verb(&app, ":workspace");
    let fx = open(&mut app, verb);
    let asks = all(&fx);
    let [(_, select::Command::Settings { thread, patch })] = &asks[..] else {
        panic!("expected one settings update, got {asks:?}");
    };
    assert_eq!(thread, "thread-1");
    assert_eq!(patch.permissions.as_deref(), Some(":workspace"));
    assert!(matches!(
        said(&app).as_slice(),
        [select::Note::PermissionsUpdated(_)]
    ));
}

/// A catalog pick of a thread is asked of the server and said once it took
/// it (`Permission selection requested: ..`); the profile the thread has is
/// neither asked nor said.
#[test]
fn a_catalog_pick_is_said_after_the_server_took_it() {
    use codex_tui_core::fixtures::script::build;
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    for (active, expected) in [(None, 1), (Some(":workspace"), 0)] {
        let mut app = build(&format!("{script}\n")).expect("plays").app;
        let fx = open(
            &mut app,
            SelectIntent::OpenPermissions(PermsCurrent {
                discover: true,
                profile: active.map(Into::into),
                approval: active.map(|_| codex_app_server_protocol::AskForApproval::OnRequest),
                ..PermsCurrent::default()
            }),
        );
        let (ticket, _) = asked(&fx);
        let fx = answer(&mut app, ticket, select::Reply::Requirements(None));
        let (ticket, _) = asked(&fx);
        answer(
            &mut app,
            ticket,
            select::Reply::Profiles {
                profiles: vec![profile(":workspace")],
                next: None,
            },
        );
        let verb = verb(&app, ":workspace");
        let fx = open(&mut app, verb);
        let asks = all(&fx);
        assert_eq!(asks.len(), expected, "{asks:?}");
        assert!(said(&app).is_empty(), "nothing before the answer");
        if let Some((ticket, _)) = asks.first() {
            answer(&mut app, *ticket, select::Reply::Updated);
            assert!(matches!(
                said(&app).as_slice(),
                [select::Note::PermissionSelectionRequested(_)]
            ));
        }
    }
}

fn app_of(id: &str, installed: bool, enabled: bool, url: bool) -> App {
    App {
        id: id.to_string(),
        name: id.to_uppercase(),
        description: None,
        install_url: url.then(|| format!("https://x/{id}")),
        installed,
        enabled,
    }
}

fn apps_cache(app: &CodexApp) -> Cache {
    app.screens().select.apps.cache.clone()
}

/// The sheets the screen asked for and nobody has shown, oldest first, the `/model` ones apart
/// (`threaded` opened that family to fill the catalog and nobody showed it: `model_shows`).
fn shows(app: &CodexApp) -> Vec<select::Opening> {
    app.screens()
        .select
        .shows
        .pending()
        .filter(|show| !matches!(show.opening, select::Opening::Models(_)))
        .map(|show| show.opening.clone())
        .collect()
}

fn apps_command(fx: &AppFx) -> (Ticket, u64, bool, Option<String>) {
    match asked(fx) {
        (
            ticket,
            select::Command::Apps {
                request,
                force,
                thread,
            },
        ) => (ticket, request, force, thread),
        (_, other) => panic!("expected app/list, got {other:?}"),
    }
}

/// `/apps` asks for its sheet (which reads the directory as a fact: loading) and asks `app/list`
/// forced for the displayed thread; the answer fills the directory in place.
#[test]
fn apps_open_loading_then_show_the_directory() {
    let mut app = threaded();
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (ticket, _, force, thread) = apps_command(&fx);
    assert!(force);
    assert_eq!(thread.as_deref(), Some("thread-1"));
    assert_eq!(shows(&app), vec![select::Opening::Apps]);
    assert_eq!(apps_cache(&app), Cache::Loading);
    assert_eq!(
        app.screens().select.apps.page(),
        select::apps::Page::Loading
    );
    answer(
        &mut app,
        ticket,
        select::Reply::Apps(vec![app_of("notion", true, true, true)]),
    );
    let select::apps::Page::Apps {
        installed, total, ..
    } = app.screens().select.apps.page()
    else {
        panic!("expected the directory page");
    };
    assert_eq!((installed, total), (1, 1));
}

/// The link dialog's switch writes `apps."<id>".enabled` and `disabled_reason` to the config
/// (`SetAppEnabled`) and the directory takes it once the write is saved; a refused write leaves
/// it as it was.
#[test]
fn an_app_switch_is_written_to_the_config_and_then_shown_in_the_directory() {
    use codex_app_server_protocol::{ClientRequest, RequestId};
    let mut app = threaded();
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (ticket, ..) = apps_command(&fx);
    answer(
        &mut app,
        ticket,
        select::Reply::Apps(vec![app_of("notion", true, true, true)]),
    );

    let fx = open(
        &mut app,
        SelectIntent::SetApp {
            id: "notion".into(),
            enabled: false,
        },
    );
    let (ticket, command) = asked(&fx);
    assert_eq!(
        command,
        select::Command::SetApp {
            id: "notion".into(),
            enabled: false
        }
    );
    let ClientRequest::ConfigBatchWrite { params, .. } =
        select::wire(&command, &Default::default(), RequestId::Integer(1))
    else {
        panic!("a config write");
    };
    let written: Vec<_> = params
        .edits
        .iter()
        .map(|edit| (edit.key_path.as_str(), edit.value.clone()))
        .collect();
    assert_eq!(
        written,
        vec![
            ("apps.\"notion\".enabled", json!(false)),
            ("apps.\"notion\".disabled_reason", json!("user")),
        ]
    );
    // not shown switched before the config has it
    assert_eq!(
        app.screens().select.apps.ready().expect("ready")[0].status(),
        Status::Installed
    );
    answer(
        &mut app,
        ticket,
        select::Reply::Written { overridden: false },
    );
    assert_eq!(
        app.screens().select.apps.ready().expect("ready")[0].status(),
        Status::Disabled
    );

    // on again: both keys are cleared
    let fx = open(
        &mut app,
        SelectIntent::SetApp {
            id: "notion".into(),
            enabled: true,
        },
    );
    let (ticket, command) = asked(&fx);
    let ClientRequest::ConfigBatchWrite { params, .. } =
        select::wire(&command, &Default::default(), RequestId::Integer(2))
    else {
        panic!("a config write");
    };
    assert!(params.edits.iter().all(|edit| edit.value.is_null()));
    app_failed(&mut app, ticket);
    assert_eq!(
        app.screens().select.apps.ready().expect("ready")[0].status(),
        Status::Disabled,
        "a refused write leaves the switch as it was"
    );
}

fn app_failed(app: &mut CodexApp, ticket: Ticket) {
    run(
        app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote("refused".into()))),
    );
}

/// Disabled apps say so; a ready empty directory, a failed one and the
/// feature off are notes, and no sheet is asked for.
#[test]
fn apps_that_cannot_open_say_why() {
    let mut app = threaded();
    open(&mut app, SelectIntent::OpenApps { enabled: false });
    assert_eq!(said(&app), vec![select::Note::AppsDisabled]);
    assert_eq!(shows(&app), vec![]);

    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (ticket, ..) = apps_command(&fx);
    answer(&mut app, ticket, select::Reply::Apps(vec![]));
    assert_eq!(shows(&app).len(), 1, "the first /apps asked for its sheet");
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    assert_eq!(said(&app).last(), Some(&select::Note::NoApps));
    assert_eq!(shows(&app).len(), 1, "an empty directory asks for none");
    // Still asked for again, forced: the cache is ready.
    assert!(apps_command(&fx).2);
}

/// A first failure replaces the loading menu by the failure and its Retry
/// row, which asks again and shows loading; a failure after a ready
/// directory keeps it.
#[test]
fn a_failed_apps_read_offers_retry_and_a_ready_directory_survives_failure() {
    let mut app = threaded();
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (ticket, ..) = apps_command(&fx);
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::error(RpcError::Remote("403 Forbidden".into())),
        ),
    );
    assert!(matches!(apps_cache(&app), Cache::Failed(_)));
    assert_eq!(app.screens().select.apps.page(), select::apps::Page::Failed);
    // the Retry row of the sheet says the verb: the directory is read again
    let fx = open(&mut app, SelectIntent::RefreshApps);
    let (ticket, _, force, _) = apps_command(&fx);
    assert!(force);
    assert_eq!(apps_cache(&app), Cache::Loading);
    answer(
        &mut app,
        ticket,
        select::Reply::Apps(vec![app_of("a", true, true, true)]),
    );
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (ticket, ..) = apps_command(&fx);
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::error(RpcError::Remote("boom".into()))),
    );
    assert!(matches!(apps_cache(&app), Cache::Ready(_)));
}

/// A second `/apps` while one runs is not asked twice; a forced one (the
/// directory is ready) follows the first.
#[test]
fn a_refresh_in_flight_queues_the_forced_one() {
    let mut app = threaded();
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (first, ..) = apps_command(&fx);
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    assert!(all(&fx).is_empty(), "one request at a time");
    let fx = answer(
        &mut app,
        first,
        select::Reply::Apps(vec![app_of("a", true, true, true)]),
    );
    assert!(all(&fx).is_empty(), "an unforced ask leaves nothing behind");

    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (second, _, force, _) = apps_command(&fx);
    assert!(force, "a ready directory is read past the caches");
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    assert!(all(&fx).is_empty());
    let fx = answer(
        &mut app,
        second,
        select::Reply::Apps(vec![app_of("a", true, true, true)]),
    );
    let (third, _, force, _) = apps_command(&fx);
    assert!(force, "the forced ask waited for the first");
    answer(
        &mut app,
        third,
        select::Reply::Apps(vec![app_of("a", true, true, true)]),
    );
}

/// An app without a page says so (the sheet decides from `App::link`, the model has the words).
#[test]
fn picking_an_app_with_no_page_says_so() {
    let mut app = threaded();
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (ticket, ..) = apps_command(&fx);
    answer(
        &mut app,
        ticket,
        select::Reply::Apps(vec![
            app_of("a", true, true, true),
            app_of("b", false, true, false),
        ]),
    );
    let directory = &app.screens().select.apps;
    assert!(directory.app("a").and_then(App::link).is_some());
    assert!(directory.app("b").and_then(App::link).is_none());
    open(&mut app, SelectIntent::AppUnlinked("b".into()));
    assert_eq!(
        said(&app),
        vec![select::Note::AppLinkUnavailable(Status::Installable)]
    );
}

/// The link dialog is the sheet's (it copies the page of the app picked); what its actions ask of
/// the app are verbs: open the page in the browser, switch the app (written to the config, said
/// when refused, the directory only following a saved write), read the directory again after an
/// install.
#[test]
fn an_app_link_dialog_asks_the_app_to_open_the_page_switch_and_read_again() {
    use codex_tui_core::screens::select::apps::Link;
    let mut app = threaded();
    let fx = open(&mut app, SelectIntent::OpenApps { enabled: true });
    let (ticket, ..) = apps_command(&fx);
    answer(
        &mut app,
        ticket,
        select::Reply::Apps(vec![
            app_of("a", true, true, true),
            app_of("b", false, true, false),
        ]),
    );
    assert_eq!(
        app.screens().select.apps.app("a").and_then(App::link),
        Some(Link {
            id: "a".into(),
            name: "A".into(),
            description: None,
            url: "https://x/a".into(),
            installed: true,
            enabled: true,
        })
    );

    // the page opens in the browser; an app with no page opens nothing
    let fx = open(&mut app, SelectIntent::OpenAppPage("a".into()));
    assert!(
        matches!(fx.commands(), [Command::Effect { ask: codex_tui_core::app::effects::Ask::OpenUrl { url, .. }, .. }] if url == "https://x/a"),
        "{:?}",
        fx.commands()
    );
    let fx = open(&mut app, SelectIntent::OpenAppPage("b".into()));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());

    // the switch is written; a refused write is said and leaves the directory as it was (the
    // dialog keeps the switch it shows: it is the sheet's copy)
    let fx = open(
        &mut app,
        SelectIntent::SetApp {
            id: "a".into(),
            enabled: false,
        },
    );
    let (ticket, _) = asked(&fx);
    app_failed(&mut app, ticket);
    assert_eq!(
        app.screens().select.apps.ready().expect("ready")[0].status(),
        Status::Installed
    );
    assert_eq!(
        said(&app),
        vec![select::Note::SaveFailed {
            setting: select::Setting::App("a".into()),
            error: "refused".into(),
        }]
    );

    // an install confirmed: the directory is read again, forced
    let fx = open(&mut app, SelectIntent::RefreshApps);
    let (_, _, force, _) = apps_command(&fx);
    assert!(force);
}

/// A switched app keeps its switch over a refreshed answer; a notified
/// directory equal to the held one is not read again, a different one is.
#[test]
fn a_switch_survives_a_refresh_and_only_a_different_notification_asks() {
    let mut model = select::Model::default();
    let mut fx = AppFx::default();
    let dir = vec![app_of("a", true, true, true)];
    let first = model.apps.open().expect("asks").request;
    select::apply(
        &mut model,
        select::Ev::AppsLoaded {
            request: first,
            result: Ok(dir.clone()),
        },
        &mut fx,
    );
    select::apply(&mut model, select::Ev::AppsNotified(dir.clone()), &mut fx);
    assert_eq!(model.apps.pending, None, "equal to the held directory");
    select::apply(
        &mut model,
        select::Ev::AppSwitched {
            id: "a".into(),
            enabled: false,
        },
        &mut fx,
    );
    assert_eq!(
        model.apps.ready().expect("ready")[0].status(),
        Status::Disabled
    );
    select::apply(&mut model, select::Ev::AppsNotified(dir.clone()), &mut fx);
    let second = model
        .apps
        .pending
        .expect("differs from the held one: read again");
    assert!(!second.force);
    select::apply(
        &mut model,
        select::Ev::AppsLoaded {
            request: second.request,
            result: Ok(dir),
        },
        &mut fx,
    );
    assert_eq!(
        model.apps.ready().expect("ready")[0].status(),
        Status::Disabled
    );
}

/// A default-provider write (`model_picker_unavailable`, `dismiss_stale_model_picker`): the
/// request in flight is dropped and the sheet of `/model` not yet shown is forgotten (one shown
/// reads `Catalog::provider` and closes itself), `/model` says why it is blocked and asks
/// nothing, and the answer of the dropped request is not read.
#[test]
fn a_provider_write_dismisses_the_model_menus_and_blocks_the_picker() {
    use codex_tui_core::screens::ScreenEv;
    use codex_tui_core::screens::select::Provider;
    let mut app = CodexApp::new();
    let fx = open(
        &mut app,
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
    );
    let (ticket, _) = asked(&fx);
    assert!(matches!(
        model_shows(&app).as_slice(),
        [select::Opening::Models(ModelMenu::Loading { .. })]
    ));
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Select(select::Ev::Provider(Provider::Changed))),
    );
    assert_eq!(app.screens().select.catalog.provider, Provider::Changed);
    assert_eq!(
        model_shows(&app),
        [],
        "the sheet not shown yet is forgotten"
    );
    assert_eq!(app.screens().select.catalog.pending, None);
    // the answer in flight is dropped
    let fx = answer(&mut app, ticket, select::Reply::Models(Vec::new()));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    assert_eq!(app.screens().select.catalog.answered, None);
    // `/model` is blocked with the restart words and sends nothing
    let fx = open(
        &mut app,
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
    );
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    assert_eq!(model_shows(&app), []);
    let fx = open(&mut app, SelectIntent::LoadModels);
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
    // while a write is in flight it says so; settled, the picker opens again
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Select(select::Ev::Provider(Provider::Saving))),
    );
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Select(select::Ev::Provider(Provider::Settled))),
    );
    let fx = open(
        &mut app,
        SelectIntent::open_models(ModelCurrent::new("gpt-5.5")),
    );
    assert_eq!(fx.commands().len(), 1, "{:?}", fx.commands());
    assert!(matches!(
        model_shows(&app).as_slice(),
        [select::Opening::Models(ModelMenu::Loading { .. })]
    ));
}
