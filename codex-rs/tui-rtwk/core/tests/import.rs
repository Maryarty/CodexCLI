//! State tests of S06 (import from other agents): the flow of the oracle's
//! `external_agent_config_migration` (`flow_tests.rs`, `mod.rs` tests,
//! `source_tests.rs`) as the model's events and intents. Core holds no word:
//! the tests with the screen's words and keys are in `tests/pages/import.rs`;
//! a test that is the screen's look is in `tests/oracle/import.rs`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::ExternalAgentConfigImportCompletedNotification as Completed;
use codex_app_server_protocol::ExternalAgentConfigMigrationItem as Item;
use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::{AppFx, Command};
use codex_tui_core::protocol::import as wire;
use codex_tui_core::screens::import::detect::{Detection, Outcome as Detected};
use codex_tui_core::screens::import::{
    Ev, Failure, Found, Intent, ItemKind, Model, Outcome, Phase, Places, Request, Sel, Source,
    apply, apply_intent, observe,
};
use pretty_assertions::assert_eq;
use serde_json::json;

fn item(value: serde_json::Value) -> Item {
    serde_json::from_value(value).expect("an item")
}

fn config(description: &str) -> Item {
    item(json!({"itemType":"CONFIG","description":description,"cwd":null,"details":null}))
}

fn completed() -> Completed {
    serde_json::from_value(json!({
        "importId": "import-1",
        "itemTypeResults": [
            {"itemType":"CONFIG","successes":[{"itemType":"CONFIG","cwd":null,"source":"settings.json","target":"config.toml","title":null}],"failures":[]},
            {"itemType":"MEMORY","successes":[
                {"itemType":"MEMORY","cwd":null,"source":"project-a","target":"memory resources","title":null},
                {"itemType":"MEMORY","cwd":null,"source":"project-b","target":"memory resources","title":null}],"failures":[]},
            {"itemType":"PLUGINS","successes":[{"itemType":"PLUGINS","cwd":null,"source":"formatter@example","target":"formatter@example","title":null}],
             "failures":[{"itemType":"PLUGINS","errorType":"plugin_install_failed","subErrorType":"failed_to_copy_plugin_file",
                "failureStage":"plugin_import","message":"install failed","cwd":"/workspace/project","source":"deployer@example"}]}
        ]
    }))
    .expect("a completion")
}

#[test]
fn memory_without_a_selection_counts_as_zero() {
    let sel = Sel {
        item: item(
            json!({"itemType":"MEMORY","description":"Import memory","cwd":null,"details":null}),
        ),
        enabled: true,
    };
    assert_eq!(sel.kind(), ItemKind::Memory);
    assert_eq!(sel.count(), 0);
}

#[test]
fn external_agent_config_detection_keeps_one_or_multiple_available_sources() {
    let mut detection = Detection::default();
    detection.record_items(Source::Claude, vec![config("first")]);
    detection.record_items(Source::Cursor, Vec::new());
    let Detected::Sources(sources) = detection.finish() else {
        panic!("expected one detected source");
    };
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].source, Source::Claude);
    assert_eq!(sources[0].items, vec![config("first")]);

    let mut detection = Detection::default();
    detection.record_items(Source::Claude, vec![config("first")]);
    detection.record_items(Source::Cursor, vec![config("second")]);
    let Detected::Sources(sources) = detection.finish() else {
        panic!("expected two detected sources");
    };
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].source, Source::Claude);
    assert_eq!(sources[1].source, Source::Cursor);
}

#[test]
fn external_agent_config_detection_tolerates_a_partial_failure() {
    let mut detection = Detection::default();
    detection.record_error(Source::Claude, "unavailable");
    detection.record_items(Source::Cursor, vec![config("available")]);
    let Detected::Sources(sources) = detection.finish() else {
        panic!("expected the available source");
    };
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].source, Source::Cursor);
}

#[test]
fn external_agent_config_detection_distinguishes_no_items_from_total_failure() {
    assert!(matches!(Detection::default().finish(), Detected::NoItems));

    let mut detection = Detection::default();
    detection.record_error(Source::Claude, "first failure");
    detection.record_error(Source::Cursor, "second failure");
    let Detected::Failed(errors) = detection.finish() else {
        panic!("expected detection failure");
    };
    assert_eq!(
        errors,
        [
            (Source::Claude, "first failure".to_owned()),
            (Source::Cursor, "second failure".to_owned())
        ]
    );
}

