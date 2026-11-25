// test root: pages
//! The model provider list and editor (oracle `app/provider.rs`, `app/provider/`, tests
//! `provider_tests.rs`, `provider/editor_tests.rs`) as the feature slice `providers`: the settings
//! panel's `Model providers` row opens the sheet, the server answers `modelProvider/list`, a write
//! is `modelProvider/config/write` or `config/value/write`. The frames are the oracle's snapshots
//! (`provider_list`, `saved_default_provider`, `editable_provider_details`,
//! `openai_provider_details`) and the one inline snapshot of the editor tests.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde_json::{Value, json};

use crate::kit::{Owner, Screen};

const PANEL: &str = r#"{"step":"fact","feature":"settings","fact":{"kind":"config_panel","rows":[{"id":"model_providers","section":"sessions","label":"Model providers","description":"Choose the default provider for new chats on the connected server","keywords":"provider gateway endpoint model default","value":{"choice":"Choose"}}]}}"#;

/// The session of `thread-1` on `provider`, and the panel with its one row.
fn script(provider: &str) -> &'static str {
    let started = json!({"step": "notify", "message": {
        "method": "thread/started", "params": {"thread": thread(provider)}}});
    let text = format!(
        "{}\n{started}\n{PANEL}\n",
        include_str!("../../src/features/providers/fixtures/list.jsonl")
    );
    Box::leak(text.into_boxed_str())
}

fn thread(provider: &str) -> Value {
    json!({
        "id": "thread-1", "environments": null, "extra": null, "sessionId": "thread-1",
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": provider,
        "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
        "recencyAt": 0, "status": {"type": "idle"}, "path": null, "cwd": "/tmp/project",
        "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
        "agentRole": null, "gitInfo": null, "name": null, "daybreakEnabled": null,
        "turns": []
    })
}

fn openai() -> Value {
    json!({
        "id": "openai", "name": "OpenAI", "canEdit": false,
        "baseUrl": "https://example.test/organization/deployment/region/service/openai/v1",
        "settings": {"type": "openai"}
    })
}

fn gateway(id: &str, name: &str, url: Option<&str>, auth: Value) -> Value {
    json!({
        "id": id, "name": name, "canEdit": true, "baseUrl": url,
        "settings": {"type": "customGateway", "auth": auth}
    })
}

fn list(data: Vec<Value>, default: &str) -> Value {
    json!({"data": data, "defaultProviderId": default, "canChangeDefault": true})
}

/// The chat on `provider`, `/config` then the `Model providers` row.
fn open(width: u16, height: u16, provider: &str, answer: Value) -> Screen {
    let mut s = Screen::build(Owner::StartupFlow)
        .size(width, height)
        .script(script(provider))
        .open();
    s.server_answers("modelProvider/list", answer);
    s.type_text("/config");
    s.keys("enter");
    s.keys("right right right");
    s.assert_contains("Model providers");
    s.keys("enter");
    s.wait(300);
    s
}

/// The last `n` rows of the screen, where the sheet sits (the oracle's `render_bottom_popup`).
fn tail(s: &Screen, n: usize) -> Vec<String> {
    let rows = s.rows();
    rows[rows.len() - n..].to_vec()
}

fn assert_rows(got: &[String], want: &[&str]) {
    let want: Vec<String> = want.iter().map(|row| (*row).to_string()).collect();
    assert_eq!(
        got,
        want.as_slice(),
        "got:\n{}\nwant:\n{}",
        got.join("\n"),
        want.join("\n")
    );
}

/// The pager's rule: dashes up to the percentage read (`─── 100% ─`).
fn rule(width: usize, percent: u8) -> String {
    let tail = format!(" {percent}% ─");
    format!("{}{tail}", "─".repeat(width - tail.chars().count()))
}

fn seed_gateway() -> Value {
    gateway(
        "seed",
        "Existing",
        None,
        json!({"mode": "environmentVariable", "envKey": "CATALOG_KEY"}),
    )
}

fn team(url: Option<&str>, auth: Value) -> Value {
    gateway("team", "Team", url, auth)
}

fn ok(provider: Value) -> Value {
    json!({"provider": provider})
}

fn written() -> Value {
    json!({"status": "ok", "version": "1", "filePath": "/home/me/.codex/config.toml",
           "overriddenMetadata": null})
}

fn settings_of(s: &Screen) -> Vec<Value> {
    s.request_params("modelProvider/config/write")
        .into_iter()
        .map(|params| params["settings"].clone())
        .collect()
}

// ---- oracle: app/provider_tests.rs

