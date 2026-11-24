//! The seeds of the `/plugins` tests: the plugin config the host reads (the feature's `Fact::Config`) and the
//! app server's plugin answers (`plugin/list`, `plugin/read`, the installs and the marketplace
//! calls), built the way the oracle's `plugins_test_*` helpers build its responses, as the wire's
//! JSON. A `Shelf` is what the server holds: a test changes it between two opens of the browser
//! (a marketplace added, removed) as the server's state changes.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use codex_tui_core::app::{AppEv, Command};
use codex_tui_core::features::plugins::{Config, Configured, Fact, Location, Part, Target, Want};
use codex_tui_core::features::{FeatureCommand, FeatureFact};
use codex_tui_core::rpc::ServerError;
use serde_json::{Value, json};

use crate::common::{Harness, start_driven_with};
use crate::kit::Screen;

/// What the app server holds of the plugins.
#[derive(Default)]
pub struct Held {
    /// The marketplaces `plugin/list` answers with a local path.
    pub local: Vec<Value>,
    /// The ones without a path: the remote sections, answered by kind.
    pub remote: Vec<Value>,
    /// `marketplaceLoadErrors`.
    pub errors: Vec<Value>,
    /// The `plugin/read` answers by plugin name.
    pub details: BTreeMap<String, Value>,
    /// The answers of the other calls by wire method.
    pub replies: BTreeMap<&'static str, Result<Value, String>>,
    /// Methods the server leaves unanswered: a loading state stays up.
    pub held: BTreeSet<&'static str>,
    /// A failure of `plugin/list` (the local one).
    pub list_error: Option<String>,
    /// A failure of the remote section asked in this kind (`workspace-directory`, ...).
    pub section_errors: BTreeMap<&'static str, String>,
}

/// The pseudo method that leaves the sections of the remote marketplaces unanswered.
pub const SECTIONS: &str = "plugin/list:sections";

pub type Shelf = Arc<Mutex<Held>>;

/// A shelf with these local marketplaces.
pub fn shelf(local: Vec<Value>) -> Shelf {
    Arc::new(Mutex::new(Held {
        local,
        ..Held::default()
    }))
}

/// Changes what the server holds.
pub fn with(shelf: &Shelf, change: impl FnOnce(&mut Held)) {
    change(&mut shelf.lock().expect("the shelf"));
}

/// The plugins feature on, the home directory of this machine, no configured marketplace.
pub fn config() -> Config {
    Config {
        enabled: true,
        remote: false,
        // the oracle's loaded-popup helper has no section error: sharing is on, the section answers empty
        sharing: true,
        home: Some(home()),
        configured: Vec::new(),
    }
}

/// The home directory of this machine.
pub fn home() -> String {
    std::env::var("HOME").expect("a home directory")
}

/// The marketplace `name` the user's config says is a git one.
pub fn git(name: &str) -> Configured {
    Configured {
        name: name.to_string(),
        git: true,
    }
}

/// The app on a terminal of this size, its plugin config read, the server answering as `shelf`
/// holds, the user's key bindings (`list.move_down` and so on) as given.
pub fn start_with(
    width: u16,
    height: u16,
    config: Config,
    shelf: &Shelf,
    keymap: &[(&str, &str)],
) -> Screen {
    let mut h: Harness = start_driven_with(false, rtwk_test::Options::new().size(width, height));
    let held = shelf.clone();
    h.fake
        .respond(move |request| answer(&held.lock().expect("the shelf"), request));
    h.connect();
    h.inject([AppEv::Feature(FeatureFact::from(Fact::Config(config)))]);
    if !keymap.is_empty() {
        h.set_keymap(keymap);
    }
    h.quiesce();
    Screen::from(h)
}

/// `start_with` the default bindings.
pub fn start(width: u16, height: u16, config: Config, shelf: &Shelf) -> Screen {
    start_with(width, height, config, shelf, &[])
}

