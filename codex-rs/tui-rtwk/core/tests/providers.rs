#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The model provider list and editor (oracle `app/provider.rs`, `app/provider/`, tests
//! `provider_tests.rs`, `provider/editor_tests.rs`) as an ADR-013 feature: the list read while a
//! surface is interested, a write of a provider's settings or of the default for new chats, what
//! the server's refusal means (`refusal`), the late answer dropped, and the form's patch
//! (`form::Draft`). Every step is a verb with data; the words are the view's.

use std::collections::BTreeMap;

use codex_app_server_protocol::{
    GatewayAuth, ModelProviderConfigWriteParams, ModelProviderListResponse, ModelProviderSummary,
    ProviderSettings,
};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::providers::form::{Draft, Field, Invalid};
use codex_tui_core::features::providers::{
    self, Failure, Interest, Load, Note, Outcome, Saved, Saving, Trouble, refusal,
};
use codex_tui_core::features::{
    self, FeatureCommand, FeatureIntent, FeatureInterest, FeatureNote, InterestStep,
};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::protocol::reply_from_server;
use codex_tui_core::rpc::{Reply, ServerError};
use codex_tui_core::threads::{Item, Note as ThreadNote, NoteKey};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new("thread-1"))),
    );
    app
}

fn verb(app: &mut CodexApp, intent: providers::Intent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
    )
}

fn interest(app: &mut CodexApp, which: Interest, open: bool) -> AppFx {
    let which = FeatureInterest::from(which);
    let step = if open {
        InterestStep::Open(which)
    } else {
        InterestStep::Close(which)
    };
    run(app, AppEv::Intent(Intent::Interest(step)))
}

fn state(app: &CodexApp) -> &providers::State {
    &app.features().providers
}

/// The requests of the feature a step left, with their tickets.
fn sent(fx: &AppFx) -> Vec<(Ticket, providers::Command)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Feature {
                ticket,
                command: FeatureCommand::Providers(command),
                ..
            } => Some((*ticket, command.clone())),
            _ => None,
        })
        .collect()
}

fn one(fx: &AppFx) -> (Ticket, providers::Command) {
    let all = sent(fx);
    let [only] = all.as_slice() else {
        panic!("expected one request, got {all:?}");
    };
    only.clone()
}

fn answer(app: &mut CodexApp, request: &(Ticket, providers::Command), result: Value) -> AppFx {
    let command = FeatureCommand::Providers(request.1.clone());
    let reply = features::decode(&command, result)
        .unwrap_or_else(|error| panic!("{:?} did not decode: {error:?}", request.1));
    run(app, AppEv::Reply(request.0, Reply::feature(reply)))
}

fn fail(app: &mut CodexApp, request: &(Ticket, providers::Command), text: &str) -> AppFx {
    run(app, AppEv::Reply(request.0, Reply::failed(text)))
}

fn refused(app: &mut CodexApp, fx: &AppFx, code: Option<i64>, message: &str) -> AppFx {
    let command = fx
        .commands()
        .iter()
        .find(|c| {
            matches!(
                c,
                Command::Feature {
                    command: FeatureCommand::Providers(_),
                    ..
                }
            )
        })
        .expect("a provider request")
        .clone();
    let error = ServerError {
        code,
        message: message.to_string(),
        data: None,
    };
    let (ticket, reply) = reply_from_server(&command, Err(error)).expect("a reply");
    run(app, AppEv::Reply(ticket, reply))
}

