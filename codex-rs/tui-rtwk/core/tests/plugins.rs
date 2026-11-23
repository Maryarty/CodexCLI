#![allow(clippy::expect_used, clippy::unwrap_used)]

//! `/plugins` (oracle `chatwidget/plugins.rs`, `plugin_catalog.rs`, `tests/plugins_tests.rs`): the
//! list and the remote sections after it, the tabs and their rows, the detail, install with the
//! setup of the apps it needs, uninstall, the switch of a plugin, and the marketplaces added,
//! removed and upgraded. The words are the view's: these tests read the data.

use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::select::plugins::{
    self, Configured, Kind, Menu as Plugin, Say, Status, Toggle, Wanted, Want,
};
use codex_tui_core::screens::select::{self, Intent as Select, Menu, Note, Picked};
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, ScreenReply};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const CWD: &str = "/work/project";

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
            Configured { name: "team".to_string(), git: true },
            Configured { name: "mine".to_string(), git: false },
        ],
    }
}

/// An app with the plugins feature on (`remote`: the curated catalog is the remote one).
fn app(remote: bool, sharing: bool) -> CodexApp {
    let mut app = ready();
    run(&mut app, AppEv::PluginConfig(config(remote, sharing)));
    app
}

fn select(app: &mut CodexApp, intent: Select) -> AppFx {
    run(app, AppEv::Intent(Intent::Screen(ScreenIntent::Select(intent))))
}

fn choose(app: &mut CodexApp, id: &str) -> AppFx {
    select(app, Select::Choose(id.to_owned()))
}

fn open(app: &mut CodexApp) -> AppFx {
    select(app, Select::OpenPlugins { cwd: CWD.to_string() })
}

fn model(app: &CodexApp) -> &plugins::Plugins {
    &app.screens().select.plugins
}

fn menu(app: &CodexApp) -> Plugin {
    assert!(
        matches!(app.screens().select.menu(), Some(Menu::Plugins)),
        "the browser is not on top: {:?}",
        app.screens().select.menu()
    );
    model(app).menu().clone()
}

/// The requests of the browser a step left, with their tickets.
fn sent(fx: &AppFx) -> Vec<(Ticket, Wanted)> {
    fx.commands()
        .iter()
        .filter_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Select(select::Command::Plugins(wanted)),
                ..
            } => Some((*ticket, wanted.clone())),
            _ => None,
        })
        .collect()
}

fn wants(fx: &AppFx) -> Vec<Want> {
    sent(fx).into_iter().map(|(_, wanted)| wanted.want).collect()
}

fn one(fx: &AppFx) -> (Ticket, Wanted) {
    let all = sent(fx);
    let [only] = all.as_slice() else {
        panic!("expected one request, got {all:?}");
    };
    only.clone()
}

fn answer(app: &mut CodexApp, request: &(Ticket, Wanted), result: Value) -> AppFx {
    let reply = select::decode(&select::Command::Plugins(request.1.clone()), result)
        .unwrap_or_else(|error| panic!("{:?} did not decode: {error:?}", request.1.want));
    run(
        app,
        AppEv::Reply(request.0, Reply::screen(ScreenReply::Select(reply))),
    )
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
        market("openai-bundled", Some("/m/bundled.json"), vec![plugin("b@b", "b", true, json!({}))]),
    ])
}

/// `/plugins` opened, its list answered and the sections after it answered empty.
fn browsing() -> CodexApp {
    let mut app = app(true, true);
    let fx = open(&mut app);
    let request = one(&fx);
    let fx = answer(&mut app, &request, local_market());
    for section in sent(&fx) {
        answer(&mut app, &section, listing(vec![]));
    }
    app
}

fn tabs(app: &CodexApp) -> Vec<plugins::Tab> {
    model(app).tabs()
}