#[test]
fn the_list_is_the_oracles_frame() {
    // provider_list
    let s = open(80, 24, "ollama", list(vec![openai()], "lmstudio"));
    assert_rows(
        &tail(&s, 9),
        &[
            "  Model providers",
            "  Current chat: ollama · Default for new chats: lmstudio",
            "",
            "",
            "› 1. ○ OpenAI                             openai",
            "  2. Create a custom provider or gateway",
            "",
            "  Use /new to start a chat with the selected provider.",
            "  enter details · space set default · esc back",
        ],
    );
    assert_eq!(s.requests_of("modelProvider/list"), 1);
}

#[test]
fn a_provider_the_catalog_reports_shows_its_endpoint_in_the_details() {
    // openai_provider_details: the first page, then PageDown
    // the oracle's pager terminal is 48x9: the pager covers the screen, so the terminal is
    // narrowed once the sheet under it has been laid out
    let mut s = open(48, 24, "ollama", list(vec![openai()], "lmstudio"));
    s.keys("enter");
    s.resize(48, 9);
    s.wait(300);
    assert_rows(
        &s.rows(),
        &[
            "/ Model provider details/ / / / / / / / / / / /",
            "OpenAI",
            "",
            "Provider ID  openai",
            "Type  Built-in OpenAI provider",
            rule(48, 0).as_str(),
            " ↑/↓ to scroll · pgup/pgdn to page · home/end to",
            " q close",
            "",
        ],
    );
    s.keys("pagedown");
    assert_rows(
        &s.rows(),
        &[
            "/ Model provider details/ / / / / / / / / / / /",
            "Type  Built-in OpenAI provider",
            "Base URL",
            "https://example.test/organization/deployment/reg",
            "ion/service/openai/v1",
            rule(48, 100).as_str(),
            " ↑/↓ to scroll · pgup/pgdn to page · home/end to",
            " q close",
            "",
        ],
    );
    // the built-in provider has nothing to edit: Enter does not open a form
    s.keys("enter");
    s.assert_contains("Model provider details");
    s.keys("q");
    s.assert_contains("Model providers");
}

#[test]
fn space_sets_the_default_and_the_list_says_what_it_did() {
    // saved_default_provider
    let two = |default: &str| {
        list(
            vec![
                gateway(
                    "gateway",
                    "Example Gateway",
                    Some("http://127.0.0.1:9/v1"),
                    json!({"mode": "none"}),
                ),
                json!({"id": "openai", "name": "OpenAI", "canEdit": false, "baseUrl": null,
                       "settings": {"type": "openai"}}),
            ],
            default,
        )
    };
    let mut s = open(100, 24, "test-provider", two("openai"));
    s.server_answers("config/value/write", written());
    s.server_answers("modelProvider/list", two("gateway"));
    s.keys("space");
    s.wait(300);
    assert_rows(
        &tail(&s, 10),
        &[
            "  Model providers",
            "  Current chat: test-provider · Default for new chats: gateway",
            "",
            "",
            "› 1. ● Example Gateway                    gateway",
            "  2. ○ OpenAI                             openai",
            "  3. Create a custom provider or gateway",
            "",
            "  /new now uses gateway. Restart the app server and TUI to refresh /model.",
            "  enter details · esc back",
        ],
    );
    let writes = s.request_params("config/value/write");
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0]["keyPath"], "model_provider");
    assert_eq!(writes[0]["value"], "gateway");
    assert_eq!(writes[0]["mergeStrategy"], "replace");
    // the chat the user is in keeps its provider
    s.assert_contains("Current chat: test-provider");
}

#[test]
fn a_default_overridden_by_a_higher_layer_says_why() {
    let mut s = open(100, 24, "ollama", list(vec![openai()], "lmstudio"));
    s.server_answers(
        "config/value/write",
        json!({"status": "okOverridden", "version": "1", "filePath": "/c.toml",
               "overriddenMetadata": {"message": "Managed config sets it.",
                   "overridingLayer": {"name": {"type": "system", "file": "/etc/c.toml"}, "version": "1"},
                   "effectiveValue": "lmstudio"}}),
    );
    s.keys("space");
    s.wait(300);
    s.assert_contains("Saved openai. Managed config sets it.");
}

#[test]
fn a_refused_default_stays_on_the_list_with_the_servers_words() {
    let mut s = open(100, 24, "ollama", list(vec![openai()], "lmstudio"));
    s.server_refuses("config/value/write", Some(-32602), "Rejected");
    s.keys("space");
    s.wait(300);
    s.assert_contains("Could not save provider:");
    s.assert_contains("Rejected");
    s.assert_contains("Model providers");
    s.assert_contains("enter details");
}

