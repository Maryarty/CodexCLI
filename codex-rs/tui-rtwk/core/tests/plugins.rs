#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/plugins` (oracle `chatwidget/plugins.rs`, `plugin_catalog.rs`, `tests/plugins_tests.rs`) as the
//! domain of an ADR-013 feature: the list of a directory and the detail of a plugin as `Remote`s
//! cached by key, the changes the user asked for as ops under an `OpKey` (a second press collapses,
//! a late reply is dropped), the list loaded because a surface said it is interested, a switch
//! written and coalesced, the setup of an installed plugin's apps as a wish. Every step is a verb
//! with ids (`plugins::Intent`): the page, the tab and the cursor are the sheet's, so these tests
//! read data only; the words are the view's.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::features::plugins::{
    self, Configured, Fact, Interest, Kind, Op, OpKey, Outcome, PluginKey, PluginStatus, Say,
    Status, Toggle, Want, Wanted, Wish,
};
use codex_tui_core::features::{
    self, FeatureCommand, FeatureFact, FeatureIntent, FeatureInterest, FeatureNote, FeatureWish,
    InterestStep, Remote,
};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const CWD: &str = "/work/project";
const OTHER: &str = "/work/other";

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

fn config(remote: bool, sharing: bool) -> plugins::Config {
    plugins::Config {
        enabled: true,
        remote,
        sharing,
        home: Some("/home/me".to_string()),
        configured: vec![
            Configured {
                name: "team".to_string(),
                git: true,
            },
            Configured {
                name: "mine".to_string(),
                git: false,
            },
        ],
    }
}

/// An app with the plugins feature on (`remote`: the curated catalog is the remote one).
fn app(remote: bool, sharing: bool) -> CodexApp {
    let mut app = ready();
    let fact = FeatureFact::from(Fact::Config(config(remote, sharing)));
    run(&mut app, AppEv::Feature(fact));
    app
}

/// A verb of the feature, as the UI sends it.
fn verb(app: &mut CodexApp, intent: plugins::Intent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Feature(FeatureIntent::from(intent))),
    )
}

fn interested(cwd: &str) -> FeatureInterest {
    FeatureInterest::from(Interest::Listing {
        cwd: cwd.to_string(),
    })
}

/// A surface shows the plugins of `cwd`.
fn open(app: &mut CodexApp, cwd: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Interest(InterestStep::Open(interested(cwd)))),
    )
}

fn close(app: &mut CodexApp, cwd: &str) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::Interest(InterestStep::Close(interested(cwd)))),
    )
}

fn state(app: &CodexApp) -> &plugins::State {
    &app.features().plugins
}

fn key(row: &str) -> PluginKey {
    PluginKey {
        cwd: CWD.to_string(),
        row: row.to_string(),
    }
}

/// The requests of the feature a step left, with their tickets.
fn sent(fx: &AppFx) -> Vec<(Ticket, Wanted)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Feature {
                ticket,
                command: FeatureCommand::Plugins(wanted),
                ..
            } => Some((*ticket, wanted.clone())),
            _ => None,
        })
        .collect()
}

fn wants(fx: &AppFx) -> Vec<Want> {
    sent(fx)
        .into_iter()
        .map(|(_, wanted)| wanted.want)
        .collect()
}

fn one(fx: &AppFx) -> (Ticket, Wanted) {
    let all = sent(fx);
    let [only] = all.as_slice() else {
        panic!("expected one request, got {all:?}");
    };
    only.clone()
}

fn answer(app: &mut CodexApp, request: &(Ticket, Wanted), result: Value) -> AppFx {
    let command = FeatureCommand::Plugins(request.1.clone());
    let reply = features::decode(&command, result)
        .unwrap_or_else(|error| panic!("{:?} did not decode: {error:?}", request.1.want));
    run(app, AppEv::Reply(request.0, Reply::feature(reply)))
}

fn fail(app: &mut CodexApp, request: &(Ticket, Wanted), text: &str) -> AppFx {
    run(app, AppEv::Reply(request.0, Reply::failed(text)))
}

