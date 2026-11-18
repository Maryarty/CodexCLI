//! State tests of S21 (the Agent command center), ported from the oracle
//! (`codex-rs/tui/src/app/agents_overview_tests.rs`, `agent_center_tests.rs`,
//! `agents_overview_actions_tests.rs`, codex-tui 0.170.0). The threads are the
//! protocol's `Thread`, built like the oracle's `overview_thread`; the view's
//! rows are `Model::center()`. Expected values are the oracle's assertions.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::screens::agents::{
    Ev, Group, Grouping, Intent as AgentsIntent, Model, Page, Sel,
};
use codex_tui_core::screens::{ScreenEv, ScreenIntent};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
const TIMESTAMP: i64 = 1_700_000_000;

fn id(n: u128) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

/// The oracle's `overview_thread` (`agents_overview_tests.rs:474`): a root
/// named `name`, or a child of `parent` (the source says so), in `/tmp/project`.
fn thread(id: &str, parent: Option<&str>, name: &str, status: Value) -> Thread {
    let source = parent.map_or(json!("cli"), |parent| {
        json!({"subAgent": {"thread_spawn": {"parent_thread_id": parent, "depth": 1,
            "agent_path": null, "agent_nickname": null, "agent_role": null}}})
    });
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": parent.unwrap_or(id),
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": TIMESTAMP,
        "updatedAt": TIMESTAMP, "recencyAt": TIMESTAMP, "status": status, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": source,
        "canAcceptDirectInput": parent.is_none(), "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn idle() -> Value {
    json!({"type": "idle"})
}
fn working() -> Value {
    json!({"type": "active", "activeFlags": []})
}
fn waiting_approval() -> Value {
    json!({"type": "active", "activeFlags": ["waitingOnApproval"]})
}

fn sandbox() -> Sandbox {
    Sandbox::play(&format!("{SESSION}\n")).expect("plays")
}

fn list(sandbox: &mut Sandbox, threads: Vec<Thread>) {
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Listed(Page {
        threads,
        unread: Vec::new(),
        next_cursor: None,
        append: false,
    }))));
}

fn current(sandbox: &mut Sandbox, thread: &str) {
    sandbox.send(AppEv::Screen(ScreenEv::Agents(Ev::Current(Some(
        thread.to_owned(),
    )))));
}

fn intent(sandbox: &mut Sandbox, intent: AgentsIntent) {
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Agents(intent))));
}

fn model(sandbox: &Sandbox) -> &Model {
    &sandbox.app().screens.agents
}

fn ids(sandbox: &Sandbox) -> Vec<String> {
    model(sandbox)
        .center()
        .rows
        .iter()
        .map(|row| row.id().to_owned())
        .collect()
}

fn selected(sandbox: &Sandbox) -> String {
    model(sandbox)
        .center()
        .selected_row()
        .expect("a highlighted task")
        .id()
        .to_owned()
}

/// `shared_overview_shows_only_root_sessions` (agents_overview_tests.rs:1588):
/// children and ephemeral side threads are not rows; the first root, whose
/// child waits for approval, needs the user; counts per tab; the default
/// grouping is the project.
#[test]
fn shared_overview_shows_only_root_sessions() {
    let (first, child, second, unloaded, side) = (id(101), id(1), id(2), id(102), id(3));
    let mut side_thread = thread(&side, None, "Ephemeral side", idle());
    side_thread.ephemeral = true;
    let mut sandbox = sandbox();
    current(&mut sandbox, &first);
    list(
        &mut sandbox,
        vec![
            thread(&first, None, "Build the dashboard", idle()),
            thread(
                &child,
                Some(&first),
                "Inspect keyboard shortcuts",
                waiting_approval(),
            ),
            thread(&second, None, "Repair authentication", working()),
            thread(
                &unloaded,
                None,
                "Review yesterday's changes",
                json!({"type": "notLoaded"}),
            ),
            side_thread,
        ],
    );
    assert_eq!(model(&sandbox).grouping, Grouping::Project);
    let center = model(&sandbox).center();
    assert_eq!(center.tab_counts, [3, 1, 1, 0, 1]);
    let mut got = ids(&sandbox);
    got.sort();
    let mut want = [first.clone(), unloaded, second.clone()];
    want.sort();
    assert_eq!(got, want);
    assert_eq!(center.rows[0].id(), first);
    assert_eq!(center.rows[0].group, Group::NeedsYou);
    // Search for a task, accept: the highlighted task is that one.
    intent(&mut sandbox, AgentsIntent::BeginSearch);
    intent(
        &mut sandbox,
        AgentsIntent::Type("Repair authentication".into()),
    );
    assert_eq!(selected(&sandbox), second);
}