#[test]
fn a_lost_reply_of_the_default_asks_to_check() {
    let mut s = open(100, 24, "ollama", list(vec![openai()], "lmstudio"));
    s.server_refuses("config/value/write", None, "connection lost");
    s.keys("space");
    s.wait(300);
    s.assert_contains("Could not confirm whether openai was saved:");
}

#[test]
fn a_server_without_the_list_says_to_update() {
    // unsupported_list_method_explains_server_compatibility, both of its codes
    for (code, message) in [
        (-32601, "Method not found"),
        (
            -32600,
            "modelProvider/list failed: Invalid request: unknown variant `modelProvider/list`",
        ),
    ] {
        let mut s = open(80, 24, "ollama", list(vec![], "openai"));
        s.keys("esc");
        s.server_refuses("modelProvider/list", Some(code), message);
        s.keys("esc");
        s.type_text("/config");
        s.keys("enter");
        s.keys("right right right enter");
        s.wait(300);
        s.assert_contains("Update the connected app server to browse model providers.");
        s.assert_absent("Could not load model providers");
    }
}

#[test]
fn another_failure_of_the_list_is_shown_with_a_way_back() {
    let mut s = open(80, 24, "ollama", list(vec![], "openai"));
    s.keys("esc");
    s.server_refuses("modelProvider/list", Some(-32603), "boom");
    s.type_text("/config");
    s.keys("enter");
    s.keys("right right right enter");
    s.wait(300);
    s.assert_contains("Could not load model providers:");
    s.assert_contains("boom");
    s.assert_contains("Back");
    s.keys("enter");
    s.assert_absent("Could not load model providers");
}

#[test]
fn esc_on_the_details_returns_to_the_list_where_it_was() {
    let mut s = open(
        80,
        24,
        "ollama",
        list(vec![openai(), seed_gateway()], "openai"),
    );
    s.keys("down enter");
    s.assert_contains("Model provider details");
    s.keys("q");
    s.assert_absent("Model provider details");
    s.assert_contains("Model providers");
    s.assert_contains("› 2. ○ Existing");
    // the list is read once while the sheet is open
    assert_eq!(s.requests_of("modelProvider/list"), 1);
}

// ---- oracle: app/provider_tests.rs adding_a_custom_provider_saves_to_the_server

#[test]
fn an_existing_providers_details_open_its_editor() {
    // editable_provider_details
    let mut s = open(
        72,
        12,
        "ollama",
        list(vec![openai(), seed_gateway()], "openai"),
    );
    s.keys("end up enter");
    s.wait(300);
    assert_rows(
        &s.rows(),
        &[
            "/ Model provider details/ / / / / / / / / / / / / / / / / / / / / / / /",
            "Existing",
            "",
            "Provider ID  seed",
            "Name  Existing",
            "Base URL  Not reported",
            "API key variable  CATALOG_KEY",
            "~",
            rule(72, 100).as_str(),
            " ↑/↓ to scroll · pgup/pgdn to page · home/end to jump",
            " q close · enter Edit configuration",
            "",
        ],
    );
    s.keys("enter");
    s.assert_contains("Edit Existing");
    s.assert_contains("› Name*  Existing");
    // Esc goes back to the list
    s.keys("esc");
    s.assert_contains("Model providers");
}

#[test]
fn a_custom_provider_is_added_with_the_variable_completed_from_the_catalog() {
    let mut s = open(
        72,
        24,
        "ollama",
        list(vec![openai(), seed_gateway()], "openai"),
    );
    s.keys("end enter");
    // the create form
    assert_rows(
        &tail(&s, 10),
        &[
            "Create a custom provider or gateway",
            "",
            "› Name*  Team gateway",
            "  Base URL*  https://gateway.example/v1",
            "  API key variable  $TEAM_API_KEY",
            "  Save",
            "",
            "* Required",
            "Enter the environment variable name on the app server.",
            "tab move · enter next/save · esc back",
        ],
    );
    s.paste("Team gateway");
    s.keys("enter");
    s.paste("https://gateway.example.test/v1");
    s.keys("enter");
    s.paste("CAT");
    s.keys("ctrl+space");
    s.assert_contains("$CATALOG_KEY");
    s.keys("tab");
    // the form saves, the server answers, the list is read again
    let created = gateway(
        "team-gateway",
        "Team gateway",
        Some("https://gateway.example.test/v1"),
        json!({"mode": "environmentVariable", "envKey": "CATALOG_KEY"}),
    );
    s.server_answers("modelProvider/config/write", ok(created.clone()));
    s.server_answers(
        "modelProvider/list",
        list(vec![openai(), seed_gateway(), created], "openai"),
    );
    s.keys("enter");
    s.wait(300);
    assert_eq!(
        settings_of(&s),
        vec![json!({"type": "customGateway", "name": "Team gateway",
            "baseUrl": "https://gateway.example.test/v1", "envKey": "CATALOG_KEY"})]
    );
    s.assert_contains("Team gateway");
    s.assert_contains("Default for new chats: openai");
    s.assert_contains("Saved provider configuration.");
}