fn sample_items() -> Vec<Item> {
    let plugins = json!({"plugins":[
        {"marketplaceName":"acme-tools","pluginNames":["deployer","formatter","lint"]},
        {"marketplaceName":"team-marketplace","pluginNames":["asana"]},
        {"marketplaceName":"debug","pluginNames":["sample"]},
        {"marketplaceName":"data-tools","pluginNames":["warehouse"]}]});
    vec![
        config("Migrate /Users/alex/.claude/settings.json into /Users/alex/.codex/config.toml"),
        item(
            json!({"itemType":"SESSIONS","description":"Migrate recent Claude Code sessions","cwd":null,
            "details":{"sessions":[{"path":"/Users/alex/.claude/projects/project/session.jsonl",
                "cwd":"/workspace/project","title":"Investigate migration UX"}]}}),
        ),
        item(
            json!({"itemType":"PLUGINS","description":"Migrate enabled plugins from /workspace/project/.claude/settings.json",
            "cwd":"/workspace/project","details":plugins}),
        ),
        item(
            json!({"itemType":"AGENTS_MD","description":"Migrate /workspace/project/CLAUDE.md to /workspace/project/AGENTS.md",
            "cwd":"/workspace/project","details":null}),
        ),
    ]
}

fn detected(sources: &[(Source, Vec<Item>)], selected: Option<Vec<usize>>) -> Ev {
    Ev::Detected {
        request: 0,
        places: Places::default(),
        found: sources
            .iter()
            .map(|(source, items)| Found {
                source: *source,
                result: Ok(items.clone()),
            })
            .collect(),
        selected,
    }
}

fn reviewing(items: Vec<Item>, selected: Option<Vec<usize>>) -> Model {
    let mut model = Model::default();
    apply(
        &mut model,
        detected(&[(Source::Claude, items)], selected),
        &mut AppFx::default(),
    );
    model
}

/// Keys of the review as the view sends them: through the `Nav` the view
/// holds, the intent it returns applied to the model.
fn send(model: &mut Model, intents: &[Intent]) {
    for intent in intents {
        apply_intent(model, intent.clone(), &mut AppFx::default());
    }
}

/// Every item of `sample_items` checked (the page's checklist sends the indexes).
fn all() -> Vec<usize> {
    vec![0, 1, 2, 3]
}

#[test]
fn proceed_returns_selected_items() {
    let mut model = reviewing(sample_items(), None);
    assert_eq!(model.phase, Phase::Review);
    send(&mut model, &[Intent::Proceed(all())]);
    assert_eq!(model.phase, Phase::Importing);
    let Some(Request::Import { source, items, .. }) = model.request() else {
        panic!("an import request");
    };
    assert_eq!(source, "claude-code");
    assert_eq!(items, sample_items());
}

#[test]
fn unchecking_an_item_then_proceed_keeps_remaining_selection() {
    let items = sample_items();
    let mut model = reviewing(items.clone(), None);
    // the page's checklist unchecked the first item: it sends the rest
    send(&mut model, &[Intent::Proceed(vec![1, 2, 3])]);
    assert_eq!(model.selected_items().len(), 3);
    let Some(Request::Import { items: chosen, .. }) = model.request() else {
        panic!("an import request");
    };
    assert_eq!(chosen, items[1..].to_vec());
}

#[test]
fn an_empty_selection_does_not_proceed() {
    let mut model = reviewing(sample_items(), Some(Vec::new()));
    send(&mut model, &[Intent::Proceed(Vec::new())]);
    assert_eq!(model.phase, Phase::Review);
    send(&mut model, &[Intent::Proceed(all())]);
    assert_eq!(model.phase, Phase::Importing);
}

#[test]
fn control_exit_shortcuts_cancel_prompt() {
    // `ctrl+c`, `ctrl+d`, skip and escape are the intent `Cancel`.
    let mut model = reviewing(sample_items(), None);
    send(&mut model, &[Intent::Cancel]);
    assert_eq!(model.outcome, Some(Outcome::Cancelled));
    assert_eq!(model.phase, Phase::Done);
}

// ---- the source prompt (oracle: source_tests.rs) and the flow (flow.rs)

fn two_sources() -> Model {
    let mut model = Model::default();
    apply(
        &mut model,
        detected(
            &[
                (Source::Claude, sample_items()),
                (Source::Cursor, vec![config("Migrate cursor")]),
            ],
            None,
        ),
        &mut AppFx::default(),
    );
    model
}

