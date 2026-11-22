//! Shard `chat_popups_3` (screens S14, owner dialogs-pane): the oracle tests of the manifest below,
//! ported as black-box screen tests with the kit. One test per manifest line,
//! named as the line says. Recipe: docs/rtwk-port/refs/screen-tests.md.
//! Replace each `kit::todo();` by the port, by `kit::pending(..)` + the port, or by
//! `kit::na(..)`; add the imports you need (`use crate::kit::{Owner, Screen};`).

use crate::kit::{self, Owner, Screen};

// What these tests need is not in the port yet (each reason says which); the
// bodies are the oracle's steps and visible assertions, written in full.
// assert_snap compares the whole screen; the oracle's popup snapshots hold the
// bottom popup only, so the popup slice sets the region (assert_snap_region).
const PLUGINS: &str = "no /plugins browser in the port: the chat names /plugins only in its command list (src/screens/chat/strings.rs:221); no marketplace tabs, plugin rows, detail, install or remove popups, and no seed for the app server's plugin list or the Feature::Plugins gate";
const APPS: &str = "/apps is live (core `select::apps`, the loading, failure and list pages, Retry); this test needs a step the kit and the protocol do not give yet: a partial directory snapshot (the oracle injects `on_connectors_loaded(.., is_final: false)`, no protocol message carries it), an RPC failure as the backend's answer (`respond` has only `result`), the `app/list/updated` notification routed to `select::Ev::AppsNotified` (core-wiring), the `$` mention popup and the plugin authentication popup (other owners)";
const HOOKS: &str = "no /hooks popup in the port: no hooks list view with its warnings and errors, and no seed for the hooks list result";

