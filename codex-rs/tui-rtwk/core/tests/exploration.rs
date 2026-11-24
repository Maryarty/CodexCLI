#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Exploration groups (oracle `history_cell/exec.rs`, `exec_cell/model.rs`,
//! `chatwidget/command_lifecycle.rs`): the read, list and search commands of a turn that follow
//! each other are one group (`Transcript::exploration_groups`), the reasoning between them stays
//! inside it, a message or a command of the user's shell ends it, a failed call stays in it, and
//! an interrupted turn ends its running calls as failed without touching a background terminal's
//! own command. The groups are data over the transcript: nothing is stored.

use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::threads::exploration::{ExplorationGroup, Outcome};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const T: &str = "thread-1";

fn notify(message: Value) -> String {
    format!(r#"{{"step":"notify","message":{message}}}"#)
}

fn turn(id: &str, status: &str) -> Value {
    json!({"id": id, "items": [], "itemsView": "summary", "status": status, "error": null})
}

fn started(id: &str) -> String {
    notify(
        json!({"method": "turn/started", "params": {"threadId": T, "turn": turn(id, "inProgress")}}),
    )
}

fn ended(id: &str, status: &str) -> String {
    notify(json!({"method": "turn/completed", "params": {"threadId": T, "turn": turn(id, status)}}))
}

fn command(id: &str, status: &str, exit: Option<i32>, source: &str, action: Value) -> Value {
    json!({"type": "commandExecution", "id": id, "command": format!("run {id}"), "cwd": "/tmp",
        "processId": null, "source": source, "status": status, "commandActions": [action],
        "aggregatedOutput": null, "exitCode": exit, "durationMs": null})
}

fn read(name: &str) -> Value {
    json!({"type": "read", "command": format!("cat {name}"), "name": name, "path": format!("/tmp/{name}")})
}

fn list() -> Value {
    json!({"type": "listFiles", "command": "ls", "path": null})
}

fn search() -> Value {
    json!({"type": "search", "command": "rg x", "query": "x", "path": null})
}

fn unknown() -> Value {
    json!({"type": "unknown", "command": "make"})
}

fn begin(turn: &str, item: Value) -> String {
    notify(json!({"method": "item/started", "params": {
        "threadId": T, "turnId": turn, "startedAtMs": 0, "item": item}}))
}

fn end(turn: &str, item: Value) -> String {
    notify(json!({"method": "item/completed", "params": {
        "threadId": T, "turnId": turn, "completedAtMs": 0, "startedAtMs": 0, "item": item}}))
}

fn reasoning(id: &str) -> String {
    end(
        "turn-1",
        json!({"type": "reasoning", "id": id, "summary": ["thinking"], "content": []}),
    )
}

fn message(id: &str) -> String {
    end(
        "turn-1",
        json!({"type": "agentMessage", "id": id, "text": "so", "phase": null}),
    )
}

fn groups(lines: &[String]) -> Vec<ExplorationGroup> {
    let mut script = vec![r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_string()];
    script.extend(lines.iter().cloned());
    let sandbox = Sandbox::play(&script.join("\n")).expect("plays");
    let thread = sandbox.app().active_thread();
    thread.transcript().exploration_groups()
}

fn outcomes(group: &ExplorationGroup) -> Vec<Outcome> {
    group.calls.iter().map(|call| call.outcome).collect()
}

#[test]
fn read_list_and_search_calls_are_one_group_and_a_message_ends_it() {
    let got = groups(&[
        started("turn-1"),
        end(
            "turn-1",
            command("c1", "completed", Some(0), "agent", read("a.rs")),
        ),
        end(
            "turn-1",
            command("c2", "completed", Some(0), "agent", list()),
        ),
        end(
            "turn-1",
            command("c3", "completed", Some(0), "agent", search()),
        ),
        message("m1"),
        end(
            "turn-1",
            command("c4", "completed", Some(0), "agent", read("b.rs")),
        ),
        ended("turn-1", "completed"),
    ]);
    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(outcomes(&got[0]), vec![Outcome::Ok; 3]);
    assert_eq!(outcomes(&got[1]), vec![Outcome::Ok]);
    assert!(!got[0].active());
}

#[test]
fn reasoning_stays_inside_the_group_and_keeps_its_place_between_the_calls() {
    let got = groups(&[
        started("turn-1"),
        end(
            "turn-1",
            command("c1", "completed", Some(0), "agent", read("a.rs")),
        ),
        reasoning("r1"),
        end(
            "turn-1",
            command("c2", "completed", Some(0), "agent", read("b.rs")),
        ),
        ended("turn-1", "completed"),
    ]);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0].calls.len(), 2);
    assert_eq!(got[0].details.len(), 1);
    assert_eq!(got[0].details[0].after, 1, "after the first call");
}