/// `start`, then `/plugins`.
pub fn open(width: u16, height: u16, config: Config, shelf: &Shelf) -> Screen {
    let mut s = start(width, height, config, shelf);
    s.type_text("/plugins").keys("enter");
    s
}

/// What the app asked of the plugin backend, oldest first, with the command to answer it by.
pub fn asked(s: &Screen) -> Vec<(Command, Want)> {
    s.commands()
        .into_iter()
        .filter_map(|command| match &command {
            Command::Feature {
                command: FeatureCommand::Plugins(wanted),
                ..
            } => {
                let want = wanted.want.clone();
                Some((command, want))
            }
            _ => None,
        })
        .collect()
}

/// A kind of ask of the plugin backend (`last_of`).
#[derive(Clone, Copy)]
pub enum Kind {
    Add,
    Remove,
    Enable,
}

impl Kind {
    fn is(self, want: &Want) -> bool {
        match self {
            Kind::Add => matches!(want, Want::Add { .. }),
            Kind::Remove => matches!(want, Want::Remove { .. }),
            Kind::Enable => matches!(want, Want::Enable { .. }),
        }
    }
}

/// The last request of the app of this kind.
pub fn last_of(s: &Screen, kind: Kind) -> Command {
    last(s, |want| kind.is(want))
}

fn remote(marketplace: &str, plugin: &str) -> Target {
    Target {
        location: Location::Remote {
            marketplace: marketplace.to_string(),
        },
        plugin: plugin.to_string(),
    }
}

/// The marketplaces the app asked to add, in order, are `sources`.
pub fn assert_added(s: &Screen, sources: &[&str]) {
    let got: Vec<Want> = wants(s).into_iter().filter(|w| Kind::Add.is(w)).collect();
    let want: Vec<Want> = sources
        .iter()
        .map(|source| Want::Add {
            source: (*source).to_string(),
        })
        .collect();
    assert_eq!(got, want);
}

/// The marketplaces the app asked to upgrade, in order, are `names` (`None`: all of them).
pub fn assert_upgraded(s: &Screen, names: &[Option<&str>]) {
    let got: Vec<Want> = wants(s)
        .into_iter()
        .filter(|w| matches!(w, Want::Upgrade { .. }))
        .collect();
    let want: Vec<Want> = names
        .iter()
        .map(|name| Want::Upgrade {
            name: name.map(str::to_string),
        })
        .collect();
    assert_eq!(got, want);
}

/// The marketplaces the app asked to remove, in order, are `names`.
pub fn assert_removed(s: &Screen, names: &[&str]) {
    let got: Vec<Want> = wants(s).into_iter().filter(|w| Kind::Remove.is(w)).collect();
    let want: Vec<Want> = names
        .iter()
        .map(|name| Want::Remove {
            name: (*name).to_string(),
        })
        .collect();
    assert_eq!(got, want);
}

/// The plugin switches the app asked for, in order, are `(id, enabled)`.
pub fn assert_enables(s: &Screen, asks: &[(&str, bool)]) {
    let got: Vec<Want> = wants(s).into_iter().filter(|w| Kind::Enable.is(w)).collect();
    let want: Vec<Want> = asks
        .iter()
        .map(|(id, enabled)| Want::Enable {
            id: (*id).to_string(),
            enabled: *enabled,
        })
        .collect();
    assert_eq!(got, want);
}

/// No plugin switch was asked.
pub fn assert_no_enable(s: &Screen) {
    assert!(
        !wants(s).iter().any(|w| Kind::Enable.is(w)),
        "{:?}",
        wants(s)
    );
}

/// No plugin install was asked.
pub fn assert_no_install(s: &Screen) {
    assert!(
        !wants(s).iter().any(|w| matches!(w, Want::Install(_))),
        "{:?}",
        wants(s)
    );
}

/// The detail of `plugin` of the remote `marketplace` was asked `times` times.
pub fn assert_read_remote(s: &Screen, marketplace: &str, plugin: &str, times: usize) {
    let target = Want::Read(remote(marketplace, plugin));
    let got = wants(s).into_iter().filter(|w| *w == target).count();
    assert_eq!(got, times, "{:?}", wants(s));
}

