//! State tests of S21 (the agents command center). Oracle:
//! `shared_overview_shows_only_root_sessions` (`app/agents_overview_tests.rs:1893`)
//! for the attention of a task from its status and its waiting child.

use codex_tui_rtwk_core::app::{AppEv, Intent};
use codex_tui_rtwk_core::fixtures::sandbox::Sandbox;
use codex_tui_rtwk_core::ids::ThreadId;
use codex_tui_rtwk_core::screens::ScreenIntent;
use codex_tui_rtwk_core::screens::agents::{Attention, Filter, Intent as AgentsIntent};
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

const LISTED: &str = r#"{"step":"screen","dir":"agents","ev":{"listed":[
{"id":"r1","title":"Build the dashboard","status":"idle","model":"gpt-6.1-sol","reasoning":null,"cwd":"/tmp/project","prompt":"Build the dashboard","child":{"name":"Inspect keyboard shortcuts","waits":["approval"]}},
{"id":"r2","title":"Repair authentication","status":"active","model":null,"reasoning":null,"cwd":"/tmp/project","prompt":"Repair authentication","child":null},
{"id":"r3","title":"Review yesterday's changes","status":"not_loaded","model":null,"reasoning":null,"cwd":"/tmp/project","prompt":"Review yesterday's changes","child":null}]}}"#;

fn play() -> Sandbox {
    Sandbox::play(&format!("{SESSION}\n{}\n", LISTED.replace('\n', ""))).expect("plays")
}

#[test]
fn a_waiting_child_makes_its_task_need_you_and_status_decides_the_rest() {
    let sandbox = play();
    let model = &sandbox.app().screens.agents;
    let kinds: Vec<Attention> = model.tasks.iter().map(|task| task.attention()).collect();
    assert_eq!(
        kinds,
        [Attention::NeedsYou, Attention::Working, Attention::Inactive]
    );
    assert_eq!(model.count(Attention::NeedsYou), 1);
    assert_eq!(model.count(Attention::Ready), 0);
}

#[test]
fn the_first_task_is_highlighted_until_one_is_selected() {
    let mut sandbox = play();
    assert_eq!(
        sandbox.app().screens.agents.highlighted().expect("task").id,
        "r1"
    );
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
        AgentsIntent::Select(ThreadId::new("r2")),
    ))));
    assert_eq!(
        sandbox.app().screens.agents.highlighted().expect("task").id,
        "r2"
    );
}

#[test]
fn selecting_an_unknown_task_changes_nothing() {
    let mut sandbox = play();
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
        AgentsIntent::Select(ThreadId::new("zz")),
    ))));
    assert_eq!(sandbox.app().screens.agents.selected, None);
}

#[test]
fn a_filter_keeps_the_matching_tasks() {
    let mut sandbox = play();
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Agents(
        AgentsIntent::SetFilter(Filter::Working),
    ))));
    let titles: Vec<&str> = sandbox
        .app()
        .screens
        .agents
        .visible()
        .iter()
        .map(|task| task.title.as_str())
        .collect();
    assert_eq!(titles, ["Repair authentication"]);
}