/// The `app/list` requests the app has made, oldest first.
fn apps_asked(s: &Screen) -> Vec<codex_tui_core::app::Command> {
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
fn apps_answer(apps: &[(&str, &str, &str)]) -> serde_json::Value {
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
fn select_tab(s: &mut Screen, text: &str) {
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("Loading available plugins...");
    s.assert_snap("plugins_popup_loading_state");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:81 marketplace_upgrade_loading_popup_snapshot kind=snap owner=dialogs-pane
#[test]
fn marketplace_upgrade_loading_popup_snapshot() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    // the oracle opens the upgrade loading popup of `debug` directly; the port reaches it from the marketplace tab
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("Upgrading debug marketplace...");
    s.assert_contains("This updates when marketplace upgrade completes");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:101 marketplace_upgrade_failure_includes_backend_messages_snapshot kind=snap owner=dialogs-pane
#[test]
fn marketplace_upgrade_failure_includes_backend_messages_snapshot() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    // the upgrade result arrives from the app server (a script line); the history cell it inserts is the oracle's text
    let s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.assert_contains("Failed to upgrade 2 marketplaces");
    s.assert_contains(
        "debug: git ls-remote marketplace source failed with status 128: authentication failed",
    );
    s.assert_contains("tools: failed to validate upgraded marketplace root");
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(320, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_snap("plugins_popup_curated_marketplace");
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("No marketplace plugins available");
    s.assert_absent("Marketplace unavailable");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:244 plugins_popup_truncates_long_descriptions_in_list_rows kind=snap owner=dialogs-pane
#[test]
fn plugins_popup_truncates_long_descriptions_in_list_rows() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane).size(70, 24).live().open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("  [-] Verbose Plugin  Available · OpenAI Curated · This description k…");
    s.assert_absent("This description keeps going and going until the row would normally wrap.");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:290 plugins_popup_add_marketplace_tab_opens_prompt_and_submits_source kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_add_marketplace_tab_opens_prompt_and_submits_source() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    select_tab(&mut s, "Add a marketplace from a Git repo or local root.");
    s.assert_contains("Add a marketplace from a Git repo or local root.");
    s.keys("enter");
    // OpenMarketplaceAddPrompt, OpenMarketplaceAddLoading and FetchMarketplaceAdd are events: not on screen
    s.assert_contains("owner/repo, git URL, or local marketplace path");
    s.paste("owner/repo").keys("enter");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:346 plugins_popup_upgrades_user_configured_git_marketplace_from_marketplace_tab kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_upgrades_user_configured_git_marketplace_from_marketplace_tab() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    select_tab(&mut s, "Repo Marketplace.");
    s.assert_contains("Repo Marketplace.");
    s.assert_contains("⌃u upgrade");
    s.assert_contains("⌃r remove");
    s.assert_contains("Debug Plugin");
    s.keys("ctrl+u ctrl+u");
    // one OpenMarketplaceUpgradeLoading and one FetchMarketplaceUpgrade for `repo` (no duplicate): events, not rows
}

// ORACLE chatwidget/tests/popups_and_settings.rs:415 marketplace_add_success_refreshes_to_new_marketplace_tab kind=snap owner=dialogs-pane snap=plugins_popup_newly_installed_marketplace
#[test]
fn marketplace_add_success_refreshes_to_new_marketplace_tab() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    select_tab(&mut s, "Add a marketplace from a Git repo or local root.");
    s.keys("enter");
    s.paste("owner/repo").keys("enter");
    s.assert_absent("owner/repo");
    // the add result (debug marketplace installed) arrives from the app server: a script line
    s.assert_snap("plugins_popup_newly_installed_marketplace");
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    select_tab(&mut s, "Repo Marketplace.");
    s.assert_contains("Repo Marketplace.");
    s.assert_contains("⌃u upgrade");
    s.assert_contains("⌃r remove");
    s.assert_contains("Debug Plugin");
    s.keys("ctrl+r");
    s.assert_contains("Remove Repo Marketplace marketplace?");
    s.assert_contains("Remove marketplace");
    s.assert_contains("Back to plugins");
    s.assert_snap("plugins_popup_marketplace_remove_confirmation");
    s.keys("enter");
    s.assert_contains("Removing Repo Marketplace...");
    s.assert_contains("Removing marketplace...");
    // the removal result and the refreshed list arrive from the app server: script lines
    s.assert_contains("Browse plugins from available marketplaces.");
    s.assert_absent("Repo Marketplace");
    s.assert_absent("Debug Plugin");
    s.assert_absent("⌃r remove");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:620 plugin_detail_popup_snapshot_labels_personal_marketplace_as_local kind=snap owner=dialogs-pane snap=plugin_detail_popup_installable
#[test]
fn plugin_detail_popup_snapshot_labels_personal_marketplace_as_local() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("enter");
    s.assert_snap("plugin_detail_popup_installable");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:672 plugin_detail_popup_snapshot_shows_npm_source kind=snap owner=dialogs-pane snap=plugin_detail_popup_npm_source
#[test]
fn plugin_detail_popup_snapshot_shows_npm_source() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("enter");
    s.assert_snap("plugin_detail_popup_npm_source");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:717 plugin_detail_popup_distinguishes_admin_installed_from_enabled kind=snap owner=dialogs-pane snap=plugin_detail_popup_installed
#[test]
fn plugin_detail_popup_distinguishes_admin_installed_from_enabled() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("enter");
    s.assert_contains("Installed by admin");
    s.assert_absent("Uninstall plugin");
    s.assert_absent("Data shared with this app is subject to the app's");
    s.assert_snap("plugin_detail_popup_installed");
    // the second detail (not installed, not enabled) arrives from the app server: a script line
    s.assert_contains("Enabled by Admin");
    s.assert_contains("Install plugin");
    s.assert_absent("Installed by admin");
    s.assert_contains("Figma · Enabled by Admin · ChatGPT Marketplace");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:786 plugins_popup_remote_row_opens_remote_detail kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_remote_row_opens_remote_detail() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("Calendar");
    s.assert_contains("Available");
    s.assert_contains("Press Enter to install or view plugin details");
    s.keys("enter");
    // OpenPluginDetailLoading and FetchPluginDetail (remote marketplace name, plugin id) are events: not on screen
}

// ORACLE chatwidget/tests/popups_and_settings.rs:842 plugin_detail_unmaterialized_default_uses_remote_install_path kind=ui owner=dialogs-pane
#[test]
fn plugin_detail_unmaterialized_default_uses_remote_install_path() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("Install plugin");
    s.assert_contains("Install this plugin now");
    s.keys("down enter");
    // OpenPluginInstallLoading and FetchPluginInstall (remote location, plugin name) are events: not on screen
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("enter");
    s.assert_snap("plugin_detail_popup_local_share_read_only");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1080 plugin_detail_popup_shows_admin_disabled_status_snapshot kind=snap owner=dialogs-pane
#[test]
fn plugin_detail_popup_shows_admin_disabled_status_snapshot() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("enter");
    s.assert_contains("  Admin Blocked · Disabled by admin · ChatGPT Marketplace");
    s.assert_contains("This plugin is disabled by your workspace admin");
    s.assert_absent("Install this plugin now");
    // the oracle then checks that no action event follows: events are not on screen
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1132 plugins_popup_admin_disabled_installed_plugin_has_no_toggle_hint kind=snap owner=dialogs-pane snap=plugins_popup_admin_disabled_installed
#[test]
fn plugins_popup_admin_disabled_installed_plugin_has_no_toggle_hint() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(120, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_snap("plugins_popup_admin_disabled_installed");
    s.assert_contains("[!] Admin Blocked");
    s.assert_contains("Disabled");
    s.assert_contains("Press Enter to view plugin details");
    s.assert_absent("Disabled by admin");
    s.assert_absent("Tab to disable");
    let before = s.shot();
    s.keys("tab");
    s.assert_same_rows(&before);
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1176 plugins_popup_admin_disabled_available_plugin_has_view_only_hint kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_admin_disabled_available_plugin_has_view_only_hint() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("Admin Blocked");
    s.assert_contains("Press Enter to view plugin details");
    s.assert_absent("install or view");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1210 plugins_popup_remote_section_fallback_states_when_remote_plugin_disabled_snapshot kind=snap owner=dialogs-pane snap=plugins_popup_empty_shared_section_hidden
#[test]
fn plugins_popup_remote_section_fallback_states_when_remote_plugin_disabled_snapshot() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    // the oracle turns RemotePlugin off; the remote sections load from the app server (script lines)
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    select_tab(&mut s, "Loading OpenAI Curated plugins...");
    s.assert_contains("Loading OpenAI Curated plugins...");
    select_tab(&mut s, "Loading Workspace plugins.");
    s.assert_contains("Loading Workspace plugins.");
    select_tab(&mut s, "Loading Shared with me plugins.");
    s.assert_contains("Loading Shared with me plugins.");
    s.assert_snap("plugins_popup_empty_shared_section_hidden");
    s.assert_absent("Shared with me");
    select_tab(&mut s, "Workspace unavailable.");
    s.assert_contains("Workspace unavailable.");
    s.assert_contains("Sign in to ChatGPT to load workspace plugins.");
    let mut remote = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    remote.type_text("/plugins").keys("enter");
    select_tab(&mut remote, "No OpenAI Curated plugins available");
    remote.assert_contains("No OpenAI Curated plugins available");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1317 plugins_popup_remote_detail_tracks_physical_and_policy_install_state kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_remote_detail_tracks_physical_and_policy_install_state() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("Installed 1 of 1 available plugins.");
    s.assert_contains("Docs");
    s.keys("right");
    s.assert_contains("Showing 1 installed plugins.");
    s.assert_contains("Docs");
    s.keys("enter");
    // OpenPluginDetailLoading and FetchPluginDetail (remote marketplace, plugin id) are events: not on screen
    // the policy refresh (an admin-assigned remote duplicate) arrives from the app server: a script line
    s.assert_contains("Installed 0 of 1 available plugins.");
    s.assert_contains("Admin assigned");
    s.keys("right");
    s.assert_contains("Showing 0 installed plugins.");
    s.assert_contains("No installed plugins");
    s.assert_absent("Docs");
    s.keys("left enter");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1464 plugin_detail_error_popup_skips_disabled_row_numbering kind=snap owner=dialogs-pane snap=plugin_detail_error_popup
#[test]
fn plugin_detail_error_popup_skips_disabled_row_numbering() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("enter");
    // the detail failure ("Failed to load plugin details.") arrives from the app server: a script line
    s.assert_snap("plugin_detail_error_popup");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1492 plugins_popup_refresh_preserves_selected_row_position kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_refresh_preserves_selected_row_position() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("down");
    s.assert_contains("› [-] Slack");
    // the refreshed list (Airtable, Notion, Slack) arrives from the app server: a script line
    s.assert_contains("› [-] Notion");
    s.assert_contains("Airtable");
    s.assert_contains("Slack");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1573 plugins_popup_refreshes_installed_counts_after_install kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_refreshes_installed_counts_after_install() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_contains("Installed 1 of 2 available plugins.");
    s.assert_contains("Available");
    // the refresh (both installed) arrives from the app server: a script line
    s.assert_contains("Installed 2 of 2 available plugins.");
    s.assert_contains("Installed   Tab to disable; Enter view details");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1642 plugins_popup_toggle_keys_preserve_search_and_selection kind=snap owner=dialogs-pane snap=plugins_popup_toggle_refresh_preserves_search
#[test]
fn plugins_popup_toggle_keys_preserve_search_and_selection() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("down space");
    // SetPluginEnabled (plugin-drive disabled, then enabled) are events: not on screen
    s.type_text("dr");
    s.keys("tab");
    // the enabled state is confirmed by the app server: a script line
    s.assert_snap("plugins_popup_toggle_refresh_preserves_search");
    s.assert_contains("› [*] Drive");
    s.assert_contains("Tab to disable");
    s.assert_contains("dr");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1722 plugins_popup_hides_tab_toggle_hint_when_tab_is_configured kind=snap owner=dialogs-pane snap=plugins_popup_custom_tab_binding_hides_toggle_hint
#[test]
fn plugins_popup_hides_tab_toggle_hint_when_tab_is_configured() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .keymap(&[("list.move_down", "tab")])
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.assert_snap("plugins_popup_custom_tab_binding_hides_toggle_hint");
    s.assert_contains("Installed   Press Enter to view plugin details");
    s.assert_absent("Tab to disable");
    s.assert_absent("tab toggle");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1754 plugins_popup_space_on_uninstalled_row_does_not_start_search kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_space_on_uninstalled_row_does_not_start_search() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    let before = s.shot();
    s.keys("space");
    // the oracle checks that Space sends no event: the rows stay the same
    s.assert_same_rows(&before);
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.type_text("sla");
    s.assert_snap("plugins_popup_search_filtered");
    s.assert_absent("Calendar");
    s.assert_absent("Drive");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1883 plugins_popup_search_matches_plugin_descriptions kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_search_matches_plugin_descriptions() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.type_text("document");
    s.assert_contains("Drive");
    s.assert_absent("Calendar");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:1921 plugins_popup_installed_tab_filters_rows_and_clears_search kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_installed_tab_filters_rows_and_clears_search() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
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
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
    s.keys("right right");
    s.assert_contains("OpenAI Curated marketplace.");
    s.assert_contains("Calendar");
    s.assert_absent("Repo Plugin");
    s.assert_absent("ChatGPT Marketplace ·");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2015 plugins_popup_refresh_preserves_duplicate_marketplace_tab_by_path kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_refresh_preserves_duplicate_marketplace_tab_by_path() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
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
    // the same two marketplaces arrive again from the app server: a script line
    s.assert_contains("Duplicate Marketplace (2/2).");
    s.assert_contains("Repo Plugin");
    s.assert_absent("Home Plugin");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2086 plugins_popup_search_no_matches_and_backspace_restores_results kind=ui owner=dialogs-pane
#[test]
fn plugins_popup_search_no_matches_and_backspace_restores_results() {
    kit::pending(Owner::DialogsPane, PLUGINS);
    let mut s = Screen::build(Owner::DialogsPane)
        .size(100, 24)
        .live()
        .open();
    s.type_text("/plugins").keys("enter");
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
    kit::pending(Owner::DialogsPane, APPS);
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
    kit::pending(Owner::DialogsPane, APPS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    // the full list, then the partial and failed refreshes arrive from the app server: script lines
    s.assert_contains("Installed 1 of 2 available apps.");
}

// ORACLE chatwidget/tests/popups_and_settings.rs:2535 apps_popup_preserves_selected_app_across_refresh kind=ui owner=dialogs-pane
#[test]
fn apps_popup_preserves_selected_app_across_refresh() {
    kit::pending(Owner::DialogsPane, APPS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    s.keys("down");
    s.assert_contains("› Slack");
    // the refreshed list (Airtable, Notion, Slack) arrives from the app server: a script line
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
    kit::pending(Owner::DialogsPane, APPS);
    let mut s = Screen::build(Owner::DialogsPane).size(80, 24).live().open();
    s.type_text("/apps").keys("enter");
    s.assert_contains("Loading apps...");
    // the failure (app/list failed: 403 Forbidden) arrives from the app server: a script line
    s.assert_absent("Loading apps...");
    s.assert_absent("403 Forbidden");
    s.assert_contains("Failed to load apps.");
    s.assert_snap("apps_popup_error_state");
    s.keys("enter");
    s.assert_contains("Loading apps...");
}