/// The install of `plugin` of the remote `marketplace` was asked.
pub fn assert_install_remote(s: &Screen, marketplace: &str, plugin: &str) {
    let target = Want::Install(remote(marketplace, plugin));
    assert!(wants(s).contains(&target), "{:?}", wants(s));
}

/// The config of a user whose `names` are git marketplaces.
pub fn config_with(names: &[&str]) -> Config {
    let mut config = config();
    config.configured = names.iter().map(|name| git(name)).collect();
    config
}

/// The wants alone.
pub fn wants(s: &Screen) -> Vec<Want> {
    asked(s).into_iter().map(|(_, want)| want).collect()
}

/// The last request of the app that `pick` takes.
pub fn last(s: &Screen, pick: impl Fn(&Want) -> bool) -> Command {
    asked(s)
        .into_iter()
        .rev()
        .find(|(_, want)| pick(want))
        .map(|(command, _)| command)
        .expect("the app asked for it")
}

/// The server leaves the next `plugin/list` unanswered until `refresh`.
pub fn hold_list(shelf: &Shelf) {
    with(shelf, |held| {
        held.held.insert("plugin/list");
    });
}

/// The pending `plugin/list` is answered with `local`, and the server answers as it holds from
/// then on (the sections that follow the list are answered at once).
pub fn refresh(s: &mut Screen, shelf: &Shelf, local: Vec<Value>) {
    let command = last(s, |want| matches!(want, Want::List));
    with(shelf, |held| {
        held.local = local.clone();
        held.held.remove("plugin/list");
    });
    s.answer_request(
        &command,
        json!({ "marketplaces": local, "marketplaceLoadErrors": [], "featuredPluginIds": [] }),
    );
}

/// The sections that were held are answered as the server holds them.
pub fn serve_sections(s: &mut Screen, shelf: &Shelf) {
    with(shelf, |held| {
        held.held.remove(SECTIONS);
    });
    let sections: Vec<(Command, Part)> = asked(s)
        .into_iter()
        .filter_map(|(command, want)| match want {
            Want::Section(part) => Some((command, part)),
            _ => None,
        })
        .collect();
    for (command, part) in sections {
        let kind = match part {
            Part::Vertical => "vertical",
            Part::Workspace => "workspace-directory",
            Part::SharedWithMe => "shared-with-me",
        };
        let request = json!({ "method": "plugin/list", "params": { "marketplaceKinds": [kind] } });
        let answered = answer(&shelf.lock().expect("the shelf"), &request);
        if let Some(Ok(result)) = answered {
            s.answer_request(&command, result);
        }
    }
}

/// `plugin/list` is answered again with `local` while the browser is shown: the user closes it, the
/// server holds the list, the user opens `/plugins` (the cached list shows) and the refresh comes
/// later, `refresh`.
pub fn reopen_held(s: &mut Screen, shelf: &Shelf) {
    s.keys("esc");
    hold_list(shelf);
    s.type_text("/plugins").keys("enter");
}

