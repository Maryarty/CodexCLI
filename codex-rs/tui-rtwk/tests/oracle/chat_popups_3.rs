//! Shard `chat_popups_3` (screens S14, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use codex_tui_core::features::plugins::{Location, Target, Want};
use serde_json::json;

use crate::kit::{self, Owner, Screen};

#[path = "fixtures/chat_popups_3/mouse.rs"]
mod mouse;
#[path = "fixtures/chat_popups_3/seed.rs"]
pub(crate) mod seed;

// What these tests need is not in the port yet (each reason says which); the
// bodies are the oracle's steps and visible assertions, written in full.
// assert_snap compares the whole screen; the oracle's popup snapshots hold the
// bottom popup only, so the popup slice sets the region (assert_snap_region).
const APPS: &str = "/apps is live and the kit answers a request with an error (`Screen::answer_error`, a `respond` step with `error`); this test needs a partial directory snapshot: the oracle injects `on_connectors_loaded(.., is_final: false)` and no protocol message carries `is_final: false`";
const MENTIONS: &str = "/apps is live and `app/list/updated` drops the uncallable apps from the mentions (core/tests/apps_directory.rs); this test also needs the oracle's separate installed-mentions fetch (`FetchInstalledConnectorMentions`, `connectors_for_mentions` keeps only enabled installed apps), a directory partial snapshot (`is_final: false`, no protocol message carries it), the submitted `app://` mention of `$` and the plugin authentication step's `Already installed in this session` (the install flow of /plugins reading the installed ids): the mention source and the plugin flow are other owners'";
const HOOKS: &str = "no /hooks popup in the port: no hooks list view with its warnings and errors, and no seed for the hooks list result";

/// The first row that contains `text`; past the screen when none does (the kit then names the row it looked for).
pub(crate) fn row_of(s: &Screen, text: &str) -> usize {
    s.rows()
        .iter()
        .position(|r| r.contains(text))
        .unwrap_or(usize::MAX)
}

