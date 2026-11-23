//! Screen tests of the `import` slot (S06): what the model's typed data is
//! told as (the labels, the lines under an item, the notices of the history,
//! the errors) and what the review's keys do (`mod flow`, over the scripted
//! backend; the oracle's key tests are in `tests/oracle/import.rs`). They carry the names of
//! the oracle's tests (`flow_tests.rs`, `mod.rs` tests, `source_tests.rs`).
//! The flow itself is `core/tests/import.rs`; the look is
//! `tests/oracle/import.rs`; its snapshots are in `tests/snapshots/import/`.

#![allow(clippy::expect_used)]

use codex_tui_core::app::AppFx;
use codex_tui_core::screens::import::{
    self as import, Failure, Intent, Mark, Model, Outcome, Source, apply, apply_intent,
};
use codex_tui_rtwk::pages::startup::import::View;
use codex_tui_rtwk::pages::startup::import::labels;
use codex_tui_rtwk::pages::startup::import::notices::{self, Line, Tone};
use codex_tui_rtwk::pages::startup::import::rows::{self, ReviewRow};
use codex_tui_rtwk::pages::startup::import::strings::EN;
use pretty_assertions::assert_eq;
use rtwk::l10n::NumSyms;
use serde_json::json;

use crate::common::slot;

const NUM: NumSyms = NumSyms::ENGLISH;

#[test]
fn import_slot_is_registered_and_empty() {
    slot("import");
}

// ---- the model, made the way a fixture script makes it: a seeded event

fn config(description: &str) -> serde_json::Value {
    json!({"itemType":"CONFIG","description":description,"cwd":null,"details":null})
}

fn selected_items() -> Vec<serde_json::Value> {
    vec![
        config("Import settings"),
        json!({"itemType":"MEMORY","description":"Import memory","cwd":null,
            "details":{"memory":["project-a","project-b"]}}),
        json!({"itemType":"SKILLS","description":"Import skills","cwd":null,
            "details":{"skills":[{"name":"triage"},{"name":"release-notes"},
                {"name":"risk-check"},{"name":"incident-review"}]}}),
        json!({"itemType":"MCP_SERVER_CONFIG","description":"Import MCP servers","cwd":null,
            "details":{"mcpServers":[{"name":"docs"},{"name":"issues"}]}}),
        json!({"itemType":"SESSIONS","description":"Import chat sessions","cwd":null,
            "details":{"sessions":[
                {"path":"/sessions/alpha.jsonl","cwd":"/workspace/project","title":"Alpha rollout"},
                {"path":"/sessions/beta.jsonl","cwd":"/workspace/project","title":"Beta review"},
                {"path":"/sessions/gamma.jsonl","cwd":"/workspace/project","title":"Gamma notes"}]}}),
        json!({"itemType":"PLUGINS","description":"Import plugins","cwd":null,
            "details":{"plugins":[{"marketplaceName":"example","pluginNames":["formatter","reviewer"]}]}}),
    ]
}

fn completed() -> serde_json::Value {
    json!({"completed": {
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
    }})
}

fn sample_items() -> Vec<serde_json::Value> {
    let plugins = json!({"plugins":[
        {"marketplaceName":"acme-tools","pluginNames":["deployer","formatter","lint"]},
        {"marketplaceName":"team-marketplace","pluginNames":["asana"]},
        {"marketplaceName":"debug","pluginNames":["sample"]},
        {"marketplaceName":"data-tools","pluginNames":["warehouse"]}]});
    vec![
        config("Migrate /Users/alex/.claude/settings.json into /Users/alex/.codex/config.toml"),
        json!({"itemType":"SESSIONS","description":"Migrate recent Claude Code sessions","cwd":null,
            "details":{"sessions":[{"path":"/Users/alex/.claude/projects/project/session.jsonl",
                "cwd":"/workspace/project","title":"Investigate migration UX"}]}}),
        json!({"itemType":"PLUGINS","description":"Migrate enabled plugins from /workspace/project/.claude/settings.json",
            "cwd":"/workspace/project","details":plugins}),
        json!({"itemType":"AGENTS_MD","description":"Migrate /workspace/project/CLAUDE.md to /workspace/project/AGENTS.md",
            "cwd":"/workspace/project","details":null}),
    ]
}