/// The wire method of a request.
fn answer(held: &Held, request: &Value) -> Option<Result<Value, ServerError>> {
    let method = request["method"].as_str()?;
    let fail = |message: &str| {
        Some(Err(ServerError {
            code: None,
            data: None,
            message: message.to_string(),
        }))
    };
    if held.held.contains(method) {
        return None;
    }
    match method {
        "plugin/list" => {
            let kinds = request["params"]["marketplaceKinds"].as_array();
            if kinds.is_some() && held.held.contains(SECTIONS) {
                return None;
            }
            match kinds {
                None => {
                    if let Some(error) = &held.list_error {
                        return fail(error);
                    }
                    Some(Ok(json!({
                        "marketplaces": held.local,
                        "marketplaceLoadErrors": held.errors,
                        "featuredPluginIds": [],
                    })))
                }
                Some(kinds) => {
                    let kind = kinds.first().and_then(Value::as_str).unwrap_or_default();
                    if let Some(error) = held.section_errors.get(kind) {
                        return fail(error);
                    }
                    let mine: Vec<&Value> = held
                        .remote
                        .iter()
                        .filter(|market| {
                            section_of(market["name"].as_str().unwrap_or_default()) == kind
                        })
                        .collect();
                    Some(Ok(json!({
                        "marketplaces": mine,
                        "marketplaceLoadErrors": [],
                        "featuredPluginIds": [],
                    })))
                }
            }
        }
        "plugin/read" => {
            if let Some(Err(message)) = held.replies.get(method) {
                return fail(message);
            }
            // `*` is the detail of any plugin asked for
            let name = request["params"]["pluginName"].as_str()?;
            match held.details.get(name).or_else(|| held.details.get("*")) {
                Some(plugin) => Some(Ok(json!({ "plugin": plugin }))),
                None => fail("plugin not found"),
            }
        }
        other => match held.replies.get(other)? {
            Ok(result) => Some(Ok(result.clone())),
            Err(message) => fail(message),
        },
    }
}

/// The section a remote marketplace is asked in, by its name.
fn section_of(name: &str) -> &'static str {
    if name.starts_with("workspace-directory") {
        "workspace-directory"
    } else if name.starts_with("workspace-shared-with-me") {
        "shared-with-me"
    } else {
        "vertical"
    }
}

// ---- the oracle's `plugins_test_*` helpers, as JSON --------------------------------------------

/// `plugins_test_absolute_path`.
pub fn path(relative: &str) -> String {
    std::env::temp_dir()
        .join("codex-plugin-menu-tests")
        .join(relative)
        .to_string_lossy()
        .into_owned()
}

/// `plugins_test_personal_marketplace_path`.
pub fn personal_path() -> String {
    std::path::Path::new(&home())
        .join(".agents/plugins/marketplace.json")
        .to_string_lossy()
        .into_owned()
}

/// `plugins_test_interface`.
pub fn interface(display: Option<&str>, short: Option<&str>, long: Option<&str>) -> Value {
    json!({
        "displayName": display, "shortDescription": short, "longDescription": long,
        "developerName": null, "category": null, "capabilities": [], "websiteUrl": null,
        "privacyPolicyUrl": null, "termsOfServiceUrl": null, "defaultPrompt": null,
        "brandColor": null, "composerIcon": null, "composerIconUrl": null, "logo": null,
        "logoDark": null, "logoUrl": null, "logoUrlDark": null, "screenshots": [],
        "screenshotUrls": [],
    })
}

/// `plugins_test_summary`; `policy` is `AVAILABLE`, `NOT_AVAILABLE` or `INSTALLED_BY_DEFAULT`.
pub fn summary(
    id: &str,
    name: &str,
    display: Option<&str>,
    description: Option<&str>,
    installed: bool,
    enabled: bool,
    policy: &str,
) -> Value {
    json!({
        "id": id, "remotePluginId": null, "version": null, "localVersion": null,
        "name": name, "shareContext": null,
        "source": { "type": "local", "path": path(&format!("plugins/{name}")) },
        "installed": installed, "installedAt": null, "enabled": enabled,
        "installPolicy": policy, "installPolicySource": null,
        "mustShowInstallationInterstitial": null, "authPolicy": "ON_INSTALL",
        "availability": "AVAILABLE", "disabledReason": null, "eligiblePlanTypes": null,
        "interface": interface(display, description, None), "keywords": [],
    })
}

/// `v` with these fields of its object replaced (`PluginSummary { share_context, ..summary }`).
pub fn set(mut v: Value, fields: &[(&str, Value)]) -> Value {
    for (key, value) in fields {
        v[*key] = value.clone();
    }
    v
}