fn plugin(id: &str, name: &str, installed: bool, extra: Value) -> Value {
    let path = ["/m/", name].concat();
    let about = ["About ", name].concat();
    let mut base = json!({
        "id": id, "name": name,
        "source": {"type": "local", "path": path},
        "installed": installed, "enabled": installed,
        "installPolicy": "AVAILABLE", "authPolicy": "ON_INSTALL",
        "interface": {
            "displayName": name, "shortDescription": about,
            "capabilities": [], "screenshots": [], "screenshotUrls": [],
        },
    });
    for (key, value) in extra.as_object().cloned().unwrap_or_default() {
        base[key] = value;
    }
    base
}

fn market(name: &str, path: Option<&str>, plugins: Vec<Value>) -> Value {
    json!({"name": name, "path": path, "interface": null, "plugins": plugins})
}

fn listing(markets: Vec<Value>) -> Value {
    json!({"marketplaces": markets})
}

fn local_market() -> Value {
    listing(vec![
        market(
            "openai-curated",
            Some("/m/curated.json"),
            vec![
                plugin("zeta@curated", "Zeta", false, json!({})),
                plugin("alpha@curated", "alpha", true, json!({})),
                plugin("beta@curated", "Beta", true, json!({})),
            ],
        ),
        market(
            "team",
            Some("/home/me/team/marketplace.json"),
            vec![plugin("mine@team", "mine", false, json!({}))],
        ),
        market(
            "openai-bundled",
            Some("/m/bundled.json"),
            vec![plugin("b@b", "b", true, json!({}))],
        ),
    ])
}

/// `/plugins` opened, its list answered and the sections after it answered empty.
fn browsing() -> CodexApp {
    let mut app = app(true, true);
    let fx = open(&mut app, CWD);
    let request = one(&fx);
    let fx = answer(&mut app, &request, local_market());
    for section in sent(&fx) {
        answer(&mut app, &section, listing(vec![]));
    }
    app
}

fn tabs(app: &CodexApp) -> Vec<plugins::Tab> {
    state(app).tabs(CWD, None)
}

/// The row of the plugin the list calls `name`, as a key.
fn row(app: &CodexApp, name: &str) -> PluginKey {
    let id = tabs(app)[0]
        .rows
        .iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("no row {name}"))
        .id
        .clone();
    key(&id)
}

fn said(app: &CodexApp) -> Vec<Say> {
    use codex_tui_core::threads::{Item, Note, NoteKey};
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(Note {
                key: NoteKey::Feature(FeatureNote::Plugins(say)),
                ..
            }) => Some(say.clone()),
            _ => None,
        })
        .collect()
}

fn op(app: &CodexApp, key: &OpKey) -> Option<Op> {
    state(app).op(key).cloned()
}

fn detail_json(name: &str, installed: bool) -> Value {
    json!({"plugin": {
        "marketplaceName": "openai-curated", "marketplacePath": "/m/curated.json",
        "summary": plugin(&format!("{}@curated", name.to_lowercase()), name, installed, json!({})),
        "description": format!("{name} does things"),
        "skills": [{"name": "s1", "description": "d", "enabled": true}],
        "hooks": [{"key": "k", "eventName": "preToolUse"}, {"key": "j", "eventName": "preToolUse"}],
        "apps": [], "appTemplates": [], "mcpServers": ["srv"], "scheduledTasks": null,
    }})
}

/// `read` of `key` done: the detail is in.
fn read(app: &mut CodexApp, key: &PluginKey, installed: bool) {
    let name = state(app).plugin_name(key).expect("a row");
    let request = one(&verb(app, plugins::Intent::Read { key: key.clone() }));
    answer(app, &request, detail_json(&name, installed));
}

const WRITTEN: fn() -> Value =
    || json!({"status": "ok", "version": "v", "filePath": "/c", "overriddenMetadata": null});

#[test]
fn the_feature_off_says_so() {
    let mut app = ready();
    verb(&mut app, plugins::Intent::Run { cwd: CWD.to_string() });
    assert_eq!(said(&app), vec![Say::Disabled]);
    // Nothing loads for a surface that asks while it is off.
    assert!(sent(&open(&mut app, CWD)).is_empty());
}

#[test]
fn an_interest_asks_the_list_and_loads() {
    let mut app = app(true, true);
    let fx = open(&mut app, CWD);
    let (_, wanted) = one(&fx);
    assert_eq!(wanted.want, Want::List);
    assert_eq!(wanted.cwd, CWD);
    assert_eq!(state(&app).catalog(CWD), Some(&Remote::Loading(None)));
    assert_eq!(app.interests().count(&interested(CWD)), 1);
}