/// The screen's text with every run of white space one space: a history cell the width wraps reads as one line.
fn flat(s: &Screen) -> String {
    s.text().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The plugin the marketplace tabs of these tests hold.
fn debug_plugin() -> serde_json::Value {
    seed::available(
        "plugin-debug",
        "debug",
        "Debug Plugin",
        "Debug marketplace plugin.",
    )
}

/// A marketplace the user's config holds as a git one, shown as `Debug Marketplace`.
fn debug_market() -> serde_json::Value {
    seed::marketplace(
        "debug",
        Some(seed::path(
            "marketplaces/debug/.agents/plugins/marketplace.json",
        )),
        Some("Debug Marketplace"),
        vec![debug_plugin()],
    )
}

/// The config of a user whose `names` are git marketplaces.
fn config_with(names: &[&str]) -> codex_tui_core::features::plugins::Config {
    let mut config = seed::config();
    config.configured = names.iter().map(|name| seed::git(name)).collect();
    config
}

/// The browser over the curated marketplace holding `plugins` (the oracle's `render_loaded_plugins_popup`).
fn browser(width: u16, plugins: Vec<serde_json::Value>) -> Screen {
    let shelf = seed::shelf(vec![seed::curated(plugins)]);
    seed::open(width, 24, seed::config(), &shelf)
}

/// A plugin of the oracle's tests: `(id, name, display, description, installed)`, on, installable.
pub(crate) fn plugin(
    id: &str,
    name: &str,
    display: &str,
    description: &str,
    installed: bool,
) -> serde_json::Value {
    seed::summary(
        id,
        name,
        Some(display),
        Some(description),
        installed,
        true,
        "AVAILABLE",
    )
}

/// The detail of the figma plugin of the detail snapshots.
fn figma_detail(summary: serde_json::Value) -> serde_json::Value {
    seed::detail(
        summary,
        Some("Turn Figma files into implementation context."),
        &["design-review", "extract-copy"],
        &[("preToolUse", 1), ("stop", 2)],
        &["Figma", "Slack"],
        &["figma-mcp", "docs-mcp"],
    )
}

/// The `app/list` requests the app has made, oldest first.
pub(crate) fn apps_asked(s: &Screen) -> Vec<codex_tui_core::app::Command> {
    use codex_tui_core::app::Command;
    use codex_tui_core::screens::ScreenCommand;
    use codex_tui_core::screens::select::Command as Select;
    s.commands()
        .into_iter()
        .filter(|command| {
            matches!(
                command,
                Command::Screen {
                    command: ScreenCommand::Select(Select::Apps { .. }),
                    ..
                }
            )
        })
        .collect()
}

/// An `app/list` answer: `(id, name, description)` of apps installed and on.
pub(crate) fn apps_answer(apps: &[(&str, &str, &str)]) -> serde_json::Value {
    let data: Vec<_> = apps
        .iter()
        .map(|(id, name, about)| {
            serde_json::json!({
                "id": id, "name": name, "description": about,
                "installUrl": format!("https://example.test/{id}"),
                "isAccessible": true, "isEnabled": true, "pluginDisplayNames": []
            })
        })
        .collect();
    serde_json::json!({ "data": data, "nextCursor": null })
}

/// The oracle's `select_plugins_tab_containing`: moves the tab right until one shows `text`.
pub(crate) fn select_tab(s: &mut Screen, text: &str) {
    for _ in 0..8 {
        if s.text().contains(text) {
            return;
        }
        s.keys("right");
    }
}

// ---- oracle: chatwidget/tests/popups_and_settings.rs (43 screen-level tests here; 4 state tests left to dialogs-pane)
// ORACLE chatwidget/tests/popups_and_settings.rs:66 plugins_popup_loading_state_snapshot kind=snap owner=dialogs-pane snap=plugins_popup_loading_state
#[test]
fn plugins_popup_loading_state_snapshot() {
    // the marketplace list has not arrived: the server leaves `plugin/list` unanswered
    let shelf = seed::shelf(Vec::new());
    seed::with(&shelf, |held| {
        held.held.insert("plugin/list");
    });
    let s = seed::open(100, 24, seed::config(), &shelf);
    s.assert_contains("Loading available plugins...");
    s.assert_snap_region("plugins_popup_loading_state", row_of(&s, "Plugins"), 0);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:81 marketplace_upgrade_loading_popup_snapshot kind=snap owner=dialogs-pane
#[test]
fn marketplace_upgrade_loading_popup_snapshot() {
    // the oracle opens the upgrade loading popup of `debug` directly; the port reaches it from the marketplace tab
    let shelf = seed::shelf(vec![seed::curated(Vec::new()), debug_market()]);
    seed::with(&shelf, |held| {
        held.held.insert("marketplace/upgrade");
    });
    let mut s = seed::open(100, 24, config_with(&["debug"]), &shelf);
    select_tab(&mut s, "Debug Marketplace.");
    s.keys("ctrl+u");
    s.assert_contains("Upgrading debug marketplace...");
    s.assert_contains("This updates when marketplace upgrade completes");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:101 marketplace_upgrade_failure_includes_backend_messages_snapshot kind=snap owner=dialogs-pane
#[test]
fn marketplace_upgrade_failure_includes_backend_messages_snapshot() {
    // the upgrade result arrives from the app server; the history cell it inserts is the oracle's text
    let shelf = seed::shelf(vec![seed::curated(Vec::new()), debug_market()]);
    seed::with(&shelf, |held| {
        held.replies.insert(
            "marketplace/upgrade",
            Ok(json!({
                "selectedMarketplaces": ["debug", "tools"],
                "upgradedRoots": [],
                "errors": [
                    { "marketplaceName": "debug", "message": "git ls-remote marketplace source failed with status 128: authentication failed" },
                    { "marketplaceName": "tools", "message": "failed to validate upgraded marketplace root: marketplace root does not contain a supported manifest" },
                ],
            })),
        );
    });
    let mut s = seed::open(100, 24, config_with(&["debug"]), &shelf);
    select_tab(&mut s, "Debug Marketplace.");
    s.keys("ctrl+u esc");
    let text = flat(&s);
    for fragment in [
        "Failed to upgrade 2 marketplaces",
        "debug: git ls-remote marketplace source failed with status 128: authentication failed",
        "tools: failed to validate upgraded marketplace root",
    ] {
        assert!(text.contains(fragment), "{fragment}\n{}", s.text());
    }
}

// ORACLE chatwidget/tests/popups_and_settings.rs:136 hooks_popup_shows_list_diagnostics kind=snap owner=dialogs-pane snap=hooks_popup_shows_list_diagnostics
#[test]
fn hooks_popup_shows_list_diagnostics() {
    kit::pending(Owner::DialogsPane, HOOKS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(112, 24)
        .live()
        .open();
    s.type_text("/hooks").keys("enter");
    s.assert_snap("hooks_popup_shows_list_diagnostics");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:160 plugins_popup_snapshot_shows_all_marketplaces_and_sorts_installed_then_name kind=snap owner=dialogs-pane snap=plugins_popup_curated_marketplace
#[test]
fn plugins_popup_snapshot_shows_all_marketplaces_and_sorts_installed_then_name() {
    let shelf = seed::shelf(vec![
        seed::curated(vec![
            plugin(
                "plugin-bravo",
                "bravo",
                "Bravo Search",
                "Search docs and tickets.",
                false,
            ),
            seed::summary(
                "plugin-alpha",
                "alpha",
                Some("Alpha Sync"),
                Some("Already installed but disabled."),
                true,
                false,
                "AVAILABLE",
            ),
            seed::summary(
                "plugin-starter",
                "starter",
                Some("Starter"),
                Some("Included by default."),
                false,
                false,
                "INSTALLED_BY_DEFAULT",
            ),
        ]),
        seed::repo(vec![plugin(
            "plugin-hidden",
            "hidden",
            "Hidden Repo Plugin",
            "Should not be shown in /plugins.",
            false,
        )]),
    ]);
    let s = seed::open(320, 24, seed::config(), &shelf);
    s.assert_snap_region(
        "plugins_popup_curated_marketplace",
        row_of(&s, "Plugins"),
        0,
    );
    s.assert_contains("Hidden Repo Plugin");
    s.assert_in_order(&[
        "Alpha Sync",
        "Bravo Search",
        "Hidden Repo Plugin",
        "Starter",
    ]);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:223 plugins_popup_keeps_loaded_marketplace_state_with_load_errors kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_keeps_loaded_marketplace_state_with_load_errors() {
    let shelf = seed::shelf(vec![seed::curated(Vec::new())]);
    seed::with(&shelf, |held| {
        held.errors.push(json!({
            "marketplacePath": seed::path("marketplaces/broken"),
            "message": "failed to load marketplace manifest",
        }));
    });
    let s = seed::open(100, 24, seed::config(), &shelf);
    s.assert_contains("No marketplace plugins available");
    s.assert_absent("Marketplace unavailable");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:244 plugins_popup_truncates_long_descriptions_in_list_rows kind=snap owner=dialogs-pane
#[test]
fn plugins_popup_truncates_long_descriptions_in_list_rows() {
    let s = browser(
        70,
        vec![
            plugin(
                "plugin-alpha",
                "alpha",
                "Alpha",
                "Short description.",
                false,
            ),
            plugin(
                "plugin-verbose",
                "verbose",
                "Verbose Plugin",
                "This description keeps going and going until the row would normally wrap.",
                false,
            ),
        ],
    );
    s.assert_contains("  [-] Verbose Plugin  Available · OpenAI Curated · This description k…");
    s.assert_absent("This description keeps going and going until the row would normally wrap.");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:290 plugins_popup_add_marketplace_tab_opens_prompt_and_submits_source kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_add_marketplace_tab_opens_prompt_and_submits_source() {
    let shelf = seed::shelf(vec![seed::curated(Vec::new())]);
    seed::with(&shelf, |held| {
        held.held.insert("marketplace/add");
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    select_tab(&mut s, "Add a marketplace from a Git repo or local root.");
    s.assert_contains("Add a marketplace from a Git repo or local root.");
    s.keys("enter");
    s.assert_contains("owner/repo, git URL, or local marketplace path");
    s.paste("owner/repo").keys("enter");
    // the add request goes out once, for the source typed
    let adds: Vec<_> = seed::wants(&s)
        .into_iter()
        .filter(|want| matches!(want, Want::Add { .. }))
        .collect();
    assert_eq!(
        adds,
        vec![Want::Add {
            source: "owner/repo".to_string()
        }]
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:346 plugins_popup_upgrades_user_configured_git_marketplace_from_marketplace_tab kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_upgrades_user_configured_git_marketplace_from_marketplace_tab() {
    let shelf = seed::shelf(vec![
        seed::curated(Vec::new()),
        seed::repo(vec![debug_plugin()]),
    ]);
    seed::with(&shelf, |held| {
        held.held.insert("marketplace/upgrade");
    });
    let mut s = seed::open(100, 24, config_with(&["repo"]), &shelf);
    select_tab(&mut s, "Repo Marketplace.");
    s.assert_contains("Repo Marketplace.");
    s.assert_contains("⌃u upgrade");
    s.assert_contains("⌃r remove");
    s.assert_contains("Debug Plugin");
    s.keys("ctrl+u ctrl+u");
    // one upgrade of `repo`, no duplicate
    let upgrades: Vec<_> = seed::wants(&s)
        .into_iter()
        .filter(|want| matches!(want, Want::Upgrade { .. }))
        .collect();
    assert_eq!(
        upgrades,
        vec![Want::Upgrade {
            name: Some("repo".to_string())
        }]
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:415 marketplace_add_success_refreshes_to_new_marketplace_tab kind=snap owner=dialogs-pane snap=plugins_popup_newly_installed_marketplace
#[test]
fn marketplace_add_success_refreshes_to_new_marketplace_tab() {
    let shelf = seed::shelf(vec![seed::curated(Vec::new())]);
    seed::with(&shelf, |held| {
        held.held.insert("marketplace/add");
    });
    let mut s = seed::open(100, 24, config_with(&["debug"]), &shelf);
    select_tab(&mut s, "Add a marketplace from a Git repo or local root.");
    s.keys("enter");
    s.paste("owner/repo").keys("enter");
    s.assert_absent("owner/repo");
    // the server adds it: the add result, then the refreshed list
    seed::with(&shelf, |held| {
        held.held.remove("marketplace/add");
        held.local = vec![seed::curated(Vec::new()), debug_market()];
        held.replies.insert(
            "marketplace/add",
            Ok(json!({
                "marketplaceName": "debug",
                "installedRoot": seed::path("marketplaces/debug"),
                "alreadyAdded": false,
            })),
        );
    });
    let add = seed::last(&s, |want| matches!(want, Want::Add { .. }));
    s.answer_request(
        &add,
        json!({
            "marketplaceName": "debug",
            "installedRoot": seed::path("marketplaces/debug"),
            "alreadyAdded": false,
        }),
    );
    s.assert_snap_region(
        "plugins_popup_newly_installed_marketplace",
        row_of(&s, "Plugins"),
        0,
    );
    s.assert_contains("Debug Marketplace installed successfully.");
    s.assert_contains("⌃u upgrade");
    s.assert_contains("⌃r remove");
    s.assert_contains("Debug Plugin");
    s.keys("esc");
    s.type_text("/plugins").keys("enter");
    select_tab(&mut s, "Installed 0 of 1 Debug Marketplace plugins.");
    s.assert_contains("Installed 0 of 1 Debug Marketplace plugins.");
    s.assert_absent("installed successfully");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:509 plugins_popup_removes_user_configured_marketplace_flow kind=snap owner=dialogs-pane snap=plugins_popup_marketplace_remove_confirmation
#[test]
fn plugins_popup_removes_user_configured_marketplace_flow() {
    let shelf = seed::shelf(vec![
        seed::curated(Vec::new()),
        seed::repo(vec![debug_plugin()]),
    ]);
    seed::with(&shelf, |held| {
        held.held.insert("marketplace/remove");
    });
    let mut s = seed::open(100, 24, config_with(&["repo"]), &shelf);
    select_tab(&mut s, "Repo Marketplace.");
    s.assert_contains("Repo Marketplace.");
    s.assert_contains("⌃u upgrade");
    s.assert_contains("⌃r remove");
    s.assert_contains("Debug Plugin");
    s.keys("ctrl+r");
    s.assert_contains("Remove Repo Marketplace marketplace?");
    s.assert_contains("Remove marketplace");
    s.assert_contains("Back to plugins");
    // The oracle's snapshot is the confirmation over its composer, which holds the first 14
    // columns of the rows (the composer is the chat's): the dialog's own columns are the rest.
    let top = row_of(&s, "Plugins");
    let dialog: Vec<String> = s.rows()[top..top + 10]
        .iter()
        .map(|row| {
            row.chars()
                .skip(14)
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect();
    assert_eq!(
        dialog,
        [
            "  Plugins",
            "  Remove Repo Marketplace marketplace?",
            "  This removes the configured marketplace from Codex.",
            "",
            "",
            "› 1. Remove marketplace  Remove this marketplace from the available",
            "                         plugin list",
            "  2. Back to plugins     Keep this marketplace installed",
            "",
            "  enter select · esc close",
        ],
        "plugins_popup_marketplace_remove_confirmation, from the dialog's column"
    );
    s.keys("enter");
    s.assert_contains("Removing Repo Marketplace...");
    s.assert_contains("Removing marketplace...");
    assert_eq!(
        seed::wants(&s)
            .into_iter()
            .filter(|want| matches!(want, Want::Remove { .. }))
            .collect::<Vec<_>>(),
        vec![Want::Remove {
            name: "repo".to_string()
        }]
    );
    // the removal result and the refreshed list arrive from the app server
    seed::with(&shelf, |held| {
        held.held.remove("marketplace/remove");
        held.local = vec![seed::curated(Vec::new())];
    });
    let remove = seed::last(&s, |want| matches!(want, Want::Remove { .. }));
    s.answer_request(
        &remove,
        json!({ "marketplaceName": "repo", "installedRoot": seed::path("marketplaces/repo") }),
    );
    s.assert_contains("Browse plugins from available marketplaces.");
    s.assert_absent("Repo Marketplace");
    s.assert_absent("Debug Plugin");
    s.assert_absent("⌃r remove");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:620 plugin_detail_popup_snapshot_labels_personal_marketplace_as_local kind=snap owner=dialogs-pane snap=plugin_detail_popup_installable
#[test]
fn plugin_detail_popup_snapshot_labels_personal_marketplace_as_local() {
    let figma = plugin("plugin-figma", "figma", "Figma", "Design handoff.", false);
    let shelf = seed::shelf(vec![seed::marketplace(
        "personal-marketplace",
        Some(seed::personal_path()),
        None,
        vec![figma.clone()],
    )]);
    seed::with(&shelf, |held| {
        let detail = seed::set(
            figma_detail(figma),
            &[
                ("marketplaceName", json!("personal-marketplace")),
                ("marketplacePath", json!(seed::personal_path())),
            ],
        );
        held.details.insert("figma".to_string(), detail);
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_snap_region("plugin_detail_popup_installable", row_of(&s, "Plugins"), 0);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:672 plugin_detail_popup_snapshot_shows_npm_source kind=snap owner=dialogs-pane snap=plugin_detail_popup_npm_source
#[test]
fn plugin_detail_popup_snapshot_shows_npm_source() {
    let figma = seed::set(
        plugin("plugin-figma", "figma", "Figma", "Design handoff.", false),
        &[(
            "source",
            json!({ "type": "npm", "package": "@acme/figma-plugin", "version": "^1.2.0", "registry": "https://npm.example.com" }),
        )],
    );
    let shelf = seed::shelf(vec![seed::curated(vec![figma.clone()])]);
    seed::with(&shelf, |held| {
        held.details
            .insert("figma".to_string(), figma_detail(figma));
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_snap_region("plugin_detail_popup_npm_source", row_of(&s, "Plugins"), 0);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:717 plugin_detail_popup_distinguishes_admin_installed_from_enabled kind=snap owner=dialogs-pane snap=plugin_detail_popup_installed
#[test]
fn plugin_detail_popup_distinguishes_admin_installed_from_enabled() {
    let figma = seed::summary(
        "plugin-figma",
        "figma",
        Some("Figma"),
        Some("Design handoff."),
        true,
        true,
        "INSTALLED_BY_DEFAULT",
    );
    let shelf = seed::shelf(vec![seed::curated(vec![figma.clone()])]);
    seed::with(&shelf, |held| {
        held.details
            .insert("figma".to_string(), figma_detail(figma.clone()));
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_contains("Installed by admin");
    s.assert_absent("Uninstall plugin");
    s.assert_absent("Data shared with this app is subject to the app's");
    s.assert_snap_region("plugin_detail_popup_installed", row_of(&s, "Plugins"), 0);
    // the second detail (not installed, not enabled) arrives from the app server
    let unmaterialized = seed::set(
        figma,
        &[("installed", json!(false)), ("enabled", json!(false))],
    );
    seed::with(&shelf, |held| {
        held.details
            .insert("figma".to_string(), figma_detail(unmaterialized));
    });
    // the oracle takes the new detail from the app server; here the way back is the row (Esc on a
    // detail closes the popup, as in the oracle) and Enter reads the detail again
    s.click_text("Back to plugins");
    s.keys("enter");
    s.assert_contains("Enabled by Admin");
    s.assert_contains("Install plugin");
    s.assert_absent("Installed by admin");
    s.assert_contains("Figma · Enabled by Admin · ChatGPT Marketplace");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:786 plugins_popup_remote_row_opens_remote_detail kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_remote_row_opens_remote_detail() {
    let shelf = seed::shelf(Vec::new());
    seed::with(&shelf, |held| {
        held.held.insert("plugin/read");
        held.remote = vec![seed::marketplace(
            "workspace-directory",
            None,
            Some("Workspace"),
            vec![seed::remote_summary(
                "plugins~Plugin_calendar",
                "calendar",
                Some("Calendar"),
                Some("Workspace schedules."),
                false,
            )],
        )];
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.assert_contains("Calendar");
    s.assert_contains("Available");
    s.assert_contains("Press Enter to install or view plugin details");
    s.keys("enter");
    // the detail is asked of the remote marketplace, by the plugin's id
    let target = Target {
        location: Location::Remote {
            marketplace: "workspace-directory".to_string(),
        },
        plugin: "plugins~Plugin_calendar".to_string(),
    };
    assert!(
        seed::wants(&s).contains(&Want::Read(target)),
        "{:?}",
        seed::wants(&s)
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:842 plugin_detail_unmaterialized_default_uses_remote_install_path kind=ui owner=dialogs-pane
#[test]
fn plugin_detail_unmaterialized_default_uses_remote_install_path() {
    let linear = seed::set(
        seed::remote_summary(
            "plugins~Plugin_linear",
            "linear",
            Some("Linear"),
            Some("Issue tracking."),
            false,
        ),
        &[("installPolicy", json!("INSTALLED_BY_DEFAULT"))],
    );
    let shelf = seed::shelf(Vec::new());
    seed::with(&shelf, |held| {
        held.remote = vec![seed::marketplace(
            "workspace-shared-with-me-private",
            None,
            Some("Shared with me"),
            vec![linear.clone()],
        )];
        held.held.insert("plugin/install");
        held.details.insert(
            "plugins~Plugin_linear".to_string(),
            seed::remote_detail(
                "workspace-shared-with-me-private",
                linear,
                Some("Install shared Linear plugin."),
            ),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_contains("Install plugin");
    s.assert_contains("Install this plugin now");
    s.keys("down enter");
    // the install is asked of the remote marketplace, by the plugin's id
    let target = Target {
        location: Location::Remote {
            marketplace: "workspace-shared-with-me-private".to_string(),
        },
        plugin: "plugins~Plugin_linear".to_string(),
    };
    assert!(
        seed::wants(&s).contains(&Want::Install(target)),
        "{:?}",
        seed::wants(&s)
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:922 plugin_detail_remote_uninstall_uses_remote_plugin_id kind=ui owner=dialogs-pane
#[test]
fn plugin_detail_remote_uninstall_uses_remote_plugin_id() {
    kit::na(
        Owner::DialogsPane,
        "asserts that Down and Enter send OpenPluginUninstallLoading and FetchPluginUninstall with the remote plugin id, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:982 plugin_detail_remote_without_remote_id_disables_uninstall_action kind=ui owner=dialogs-pane
#[test]
fn plugin_detail_remote_without_remote_id_disables_uninstall_action() {
    // the row names its plugin by a remote id (so Enter can ask for the detail); the detail the
    // server answers has none: the oracle injects that detail while the browser is up
    let row = seed::remote_summary(
        "plugins~Plugin_linear",
        "linear",
        Some("Linear"),
        Some("Issue tracking."),
        true,
    );
    let detail = seed::set(row.clone(), &[("remotePluginId", serde_json::Value::Null)]);
    let shelf = seed::shelf(Vec::new());
    seed::with(&shelf, |held| {
        held.remote = vec![seed::marketplace(
            "workspace-shared-with-me-private",
            None,
            Some("Shared with me"),
            vec![row],
        )];
        held.details.insert(
            "*".to_string(),
            seed::remote_detail(
                "workspace-shared-with-me-private",
                detail,
                Some("Installed shared Linear plugin."),
            ),
        );
    });
    let mut s = seed::open(120, 24, seed::config(), &shelf);
    s.wait_for("Linear");
    s.keys("enter");
    s.assert_contains("This remote plugin did not provide an uninstall identity");
    s.assert_absent("Remove this plugin now");
    let before = s.shot();
    s.keys("down");
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1037 plugin_detail_popup_shows_local_share_context_as_read_only_snapshot kind=snap owner=dialogs-pane snap=plugin_detail_popup_local_share_read_only
#[test]
fn plugin_detail_popup_shows_local_share_context_as_read_only_snapshot() {
    let share = json!({
        "remotePluginId": "plugins~Plugin_docs", "remoteVersion": "7", "discoverability": "PRIVATE",
        "shareUrl": "https://chatgpt.com/codex/plugins/share/docs", "creatorAccountUserId": null,
        "creatorName": "Test User", "sharePrincipals": null, "canPublishToWorkspace": null,
    });
    let docs = seed::set(
        plugin("plugin-docs", "docs", "Docs", "Workspace docs.", false),
        &[("shareContext", share)],
    );
    let shelf = seed::shelf(vec![seed::curated(vec![docs.clone()])]);
    seed::with(&shelf, |held| {
        let detail = seed::set(
            seed::detail(docs, Some("Workspace docs."), &[], &[], &[], &[]),
            &[("marketplacePath", json!(seed::personal_path()))],
        );
        held.details.insert("docs".to_string(), detail);
    });
    let mut s = seed::open(120, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_snap_region(
        "plugin_detail_popup_local_share_read_only",
        row_of(&s, "Plugins"),
        0,
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1080 plugin_detail_popup_shows_admin_disabled_status_snapshot kind=snap owner=dialogs-pane
#[test]
fn plugin_detail_popup_shows_admin_disabled_status_snapshot() {
    let blocked = seed::set(
        plugin(
            "plugin-admin-blocked",
            "admin-blocked",
            "Admin Blocked",
            "Blocked by policy.",
            false,
        ),
        &[("availability", json!("DISABLED_BY_ADMIN"))],
    );
    let shelf = seed::shelf(vec![seed::curated(vec![blocked.clone()])]);
    seed::with(&shelf, |held| {
        held.details.insert(
            "admin-blocked".to_string(),
            seed::detail(blocked, Some("Blocked by policy."), &[], &[], &[], &[]),
        );
    });
    let mut s = seed::open(120, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_contains("  Admin Blocked · Disabled by admin · ChatGPT Marketplace");
    s.assert_contains("This plugin is disabled by your workspace admin");
    s.assert_absent("Install this plugin now");
    // no action goes out after the disabled install state is drawn
    assert!(
        !seed::wants(&s)
            .iter()
            .any(|want| matches!(want, Want::Install(_)))
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1132 plugins_popup_admin_disabled_installed_plugin_has_no_toggle_hint kind=snap owner=dialogs-pane snap=plugins_popup_admin_disabled_installed
#[test]
fn plugins_popup_admin_disabled_installed_plugin_has_no_toggle_hint() {
    let blocked = seed::set(
        plugin(
            "plugin-admin-blocked",
            "admin-blocked",
            "Admin Blocked",
            "Blocked by policy.",
            true,
        ),
        &[("availability", json!("DISABLED_BY_ADMIN"))],
    );
    let mut s = browser(120, vec![blocked]);
    s.assert_snap_region(
        "plugins_popup_admin_disabled_installed",
        row_of(&s, "Plugins"),
        0,
    );
    s.assert_contains("[!] Admin Blocked");
    s.assert_contains("Disabled");
    s.assert_contains("Press Enter to view plugin details");
    s.assert_absent("Disabled by admin");
    s.assert_absent("Tab to disable");
    let before = s.shot();
    s.keys("tab");
    s.assert_same_rows(&before);
    assert!(
        !seed::wants(&s)
            .iter()
            .any(|want| matches!(want, Want::Enable { .. }))
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1176 plugins_popup_admin_disabled_available_plugin_has_view_only_hint kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_admin_disabled_available_plugin_has_view_only_hint() {
    let blocked = seed::set(
        plugin(
            "plugin-admin-blocked",
            "admin-blocked",
            "Admin Blocked",
            "Blocked by policy.",
            false,
        ),
        &[("availability", json!("DISABLED_BY_ADMIN"))],
    );
    let s = browser(100, vec![blocked]);
    s.assert_contains("Admin Blocked");
    s.assert_contains("Press Enter to view plugin details");
    s.assert_absent("install or view");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1210 plugins_popup_remote_section_fallback_states_when_remote_plugin_disabled_snapshot kind=snap owner=dialogs-pane snap=plugins_popup_empty_shared_section_hidden
#[test]
fn plugins_popup_remote_section_fallback_states_when_remote_plugin_disabled_snapshot() {
    // the remote plugin feature is off: the curated section is a request of its own, the sections load from the app server
    let shelf = seed::shelf(vec![seed::curated(Vec::new())]);
    seed::with(&shelf, |held| {
        held.held.insert(seed::SECTIONS);
    });
    let mut config = seed::config();
    config.sharing = true;
    let mut s = seed::open(100, 24, config, &shelf);
    select_tab(&mut s, "Loading OpenAI Curated plugins...");
    s.assert_contains("Loading OpenAI Curated plugins...");
    select_tab(&mut s, "Loading Workspace plugins.");
    s.assert_contains("Loading Workspace plugins.");
    select_tab(&mut s, "Loading Shared with me plugins.");
    s.assert_contains("Loading Shared with me plugins.");
    select_tab(&mut s, "Loading Workspace plugins.");
    seed::serve_sections(&mut s, &shelf);
    s.assert_snap_region(
        "plugins_popup_empty_shared_section_hidden",
        row_of(&s, "Plugins"),
        0,
    );
    s.assert_absent("Shared with me");
    // the workspace section fails
    seed::with(&shelf, |held| {
        held.section_errors.insert(
            "workspace-directory",
            "Sign in to ChatGPT to load workspace plugins.".to_string(),
        );
    });
    s.keys("esc");
    s.type_text("/plugins").keys("enter");
    select_tab(&mut s, "Workspace unavailable.");
    s.assert_contains("Workspace unavailable.");
    s.assert_contains("Sign in to ChatGPT to load workspace plugins.");
    // the remote plugin feature on: the curated marketplace is the OpenAI Curated tab
    let mut config = seed::config();
    config.remote = true;
    let shelf = seed::shelf(vec![seed::curated(Vec::new())]);
    let mut remote = seed::open(100, 24, config, &shelf);
    select_tab(&mut remote, "No OpenAI Curated plugins available");
    remote.assert_contains("No OpenAI Curated plugins available");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1317 plugins_popup_remote_detail_tracks_physical_and_policy_install_state kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_remote_detail_tracks_physical_and_policy_install_state() {
    let id = "plugins~Plugin_docs";
    let market = "workspace-shared-with-me-private";
    let share = json!({
        "remotePluginId": id, "remoteVersion": null, "discoverability": null, "shareUrl": null,
        "creatorAccountUserId": null, "creatorName": null, "sharePrincipals": null,
        "canPublishToWorkspace": null,
    });
    let local = |policy: &str| {
        seed::set(
            seed::summary(
                "plugin-docs",
                "docs",
                Some("Docs"),
                Some("Local editable docs plugin."),
                false,
                true,
                policy,
            ),
            &[("shareContext", share.clone())],
        )
    };
    let remote = |installed: bool| {
        seed::remote_marketplace(
            market,
            "Shared with me",
            vec![seed::remote_summary(
                id,
                "docs",
                Some("Docs"),
                Some("Shared docs plugin."),
                installed,
            )],
        )
    };
    let shelf = seed::shelf(vec![seed::curated(vec![local("INSTALLED_BY_DEFAULT")])]);
    seed::with(&shelf, |held| {
        held.held.insert("plugin/read");
        held.remote = vec![remote(true)];
    });
    let mut config = seed::config();
    config.sharing = true;
    let mut s = seed::open(100, 24, config, &shelf);
    s.assert_contains("Installed 1 of 1 available plugins.");
    s.assert_contains("Docs");
    s.assert_absent("Available");
    s.keys("right");
    s.assert_contains("Showing 1 installed plugins.");
    s.assert_contains("Docs");
    s.keys("enter");
    // the detail is asked of the remote marketplace, by the remote id
    let target = || Target {
        location: Location::Remote {
            marketplace: market.to_string(),
        },
        plugin: id.to_string(),
    };
    assert!(
        seed::wants(&s).contains(&Want::Read(target())),
        "{:?}",
        seed::wants(&s)
    );
    // the policy refresh (an admin-assigned remote duplicate that is not installed) arrives
    seed::with(&shelf, |held| held.remote = vec![remote(false)]);
    let unmaterialized = seed::set(
        seed::remote_summary(id, "docs", Some("Docs"), Some("Shared docs plugin."), false),
        &[("installPolicy", json!("INSTALLED_BY_DEFAULT"))],
    );
    seed::with(&shelf, |held| {
        held.remote = vec![seed::remote_marketplace(
            market,
            "Shared with me",
            vec![unmaterialized],
        )];
    });
    s.keys("esc");
    seed::hold_list(&shelf);
    s.type_text("/plugins").keys("enter");
    seed::refresh(
        &mut s,
        &shelf,
        vec![seed::curated(vec![local("AVAILABLE")])],
    );
    s.assert_contains("Installed 0 of 1 available plugins.");
    s.assert_contains("Admin assigned");
    s.keys("right");
    s.assert_contains("Showing 0 installed plugins.");
    s.assert_contains("No installed plugins");
    s.assert_absent("Docs");
    s.keys("left enter");
    let reads = seed::wants(&s)
        .into_iter()
        .filter(|want| *want == Want::Read(target()))
        .count();
    assert_eq!(reads, 2);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1464 plugin_detail_error_popup_skips_disabled_row_numbering kind=snap owner=dialogs-pane snap=plugin_detail_error_popup
#[test]
fn plugin_detail_error_popup_skips_disabled_row_numbering() {
    let shelf = seed::shelf(vec![seed::curated(vec![plugin(
        "plugin-figma",
        "figma",
        "Figma",
        "Design handoff.",
        false,
    )])]);
    seed::with(&shelf, |held| {
        held.replies.insert(
            "plugin/read",
            Err("Failed to load plugin details.".to_string()),
        );
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("enter");
    s.assert_snap_region("plugin_detail_error_popup", row_of(&s, "Plugins"), 0);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1492 plugins_popup_refresh_preserves_selected_row_position kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_refresh_preserves_selected_row_position() {
    let notion = || {
        plugin(
            "plugin-notion",
            "notion",
            "Notion",
            "Workspace docs.",
            false,
        )
    };
    let slack = || plugin("plugin-slack", "slack", "Slack", "Team chat.", false);
    let shelf = seed::shelf(vec![seed::curated(vec![notion(), slack()])]);
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    // the refresh of the list arrives while the cached one is shown
    seed::reopen_held(&mut s, &shelf);
    s.keys("down");
    s.assert_contains("› [-] Slack");
    let airtable = plugin(
        "plugin-airtable",
        "airtable",
        "Airtable",
        "Structured records.",
        false,
    );
    seed::refresh(
        &mut s,
        &shelf,
        vec![seed::curated(vec![airtable, notion(), slack()])],
    );
    s.assert_contains("› [-] Notion");
    s.assert_contains("Airtable");
    s.assert_contains("Slack");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1573 plugins_popup_refreshes_installed_counts_after_install kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_refreshes_installed_counts_after_install() {
    let calendar = |installed| {
        plugin(
            "plugin-calendar",
            "calendar",
            "Calendar",
            "Schedule management.",
            installed,
        )
    };
    let drive = plugin("plugin-drive", "drive", "Drive", "Document access.", true);
    let shelf = seed::shelf(vec![seed::curated(vec![calendar(false), drive.clone()])]);
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.assert_contains("Installed 1 of 2 available plugins.");
    s.assert_contains("Available");
    seed::reopen_held(&mut s, &shelf);
    seed::refresh(
        &mut s,
        &shelf,
        vec![seed::curated(vec![calendar(true), drive])],
    );
    s.assert_contains("Installed 2 of 2 available plugins.");
    s.assert_contains("Installed   Tab to disable; Enter view details");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1642 plugins_popup_toggle_keys_preserve_search_and_selection kind=snap owner=dialogs-pane snap=plugins_popup_toggle_refresh_preserves_search
#[test]
fn plugins_popup_toggle_keys_preserve_search_and_selection() {
    let shelf = seed::shelf(vec![seed::curated(vec![
        plugin(
            "plugin-calendar",
            "calendar",
            "Calendar",
            "Schedule management.",
            true,
        ),
        plugin("plugin-drive", "drive", "Drive", "Document access.", true),
    ])]);
    seed::with(&shelf, |held| {
        held.held.insert("config/value/write");
        held.replies.insert("config/value/write", Ok(json!({})));
    });
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("down space");
    let enables = |s: &Screen| -> Vec<Want> {
        seed::wants(s)
            .into_iter()
            .filter(|want| matches!(want, Want::Enable { .. }))
            .collect()
    };
    assert_eq!(
        enables(&s),
        vec![Want::Enable {
            id: "plugin-drive".to_string(),
            enabled: false
        }]
    );
    s.type_text("dr");
    s.keys("tab");
    // the write goes on: the server confirms the first, then the one that flips it back
    seed::with(&shelf, |held| {
        held.held.remove("config/value/write");
    });
    let first = seed::last(&s, |want| matches!(want, Want::Enable { .. }));
    s.answer_request(&first, json!({}));
    assert_eq!(
        enables(&s),
        vec![
            Want::Enable {
                id: "plugin-drive".to_string(),
                enabled: false
            },
            Want::Enable {
                id: "plugin-drive".to_string(),
                enabled: true
            },
        ]
    );
    s.assert_snap_region(
        "plugins_popup_toggle_refresh_preserves_search",
        row_of(&s, "Plugins"),
        0,
    );
    s.assert_contains("› [*] Drive");
    s.assert_contains("Tab to disable");
    s.assert_contains("dr");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1722 plugins_popup_hides_tab_toggle_hint_when_tab_is_configured kind=snap owner=dialogs-pane snap=plugins_popup_custom_tab_binding_hides_toggle_hint
#[test]
fn plugins_popup_hides_tab_toggle_hint_when_tab_is_configured() {
    let shelf = seed::shelf(vec![seed::curated(vec![plugin(
        "plugin-drive",
        "drive",
        "Drive",
        "Document access.",
        true,
    )])]);
    let mut s = seed::start_with(
        100,
        24,
        seed::config(),
        &shelf,
        &[("list.move_down", "tab")],
    );
    s.type_text("/plugins").keys("enter");
    s.assert_snap_region(
        "plugins_popup_custom_tab_binding_hides_toggle_hint",
        row_of(&s, "Plugins"),
        0,
    );
    s.assert_contains("Installed   Press Enter to view plugin details");
    s.assert_absent("Tab to disable");
    s.assert_absent("tab toggle");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1754 plugins_popup_space_on_uninstalled_row_does_not_start_search kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_space_on_uninstalled_row_does_not_start_search() {
    let mut s = browser(
        100,
        vec![
            plugin(
                "plugin-calendar",
                "calendar",
                "Calendar",
                "Schedule management.",
                false,
            ),
            plugin("plugin-drive", "drive", "Drive", "Document access.", false),
        ],
    );
    let before = s.shot();
    s.keys("space");
    // Space sends no write and starts no search: the rows stay the same
    s.assert_same_rows(&before);
    assert!(
        !seed::wants(&s)
            .iter()
            .any(|want| matches!(want, Want::Enable { .. }))
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1795 plugins_popup_space_with_active_search_does_not_toggle_installed_plugin kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_space_with_active_search_does_not_toggle_installed_plugin() {
    kit::na(
        Owner::DialogsPane,
        "asserts that Space during an active search sends no SetPluginEnabled event, not rows: a state test",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1835 plugins_popup_search_filters_visible_rows_snapshot kind=snap owner=dialogs-pane snap=plugins_popup_search_filtered
#[test]
fn plugins_popup_search_filters_visible_rows_snapshot() {
    let mut s = browser(
        100,
        vec![
            plugin(
                "plugin-calendar",
                "calendar",
                "Calendar",
                "Schedule management.",
                false,
            ),
            plugin("plugin-slack", "slack", "Slack", "Team chat.", false),
            plugin("plugin-drive", "drive", "Drive", "Document access.", false),
        ],
    );
    s.type_text("sla");
    s.assert_snap_region("plugins_popup_search_filtered", row_of(&s, "Plugins"), 0);
    s.assert_absent("Calendar");
    s.assert_absent("Drive");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1883 plugins_popup_search_matches_plugin_descriptions kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_search_matches_plugin_descriptions() {
    let mut s = browser(
        100,
        vec![
            plugin(
                "plugin-calendar",
                "calendar",
                "Calendar",
                "Schedule management.",
                false,
            ),
            plugin("plugin-drive", "drive", "Drive", "Document access.", false),
        ],
    );
    s.type_text("document");
    s.assert_contains("Drive");
    s.assert_absent("Calendar");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1921 plugins_popup_installed_tab_filters_rows_and_clears_search kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_installed_tab_filters_rows_and_clears_search() {
    let mut s = browser(
        100,
        vec![
            plugin(
                "plugin-calendar",
                "calendar",
                "Calendar",
                "Schedule management.",
                true,
            ),
            plugin("plugin-slack", "slack", "Slack", "Team chat.", false),
        ],
    );
    s.type_text("sla");
    s.keys("right");
    s.assert_contains("Installed plugins.");
    s.assert_contains("Showing 1 installed plugins.");
    s.assert_contains("Calendar");
    s.assert_absent("Slack");
    s.assert_absent("sla");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1968 plugins_popup_openai_curated_tab_omits_marketplace_in_rows kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_openai_curated_tab_omits_marketplace_in_rows() {
    let shelf = seed::shelf(vec![
        seed::curated(vec![plugin(
            "plugin-calendar",
            "calendar",
            "Calendar",
            "Schedule management.",
            false,
        )]),
        seed::repo(vec![plugin(
            "plugin-repo",
            "repo",
            "Repo Plugin",
            "Repo-only plugin.",
            false,
        )]),
    ]);
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    s.keys("right right");
    s.assert_contains("OpenAI Curated marketplace.");
    s.assert_contains("Calendar");
    s.assert_absent("Repo Plugin");
    s.assert_absent("ChatGPT Marketplace ·");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2015 plugins_popup_refresh_preserves_duplicate_marketplace_tab_by_path kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_refresh_preserves_duplicate_marketplace_tab_by_path() {
    let duplicate = |file: &str, id: &str, name: &str, display: &str, description: &str| {
        seed::marketplace(
            "duplicate",
            Some(seed::path(file)),
            Some("Duplicate Marketplace"),
            vec![plugin(id, name, display, description, false)],
        )
    };
    let markets = || {
        vec![
            duplicate(
                "marketplaces/home/marketplace.json",
                "plugin-home",
                "home",
                "Home Plugin",
                "Home marketplace plugin.",
            ),
            duplicate(
                "marketplaces/repo/marketplace.json",
                "plugin-repo",
                "repo",
                "Repo Plugin",
                "Repo marketplace plugin.",
            ),
        ]
    };
    let shelf = seed::shelf(markets());
    let mut s = seed::open(100, 24, seed::config(), &shelf);
    // the same two marketplaces arrive again while the second one's tab is shown
    seed::reopen_held(&mut s, &shelf);
    for _ in 0..10 {
        let text = s.text();
        if text.contains("Duplicate Marketplace (2/2).")
            && text.contains("Repo Plugin")
            && !text.contains("Home Plugin")
        {
            break;
        }
        s.keys("right");
    }
    seed::refresh(&mut s, &shelf, markets());
    s.assert_contains("Duplicate Marketplace (2/2).");
    s.assert_contains("Repo Plugin");
    s.assert_absent("Home Plugin");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2086 plugins_popup_search_no_matches_and_backspace_restores_results kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_search_no_matches_and_backspace_restores_results() {
    let mut s = browser(
        100,
        vec![
            plugin(
                "plugin-calendar",
                "calendar",
                "Calendar",
                "Schedule management.",
                false,
            ),
            plugin("plugin-slack", "slack", "Slack", "Team chat.", false),
        ],
    );
    s.type_text("zzz");
    s.assert_contains("zzz");
    s.assert_contains("no matches");
    s.keys("backspace backspace backspace");
    s.assert_contains("Calendar");
    s.assert_contains("Slack");
    s.assert_absent("no matches");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2142 apps_popup_stays_loading_until_final_snapshot_updates kind=snap owner=dialogs-pane snap=apps_popup_loading_state
#[test]
fn apps_popup_stays_loading_until_final_snapshot_updates() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    s.assert_contains("Loading installed and available apps...");
    s.assert_snap_region("apps_popup_loading_state", 17, 0);
    let asked = apps_asked(&s);
    assert_eq!(asked.len(), 1, "{asked:?}");
    s.answer_request(
        &asked[0],
        apps_answer(&[
            ("notion", "Notion", "Workspace docs"),
            ("linear", "Linear", "Project tracking"),
        ]),
    );
    s.assert_contains("Installed 2 of 2 available apps.");
    s.assert_contains("Linear");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2251 apps_notification_update_excludes_inaccessible_apps_from_mentions kind=snap owner=dialogs-pane snap=apps_mentions_only_callable_installed
#[test]
fn apps_notification_update_excludes_inaccessible_apps_from_mentions() {
    kit::pending(Owner::DialogsPane, MENTIONS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("$");
    // the directory notification (Google Drive, % Arabica UAE) arrives from the app server: a script line
    s.assert_contains("Google Drive");
    s.assert_absent("% Arabica UAE");
    s.keys("enter");
    s.type_text("$arabica-uae ");
    s.keys("enter");
    // the submitted mention (app://google_drive) is an op: not on screen
    // the plugin authentication popup comes from the install result: a script line
    s.assert_contains("Already installed");
    s.assert_contains("Continue");
    s.assert_snap_section(
        "apps_mentions_only_callable_installed",
        "plugin authentication",
    );
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2440 apps_refresh_failure_keeps_existing_full_snapshot kind=ui owner=dialogs-pane
#[test]
fn apps_refresh_failure_keeps_existing_full_snapshot() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    let asked = apps_asked(&s);
    assert_eq!(asked.len(), 1, "{asked:?}");
    s.answer_request(
        &asked[0],
        serde_json::json!({"data": [
            {"id": "connector_1", "name": "Notion", "description": "Workspace docs",
             "installUrl": "https://example.test/notion", "isAccessible": true,
             "isEnabled": true, "pluginDisplayNames": []},
            {"id": "connector_2", "name": "Linear", "description": "Project tracking",
             "installUrl": "https://example.test/linear", "isAccessible": false,
             "isEnabled": true, "pluginDisplayNames": []},
        ], "nextCursor": null}),
    );
    s.assert_contains("Installed 1 of 2 available apps.");
    // the oracle's partial refresh (`is_final: false`, Notion alone) has no protocol message; the
    // refresh that fails follows the full list
    s.keys("esc");
    s.type_text("/apps").keys("enter");
    let asked = apps_asked(&s);
    assert_eq!(asked.len(), 2, "{asked:?}");
    s.answer_error(&asked[1], "failed to load apps");
    // the previous full snapshot is kept
    s.assert_contains("Installed 1 of 2 available apps.");
    s.assert_absent("Failed to load apps.");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2535 apps_popup_preserves_selected_app_across_refresh kind=ui owner=dialogs-pane
#[test]
fn apps_popup_preserves_selected_app_across_refresh() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    let asked = apps_asked(&s);
    s.answer_request(
        &asked[0],
        apps_answer(&[
            ("notion", "Notion", "Workspace docs"),
            ("slack", "Slack", "Team chat"),
        ]),
    );
    s.keys("down");
    s.assert_contains("› Slack");
    // the refreshed list (Airtable, Notion, Slack) arrives: the app server says the directory
    // changed, the app reads it again and the popup shows the new list
    let refreshed = apps_answer(&[
        ("airtable", "Airtable", "Spreadsheets"),
        ("notion", "Notion", "Workspace docs"),
        ("slack", "Slack", "Team chat"),
    ]);
    s.feed(&format!(
        "{}\n",
        serde_json::json!({"step": "notify", "message": {"method": "app/list/updated",
            "params": {"data": refreshed["data"]}}})
    ));
    let asked = apps_asked(&s);
    assert_eq!(asked.len(), 2, "{asked:?}");
    s.answer_request(&asked[1], refreshed);
    s.assert_contains("Airtable");
    s.assert_contains("› Slack");
    s.assert_absent("› Notion");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2711 apps_popup_keeps_existing_full_snapshot_while_partial_refresh_loads kind=ui owner=dialogs-pane
#[test]
fn apps_popup_keeps_existing_full_snapshot_while_partial_refresh_loads() {
    kit::pending(Owner::DialogsPane, APPS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    // the full list, then a partial refresh (Hidden OpenAI) arrive from the app server: script lines
    s.assert_contains("Installed 1 of 2 available apps.");
    s.assert_absent("Hidden OpenAI");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2823 apps_popup_replaces_loading_state_after_initial_refresh_failure kind=snap owner=dialogs-pane snap=apps_popup_error_state
#[test]
fn apps_popup_replaces_loading_state_after_initial_refresh_failure() {
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    s.assert_contains("Loading apps...");
    // the failure (app/list failed: 403 Forbidden) arrives from the app server
    let asked = apps_asked(&s);
    assert_eq!(asked.len(), 1, "{asked:?}");
    s.answer_error(&asked[0], "app/list failed: 403 Forbidden");
    s.assert_absent("Loading apps...");
    s.assert_absent("403 Forbidden");
    s.assert_contains("Failed to load apps.");
    s.assert_snap_region("apps_popup_error_state", 15, 0);
    // Enter on Retry reads the directory again
    s.keys("enter");
    s.assert_contains("Loading apps...");
}