fn said(app: &CodexApp) -> Vec<Say> {
    use codex_tui_core::threads::{Item, NoteKey};
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(note) => match &note.key {
                NoteKey::Select(Note::Plugins(say)) => Some(say.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn the_feature_off_says_so() {
    let mut app = ready();
    open(&mut app);
    assert!(app.screens().select.menu().is_none());
    assert_eq!(said(&app), vec![Say::Disabled]);
}

#[test]
fn opening_asks_the_list_and_loads() {
    let mut app = app(true, true);
    let fx = open(&mut app);
    let (_, wanted) = one(&fx);
    assert_eq!(wanted.want, Want::List);
    assert_eq!(wanted.cwd.as_deref(), Some(CWD));
    assert_eq!(menu(&app), Plugin::Loading);
}

#[test]
fn the_list_shows_and_the_sections_follow() {
    let mut app = app(false, true);
    let fx = open(&mut app);
    let request = one(&fx);
    let fx = answer(&mut app, &request, local_market());
    assert_eq!(menu(&app), Plugin::List);
    // Curated is the section of its own while the remote catalog is off.
    assert_eq!(
        wants(&fx),
        vec![
            Want::Section(plugins::Part::Vertical),
            Want::Section(plugins::Part::Workspace),
            Want::Section(plugins::Part::SharedWithMe),
        ]
    );
    assert!(tabs(&app).iter().any(|tab| matches!(
        tab.kind,
        Kind::Curated
    )));
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
    assert_eq!(all[0].rows[0].status, Status::Installed);
    assert_eq!(all[0].rows[2].status, Status::Available);
    assert_eq!(all[0].rows[0].toggle, Toggle::Switch(true));
    assert_eq!(all[0].rows[2].toggle, Toggle::Unavailable);
}

#[test]
fn a_remote_plugin_and_its_local_copy_are_one_row() {
    let mut app = app(true, true);
    let fx = open(&mut app);
    let request = one(&fx);
    let share = json!({"remotePluginId": "remote-1", "discoverability": "LISTED"});
    let local = plugin("shared@team", "shared", true, json!({"shareContext": share}));
    let remote = plugin(
        "shared-remote", "shared", false,
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
fn the_detail_is_read_from_the_marketplace_and_installed() {
    let mut app = browsing();
    let zeta = tabs(&app)[0].rows.iter().find(|row| row.name == "Zeta").unwrap().id.clone();
    let fx = choose(&mut app, &zeta);
    let request = one(&fx);
    assert_eq!(
        request.1.want,
        Want::Read(plugins::Target {
            location: plugins::Location::Local { path: "/m/curated.json".to_string() },
            plugin: "Zeta".to_string(),
        })
    );
    assert!(matches!(menu(&app), Plugin::DetailLoading { .. }));
    let detail = json!({"plugin": {
        "marketplaceName": "openai-curated", "marketplacePath": "/m/curated.json",
        "summary": plugin("zeta@curated", "Zeta", false, json!({})),
        "description": "Zeta does things", "skills": [{"name": "s1", "description": "d", "enabled": true}],
        "hooks": [{"key": "k", "eventName": "preToolUse"}, {"key": "j", "eventName": "preToolUse"}],
        "apps": [], "appTemplates": [], "mcpServers": ["srv"], "scheduledTasks": null,
    }});
    answer(&mut app, &request, detail);
    let Plugin::Detail(detail) = menu(&app) else {
        panic!("expected the detail, got {:?}", menu(&app));
    };
    assert_eq!(detail.skills, vec!["s1".to_string()]);
    assert_eq!(detail.hooks, vec![("PreToolUse".to_string(), 2)]);
    assert_eq!(detail.description.as_deref(), Some("Zeta does things"));
    assert!(matches!(detail.action(), plugins::Action::Install(_)));
    let fx = choose(&mut app, plugins::INSTALL);
    let request = one(&fx);
    assert!(matches!(request.1.want, Want::Install(_)));
    assert!(matches!(menu(&app), Plugin::InstallLoading { .. }));
    // Nothing more to sign in: the detail and the list are read again.
    let fx = answer(&mut app, &request, json!({"authPolicy": "ON_INSTALL", "appsNeedingAuth": []}));
    assert!(matches!(&wants(&fx)[..], [Want::List, Want::Read(_)]));
    assert_eq!(said(&app), vec![Say::Installed { name: "Zeta".to_string(), apps: vec![] }]);
}

#[test]
fn an_install_needing_apps_walks_through_their_setup() {
    let mut app = browsing();
    let zeta = tabs(&app)[0].rows.iter().find(|row| row.name == "Zeta").unwrap().id.clone();
    let read = one(&choose(&mut app, &zeta));
    answer(
        &mut app,
        &read,
        json!({"plugin": {
            "marketplaceName": "openai-curated", "marketplacePath": "/m/curated.json",
            "summary": plugin("zeta@curated", "Zeta", false, json!({})),
            "skills": [], "hooks": [], "apps": [], "appTemplates": [], "mcpServers": [],
        }}),
    );
    let install = one(&choose(&mut app, plugins::INSTALL));
    let apps = json!([
        {"id": "a1", "name": "First", "installUrl": "https://x/1"},
        {"id": "a2", "name": "Second"},
    ]);
    let fx = answer(&mut app, &install, json!({"authPolicy": "ON_INSTALL", "appsNeedingAuth": apps}));
    // The list is read again, the detail is not: the setup comes first.
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(menu(&app), Plugin::Auth);
    let step = app.screens().select.plugin_auth().expect("a step");
    assert_eq!((step.current, step.total, step.app.name.as_str()), (1, 2, "First"));
    // The list arriving does not replace the setup.
    let list = (sent(&fx)[0].0, sent(&fx)[0].1.clone());
    answer(&mut app, &list, local_market());
    assert_eq!(menu(&app), Plugin::Auth);

    choose(&mut app, plugins::OPEN);
    assert_eq!(
        app.screens().select.picked,
        vec![Picked::Open("https://x/1".to_string())]
    );
    let fx = choose(&mut app, plugins::NEXT);
    let _ = fx;
    let step = app.screens().select.plugin_auth().expect("the second step");
    assert_eq!((step.current, step.app.name.as_str()), (2, "Second"));
    choose(&mut app, plugins::NEXT);
    assert_eq!(menu(&app), Plugin::List);
    assert_eq!(
        said(&app).last(),
        Some(&Say::Completed { name: "Zeta".to_string() })
    );
}

#[test]
fn skipping_the_setup_says_so() {
    let mut app = browsing();
    let zeta = tabs(&app)[0].rows.iter().find(|row| row.name == "Zeta").unwrap().id.clone();
    let read = one(&choose(&mut app, &zeta));
    answer(
        &mut app,
        &read,
        json!({"plugin": {
            "marketplaceName": "openai-curated", "marketplacePath": "/m/curated.json",
            "summary": plugin("zeta@curated", "Zeta", false, json!({})),
            "skills": [], "hooks": [], "apps": [], "appTemplates": [], "mcpServers": [],
        }}),
    );
    let install = one(&choose(&mut app, plugins::INSTALL));
    answer(
        &mut app,
        &install,
        json!({"authPolicy": "ON_INSTALL", "appsNeedingAuth": [{"id": "a1", "name": "First"}]}),
    );
    choose(&mut app, plugins::SKIP);
    assert_eq!(menu(&app), Plugin::List);
    assert_eq!(said(&app).last(), Some(&Say::Skipped { name: "Zeta".to_string() }));
}

#[test]
fn a_failed_read_offers_the_way_back() {
    let mut app = browsing();
    let zeta = tabs(&app)[0].rows.iter().find(|row| row.name == "Zeta").unwrap().id.clone();
    let read = one(&choose(&mut app, &zeta));
    fail(&mut app, &read, "no such plugin");
    assert_eq!(
        menu(&app),
        Plugin::DetailFailed { error: "no such plugin".to_string(), back: true }
    );
    choose(&mut app, plugins::BACK);
    assert_eq!(menu(&app), Plugin::List);
}

#[test]
fn uninstall_says_so_and_reads_the_list_again() {
    let mut app = browsing();
    let alpha = tabs(&app)[0].rows.iter().find(|row| row.name == "alpha").unwrap().id.clone();
    let read = one(&choose(&mut app, &alpha));
    answer(
        &mut app,
        &read,
        json!({"plugin": {
            "marketplaceName": "openai-curated", "marketplacePath": "/m/curated.json",
            "summary": plugin("alpha@curated", "alpha", true, json!({})),
            "skills": [], "hooks": [], "apps": [], "appTemplates": [], "mcpServers": [],
        }}),
    );
    let fx = choose(&mut app, plugins::UNINSTALL);
    let request = one(&fx);
    assert_eq!(request.1.want, Want::Uninstall { id: "alpha@curated".to_string() });
    let fx = answer(&mut app, &request, json!({}));
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(said(&app), vec![Say::Uninstalled { name: "alpha".to_string() }]);
    // The list coming back replaces the loading page.
    let list = one(&fx);
    answer(&mut app, &list, local_market());
    assert_eq!(menu(&app), Plugin::List);
}

#[test]
fn a_failed_list_is_the_unavailable_page() {
    let mut app = app(true, true);
    let request = one(&open(&mut app));
    fail(&mut app, &request, "boom");
    assert_eq!(menu(&app), Plugin::Failed("boom".to_string()));
    // Opening again asks again.
    app.screens();
    select(&mut app, Select::Close);
    let fx = open(&mut app);
    assert_eq!(wants(&fx), vec![Want::List]);
}

#[test]
fn section_errors_say_what_to_do() {
    let mut app = app(true, false);
    let request = one(&open(&mut app));
    let fx = answer(&mut app, &request, local_market());
    let sections = sent(&fx);
    // Sharing is off: no shared request, and its error says so.
    assert_eq!(
        sections.iter().map(|(_, wanted)| wanted.want.clone()).collect::<Vec<_>>(),
        vec![Want::Section(plugins::Part::Workspace)]
    );
    fail(&mut app, &sections[0], "plugin sharing is disabled");
    let all = tabs(&app);
    let placeholder = all
        .iter()
        .find_map(|tab| match &tab.kind {
            Kind::Section { state: plugins::Placeholder::Failed(error), .. } => Some(error.clone()),
            _ => None,
        })
        .expect("the workspace failed");
    assert_eq!(placeholder.part, plugins::Part::Workspace);
    assert!(matches!(
        placeholder.cause,
        plugins::catalog::Cause::Failed { next: plugins::catalog::Next::AskAdminToEnable, .. }
    ));
}

#[test]
fn a_switch_writes_the_config_and_coalesces() {
    let mut app = browsing();
    let alpha = tabs(&app)[0].rows.iter().find(|row| row.name == "alpha").unwrap().id.clone();
    let fx = select(&mut app, Select::Toggle(alpha.clone()));
    let first = one(&fx);
    assert_eq!(
        first.1.want,
        Want::Enable { id: "alpha@curated".to_string(), enabled: false }
    );
    // The list shows the switch as flipped at once.
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(false));
    // Flipped twice while the write runs: nothing is sent until it is back.
    assert!(sent(&select(&mut app, Select::Toggle(alpha.clone()))).is_empty());
    assert!(sent(&select(&mut app, Select::Toggle(alpha.clone()))).is_empty());
    let fx = answer(&mut app, &first, json!({"status": "ok", "version": "v", "filePath": "/c", "overriddenMetadata": null}));
    // The last value asked for (off) differs from the one written (off)? It is the same: done.
    let _ = fx;
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(false));
}

#[test]
fn a_queued_switch_is_written_after_the_first() {
    let mut app = browsing();
    let alpha = tabs(&app)[0].rows.iter().find(|row| row.name == "alpha").unwrap().id.clone();
    let first = one(&select(&mut app, Select::Toggle(alpha.clone())));
    // Back on, queued behind the write that turns it off.
    select(&mut app, Select::Toggle(alpha.clone()));
    let fx = answer(&mut app, &first, json!({"status": "ok", "version": "v", "filePath": "/c", "overriddenMetadata": null}));
    let second = one(&fx);
    assert_eq!(
        second.1.want,
        Want::Enable { id: "alpha@curated".to_string(), enabled: true }
    );
    answer(&mut app, &second, json!({"status": "ok", "version": "v", "filePath": "/c", "overriddenMetadata": null}));
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(true));
}

#[test]
fn a_refused_switch_is_said() {
    let mut app = browsing();
    let alpha = tabs(&app)[0].rows.iter().find(|row| row.name == "alpha").unwrap().id.clone();
    let write = one(&select(&mut app, Select::Toggle(alpha)));
    fail(&mut app, &write, "read only");
    assert_eq!(
        said(&app),
        vec![Say::EnableFailed { id: "alpha@curated".to_string(), error: "read only".to_string() }]
    );
    assert_eq!(tabs(&app)[0].rows[0].toggle, Toggle::Switch(true));
}

#[test]
fn a_marketplace_is_added_from_a_prompt() {
    let mut app = browsing();
    select(&mut app, Select::Plugins(plugins::Intent::Tab("add-marketplace".to_string())));
    choose(&mut app, plugins::ADD_ROW);
    assert_eq!(menu(&app), Plugin::AddPrompt);
    let fx = select(&mut app, Select::Plugins(plugins::Intent::Source("./local#main".to_string())));
    let request = one(&fx);
    assert_eq!(request.1.want, Want::Add { source: "/work/project/local#main".to_string() });
    assert_eq!(menu(&app), Plugin::AddLoading);
    let fx = answer(
        &mut app,
        &request,
        json!({"marketplaceName": "local", "installedRoot": "/home/me/market", "alreadyAdded": false}),
    );
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(model(&app).tab(), "marketplace:/home/me/market");
    assert_eq!(
        said(&app),
        vec![Say::MarketAdded {
            name: "local".to_string(),
            root: "/home/me/market".to_string(),
            already: false
        }]
    );
}

#[test]
fn a_failed_add_offers_to_try_again() {
    let mut app = browsing();
    choose(&mut app, plugins::ADD_ROW);
    let fx = select(&mut app, Select::Plugins(plugins::Intent::Source("owner/repo".to_string())));
    fail(&mut app, &one(&fx), "no such repo");
    assert_eq!(menu(&app), Plugin::AddFailed);
    assert_eq!(model(&app).tab(), "add-marketplace");
    choose(&mut app, plugins::RETRY);
    assert_eq!(menu(&app), Plugin::AddPrompt);
}

#[test]
fn a_configured_marketplace_can_be_removed() {
    let mut app = browsing();
    let team = "marketplace:/home/me/team/marketplace.json".to_string();
    // The curated tab is not the user's: nothing to remove.
    select(&mut app, Select::Plugins(plugins::Intent::Tab("marketplace:openai-curated".to_string())));
    select(&mut app, Select::Plugins(plugins::Intent::Remove));
    assert_eq!(menu(&app), Plugin::List);
    select(&mut app, Select::Plugins(plugins::Intent::Tab(team)));
    select(&mut app, Select::Plugins(plugins::Intent::Remove));
    assert_eq!(
        menu(&app),
        Plugin::RemoveConfirm { name: "team".to_string(), label: "team".to_string() }
    );
    let fx = choose(&mut app, plugins::REMOVE);
    let request = one(&fx);
    assert_eq!(request.1.want, Want::Remove { name: "team".to_string() });
    let fx = answer(&mut app, &request, json!({"marketplaceName": "team", "installedRoot": null}));
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(model(&app).tab(), "all-plugins");
    assert_eq!(
        said(&app),
        vec![Say::MarketRemoved { label: "team".to_string(), name: "team".to_string(), root: None }]
    );
}

#[test]
fn a_git_marketplace_is_upgraded_and_a_failure_offers_the_confirmation_again() {
    let mut app = browsing();
    let team = "marketplace:/home/me/team/marketplace.json".to_string();
    select(&mut app, Select::Plugins(plugins::Intent::Tab(team)));
    let fx = select(&mut app, Select::Plugins(plugins::Intent::Upgrade));
    let request = one(&fx);
    assert_eq!(request.1.want, Want::Upgrade { name: Some("team".to_string()) });
    let fx = answer(
        &mut app,
        &request,
        json!({"selectedMarketplaces": ["team"], "upgradedRoots": ["/home/me/team"], "errors": []}),
    );
    assert_eq!(wants(&fx), vec![Want::List]);
    assert_eq!(model(&app).tab(), "marketplace:/home/me/team");
    assert_eq!(said(&app), vec![Say::Upgraded { roots: vec!["/home/me/team".to_string()] }]);
}

#[test]
fn a_relative_source_is_made_absolute() {
    assert_eq!(plugins::add_source("/w/p", "./m"), "/w/p/m");
    assert_eq!(plugins::add_source("/w/p", "../m#main"), "/w/m#main");
    assert_eq!(plugins::add_source("/w/p", "owner/repo"), "owner/repo");
    assert_eq!(plugins::add_source("/w/p", "~/m"), "~/m");
    assert_eq!(plugins::add_source("/w/p", "."), "/w/p");
}

#[test]
fn escape_goes_back_from_a_page_and_closes_the_list() {
    let mut app = browsing();
    choose(&mut app, plugins::ADD_ROW);
    assert_eq!(menu(&app), Plugin::AddPrompt);
    select(&mut app, Select::Close);
    assert_eq!(menu(&app), Plugin::List);
    select(&mut app, Select::Close);
    assert!(app.screens().select.menu().is_none());
}