fn seed(value: serde_json::Value) -> import::Ev {
    import::seed(value).expect("a seeded event")
}

fn detected(sources: serde_json::Value, selected: Option<Vec<usize>>) -> serde_json::Value {
    json!({"detected": {"found": sources, "selected": selected}})
}

fn reviewing(items: Vec<serde_json::Value>, selected: Option<Vec<usize>>) -> Model {
    let mut model = Model::default();
    let found = json!([{"source": "claude-code", "items": items}]);
    apply(
        &mut model,
        seed(detected(found, selected)),
        &mut AppFx::default(),
    );
    model
}

// ---- the notices of the history (oracle: flow_tests.rs)

fn texts(lines: &[Line]) -> String {
    lines.iter().map(Line::text).collect::<Vec<_>>().join("\n")
}

/// What an import of `selected_items` started and what the notice finished
/// with, as the model holds them.
fn notices_of_the_flow() -> (import::Started, import::Finished) {
    let mut model = reviewing(selected_items(), None);
    let all: Vec<usize> = (0..model.items.len()).collect();
    apply_intent(&mut model, Intent::Proceed(all), &mut AppFx::default());
    let Some(import::Request::Import { request, .. }) = model.request() else {
        panic!("an import request");
    };
    apply(
        &mut model,
        import::Ev::Imported {
            request,
            result: Ok("import-1".into()),
        },
        &mut AppFx::default(),
    );
    let Some(Outcome::Started(started)) = model.outcome.clone() else {
        panic!("started");
    };
    apply(&mut model, seed(completed()), &mut AppFx::default());
    (started, model.finished.expect("the completion"))
}

#[test]
fn external_agent_config_migration_messages_snapshot() {
    let (started, finished) = notices_of_the_flow();
    let mut lines: Vec<Line> = Vec::new();
    for remaining in [0, 1, 2] {
        let started = import::Started {
            remaining,
            ..started.clone()
        };
        lines.extend(notices::started(&EN, &started, &NUM));
    }
    lines.extend(notices::finished(&EN, &finished, &NUM));
    let mut text = texts(&lines);
    text.push('\n');
    text.push_str(EN.no_items);
    text.push('\n');
    text.push_str(EN.in_progress);
    let started = "• Import started. You can keep working while it finishes.
  Imported setup will apply to new chats.
  Importing:
    Settings: 1
    Memory: 2 — project-a, project-b
    Skills: 4 — triage, release-notes, risk-check, +1 more
    MCP servers: 2 — docs, issues
    Chat sessions: 3 — Alpha rollout, Beta review, Gamma notes
    Plugins: 2 — formatter, reviewer";
    let expected = format!(
        "{started}\n{started}\n  1 additional item remains. After it finishes, run /import again to review it.\n{started}\n  2 additional items remain. After it finishes, run /import again to review them.\n• Import finished: 4 imported, 1 failed.
  Results by type:
    Settings: 1 imported, 0 failed
    Memory: 2 imported, 0 failed
    Plugins: 1 imported, 1 failed
  Run /import again to check for additional items.
No compatible setup was found to import.
A previous external agent import is still running. Wait for it to finish before importing again."
    );
    assert_eq!(text, expected);
}

fn seg(tone: Tone, text: &str) -> (Tone, String) {
    (tone, text.to_owned())
}

fn tones(line: &Line) -> Vec<(Tone, String)> {
    line.0.iter().map(|s| (s.tone, s.text.clone())).collect()
}