#[test]
fn a_second_surface_does_not_ask_again_and_the_last_to_leave_lets_go() {
    let mut app = app(true, true);
    assert_eq!(wants(&open(&mut app, CWD)), vec![Want::List]);
    assert!(sent(&open(&mut app, CWD)).is_empty());
    assert_eq!(app.interests().count(&interested(CWD)), 2);
    close(&mut app, CWD);
    assert_eq!(app.interests().count(&interested(CWD)), 1);
    close(&mut app, CWD);
    assert_eq!(app.interests().count(&interested(CWD)), 0);
    // Closing what nobody holds is nothing.
    assert!(sent(&close(&mut app, CWD)).is_empty());
    assert_eq!(app.interests().count(&interested(CWD)), 0);
}

#[test]
fn the_list_shows_and_the_sections_follow() {
    let mut app = app(false, true);
    let request = one(&open(&mut app, CWD));
    let fx = answer(&mut app, &request, local_market());
    assert!(matches!(
        state(&app).catalog(CWD),
        Some(Remote::Ready(_))
    ));
    // Curated is the section of its own while the remote catalog is off.
    assert_eq!(
        wants(&fx),
        vec![
            Want::Section(plugins::Part::Vertical),
            Want::Section(plugins::Part::Workspace),
            Want::Section(plugins::Part::SharedWithMe),
        ]
    );
    assert!(
        tabs(&app)
            .iter()
            .any(|tab| matches!(tab.kind, Kind::Curated))
    );
}

#[test]
fn rows_are_sorted_installed_first_and_the_hidden_marketplace_is_gone() {
    let app = browsing();
    let all = tabs(&app);
    let ids: Vec<_> = all.iter().map(|tab| tab.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "all-plugins",
            "installed-plugins",
            "marketplace:openai-curated",
            "remote-empty:workspace",
            "marketplace:/home/me/team/marketplace.json",
            "add-marketplace",
        ]
    );
    let names: Vec<_> = all[0].rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "Beta", "mine", "Zeta"]);
    assert_eq!((all[0].installed, all[0].total), (2, 4));
    let installed: Vec<_> = all[1].rows.iter().map(|row| row.plugin.as_str()).collect();
    assert_eq!(installed, vec!["alpha@curated", "beta@curated"]);
    assert_eq!(all[0].rows[0].status, PluginStatus::Installed);
    assert_eq!(all[0].rows[2].status, PluginStatus::Available);
    assert_eq!(all[0].rows[0].toggle, Toggle::Switch(true));
    assert_eq!(all[0].rows[2].toggle, Toggle::Unavailable);
}

#[test]
fn a_remote_plugin_and_its_local_copy_are_one_row() {
    let mut app = app(true, true);
    let request = one(&open(&mut app, CWD));
    let share = json!({"remotePluginId": "remote-1", "discoverability": "LISTED"});
    let local = plugin("shared@team", "shared", true, json!({"shareContext": share}));
    let remote = plugin(
        "shared-remote",
        "shared",
        false,
        json!({"source": {"type": "remote"}, "remotePluginId": "remote-1"}),
    );
    answer(
        &mut app,
        &request,
        listing(vec![
            market("team", Some("/home/me/team/marketplace.json"), vec![local]),
            market("workspace-directory", None, vec![remote]),
        ]),
    );
    let all = tabs(&app);
    assert_eq!(all[0].rows.len(), 1);
    assert_eq!(all[0].rows[0].plugin, "shared@team");
}

#[test]
fn the_detail_is_read_by_key_and_kept() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    assert!(state(&app).readable(&zeta));
    let fx = verb(&mut app, plugins::Intent::Read { key: zeta.clone() });
    let request = one(&fx);
    assert_eq!(
        request.1.want,
        Want::Read(plugins::Target {
            location: plugins::Location::Local {
                path: "/m/curated.json".to_string()
            },
            plugin: "Zeta".to_string(),
        })
    );
    assert_eq!(state(&app).detail(&zeta), Some(&Remote::Loading(None)));
    answer(&mut app, &request, detail_json("Zeta", false));
    let Some(Remote::Ready(detail)) = state(&app).detail(&zeta) else {
        panic!("expected the detail, got {:?}", state(&app).detail(&zeta));
    };
    assert_eq!(detail.skills, vec!["s1".to_string()]);
    assert_eq!(detail.hooks, vec![("PreToolUse".to_string(), 2)]);
    assert_eq!(detail.description.as_deref(), Some("Zeta does things"));
    assert!(matches!(detail.action(), plugins::Action::Install(_)));
}