/// A plugin that can be installed and is on.
pub fn available(id: &str, name: &str, display: &str, description: &str) -> Value {
    summary(
        id,
        name,
        Some(display),
        Some(description),
        false,
        true,
        "AVAILABLE",
    )
}

/// `plugins_test_remote_summary`.
pub fn remote_summary(
    remote_id: &str,
    name: &str,
    display: Option<&str>,
    description: Option<&str>,
    installed: bool,
) -> Value {
    json!({
        "id": remote_id, "remotePluginId": remote_id, "version": null, "localVersion": null,
        "name": name, "shareContext": null, "source": { "type": "remote" },
        "installed": installed, "installedAt": null, "enabled": true,
        "installPolicy": "AVAILABLE", "installPolicySource": null,
        "mustShowInstallationInterstitial": null, "authPolicy": "ON_INSTALL",
        "availability": "AVAILABLE", "disabledReason": null, "eligiblePlanTypes": null,
        "interface": interface(display, description, None), "keywords": [],
    })
}

/// A marketplace entry of `plugin/list`.
pub fn marketplace(
    name: &str,
    path: Option<String>,
    display: Option<&str>,
    plugins: Vec<Value>,
) -> Value {
    json!({
        "name": name, "path": path,
        "interface": display.map(|display| json!({ "displayName": display })),
        "plugins": plugins,
    })
}

/// `plugins_test_remote_marketplace`.
pub fn remote_marketplace(name: &str, display: &str, plugins: Vec<Value>) -> Value {
    marketplace(name, None, Some(display), plugins)
}

/// `plugins_test_curated_marketplace`.
pub fn curated(plugins: Vec<Value>) -> Value {
    marketplace(
        "openai-curated",
        Some(path("marketplaces/chatgpt")),
        Some("ChatGPT Marketplace"),
        plugins,
    )
}

/// `plugins_test_repo_marketplace`.
pub fn repo(plugins: Vec<Value>) -> Value {
    marketplace(
        "repo",
        Some(path("marketplaces/repo")),
        Some("Repo Marketplace"),
        plugins,
    )
}

/// `plugins_test_detail`: `hooks` are `(event name as the wire says it, count)`.
pub fn detail(
    summary: Value,
    description: Option<&str>,
    skills: &[&str],
    hooks: &[(&str, usize)],
    apps: &[&str],
    mcp_servers: &[&str],
) -> Value {
    let skills: Vec<Value> = skills
        .iter()
        .map(|name| {
            json!({
                "name": name, "description": format!("{name} description"),
                "shortDescription": null, "interface": null,
                "path": path(&format!("skills/{name}/SKILL.md")), "enabled": true,
            })
        })
        .collect();
    let hooks: Vec<Value> = hooks
        .iter()
        .enumerate()
        .flat_map(|(event, (name, count))| {
            (0..*count).map(move |handler| {
                json!({ "key": format!("plugin:{event}:{handler}"), "eventName": name })
            })
        })
        .collect();
    let apps: Vec<Value> = apps
        .iter()
        .map(|name| {
            json!({
                "id": format!("{name}-id"), "name": name, "description": format!("{name} app"),
                "installUrl": format!("https://example.test/{name}"), "category": null,
            })
        })
        .collect();
    json!({
        "marketplaceName": "ChatGPT Marketplace",
        "marketplacePath": path("marketplaces/chatgpt"),
        "summary": summary, "shareUrl": null, "description": description,
        "skills": skills, "onboardingSkill": null, "hooks": hooks, "apps": apps,
        "appTemplates": [], "mcpServers": mcp_servers, "scheduledTasks": null,
    })
}

/// `plugins_test_remote_detail`.
pub fn remote_detail(marketplace: &str, summary: Value, description: Option<&str>) -> Value {
    json!({
        "marketplaceName": marketplace, "marketplacePath": null,
        "summary": summary, "shareUrl": null, "description": description,
        "skills": [], "onboardingSkill": null, "hooks": [], "apps": [],
        "appTemplates": [], "mcpServers": [], "scheduledTasks": null,
    })
}