#[test]
fn external_agent_config_source_prompt_selects_highlighted_source() {
    let mut model = two_sources();
    assert_eq!(model.phase, Phase::Source);
    assert_eq!(
        model
            .detected
            .iter()
            .map(|found| found.source)
            .collect::<Vec<_>>(),
        [Source::Claude, Source::Cursor]
    );
    send(&mut model, &[Intent::Source("cursor".into())]);
    assert_eq!(model.phase, Phase::Review);
    assert_eq!(model.source, Some(Source::Cursor));
    assert_eq!(model.items.len(), 1);
}

#[test]
fn external_agent_config_source_prompt_can_cancel() {
    let mut model = two_sources();
    send(&mut model, &[Intent::Cancel]);
    assert_eq!(model.outcome, Some(Outcome::Cancelled));
}

#[test]
fn one_source_with_items_is_reviewed_without_asking() {
    let model = reviewing(sample_items(), None);
    assert_eq!(
        (model.phase, model.source),
        (Phase::Review, Some(Source::Claude))
    );
}

#[test]
fn an_answer_for_another_request_is_dropped() {
    let mut model = Model::default();
    send(
        &mut model,
        &[Intent::Open {
            cwd: Some("/workspace/project".into()),
        }],
    );
    assert_eq!(model.phase, Phase::Detecting);
    let Some(Request::Detect { request, cwd }) = model.request() else {
        panic!("a detect request");
    };
    assert_eq!(cwd.as_deref(), Some("/workspace/project"));
    let answer = |request| Ev::Detected {
        request,
        places: Places::default(),
        found: vec![Found {
            source: Source::Claude,
            result: Ok(sample_items()),
        }],
        selected: None,
    };
    apply(&mut model, answer(request + 1), &mut AppFx::default());
    assert_eq!(model.phase, Phase::Detecting);
    apply(&mut model, answer(request), &mut AppFx::default());
    assert_eq!(model.phase, Phase::Review);
}

#[test]
fn a_detection_cancelled_before_it_answers_drops_the_late_answer() {
    let mut model = Model::default();
    send(&mut model, &[Intent::Open { cwd: None }]);
    let Some(Request::Detect { request, .. }) = model.request() else {
        panic!("a detect request");
    };
    send(&mut model, &[Intent::Cancel]);
    assert_eq!(
        (model.phase, model.outcome.clone()),
        (Phase::Done, Some(Outcome::Cancelled))
    );
    let late = Ev::Detected {
        request,
        places: Places::default(),
        found: vec![Found {
            source: Source::Claude,
            result: Ok(sample_items()),
        }],
        selected: None,
    };
    apply(&mut model, late, &mut AppFx::default());
    assert_eq!(model.phase, Phase::Done);
}

#[test]
fn host_asks_import_latest_detection_wins_before_the_screen_handles_it() {
    use codex_tui_core::app::{AppEv, CodexApp, Ingress, Intent as AppIntent};
    use codex_tui_core::ids::BackendId;
    use codex_tui_core::screens::{ScreenEv, ScreenIntent};

    let mut app = CodexApp::new();
    let epoch = app.attach();
    let foreign = BackendId(1);
    let foreign_epoch = app.attach_backend(foreign);
    let open = |app: &mut CodexApp, cwd: &str| {
        let ev = AppEv::Intent(AppIntent::Screen(ScreenIntent::Import(Intent::Open {
            cwd: Some(cwd.to_owned()),
        })));
        let fx = app.ingest(Ingress::new(epoch, ev)).expect("import intent");
        let asks = asked(&fx);
        let [Request::Detect { request, .. }] = asks.as_slice() else {
            panic!("one detection: {asks:?}");
        };
        *request
    };
    let old = open(&mut app, "/old");
    let latest = open(&mut app, "/latest");
    assert_ne!(old, latest);
    let reply = |request, label| {
        AppEv::Screen(ScreenEv::Import(Ev::Detected {
            request,
            places: Places::default(),
            found: vec![Found {
                source: Source::Claude,
                result: Ok(vec![config(label)]),
            }],
            selected: None,
        }))
    };
    assert!(
        app.ingest(Ingress::new(epoch, reply(0, "unissued")))
            .is_none()
    );
    assert!(
        app.ingest(Ingress::of(
            foreign,
            foreign_epoch,
            reply(latest, "wrong backend")
        ))
        .is_none()
    );
    assert!(app.ingest(Ingress::new(epoch, reply(old, "old"))).is_none());
    assert_eq!(app.screens().import.phase, Phase::Detecting);
    assert!(
        app.ingest(Ingress::new(epoch, reply(latest, "latest")))
            .is_some()
    );
    assert_eq!(app.screens().import.items[0].item, config("latest"));
    assert!(
        app.ingest(Ingress::new(epoch, reply(latest, "duplicate")))
            .is_none()
    );
}