#[test]
fn a_late_reply_of_an_older_read_is_dropped() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    let first = one(&verb(&mut app, plugins::Intent::Read { key: zeta.clone() }));
    let second = one(&verb(&mut app, plugins::Intent::Read { key: zeta.clone() }));
    // The second came back first; the first, late, changes nothing.
    answer(&mut app, &second, detail_json("Zeta", false));
    answer(&mut app, &first, detail_json("Other", false));
    let Some(Remote::Ready(detail)) = state(&app).detail(&zeta) else {
        panic!("expected the detail");
    };
    assert_eq!(detail.description.as_deref(), Some("Zeta does things"));
}

#[test]
fn a_failed_read_is_kept_as_failed_and_read_again_on_the_next_verb() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    let request = one(&verb(&mut app, plugins::Intent::Read { key: zeta.clone() }));
    fail(&mut app, &request, "no such plugin");
    assert_eq!(
        state(&app).detail(&zeta),
        Some(&Remote::Failed("no such plugin".to_string()))
    );
    let again = one(&verb(&mut app, plugins::Intent::Read { key: zeta.clone() }));
    assert_eq!(state(&app).detail(&zeta), Some(&Remote::Loading(None)));
    answer(&mut app, &again, detail_json("Zeta", false));
    assert!(matches!(state(&app).detail(&zeta), Some(Remote::Ready(_))));
}

#[test]
fn an_install_is_an_op_and_reads_the_list_and_the_detail_again() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    read(&mut app, &zeta, false);
    let install = OpKey::Install(zeta.clone());
    assert_eq!(op(&app, &install), None);
    let fx = verb(&mut app, plugins::Intent::Install { key: zeta.clone() });
    let request = one(&fx);
    assert!(matches!(request.1.want, Want::Install(_)));
    assert_eq!(op(&app, &install).map(|op| op.status), Some(Status::Running));
    // Nothing more to sign in: the list and the detail are read again, and the op is done when
    // the detail is back.
    let fx = answer(
        &mut app,
        &request,
        json!({"authPolicy": "ON_INSTALL", "appsNeedingAuth": []}),
    );
    let asked = sent(&fx);
    assert!(matches!(
        &asked.iter().map(|(_, w)| w.want.clone()).collect::<Vec<_>>()[..],
        [Want::List, Want::Read(_)]
    ));
    assert_eq!(
        said(&app),
        vec![Say::Installed {
            name: "Zeta".to_string(),
            apps: vec![]
        }]
    );
    assert_eq!(op(&app, &install).map(|op| op.status), Some(Status::Running));
    answer(&mut app, &asked[1], detail_json("Zeta", true));
    assert_eq!(
        op(&app, &install).map(|op| op.status),
        Some(Status::Done(Outcome::Nothing))
    );
}

#[test]
fn a_double_press_collapses_into_one_ask() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    read(&mut app, &zeta, false);
    let first = verb(&mut app, plugins::Intent::Install { key: zeta.clone() });
    assert_eq!(sent(&first).len(), 1);
    let at = op(&app, &OpKey::Install(zeta.clone())).unwrap().at;
    let second = verb(&mut app, plugins::Intent::Install { key: zeta.clone() });
    assert!(sent(&second).is_empty());
    assert_eq!(op(&app, &OpKey::Install(zeta)).unwrap().at, at);
}

#[test]
fn a_failed_install_fails_its_op_and_can_be_pressed_again() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    read(&mut app, &zeta, false);
    let install = OpKey::Install(zeta.clone());
    let request = one(&verb(&mut app, plugins::Intent::Install { key: zeta.clone() }));
    let first_at = op(&app, &install).unwrap().at;
    fail(&mut app, &request, "no network");
    assert_eq!(
        op(&app, &install).map(|op| op.status),
        Some(Status::Failed("no network".to_string()))
    );
    // The press after a failure is a new op, newer than the one the surface saw before.
    let fx = verb(&mut app, plugins::Intent::Install { key: zeta });
    assert_eq!(sent(&fx).len(), 1);
    let again = op(&app, &install).unwrap();
    assert!(again.at > first_at);
    assert_eq!(again.status, Status::Running);
}