/// `overview_model_grouping_shows_details_and_preserves_selection`
/// (agents_overview_tests.rs:1531): `g` three times groups by model, the
/// selection stays, newer tasks come first inside a model, navigation
/// crosses the model groups.
#[test]
fn overview_model_grouping_shows_details_and_preserves_selection() {
    let mut sandbox = sandbox();
    let threads = [
        ("Older task", Some("model-a"), 1),
        ("Other model", Some("model-b"), 2),
        ("Recent task", Some("model-a"), 3),
        ("Legacy task", None, 4),
    ]
    .map(|(name, model, index)| {
        let mut thread = thread(&id(index), None, name, idle());
        thread.model = model.map(str::to_owned);
        thread.updated_at = TIMESTAMP + index as i64 - 30;
        thread
    });
    list(&mut sandbox, threads.to_vec());
    intent(&mut sandbox, AgentsIntent::Select(Sel::Task(id(1))));
    for _ in 0..3 {
        intent(&mut sandbox, AgentsIntent::CycleGrouping);
    }
    assert_eq!(model(&sandbox).grouping, Grouping::Model);
    assert_eq!(selected(&sandbox), id(1));
    for (forward, expected) in [(false, 3), (true, 1), (true, 2), (true, 4)] {
        intent(&mut sandbox, AgentsIntent::Move { forward });
        assert_eq!(selected(&sandbox), id(expected));
    }
}

/// `live_center_sorts_arrivals_and_status_transitions_by_update_time`
/// (agent_center_tests.rs:151).
#[test]
fn live_center_sorts_arrivals_and_status_transitions_by_update_time() {
    let [alpha, bravo, charlie, delta, child_id] = [1, 2, 3, 4, 5].map(id);
    let mut threads = vec![
        thread(&alpha, None, "Alpha", working()),
        thread(&bravo, None, "Bravo", working()),
        thread(&charlie, None, "Charlie", idle()),
    ];
    let mut sandbox = sandbox();
    current(&mut sandbox, &bravo);
    list(&mut sandbox, threads.clone());
    assert_ids(&sandbox, &[&alpha, &bravo, &charlie]);
    let mut arrival = thread(&delta, None, "Delta", working());
    arrival.updated_at += 100;
    threads.push(arrival);
    let mut child = thread(&child_id, None, "Child", json!({"type": "systemError"}));
    child.parent_thread_id = Some(bravo.clone());
    threads.push(child);
    list(&mut sandbox, threads.clone());
    assert_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
    threads[1] = {
        let mut bravo_thread = threads[1].clone();
        bravo_thread.status = serde_json::from_value(idle()).unwrap();
        bravo_thread
    };
    list(&mut sandbox, threads.clone());
    assert_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
    let center = model(&sandbox).center();
    assert_eq!(center.selected_row().unwrap().group, Group::Ready);
    assert!(center.selected_row().unwrap().details.lines.is_empty());
    // A request from a descendant can still bring its idle parent to the top.
    threads.last_mut().unwrap().status = serde_json::from_value(waiting_approval()).unwrap();
    list(&mut sandbox, threads.clone());
    assert_ids(&sandbox, &[&bravo, &delta, &alpha, &charlie]);
    threads.pop();
    list(&mut sandbox, threads.clone());
    assert_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
    assert_eq!(selected(&sandbox), bravo);
    // Rediscovery restores the task to its position by update time.
    let removed = threads.remove(0);
    list(&mut sandbox, threads.clone());
    assert_ids(&sandbox, &[&delta, &bravo, &charlie]);
    threads.push(removed);
    list(&mut sandbox, threads);
    assert_ids(&sandbox, &[&delta, &alpha, &bravo, &charlie]);
}

/// `external_removals_preserve_adjacent_selection`
/// (agents_overview_actions_tests.rs:447), the project grouping, unfiltered:
/// hiding the highlighted task moves the highlight to the next displayed one,
/// the last one to the one before.
#[test]
fn external_removals_preserve_adjacent_selection() {
    let mut sandbox = sandbox();
    let threads = (0..4)
        .map(|index| {
            let mut thread = thread(&id(index + 1), None, "Task", idle());
            thread.updated_at = index as i64;
            thread
        })
        .collect();
    list(&mut sandbox, threads);
    let order = ids(&sandbox);
    intent(
        &mut sandbox,
        AgentsIntent::Select(Sel::Task(order[1].clone())),
    );
    intent(&mut sandbox, AgentsIntent::Hide);
    assert_eq!(selected(&sandbox), order[2]);
    intent(
        &mut sandbox,
        AgentsIntent::Select(Sel::Task(order[3].clone())),
    );
    intent(&mut sandbox, AgentsIntent::Hide);
    assert_eq!(selected(&sandbox), order[2]);
}

fn assert_ids(sandbox: &Sandbox, expected: &[&String]) {
    let expected: Vec<&str> = expected.iter().map(|id| id.as_str()).collect();
    let got = ids(sandbox);
    assert_eq!(got.iter().map(String::as_str).collect::<Vec<_>>(), expected);
}