#[test]
fn external_agent_config_migration_status_lines_use_semantic_colors() {
    let (started, finished) = notices_of_the_flow();
    let started = notices::started(&EN, &started, &NUM);
    assert_eq!(
        tones(&started[0]),
        [
            seg(Tone::Dim, "• "),
            seg(Tone::Cyan, "Import started."),
            seg(Tone::Plain, " You can keep working while it finishes."),
        ]
    );
    assert_eq!(
        tones(&started[1]),
        [
            seg(Tone::Plain, "  "),
            seg(Tone::Dim, "Imported setup will apply to new chats.")
        ]
    );
    assert_eq!(
        tones(&started[2]),
        [seg(Tone::Plain, "  "), seg(Tone::CyanBold, "Importing:")]
    );
    assert_eq!(
        tones(&started[3]),
        [
            seg(Tone::Plain, "    "),
            seg(Tone::Cyan, "Settings"),
            seg(Tone::Plain, ": "),
            seg(Tone::Green, "1")
        ]
    );
    assert_eq!(
        tones(&started[5]),
        [
            seg(Tone::Plain, "    "),
            seg(Tone::Cyan, "Skills"),
            seg(Tone::Plain, ": "),
            seg(Tone::Green, "4"),
            seg(Tone::Dim, " — "),
            seg(Tone::Plain, "triage, release-notes, risk-check, +1 more"),
        ]
    );
    assert_eq!(started.len(), 9);

    let finished = notices::finished(&EN, &finished, &NUM);
    assert_eq!(
        tones(&finished[0]),
        [
            seg(Tone::Dim, "• "),
            seg(Tone::Plain, "Import finished: "),
            seg(Tone::Green, "4 imported"),
            seg(Tone::Plain, ", "),
            seg(Tone::Red, "1 failed"),
            seg(Tone::Plain, "."),
        ]
    );
    assert_eq!(
        tones(&finished[1]),
        [
            seg(Tone::Plain, "  "),
            seg(Tone::CyanBold, "Results by type:")
        ]
    );
    assert_eq!(
        tones(&finished[2]),
        [
            seg(Tone::Plain, "    "),
            seg(Tone::Cyan, "Settings"),
            seg(Tone::Plain, ": "),
            seg(Tone::Green, "1 imported"),
            seg(Tone::Plain, ", "),
            seg(Tone::Green, "0 failed"),
        ]
    );
    assert_eq!(
        tones(&finished[4]),
        [
            seg(Tone::Plain, "    "),
            seg(Tone::Cyan, "Plugins"),
            seg(Tone::Plain, ": "),
            seg(Tone::Green, "1 imported"),
            seg(Tone::Plain, ", "),
            seg(Tone::Red, "1 failed"),
        ]
    );
    assert_eq!(
        tones(&finished[5]),
        [
            seg(Tone::Plain, "  "),
            seg(
                Tone::Dim,
                "Run /import again to check for additional items."
            )
        ]
    );
}

// ---- the words of the flow's errors and of the source prompt

#[test]
fn external_agent_config_detection_failure_names_each_source() {
    let failure = Failure::Detect(vec![
        (Source::Claude, "first failure".into()),
        (Source::Cursor, "second failure".into()),
    ]);
    assert_eq!(
        labels::failure(&EN, &failure, &NUM),
        "Could not check for importable setup: Claude Code: first failure; Cursor: second failure"
    );
    assert_eq!(labels::source(&EN, Source::Cursor), "Cursor");
    assert_eq!(EN.import_failed("boom", &NUM), "Import failed: boom");
}

// ---- the review (oracle: external_agent_config_migration/mod.rs tests)