#[test]
fn an_install_needing_apps_is_a_wish_and_the_setup_is_in_the_state() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    read(&mut app, &zeta, false);
    let install = one(&verb(&mut app, plugins::Intent::Install { key: zeta.clone() }));
    let apps = json!([
        {"id": "a1", "name": "First", "installUrl": "https://x/1"},
        {"id": "a2", "name": "Second"},
    ]);
    let fx = answer(
        &mut app,
        &install,
        json!({"authPolicy": "ON_INSTALL", "appsNeedingAuth": apps}),
    );
    // The list is read again, the detail is not: the setup comes first.
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(
        op(&app, &OpKey::Install(zeta.clone())).map(|op| op.status),
        Some(Status::Done(Outcome::Nothing))
    );
    let setup = state(&app).setup(&zeta).expect("a setup");
    assert_eq!(setup.plugin, "Zeta");
    assert_eq!(
        setup.apps.iter().map(|app| app.name.as_str()).collect::<Vec<_>>(),
        vec!["First", "Second"]
    );
    let wishes: Vec<_> = app.wishes().pending().cloned().collect();
    assert_eq!(wishes.len(), 1);
    assert_eq!(wishes[0].scope, CWD);
    assert_eq!(
        wishes[0].wish,
        FeatureWish::from(Wish::Auth(zeta.clone()))
    );
    // The surface shows it once and says so.
    run(&mut app, AppEv::Intent(Intent::WishShown(wishes[0].seq)));
    assert_eq!(app.wishes().pending().count(), 0);
    // The setup is still the feature's until it ends.
    assert!(state(&app).setup(&zeta).is_some());
}

#[test]
fn a_setup_page_opens_in_the_browser_and_the_end_says_how_it_went() {
    let mut app = browsing();
    let zeta = row(&app, "Zeta");
    read(&mut app, &zeta, false);
    let install = one(&verb(&mut app, plugins::Intent::Install { key: zeta.clone() }));
    answer(
        &mut app,
        &install,
        json!({"authPolicy": "ON_INSTALL", "appsNeedingAuth": [
            {"id": "a1", "name": "First", "installUrl": "https://x/1"},
            {"id": "a2", "name": "Second"},
        ]}),
    );
    let fx = verb(
        &mut app,
        plugins::Intent::OpenSetupPage {
            key: zeta.clone(),
            app: "a1".to_string(),
        },
    );
    assert!(fx.commands().iter().any(|command| matches!(
        command,
        Command::OpenUrl { url, .. } if url == "https://x/1"
    )));
    // An app without a page opens nothing.
    let fx = verb(
        &mut app,
        plugins::Intent::OpenSetupPage {
            key: zeta.clone(),
            app: "a2".to_string(),
        },
    );
    assert!(!fx
        .commands()
        .iter()
        .any(|command| matches!(command, Command::OpenUrl { .. })));
    verb(
        &mut app,
        plugins::Intent::EndSetup {
            key: zeta.clone(),
            how: plugins::SetupEnd::Completed,
        },
    );
    assert_eq!(
        said(&app).last(),
        Some(&Say::Completed {
            name: "Zeta".to_string()
        })
    );
    assert!(state(&app).setup(&zeta).is_none());
    // Ending what is over is nothing.
    verb(
        &mut app,
        plugins::Intent::EndSetup {
            key: zeta,
            how: plugins::SetupEnd::Skipped,
        },
    );
    assert_eq!(said(&app).len(), 2);
}

#[test]
fn skipping_the_setup_says_so_and_dismissing_it_does_not() {
    for (how, says) in [
        (plugins::SetupEnd::Skipped, true),
        (plugins::SetupEnd::Dismissed, false),
    ] {
        let mut app = browsing();
        let zeta = row(&app, "Zeta");
        read(&mut app, &zeta, false);
        let install = one(&verb(&mut app, plugins::Intent::Install { key: zeta.clone() }));
        answer(
            &mut app,
            &install,
            json!({"authPolicy": "ON_INSTALL", "appsNeedingAuth": [{"id": "a1", "name": "First"}]}),
        );
        let before = said(&app).len();
        verb(&mut app, plugins::Intent::EndSetup { key: zeta, how });
        let after = said(&app);
        assert_eq!(after.len(), before + usize::from(says));
        if says {
            assert_eq!(
                after.last(),
                Some(&Say::Skipped {
                    name: "Zeta".to_string()
                })
            );
        }
    }
}