#[test]
fn a_user_shell_or_unknown_command_is_no_call_and_ends_the_group() {
    let got = groups(&[
        started("turn-1"),
        end(
            "turn-1",
            command("c1", "completed", Some(0), "agent", read("a.rs")),
        ),
        end(
            "turn-1",
            command("c2", "completed", Some(0), "agent", unknown()),
        ),
        end(
            "turn-1",
            command("c3", "completed", Some(0), "agent", read("b.rs")),
        ),
        end(
            "turn-1",
            command("c4", "completed", Some(0), "userShell", read("c.rs")),
        ),
        end(
            "turn-1",
            command("c5", "completed", Some(0), "agent", read("d.rs")),
        ),
        ended("turn-1", "completed"),
    ]);
    // c1 | c3 | c5: the unknown action and the user's own shell each cut the run.
    assert_eq!(got.len(), 3, "{got:?}");
}

#[test]
fn a_failed_call_stays_in_the_group_with_its_exit_code() {
    let got = groups(&[
        started("turn-1"),
        end(
            "turn-1",
            command("c1", "completed", Some(0), "agent", read("a.rs")),
        ),
        end(
            "turn-1",
            command("c2", "completed", Some(2), "agent", search()),
        ),
        ended("turn-1", "completed"),
    ]);
    assert_eq!(got.len(), 1);
    assert_eq!(
        outcomes(&got[0]),
        vec![Outcome::Ok, Outcome::Failed { exit_code: 2 }]
    );
    assert_eq!(got[0].failures().count(), 1);
}

#[test]
fn a_running_call_makes_the_group_active() {
    let got = groups(&[
        started("turn-1"),
        end(
            "turn-1",
            command("c1", "completed", Some(0), "agent", read("a.rs")),
        ),
        begin("turn-1", command("c2", "inProgress", None, "agent", list())),
    ]);
    assert_eq!(got.len(), 1);
    assert!(got[0].active(), "{got:?}");
    assert_eq!(outcomes(&got[0]), vec![Outcome::Ok, Outcome::Running]);
}

#[test]
fn an_interrupt_fails_the_running_calls_but_not_a_background_terminals_command() {
    let got = groups(&[
        started("turn-1"),
        begin(
            "turn-1",
            command("c1", "inProgress", None, "agent", read("a.rs")),
        ),
        begin(
            "turn-1",
            command("c2", "inProgress", None, "unifiedExecStartup", list()),
        ),
        ended("turn-1", "interrupted"),
    ]);
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(
        outcomes(&got[0]),
        vec![Outcome::Failed { exit_code: 1 }, Outcome::Running],
        "the agent's call failed with exit code 1; the terminal's command still runs"
    );
}

#[test]
fn groups_of_two_turns_are_two_groups() {
    let got = groups(&[
        started("turn-1"),
        end(
            "turn-1",
            command("c1", "completed", Some(0), "agent", read("a.rs")),
        ),
        ended("turn-1", "completed"),
        started("turn-2"),
        end(
            "turn-2",
            command("c2", "completed", Some(0), "agent", read("b.rs")),
        ),
        ended("turn-2", "completed"),
    ]);
    assert_eq!(got.len(), 2, "{got:?}");
}
