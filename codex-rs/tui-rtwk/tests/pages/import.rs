//! Screen tests of the `import` slot (S06): what the model's typed data is
//! told as (the labels, the lines under an item, the notices of the history,
//! the errors) and what the review's keys do (`Nav`). They carry the names of
//! the oracle's tests (`flow_tests.rs`, `mod.rs` tests, `source_tests.rs`).
//! The flow itself is `core/tests/import.rs`; the look is
//! `tests/oracle/import.rs`; its snapshots are in `tests/snapshots/import/`.

#![allow(clippy::expect_used)]

use codex_tui_core::app::AppFx;
use codex_tui_core::screens::import::{
    self as import, Failure, Intent, Mark, Model, Outcome, Phase, Source, apply, apply_intent,
};
use codex_tui_rtwk::pages::startup::import::labels;
use codex_tui_rtwk::pages::startup::import::nav::{Action, Focus, Key, Nav, View};
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

/// Keys of the review as the view sends them: through the `Nav` the view
/// holds, the intent it returns applied to the model.
fn press(model: &mut Model, nav: &mut Nav, keys: &[Key]) {
    for key in keys {
        if let Some(intent) = nav.key(model, *key) {
            apply_intent(model, intent, &mut AppFx::default());
        }
        nav.settle(model);
    }
}

// ---- the notices of the history (oracle: flow_tests.rs)

fn texts(lines: &[Line]) -> String {
    lines.iter().map(Line::text).collect::<Vec<_>>().join("\n")
}

/// What an import of `selected_items` started and what the notice finished
/// with, as the model holds them.
fn notices_of_the_flow() -> (import::Started, import::Finished) {
    let mut model = reviewing(selected_items(), None);
    apply_intent(&mut model, Intent::Proceed, &mut AppFx::default());
    let Some(import::Request::Import { request, .. }) = model.request() else {
        panic!("an import request");
    };
    apply(
        &mut model,
        import::Ev::Imported {
            request,
            result: Ok(()),
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
fn escape_skips_prompt() {
    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    press(&mut model, &mut nav, &[Key::Escape]);
    assert_eq!(model.outcome, Some(Outcome::Cancelled));
    assert_eq!(model.phase, Phase::Done);
}

#[test]
fn numeric_shortcuts_follow_visible_actions_when_proceed_is_disabled() {
    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    press(
        &mut model,
        &mut nav,
        &[Key::Customize, Key::SelectNone, Key::Back],
    );
    assert_eq!(nav.actions(&model), [Action::Customize, Action::Skip]);
    press(&mut model, &mut nav, &[Key::Number(1)]);
    assert_eq!(nav.view, View::Customize);
    assert_eq!(model.phase, Phase::Review);
}

#[test]
fn empty_selection_enter_opens_customize_instead_of_proceeding() {
    let mut model = reviewing(sample_items(), Some(Vec::new()));
    let mut nav = Nav::open(&model);
    press(&mut model, &mut nav, &[Key::Confirm]);
    assert_eq!(model.phase, Phase::Review);
    assert_eq!(nav.view, View::Customize);
}

#[test]
fn numeric_shortcuts_choose_actions() {
    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    press(&mut model, &mut nav, &[Key::Number(1)]);
    assert_eq!(model.phase, Phase::Importing);

    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    press(&mut model, &mut nav, &[Key::Number(2)]);
    assert_eq!(nav.view, View::Customize);
    press(&mut model, &mut nav, &[Key::Number(1)]);
    assert_eq!(nav.view, View::Summary);

    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    press(&mut model, &mut nav, &[Key::Number(3)]);
    assert_eq!(model.outcome, Some(Outcome::Cancelled));
}

#[test]
fn summary_does_not_toggle_selection() {
    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    let before = model.clone();
    press(&mut model, &mut nav, &[Key::Toggle]);
    assert_eq!(model, before);
}

#[test]
fn enter_proceeds_with_the_toggled_selection() {
    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    press(
        &mut model,
        &mut nav,
        &[Key::Customize, Key::Toggle, Key::Back, Key::Confirm],
    );
    assert_eq!(model.phase, Phase::Importing);
    assert_eq!(model.selected_count(), 3);
}

#[test]
fn the_focus_walks_the_items_and_the_actions() {
    // Oracle: `move_up` and `move_down`.
    let mut model = reviewing(sample_items(), None);
    let mut nav = Nav::open(&model);
    press(&mut model, &mut nav, &[Key::Customize]);
    assert_eq!((nav.focus, nav.cursor), (Focus::Items, Some(0)));
    press(&mut model, &mut nav, &[Key::Up]);
    assert_eq!((nav.focus, nav.action), (Focus::Actions, Action::Back));
    press(&mut model, &mut nav, &[Key::Down]);
    assert_eq!((nav.focus, nav.cursor), (Focus::Items, Some(0)));
    press(
        &mut model,
        &mut nav,
        &[Key::Down, Key::Down, Key::Down, Key::Down],
    );
    assert_eq!(nav.focus, Focus::Actions);
}

#[test]
fn the_checklist_and_summary_rows_are_the_oracles() {
    // Oracle: `build_summary_render_lines` and `build_customize_render_lines`
    // (the customize snapshot's text, the plugin lines, the descriptions).
    use ReviewRow as Row;
    let model = reviewing(sample_items(), None);
    let summary = rows::of(View::Summary, &model, &EN, &NUM);
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

    let rows = rows::of(View::Customize, &model, &EN, &NUM);
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