#[test]
fn uninstall_says_so_and_reads_the_list_again() {
    let mut app = browsing();
    let alpha = row(&app, "alpha");
    read(&mut app, &alpha, true);
    let fx = verb(&mut app, plugins::Intent::Uninstall { key: alpha.clone() });
    let request = one(&fx);
    assert_eq!(
        request.1.want,
        Want::Uninstall {
            id: "alpha@curated".to_string()
        }
    );
    let fx = answer(&mut app, &request, json!({}));
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(
        said(&app),
        vec![Say::Uninstalled {
            name: "alpha".to_string()
        }]
    );
    // The op is done when the list is back.
    let uninstall = OpKey::Uninstall(alpha);
    assert_eq!(
        op(&app, &uninstall).map(|op| op.status),
        Some(Status::Running)
    );
    answer(&mut app, &one(&fx), local_market());
    assert_eq!(
        op(&app, &uninstall).map(|op| op.status),
        Some(Status::Done(Outcome::Nothing))
    );
}

#[test]
fn a_failed_list_is_kept_as_failed_and_asked_again_by_the_next_interest() {
    let mut app = app(true, true);
    let request = one(&open(&mut app, CWD));
    fail(&mut app, &request, "boom");
    assert_eq!(
        state(&app).catalog(CWD),
        Some(&Remote::Failed("boom".to_string()))
    );
    close(&mut app, CWD);
    let fx = open(&mut app, CWD);
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(state(&app).catalog(CWD), Some(&Remote::Loading(None)));
}

#[test]
fn a_known_list_stays_on_show_while_it_is_read_again() {
    let mut app = browsing();
    close(&mut app, CWD);
    let fx = open(&mut app, CWD);
    assert_eq!(wants(&fx), vec![Want::List]);
    let Some(Remote::Loading(Some(_))) = state(&app).catalog(CWD) else {
        panic!("expected the old list under the loading");
    };
    assert!(!tabs(&app).is_empty());
}

#[test]
fn directories_are_listed_apart() {
    let mut app = app(true, true);
    let here = one(&open(&mut app, CWD));
    let there = one(&open(&mut app, OTHER));
    assert_eq!((here.1.cwd.as_str(), there.1.cwd.as_str()), (CWD, OTHER));
    fail(&mut app, &there, "unreadable");
    // The one directory failing does not touch the other.
    assert_eq!(
        state(&app).catalog(OTHER),
        Some(&Remote::Failed("unreadable".to_string()))
    );
    assert_eq!(state(&app).catalog(CWD), Some(&Remote::Loading(None)));
    answer(&mut app, &here, local_market());
    assert!(matches!(state(&app).catalog(CWD), Some(Remote::Ready(_))));
    assert!(state(&app).tabs(OTHER, None).is_empty());
    // A plugin is named by directory and row: the other directory has none of this one's.
    let zeta = row(&app, "Zeta");
    let elsewhere = PluginKey {
        cwd: OTHER.to_string(),
        row: zeta.row,
    };
    assert!(!state(&app).readable(&elsewhere));
}

#[test]
fn a_reconnect_asks_again_for_every_interest_still_held() {
    let mut app = browsing();
    open(&mut app, OTHER);
    let fx = run(&mut app, AppEv::Connected);
    let mut asked: Vec<_> = sent(&fx)
        .into_iter()
        .filter(|(_, wanted)| wanted.want == Want::List)
        .map(|(_, wanted)| wanted.cwd)
        .collect();
    asked.sort();
    assert_eq!(asked, vec![OTHER.to_string(), CWD.to_string()]);
}