#[test]
fn a_failed_import_returns_to_the_review_with_the_selection_and_the_error() {
    let mut model = reviewing(sample_items(), None);
    send(&mut model, &[Intent::Proceed(vec![1, 2, 3])]);
    let Some(Request::Import { request, .. }) = model.request() else {
        panic!("an import request");
    };
    apply(
        &mut model,
        Ev::Imported {
            request: request + 1,
            result: Ok("import-1".into()),
        },
        &mut AppFx::default(),
    );
    assert_eq!(model.phase, Phase::Importing);
    apply(
        &mut model,
        Ev::Imported {
            request,
            result: Err("boom".into()),
        },
        &mut AppFx::default(),
    );
    assert_eq!(model.phase, Phase::Review);
    assert_eq!(model.error.as_deref(), Some("boom"));
    assert_eq!(model.selected_items().len(), 3);
    assert_eq!(
        model.review_id, 2,
        "a new review opened: the view's `Nav` starts again"
    );
}

#[test]
fn a_started_import_tells_what_remains_and_blocks_the_next_one() {
    let mut model = reviewing(sample_items(), None);
    send(&mut model, &[Intent::Proceed(vec![1, 2, 3])]);
    let Some(Request::Import { request, .. }) = model.request() else {
        panic!("an import request");
    };
    apply(
        &mut model,
        Ev::Imported {
            request,
            result: Ok("import-1".into()),
        },
        &mut AppFx::default(),
    );
    let Some(Outcome::Started(started)) = model.outcome.clone() else {
        panic!("started");
    };
    assert_eq!((started.items.len(), started.remaining), (3, 1));
    assert!(model.running);
    send(&mut model, &[Intent::Open { cwd: None }]);
    assert_eq!(model.outcome, Some(Outcome::Failed(Failure::InProgress)));
    apply(
        &mut model,
        Ev::Completed(Box::new(completed())),
        &mut AppFx::default(),
    );
    assert!(!model.running);
    let finished = model.finished.clone().expect("the completion");
    assert_eq!((finished.imported(), finished.failed()), (4, 1));
    send(&mut model, &[Intent::Open { cwd: None }]);
    assert_eq!(model.phase, Phase::Detecting);
}

#[test]
fn no_items_and_a_total_failure_end_the_flow() {
    let mut model = Model::default();
    apply(&mut model, detected(&[], None), &mut AppFx::default());
    assert_eq!(model.outcome, Some(Outcome::NoItems));
    let mut model = Model::default();
    apply(
        &mut model,
        Ev::Detected {
            request: 0,
            places: Places::default(),
            found: vec![Found {
                source: Source::Claude,
                result: Err("down".into()),
            }],
            selected: None,
        },
        &mut AppFx::default(),
    );
    assert_eq!(
        model.outcome,
        Some(Outcome::Failed(Failure::Detect(vec![(
            Source::Claude,
            "down".into()
        )])))
    );
}

// ---- the requests the shell makes (oracle: `flow.rs`, `external_agent_config.rs`)

fn asked(fx: &AppFx) -> Vec<Request> {
    fx.commands()
        .iter()
        .map(|command| match command {
            Command::Import { ask, .. } => ask.clone(),
            other => panic!("not an import ask: {other:?}"),
        })
        .collect()
}

#[test]
fn opening_asks_every_source_about_the_project() {
    let mut model = Model::default();
    let mut fx = AppFx::default();
    apply_intent(
        &mut model,
        Intent::Open {
            cwd: Some("/workspace/project".into()),
        },
        &mut fx,
    );
    assert_eq!(
        asked(&fx),
        [Request::Detect {
            request: 1,
            cwd: Some("/workspace/project".into())
        }]
    );
}

#[test]
fn proceeding_asks_the_import_of_the_checked_items_of_the_source() {
    let items = sample_items();
    let mut model = reviewing(items.clone(), None);
    let mut fx = AppFx::default();
    apply_intent(&mut model, Intent::Proceed(vec![1, 2, 3]), &mut fx);
    assert_eq!(
        asked(&fx),
        [Request::Import {
            request: 1,
            source: "claude-code",
            items: items[1..].to_vec()
        }]
    );
}