#[test]
fn a_create_whose_reply_is_lost_is_not_retried_blindly() {
    let mut s = open(72, 24, "ollama", list(vec![openai()], "openai"));
    s.keys("end enter");
    s.paste("Team gateway");
    s.keys("enter");
    s.paste("https://gateway.example.test/v1");
    s.server_refuses("modelProvider/config/write", Some(-32603), "Internal error");
    s.keys("tab tab enter");
    s.wait(300);
    s.assert_contains("Could not confirm the save. Check the provider list before adding it");
}

// ---- oracle: app/provider/editor_tests.rs

#[test]
fn entering_a_key_for_a_legacy_gateway_prompts_for_its_address() {
    let mut s = open(
        72,
        24,
        "ollama",
        list(vec![team(None, json!({"mode": "none"}))], "openai"),
    );
    s.keys("enter enter");
    s.keys("tab tab");
    s.paste("TEAM_KEY");
    s.keys("tab enter");
    assert_rows(
        &tail(&s, 11),
        &[
            "Edit Team",
            "",
            "  Name*  Team",
            "  Base URL*  https://gateway.example/v1",
            "  API key variable  $TEAM_KEY",
            "› Save",
            "Enter a base URL to use this API key variable.",
            "",
            "* Required",
            "Enter the environment variable name on the app server.",
            "tab move · enter next/save · esc back",
        ],
    );
    assert!(settings_of(&s).is_empty(), "nothing was submitted");
}

#[test]
fn clearing_the_configured_url_keeps_the_form_open_until_it_is_corrected() {
    let old = "https://old.example";
    let mut s = open(
        72,
        24,
        "ollama",
        list(
            vec![team(
                Some(old),
                json!({"mode": "environmentVariable", "envKey": "TEAM_KEY"}),
            )],
            "openai",
        ),
    );
    s.keys("enter enter");
    s.keys("tab");
    for _ in 0..old.len() {
        s.keys("backspace");
    }
    s.keys("tab tab enter");
    s.assert_contains("Enter a base URL.");
    assert!(settings_of(&s).is_empty());
    s.keys("shift+tab shift+tab");
    s.paste("https://new.example");
    // Enter walks to Save; the Enter still held there does not save (the guard of `KeyGuard`: a
    // repeat is a press for the focus path, so the guard stands for the oracle's `Press` check)
    s.keys("enter enter");
    s.key_repeat("enter");
    assert!(settings_of(&s).is_empty());
    s.wait(1100);
    s.server_refuses(
        "modelProvider/config/write",
        Some(-32602),
        "Server validation failed.",
    );
    s.keys("enter");
    s.wait(300);
    assert_eq!(
        settings_of(&s),
        vec![json!({"type": "customGateway", "providerId": "team",
            "baseUrl": "https://new.example"})]
    );
    // the form stays, with what was typed and what the server said
    s.assert_contains("https://new.example");
    s.assert_contains("Server validation failed.");
    s.assert_contains("Edit Team");
}

#[test]
fn a_rename_leaves_the_settings_the_form_does_not_show_out_of_the_write() {
    // hidden_gateway_settings_stay_out_of_rename_patch, through the form
    let hidden = gateway(
        "team",
        "Team",
        None,
        json!({"mode": "externallyConfigured", "envKey": "EXTERNAL"}),
    );
    let mut s = open(72, 24, "ollama", list(vec![hidden.clone()], "openai"));
    s.keys("enter enter");
    s.assert_contains("Edit Team");
    s.assert_absent("Base URL");
    s.keys("ctrl+a ctrl+k");
    s.paste("Renamed");
    s.keys("tab");
    s.server_answers("modelProvider/config/write", ok(hidden));
    s.keys("enter");
    s.wait(300);
    assert_eq!(
        settings_of(&s),
        vec![json!({"type": "customGateway", "providerId": "team", "name": "Renamed"})]
    );
}