#[test]
fn section_errors_say_what_to_do() {
    let mut app = app(true, false);
    let request = one(&open(&mut app, CWD));
    let fx = answer(&mut app, &request, local_market());
    let sections = sent(&fx);
    // Sharing is off: no shared request, and its error says so.
    assert_eq!(
        sections
            .iter()
            .map(|(_, wanted)| wanted.want.clone())
            .collect::<Vec<_>>(),
        vec![Want::Section(plugins::Part::Workspace)]
    );
    fail(&mut app, &sections[0], "plugin sharing is disabled");
    let all = tabs(&app);
    let placeholder = all
        .iter()
        .find_map(|tab| match &tab.kind {
            Kind::Section {
                state: plugins::Placeholder::Failed(error),
                ..
            } => Some(error.clone()),
            _ => None,
        })
        .expect("the workspace failed");
    assert_eq!(placeholder.part, plugins::Part::Workspace);
    assert!(matches!(
        placeholder.cause,
        plugins::catalog::Cause::Failed {
            next: plugins::catalog::Next::AskAdminToEnable,
            ..
        }
    ));
}

#[test]
fn a_switch_writes_the_config_and_coalesces() {
    let mut app = browsing();
    let alpha = row(&app, "alpha");
    let flip = |app: &mut CodexApp, on: bool| {
        verb(
            app,
            plugins::Intent::SetEnabled {
                key: alpha.clone(),
                enabled: on,
            },
        )
    };
    let first = one(&flip(&mut app, false));
    assert_eq!(
        first.1.want,
        Want::Enable {
            id: "alpha@curated".to_string(),
            enabled: false
        }
    );
    // The list shows the switch as flipped at once.
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(false));
    // Flipped twice while the write runs: nothing is sent until it is back.
    assert!(sent(&flip(&mut app, true)).is_empty());
    assert!(sent(&flip(&mut app, false)).is_empty());
    // The last value asked for is the one written: done.
    let fx = answer(&mut app, &first, WRITTEN());
    assert!(sent(&fx).is_empty());
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(false));
}

#[test]
fn a_queued_switch_is_written_after_the_first() {
    let mut app = browsing();
    let alpha = row(&app, "alpha");
    let flip = |app: &mut CodexApp, on: bool| {
        verb(
            app,
            plugins::Intent::SetEnabled {
                key: alpha.clone(),
                enabled: on,
            },
        )
    };
    let first = one(&flip(&mut app, false));
    // Back on, queued behind the write that turns it off.
    flip(&mut app, true);
    let fx = answer(&mut app, &first, WRITTEN());
    let second = one(&fx);
    assert_eq!(
        second.1.want,
        Want::Enable {
            id: "alpha@curated".to_string(),
            enabled: true
        }
    );
    answer(&mut app, &second, WRITTEN());
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(true));
}

#[test]
fn a_refused_switch_is_said() {
    let mut app = browsing();
    let alpha = row(&app, "alpha");
    let write = one(&verb(
        &mut app,
        plugins::Intent::SetEnabled {
            key: alpha,
            enabled: false,
        },
    ));
    fail(&mut app, &write, "read only");
    assert_eq!(
        said(&app),
        vec![Say::EnableFailed {
            id: "alpha@curated".to_string(),
            error: "read only".to_string()
        }]
    );
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(true));
}

#[test]
fn a_marketplace_is_added_from_a_source() {
    let mut app = browsing();
    let add = OpKey::Add {
        cwd: CWD.to_string(),
    };
    // Nothing typed: nothing asked.
    let fx = verb(
        &mut app,
        plugins::Intent::AddMarketplace {
            cwd: CWD.to_string(),
            source: "  ".to_string(),
        },
    );
    assert!(sent(&fx).is_empty());
    let fx = verb(
        &mut app,
        plugins::Intent::AddMarketplace {
            cwd: CWD.to_string(),
            source: "./local#main".to_string(),
        },
    );
    let request = one(&fx);
    assert_eq!(
        request.1.want,
        Want::Add {
            source: "/work/project/local#main".to_string()
        }
    );
    assert_eq!(op(&app, &add).map(|op| op.status), Some(Status::Running));
    let fx = answer(
        &mut app,
        &request,
        json!({"marketplaceName": "local", "installedRoot": "/home/me/market", "alreadyAdded": false}),
    );
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(
        said(&app),
        vec![Say::MarketAdded {
            name: "local".to_string(),
            root: "/home/me/market".to_string(),
            already: false
        }]
    );
    // Done when the list with the new marketplace is in; the outcome names where it is.
    assert_eq!(op(&app, &add).map(|op| op.status), Some(Status::Running));
    answer(&mut app, &one(&fx), local_market());
    assert_eq!(
        op(&app, &add).map(|op| op.status),
        Some(Status::Done(Outcome::Added {
            root: "/home/me/market".to_string(),
            already: false
        }))
    );
}