fn notes(app: &CodexApp) -> Vec<Note> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(ThreadNote {
                key: NoteKey::Feature(FeatureNote::Providers(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

fn catalog(default: &str) -> Value {
    json!({
        "data": [
            {"id": "gateway", "name": "Example Gateway", "canEdit": true,
             "baseUrl": "http://127.0.0.1:9/v1",
             "settings": {"type": "customGateway", "auth": {"mode": "none"}}},
            {"id": "openai", "name": "OpenAI", "canEdit": false, "baseUrl": null,
             "settings": {"type": "openai"}},
        ],
        "defaultProviderId": default,
        "canChangeDefault": true,
    })
}

fn written() -> Value {
    json!({"status": "ok", "version": "1", "filePath": "/home/me/.codex/config.toml",
           "overriddenMetadata": null})
}

/// The sheet is open and the list read.
fn listed(default: &str) -> CodexApp {
    let mut app = ready();
    let fx = interest(&mut app, Interest::List, true);
    let request = one(&fx);
    assert_eq!(request.1, providers::Command::List);
    assert_eq!(state(&app).load(), &Load::Loading);
    answer(&mut app, &request, catalog(default));
    app
}

fn page(app: &CodexApp) -> (ModelProviderListResponse, Option<Outcome>) {
    match state(app).load() {
        Load::Ready(page) => (page.list.clone(), page.notice.clone()),
        other => panic!("expected a list, got {other:?}"),
    }
}

#[test]
fn the_list_is_read_when_a_surface_wants_it_and_dropped_when_it_lets_go() {
    let mut app = listed("openai");
    assert_eq!(page(&app).0.default_provider_id, "openai");
    interest(&mut app, Interest::List, false);
    assert_eq!(state(&app).load(), &Load::Idle);
}

#[test]
fn a_late_answer_of_a_sheet_that_closed_is_dropped() {
    let mut app = ready();
    let fx = interest(&mut app, Interest::List, true);
    let late = one(&fx);
    interest(&mut app, Interest::List, false);
    let fx = interest(&mut app, Interest::List, true);
    let current = one(&fx);
    answer(&mut app, &late, catalog("openai"));
    assert_eq!(state(&app).load(), &Load::Loading);
    answer(&mut app, &current, catalog("gateway"));
    assert_eq!(page(&app).0.default_provider_id, "gateway");
}

#[test]
fn a_server_without_the_list_is_unsupported() {
    for (code, message) in [
        (-32601, "Method not found"),
        (
            -32600,
            "modelProvider/list failed: Invalid request: unknown variant `modelProvider/list`",
        ),
    ] {
        let mut app = ready();
        let fx = interest(&mut app, Interest::List, true);
        refused(&mut app, &fx, Some(code), message);
        assert_eq!(state(&app).load(), &Load::Failed(Failure::Unsupported));
    }
}

#[test]
fn any_other_failure_of_the_list_is_the_servers_text() {
    let mut app = ready();
    let fx = interest(&mut app, Interest::List, true);
    fail(&mut app, &one(&fx), "boom");
    let Load::Failed(Failure::Server(text)) = state(&app).load() else {
        panic!("expected a failure, got {:?}", state(&app).load());
    };
    assert!(text.contains("boom"), "{text}");
}

#[test]
fn the_default_is_written_once_and_the_list_read_again_with_the_outcome() {
    let mut app = listed("openai");
    interest(&mut app, Interest::Saving, true);
    let fx = verb(&mut app, providers::Intent::Choose("gateway".into()));
    let write = one(&fx);
    assert_eq!(write.1, providers::Command::SetDefault("gateway".into()));
    assert_eq!(
        state(&app).saving(),
        Some(&Saving::Default("gateway".into()))
    );
    // a second press while one write is under way is told, not sent
    let fx = verb(&mut app, providers::Intent::Choose("openai".into()));
    assert!(sent(&fx).is_empty());
    assert_eq!(notes(&app), vec![Note::Busy]);
    let fx = answer(&mut app, &write, written());
    let again = one(&fx);
    assert_eq!(again.1, providers::Command::List);
    assert_eq!(state(&app).saving(), None);
    answer(&mut app, &again, catalog("gateway"));
    let (list, notice) = page(&app);
    assert_eq!(list.default_provider_id, "gateway");
    assert_eq!(
        notice,
        Some(Outcome::Saved(Saved::Default {
            id: "gateway".into()
        }))
    );
}

#[test]
fn a_write_that_finishes_after_the_page_went_is_said_in_the_transcript() {
    let mut app = listed("openai");
    let fx = verb(&mut app, providers::Intent::Choose("gateway".into()));
    let write = one(&fx);
    // the user left the page that waited for the write
    let fx = answer(&mut app, &write, written());
    assert_eq!(one(&fx).1, providers::Command::List);
    assert_eq!(
        notes(&app),
        vec![Note::Outcome(Outcome::Saved(Saved::Default {
            id: "gateway".into()
        }))]
    );
}

#[test]
fn an_overridden_default_keeps_the_reason_the_server_gave() {
    let mut app = listed("openai");
    interest(&mut app, Interest::Saving, true);
    let fx = verb(&mut app, providers::Intent::Choose("gateway".into()));
    let write = one(&fx);
    let fx = answer(
        &mut app,
        &write,
        json!({"status": "okOverridden", "version": "1", "filePath": "/c.toml",
               "overriddenMetadata": {"message": "Set by managed config.",
                                      "overridingLayer": {"name": {"type": "system", "file": "/etc/c.toml"}, "version": "1"},
                                      "effectiveValue": "openai"}}),
    );
    answer(&mut app, &one(&fx), catalog("openai"));
    assert_eq!(
        page(&app).1,
        Some(Outcome::Saved(Saved::Overridden {
            id: "gateway".into(),
            reason: Some("Set by managed config.".into())
        }))
    );
}

#[test]
fn a_lost_reply_of_the_default_may_be_a_committed_write() {
    let mut app = listed("openai");
    interest(&mut app, Interest::Saving, true);
    let fx = verb(&mut app, providers::Intent::Choose("gateway".into()));
    let fx = refused(&mut app, &fx, None, "connection lost");
    // the page that waited reads the list again and shows what the write may have done
    assert_eq!(one(&fx).1, providers::Command::List);
    answer(&mut app, &one(&fx), catalog("gateway"));
    assert_eq!(
        page(&app).1,
        Some(Outcome::Trouble(Trouble::Lost {
            id: "gateway".into(),
            error: "connection lost".into()
        }))
    );
}

fn params(settings: Value) -> ModelProviderConfigWriteParams {
    ModelProviderConfigWriteParams {
        settings: serde_json::from_value(settings).unwrap(),
    }
}

fn create() -> ModelProviderConfigWriteParams {
    params(json!({"type": "customGateway", "name": "Team", "baseUrl": "https://t.example"}))
}

fn edit() -> ModelProviderConfigWriteParams {
    params(json!({"type": "customGateway", "providerId": "team", "name": "Team"}))
}

#[test]
fn only_a_create_can_be_unconfirmed() {
    let error = |code| ServerError {
        code,
        message: "Internal error".into(),
        data: None,
    };
    let write = |p| providers::Command::Write(p);
    assert!(matches!(
        refusal(&write(create()), &error(Some(-32603))),
        Some(providers::Reply::Unconfirmed(_))
    ));
    assert!(matches!(
        refusal(&write(create()), &error(None)),
        Some(providers::Reply::Unconfirmed(_))
    ));
    // a validation refusal is the server's answer
    assert_eq!(refusal(&write(create()), &error(Some(-32602))), None);
    // an edit names its provider: a refusal is a refusal
    assert_eq!(refusal(&write(edit()), &error(Some(-32603))), None);
}

#[test]
fn a_failed_save_stays_on_the_form_with_the_servers_words() {
    let mut app = listed("openai");
    interest(&mut app, Interest::Saving, true);
    let before = state(&app).writes();
    let fx = verb(&mut app, providers::Intent::Save(edit()));
    assert_eq!(sent(&fx).len(), 1);
    assert_eq!(state(&app).writes(), before + 1);
    refused(&mut app, &fx, Some(-32602), "Server validation failed.");
    assert_eq!(state(&app).saving(), None);
    assert_eq!(
        state(&app).failure_since(before),
        Some(&Trouble::Message("Server validation failed.".into()))
    );
    // a later form does not see the earlier form's failure
    assert_eq!(state(&app).failure_since(before + 1), None);
}

#[test]
fn a_create_whose_reply_is_lost_says_so_wherever_the_user_is() {
    let mut app = listed("openai");
    interest(&mut app, Interest::Saving, true);
    let fx = verb(&mut app, providers::Intent::Save(create()));
    let fx = refused(&mut app, &fx, Some(-32603), "Internal error");
    assert_eq!(one(&fx).1, providers::Command::List);
    assert_eq!(
        notes(&app),
        vec![Note::Outcome(Outcome::Trouble(Trouble::UnconfirmedCreate))]
    );
}

#[test]
fn offline_nothing_is_sent_and_the_list_says_to_reconnect() {
    let mut app = listed("openai");
    interest(&mut app, Interest::Saving, true);
    run(&mut app, AppEv::Disconnected);
    let fx = verb(&mut app, providers::Intent::Choose("gateway".into()));
    assert!(sent(&fx).is_empty());
    assert_eq!(state(&app).saving(), None);
    // the list is read again, and a read offline is not sent either
    assert_eq!(state(&app).load(), &Load::Failed(Failure::Offline));
    let fx = verb(&mut app, providers::Intent::Save(edit()));
    assert!(sent(&fx).is_empty());
    assert!(matches!(
        state(&app).failure_since(0),
        Some(Trouble::Offline)
    ));
}

// ---- the form's patch (oracle provider/editor_tests.rs)

fn summary(value: Value) -> ModelProviderSummary {
    serde_json::from_value(value).unwrap()
}

fn team(auth: GatewayAuth, url: Option<&str>) -> ModelProviderSummary {
    ModelProviderSummary {
        id: "team".into(),
        name: "Team".into(),
        can_edit: true,
        base_url: url.map(str::to_owned),
        settings: ProviderSettings::CustomGateway { auth },
    }
}

fn wire(params: ModelProviderConfigWriteParams) -> Value {
    serde_json::to_value(params.settings).unwrap()
}

fn changes(pairs: &[(Field, &str)]) -> BTreeMap<Field, String> {
    pairs.iter().map(|(f, v)| (*f, (*v).to_owned())).collect()
}

#[test]
fn hidden_gateway_settings_stay_out_of_the_rename_patch() {
    let provider = team(
        GatewayAuth::ExternallyConfigured {
            env_key: Some("EXTERNAL".into()),
        },
        None,
    );
    let draft = Draft::new(Some(provider), &[]);
    assert_eq!(draft.fields(), vec![(Field::Name, Some("Team"))]);
    assert_eq!(
        wire(draft.patch(&changes(&[(Field::Name, "Renamed")])).unwrap()),
        json!({"type": "customGateway", "providerId": "team", "name": "Renamed"})
    );
}

#[test]
fn cloud_provider_edits_preserve_credentials() {
    let mut provider = team(GatewayAuth::None, None);
    provider.id = "amazon-bedrock-runtime".into();
    provider.settings = ProviderSettings::Bedrock {
        profile: None,
        region: None,
    };
    let bedrock = Draft::new(Some(provider.clone()), &[]);
    assert_eq!(bedrock.fields(), vec![(Field::Region, None)]);
    assert_eq!(
        wire(
            bedrock
                .patch(&changes(&[(Field::Region, "us-west-2")]))
                .unwrap()
        ),
        json!({"type": "bedrock", "providerId": "amazon-bedrock-runtime", "region": "us-west-2"})
    );
    provider.id = "gcp".into();
    provider.settings = ProviderSettings::Gcp {
        project_id: Some("dev".into()),
        location: Some("us-east1".into()),
        credential_file: Some("/srv/gcp.json".into()),
    };
    let gcp = Draft::new(Some(provider), &[]);
    assert_eq!(
        wire(gcp.patch(&changes(&[(Field::Location, "")])).unwrap()),
        json!({"type": "gcp", "providerId": "gcp", "location": null})
    );
}

#[test]
fn a_cleared_url_and_a_key_without_one_are_not_saved() {
    let draft = Draft::new(
        Some(team(
            GatewayAuth::EnvironmentVariable {
                env_key: "TEAM_KEY".into(),
            },
            Some("https://old.example"),
        )),
        &[],
    );
    assert_eq!(
        draft.patch(&changes(&[(Field::Url, "")])).err(),
        Some(Invalid::EnterUrl)
    );
    assert_eq!(
        wire(
            draft
                .patch(&changes(&[(Field::Url, "https://new.example")]))
                .unwrap()
        ),
        json!({"type": "customGateway", "providerId": "team", "baseUrl": "https://new.example"})
    );
    let legacy = Draft::new(Some(team(GatewayAuth::None, None)), &[]);
    assert_eq!(
        legacy
            .patch(&changes(&[(Field::Variable, "TEAM_KEY")]))
            .err(),
        Some(Invalid::KeyNeedsUrl)
    );
    let named = Draft::new(None, &[]);
    assert_eq!(named.patch(&changes(&[])).err(), Some(Invalid::EnterName));
    assert_eq!(
        wire(summary_patch(&named)),
        json!({"type": "customGateway", "name": "Team", "baseUrl": "https://t.example"})
    );
}

fn summary_patch(draft: &Draft) -> ModelProviderConfigWriteParams {
    draft
        .patch(&changes(&[
            (Field::Name, "Team"),
            (Field::Url, "https://t.example"),
        ]))
        .unwrap()
}

#[test]
fn a_built_in_or_read_only_provider_cannot_be_saved() {
    let mut provider = team(GatewayAuth::None, None);
    provider.can_edit = false;
    assert_eq!(
        Draft::new(Some(provider), &[]).patch(&changes(&[])).err(),
        Some(Invalid::ReadOnly)
    );
    let openai = summary(json!({"id": "openai", "name": "OpenAI", "canEdit": true,
        "baseUrl": null, "settings": {"type": "openai"}}));
    assert_eq!(
        Draft::new(Some(openai), &[]).patch(&changes(&[])).err(),
        Some(Invalid::ReadOnly)
    );
}