#[test]
fn the_checklist_and_summary_rows_are_the_oracles() {
    // Oracle: `build_summary_render_lines` and `build_customize_render_lines`
    // (the customize snapshot's text, the plugin lines, the descriptions).
    use ReviewRow as Row;
    let model = reviewing(sample_items(), None);
    let summary = rows::of(View::Summary, &model, &model.preselected(), &EN, &NUM);
    assert_eq!(summary.len(), 9);
    assert_eq!(
        summary[..3],
        [
            Row::Group {
                index: 0,
                mark: Mark::All,
                label: "Tools & setup".into()
            },
            Row::Detail(
                "Settings, instructions, integrations, agents, commands, and skills".into()
            ),
            Row::Importing(Some("Settings 1".into())),
        ]
    );
    assert_eq!(
        summary[3],
        Row::Group {
            index: 1,
            mark: Mark::All,
            label: "Current project".into()
        }
    );
    assert_eq!(
        summary[5],
        Row::Importing(Some("Plugins 6, Instructions 1".into()))
    );
    assert_eq!(summary[8], Row::Importing(Some("Chat sessions 1".into())));

    let rows = rows::of(View::Customize, &model, &model.preselected(), &EN, &NUM);
    let details: Vec<String> = rows
        .iter()
        .filter_map(|row| match row {
            Row::Detail(text) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        details,
        [
            "Import /Users/alex/.claude/settings.json into /Users/alex/.codex/config.toml",
            "Import recent Claude Code sessions",
            "1 chat session: Investigate migration UX",
            "Import enabled plugins from .claude/settings.json (4 marketplaces, 6 plugins)",
            "• acme-tools: deployer, formatter, +1 more",
            "• team-marketplace: asana",
            "• debug: sample",
            "• +1 more marketplaces",
            "Import CLAUDE.md to AGENTS.md",
        ]
    );
    assert_eq!(rows[0], Row::Section(None));
    assert_eq!(rows.iter().filter(|row| **row == Row::Gap).count(), 1);
}

// ---- the flow on the scripted backend: `/import` asks the sources and the
// import through the shell (oracle: `flow.rs` `run_external_agent_config_migration_flow`)

mod flow {
    use codex_tui_core::rpc::ServerError;
    use serde_json::{Value, json};

    use crate::common::{Harness, ms, start};

    fn answer(request: &Value) -> Option<Result<Value, ServerError>> {
        match request["method"].as_str()? {
            "externalAgentConfig/detect" => {
                let found = request["params"]["migrationSource"] == "claude-code";
                let items = if found {
                    vec![
                        json!({"itemType":"CONFIG","description":"Import settings","cwd":null,"details":null}),
                    ]
                } else {
                    Vec::new()
                };
                Some(Ok(json!({"items": items, "connectors": []})))
            }
            "externalAgentConfig/import" => Some(Ok(json!({"importId": "import-1"}))),
            _ => None,
        }
    }

    fn methods(h: &Harness) -> Vec<String> {
        h.fake
            .requests()
            .iter()
            .filter_map(|r| r["method"].as_str().map(str::to_owned))
            .filter(|m| m.starts_with("externalAgentConfig"))
            .collect()
    }

    #[test]
    fn slash_import_asks_every_source_and_reviews_what_one_found() {
        let mut h = start();
        h.fake.respond(answer);
        h.t.type_text("/import");
        h.press("enter");
        h.wait_for("the review", |screen| screen.contains("Import setup"));
        assert_eq!(
            methods(&h),
            ["externalAgentConfig/detect", "externalAgentConfig/detect"],
            "one per source, in the oracle's order"
        );
        let sources: Vec<Value> = h
            .fake
            .requests()
            .iter()
            .filter(|r| r["method"] == "externalAgentConfig/detect")
            .map(|r| r["params"]["migrationSource"].clone())
            .collect();
        assert_eq!(sources, [json!("claude-code"), json!("cursor")]);
    }

    #[test]
    fn importing_sends_the_checked_items_and_leaves_the_page_for_the_chat() {
        let mut h = start();
        h.fake.respond(answer);
        h.t.type_text("/import");
        h.press("enter");
        h.wait_for("the review", |screen| screen.contains("Import setup"));
        h.press("enter");
        h.t.pump_for(ms(400));
        h.wait_for("the chat back", |screen| !screen.contains("Import setup"));
        let sent = sent_imports(&mut h);
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0]["params"]["migrationSource"], "claude-code");
        assert_eq!(
            sent[0]["params"]["migrationItems"][0]["description"],
            "Import settings"
        );
    }

    /// The detection finds two items in the first source.
    fn answer_two(request: &Value) -> Option<Result<Value, ServerError>> {
        match request["method"].as_str()? {
            "externalAgentConfig/detect" => {
                let items = if request["params"]["migrationSource"] == "claude-code" {
                    vec![
                        json!({"itemType":"CONFIG","description":"Import settings","cwd":null,"details":null}),
                        json!({"itemType":"MEMORY","description":"Import memory","cwd":null,"details":{"memory":["a"]}}),
                    ]
                } else {
                    Vec::new()
                };
                Some(Ok(json!({"items": items, "connectors": []})))
            }
            _ => answer(request),
        }
    }

    /// The import requests the server got: the page is gone once the flow
    /// is, the request goes out on the shell's own thread, so wait for it.
    fn sent_imports(h: &mut Harness) -> Vec<Value> {
        for _ in 0..400 {
            h.t.pump_for(ms(20));
            let sent: Vec<Value> = h
                .fake
                .requests()
                .into_iter()
                .filter(|r| r["method"] == "externalAgentConfig/import")
                .collect();
            if !sent.is_empty() {
                return sent;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Vec::new()
    }

    fn review() -> Harness {
        let mut h = start();
        h.fake.respond(answer_two);
        h.t.type_text("/import");
        h.press("enter");
        h.wait_for("the review", |screen| screen.contains("Import setup"));
        h
    }

    fn shows(h: &Harness, text: &str) -> bool {
        h.t.screen().contains(text)
    }

    #[test]
    fn the_summary_actions_go_round_with_up_and_down() {
        let mut h = review();
        h.wait_for("the first action chosen", |s| {
            s.contains("› 1. Import selected")
        });
        h.press("up");
        assert!(shows(&h, "› 3. Cancel"), "{}", h.t.screen());
        h.press("down");
        assert!(shows(&h, "› 1. Import selected"), "{}", h.t.screen());
        h.press("down");
        assert!(shows(&h, "› 2. Customize selection"), "{}", h.t.screen());
    }

    #[test]
    fn the_focus_walks_the_checklist_and_the_actions() {
        // oracle: `move_up` and `move_down`: the checklist and the one action
        // under it are one ring
        let mut h = review();
        h.press("c");
        h.wait_for("the checklist", |s| s.contains("› [x] Settings"));
        assert!(shows(&h, "  1. Review selection") && !shows(&h, "› 1. Review"));
        h.press("down");
        assert!(shows(&h, "› [x] Memory"), "{}", h.t.screen());
        h.press("up");
        h.press("up");
        assert!(
            shows(&h, "› 1. Review selection") && !shows(&h, "› [x]"),
            "{}",
            h.t.screen()
        );
        h.press("down");
        assert!(
            shows(&h, "› [x] Settings") && !shows(&h, "› 1. Review"),
            "{}",
            h.t.screen()
        );
        h.press("down");
        h.press("down");
        assert!(shows(&h, "› 1. Review selection"), "{}", h.t.screen());
        h.press("up");
        assert!(
            shows(&h, "› [x] Memory"),
            "the last entry: {}",
            h.t.screen()
        );
    }

    /// A short terminal: the checklist is the room the other lines leave.
    fn short_review() -> Harness {
        let mut h = crate::common::start_sized(false, 80, 13);
        h.fake.respond(answer_two);
        h.t.type_text("/import");
        h.press("enter");
        h.wait_for("the review", |screen| screen.contains("Import setup"));
        h
    }

    #[test]
    fn the_checklist_scrolls_to_the_cursor_in_the_room_the_other_lines_leave() {
        // oracle: `render_ref` gives the list what is left, down to one row,
        // and the window follows the highlighted item
        let mut h = short_review();
        h.press("c");
        h.wait_for("the checklist", |s| s.contains("› [x] Settings"));
        assert!(!shows(&h, "Memory"), "{}", h.t.screen());
        assert!(shows(&h, "Selected 2 of 2 items."), "{}", h.t.screen());
        h.press("down");
        assert!(shows(&h, "› [x] Memory"), "{}", h.t.screen());
        assert!(!shows(&h, "Home"), "{}", h.t.screen());
        assert!(shows(&h, "1. Review selection"), "{}", h.t.screen());
        h.press("up");
        assert!(shows(&h, "› [x] Settings"), "{}", h.t.screen());
    }

    #[test]
    fn enter_on_the_back_action_returns_to_the_summary() {
        let mut h = review();
        h.press("c");
        h.wait_for("the checklist", |s| s.contains("Choose what to import"));
        h.press("up");
        h.press("enter");
        h.wait_for("the summary", |s| s.contains("Import setup"));
    }

    #[test]
    fn enter_toggles_in_the_checklist_and_the_toggled_selection_is_imported() {
        let mut h = review();
        h.press("c");
        h.wait_for("the checklist", |s| s.contains("› [x] Settings"));
        h.press("enter");
        h.wait_for("the toggle", |s| {
            s.contains("Selected 1 of 2 items.") && s.contains("[ ] Settings")
        });
        h.press("b");
        h.wait_for("the summary", |s| s.contains("Import setup"));
        h.press("enter");
        h.wait_for("the chat", |s| !s.contains("Import setup"));
        let sent = sent_imports(&mut h);
        assert_eq!(sent.len(), 1);
        let items = sent[0]["params"]["migrationItems"]
            .as_array()
            .expect("items");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["description"], "Import memory");
    }
}