#[test]
fn a_failed_add_fails_its_op_and_can_be_tried_again() {
    let mut app = browsing();
    let add = OpKey::Add {
        cwd: CWD.to_string(),
    };
    let ask = |app: &mut CodexApp| {
        verb(
            app,
            plugins::Intent::AddMarketplace {
                cwd: CWD.to_string(),
                source: "owner/repo".to_string(),
            },
        )
    };
    let fx = ask(&mut app);
    fail(&mut app, &one(&fx), "no such repo");
    assert_eq!(
        op(&app, &add).map(|op| op.status),
        Some(Status::Failed("no such repo".to_string()))
    );
    assert_eq!(sent(&ask(&mut app)).len(), 1);
    assert_eq!(op(&app, &add).map(|op| op.status), Some(Status::Running));
}

#[test]
fn a_configured_marketplace_can_be_removed() {
    let mut app = browsing();
    let team = "marketplace:/home/me/team/marketplace.json";
    // The curated tab is not the user's: nothing to remove.
    assert_eq!(state(&app).market(CWD, "marketplace:openai-curated", None), None);
    let market = state(&app).market(CWD, team, None).expect("a configured tab");
    assert_eq!((market.name.as_str(), market.label.as_str()), ("team", "team"));
    let fx = verb(
        &mut app,
        plugins::Intent::RemoveMarketplace {
            cwd: CWD.to_string(),
            name: "team".to_string(),
        },
    );
    let request = one(&fx);
    assert_eq!(
        request.1.want,
        Want::Remove {
            name: "team".to_string()
        }
    );
    let fx = answer(
        &mut app,
        &request,
        json!({"marketplaceName": "team", "installedRoot": null}),
    );
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(
        said(&app),
        vec![Say::MarketRemoved {
            label: "team".to_string(),
            name: "team".to_string(),
            root: None
        }]
    );
    answer(&mut app, &one(&fx), local_market());
    assert_eq!(
        op(
            &app,
            &OpKey::Remove {
                cwd: CWD.to_string(),
                name: "team".to_string()
            }
        )
        .map(|op| op.status),
        Some(Status::Done(Outcome::Removed))
    );
}

#[test]
fn a_git_marketplace_is_upgraded() {
    let mut app = browsing();
    let team = "marketplace:/home/me/team/marketplace.json";
    assert!(state(&app).market(CWD, team, None).unwrap().upgrade);
    let fx = verb(
        &mut app,
        plugins::Intent::UpgradeMarketplace {
            cwd: CWD.to_string(),
            name: Some("team".to_string()),
        },
    );
    let request = one(&fx);
    assert_eq!(
        request.1.want,
        Want::Upgrade {
            name: Some("team".to_string())
        }
    );
    // A second press while it runs is the same op.
    assert!(sent(&verb(
        &mut app,
        plugins::Intent::UpgradeMarketplace {
            cwd: CWD.to_string(),
            name: Some("team".to_string()),
        },
    ))
    .is_empty());
    let fx = answer(
        &mut app,
        &request,
        json!({"selectedMarketplaces": ["team"], "upgradedRoots": ["/home/me/team"], "errors": []}),
    );
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(
        said(&app),
        vec![Say::Upgraded {
            roots: vec!["/home/me/team".to_string()]
        }]
    );
    answer(&mut app, &one(&fx), local_market());
    assert_eq!(
        op(
            &app,
            &OpKey::Upgrade {
                cwd: CWD.to_string(),
                name: Some("team".to_string())
            }
        )
        .map(|op| op.status),
        Some(Status::Done(Outcome::Upgraded {
            root: Some("/home/me/team".to_string())
        }))
    );
}

#[test]
fn a_relative_source_is_made_absolute() {
    assert_eq!(plugins::add_source("/w/p", "./m"), "/w/p/m");
    assert_eq!(plugins::add_source("/w/p", "../m#main"), "/w/m#main");
    assert_eq!(plugins::add_source("/w/p", "owner/repo"), "owner/repo");
    assert_eq!(plugins::add_source("/w/p", "~/m"), "~/m");
    assert_eq!(plugins::add_source("/w/p", "."), "/w/p");
}