#[test]
fn a_flow_that_asks_nothing_makes_no_request() {
    // an empty selection does not proceed
    let mut model = reviewing(sample_items(), Some(Vec::new()));
    let mut fx = AppFx::default();
    apply_intent(&mut model, Intent::Proceed(Vec::new()), &mut fx);
    // a cancelled flow
    apply_intent(&mut model, Intent::Cancel, &mut fx);
    assert_eq!(asked(&fx), Vec::<Request>::new());
    // one import runs: the next `/import` is refused, nothing is asked
    let mut model = reviewing(sample_items(), None);
    send(&mut model, &[Intent::Proceed(all())]);
    let Some(Request::Import { request, .. }) = model.request() else {
        panic!("an import")
    };
    let imported = Ev::Imported {
        request,
        result: Ok("import-1".into()),
    };
    apply(&mut model, imported, &mut fx);
    apply_intent(&mut model, Intent::Open { cwd: None }, &mut fx);
    assert_eq!(model.outcome, Some(Outcome::Failed(Failure::InProgress)));
    assert_eq!(asked(&fx), Vec::<Request>::new());
}

fn started(model: &mut Model, id: &str) {
    let checked = (0..model.items.len()).collect();
    send(model, &[Intent::Proceed(checked)]);
    let Some(Request::Import { request, .. }) = model.request() else {
        panic!("an import")
    };
    apply(
        model,
        Ev::Imported {
            request,
            result: Ok(id.into()),
        },
        &mut AppFx::default(),
    );
}

fn completion(id: &str) -> ServerNotification {
    let mut done = completed();
    done.import_id = id.to_owned();
    ServerNotification::ExternalAgentConfigImportCompleted(done)
}

#[test]
fn only_the_completion_of_our_import_is_taken() {
    // oracle: `external_agent_config_migration_over_shared_transport`: a
    // shared daemon broadcasts the completions of other clients' imports
    let mut model = reviewing(sample_items(), None);
    started(&mut model, "import-1");
    observe(
        &mut model,
        &completion("someone-elses"),
        &mut AppFx::default(),
    );
    assert!(model.running, "another client's completion is not ours");
    assert_eq!(model.finished, None);
    observe(&mut model, &completion("import-1"), &mut AppFx::default());
    assert!(!model.running);
    let finished = model.finished.take().expect("our completion");
    assert_eq!((finished.imported(), finished.failed()), (4, 1));
    // taken once: the id is consumed
    observe(&mut model, &completion("import-1"), &mut AppFx::default());
    assert_eq!(model.finished, None);
}

// ---- the wire (oracle: `flow.rs` detect params, `external_agent_config.rs`)

fn id(n: i64) -> codex_app_server_protocol::RequestId {
    codex_app_server_protocol::RequestId::Integer(n)
}

#[test]
fn detect_asks_the_home_and_the_project_for_the_source() {
    let request = serde_json::to_value(wire::detect(Source::Cursor, Some("/w"), id(3))).unwrap();
    assert_eq!(request["method"], "externalAgentConfig/detect");
    assert_eq!(request["id"], 3);
    assert_eq!(request["params"]["includeHome"], true);
    assert_eq!(request["params"]["cwds"], json!(["/w"]));
    assert_eq!(request["params"]["migrationSource"], "cursor");
    assert!(request["params"]["source"].is_null());
    let request = serde_json::to_value(wire::detect(Source::Claude, None, id(4))).unwrap();
    assert!(request["params"]["cwds"].is_null());
    assert_eq!(request["params"]["migrationSource"], "claude-code");
}

#[test]
fn import_names_the_cli_and_the_source() {
    let items = sample_items();
    let request = serde_json::to_value(wire::import("claude-code", items.clone(), id(5))).unwrap();
    assert_eq!(request["method"], "externalAgentConfig/import");
    assert_eq!(
        request["params"]["migrationItems"],
        serde_json::to_value(&items).unwrap()
    );
    assert_eq!(request["params"]["source"], "cli");
    assert_eq!(request["params"]["providerId"], "claude-code");
    assert_eq!(request["params"]["migrationSource"], "claude-code");
}

#[test]
fn the_answers_are_read_as_items_and_an_import_id() {
    let items = sample_items();
    let answer = json!({"items": items, "connectors": []});
    assert_eq!(wire::found(answer).unwrap(), items);
    assert!(wire::found(json!({"items": 3})).is_err());
    assert_eq!(
        wire::imported(json!({"importId": "import-9"})).unwrap(),
        "import-9"
    );
    assert!(wire::imported(json!({})).is_err());
}
