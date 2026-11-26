#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Unified exec, the thread's side and `/ps` (oracle `chatwidget/command_lifecycle.rs`,
//! `chatwidget/exec_state.rs`, `bottom_pane/unified_exec_footer.rs`, `chatwidget.rs`
//! `add_ps_output`): a unified-exec command that runs is a background terminal with its last
//! output lines; an empty poll starts the wait streak, which a completed turn writes as one
//! interaction entry and an interrupted turn drops; the terminals outlive an interrupt; `/ps`
//! says them as a note of the thread. The words and the cell are the view's.

use codex_app_server_protocol::Thread;
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::commands::SlashCommand;
use codex_tui_core::features::processes::{Intent as PsIntent, Note as PsNote};
use codex_tui_core::features::{self, FeatureIntent, FeatureNote, FeatureTarget};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{ThreadId, Ticket};
use codex_tui_core::rpc::{Reply, RpcError};
use codex_tui_core::screens::{ScreenCommand, ScreenEv, ScreenReply, agents, chat};
use codex_tui_core::threads::{Item, Listed, Note, NoteKey, Notice};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};

const T: &str = "thread-1";

fn notify(message: Value) -> String {
    format!(r#"{{"step":"notify","message":{message}}}"#)
}

fn turn(status: &str) -> Value {
    json!({"id": "turn-1", "items": [], "itemsView": "summary", "status": status, "error": null})
}

fn started() -> String {
    notify(json!({"method": "turn/started", "params": {"threadId": T, "turn": turn("inProgress")}}))
}

fn ended(status: &str) -> String {
    notify(json!({"method": "turn/completed", "params": {"threadId": T, "turn": turn(status)}}))
}

fn terminal(id: &str, process: &str, status: &str) -> Value {
    json!({"type": "commandExecution", "id": id, "command": "/bin/bash -lc 'npm run dev'",
        "cwd": "/tmp", "processId": process, "source": "unifiedExecStartup", "status": status,
        "commandActions": [], "aggregatedOutput": null, "exitCode": null, "durationMs": null})
}

fn begin(item: Value) -> String {
    notify(json!({"method": "item/started", "params": {
        "threadId": T, "turnId": "turn-1", "startedAtMs": 0, "item": item}}))
}

fn end(item: Value) -> String {
    notify(json!({"method": "item/completed", "params": {
        "threadId": T, "turnId": "turn-1", "completedAtMs": 0, "startedAtMs": 0, "item": item}}))
}

fn output(item: &str, delta: &str) -> String {
    notify(
        json!({"method": "item/commandExecution/outputDelta", "params": {
        "threadId": T, "turnId": "turn-1", "itemId": item, "delta": delta}}),
    )
}

fn poll(process: &str, stdin: &str) -> String {
    notify(
        json!({"method": "item/commandExecution/terminalInteraction", "params": {
        "threadId": T, "turnId": "turn-1", "itemId": "c1", "processId": process, "stdin": stdin}}),
    )
}

fn play(lines: &[String]) -> Sandbox {
    let mut script = vec![r#"{"step":"seed","seed":"session","thread":"thread-1"}"#.to_string()];
    script.extend(lines.iter().cloned());
    Sandbox::play(&script.join("\n")).expect("plays")
}

fn interactions(sandbox: &Sandbox) -> Vec<(Option<String>, String)> {
    sandbox
        .app()
        .active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Notice(Notice::TerminalInteraction { command, stdin }) => {
                Some((command.clone(), stdin.clone()))
            }
            _ => None,
        })
        .collect()
}

/// What the UI half's `open` says for `/ps`: the verb, for the thread in use.
fn ps(sandbox: &mut Sandbox) {
    sandbox.send(AppEv::Intent(Intent::Feature(FeatureIntent::Processes(
        PsIntent::Show {
            source: Some(ThreadId::new(T)),
        },
    ))));
}

#[test]
fn the_slash_command_is_the_features_through_opens() {
    assert!(matches!(
        features::target_of(SlashCommand::Ps),
        Some(FeatureTarget::Processes(_))
    ));
}

fn ps_notes(sandbox: &Sandbox) -> Vec<PsNote> {
    sandbox
        .app()
        .active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Note(Note {
                key: NoteKey::Feature(FeatureNote::Processes(note)),
                ..
            }) => Some(note.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_running_unified_exec_command_is_a_terminal_with_its_last_output_lines() {
    let sandbox = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        output("c1", "one\ntwo\n\nthree\nfour\n"),
    ]);
    let thread = sandbox.app().active_thread();
    let terminals = thread.terminals();
    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0].key, "p1");
    assert_eq!(
        terminals[0].command, "npm run dev",
        "the shell wrapper is stripped"
    );
    assert_eq!(
        terminals[0].recent,
        vec!["two", "three", "four"],
        "the last three"
    );
}

#[test]
fn the_terminal_is_gone_when_its_command_ends_and_stays_over_an_interrupt() {
    let ended_terminal = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        end(terminal("c1", "p1", "completed")),
    ]);
    assert!(ended_terminal.app().active_thread().terminals().is_empty());

    let interrupted = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        ended("interrupted"),
    ]);
    assert_eq!(interrupted.app().active_thread().terminals().len(), 1);
}

/// The transcript's tool entries (what the screen draws as `Ran` / `Explored` cells).
fn tools(sandbox: &Sandbox) -> usize {
    sandbox
        .app()
        .active_thread()
        .transcript()
        .iter()
        .filter(|(_, item)| matches!(item, Item::Tool(_)))
        .count()
}

/// A startup command whose command line parses to a known action (`cat file`).
fn parsed_terminal(id: &str, process: &str, status: &str) -> Value {
    let mut item = terminal(id, process, status);
    item["command"] = json!("cat notes.txt");
    item["commandActions"] = json!([{"type": "read", "command": "cat notes.txt", "name": "notes.txt", "path": "/tmp/notes.txt"}]);
    item
}

#[test]
fn a_running_startup_command_draws_no_entry_and_its_end_in_the_turn_draws_one() {
    // oracle `on_command_execution_started`: a unified-exec startup command that parses to no
    // known action draws nothing while it runs (the footer and `/ps` say it); its end in a
    // running turn is the one `Ran` entry (`on_command_execution_completed`).
    let running = play(&[started(), begin(terminal("c1", "p1", "inProgress"))]);
    assert_eq!(tools(&running), 0);
    assert_eq!(running.app().active_thread().terminals().len(), 1);

    let done = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        end(terminal("c1", "p1", "completed")),
    ]);
    assert_eq!(tools(&done), 1);
    assert!(done.app().active_thread().terminals().is_empty());
}

#[test]
fn the_end_of_a_startup_command_after_its_turn_draws_no_entry() {
    // oracle `on_command_execution_completed`: the task is not running, so the late end only
    // settles the process.
    let sandbox = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        ended("completed"),
        end(terminal("c1", "p1", "completed")),
    ]);
    assert_eq!(tools(&sandbox), 0);
    assert!(sandbox.app().active_thread().terminals().is_empty());
}

#[test]
fn a_startup_command_that_parses_to_an_action_still_draws() {
    let sandbox = play(&[started(), begin(parsed_terminal("c1", "p1", "inProgress"))]);
    assert_eq!(tools(&sandbox), 1);
    assert_eq!(sandbox.app().active_thread().terminals().len(), 1);
}

#[test]
fn empty_polls_of_one_process_are_one_streak_that_a_completed_turn_writes() {
    let mut script = vec![started(), begin(terminal("c1", "p1", "inProgress"))];
    script.extend([poll("p1", ""), poll("p1", "")]);
    let live = play(&script);
    let thread = live.app().active_thread();
    let streak = thread.terminal_wait().expect("a streak");
    assert_eq!(streak.process, "p1");
    assert_eq!(streak.command.as_deref(), Some("npm run dev"));
    assert!(
        interactions(&live).is_empty(),
        "nothing written while it goes on"
    );

    script.push(ended("completed"));
    let done = play(&script);
    assert_eq!(
        interactions(&done),
        vec![(Some("npm run dev".to_owned()), String::new())],
        "one entry for the two polls"
    );
    assert!(done.app().active_thread().terminal_wait().is_none());
}

#[test]
fn an_interrupted_turn_drops_the_streak() {
    let sandbox = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        poll("p1", ""),
        ended("interrupted"),
    ]);
    assert!(interactions(&sandbox).is_empty());
    assert!(sandbox.app().active_thread().terminal_wait().is_none());
}

#[test]
fn a_write_to_the_terminal_settles_the_streak_first_and_is_its_own_entry() {
    let sandbox = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        poll("p1", ""),
        poll("p1", "yes\n"),
    ]);
    assert_eq!(
        interactions(&sandbox),
        vec![
            (Some("npm run dev".to_owned()), String::new()),
            (Some("npm run dev".to_owned()), "yes\n".to_owned()),
        ]
    );
}

#[test]
fn a_poll_of_an_unknown_process_is_nothing() {
    let sandbox = play(&[started(), poll("nope", "")]);
    assert!(sandbox.app().active_thread().terminal_wait().is_none());
    assert!(interactions(&sandbox).is_empty());
}

#[test]
fn ps_says_the_terminals_and_none_when_there_are_none() {
    let mut sandbox = play(&[
        started(),
        begin(terminal("c1", "p1", "inProgress")),
        output("c1", "listening\n"),
    ]);
    ps(&mut sandbox);
    let notes = ps_notes(&sandbox);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].terminals.len(), 1);
    assert_eq!(notes[0].terminals[0].command, "npm run dev");
    assert_eq!(notes[0].terminals[0].recent, vec!["listening"]);

    let mut empty = play(&[]);
    ps(&mut empty);
    assert_eq!(
        ps_notes(&empty),
        vec![PsNote {
            terminals: Vec::new()
        }]
    );
}

// ---- the restore of a resumed thread's terminals (oracle `restore_background_terminals`)

fn listed(item: &str, process: &str, command: &str) -> Listed {
    Listed {
        item: item.to_owned(),
        process: process.to_owned(),
        command: command.to_owned(),
    }
}

fn dispatch(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn server_thread() -> Thread {
    serde_json::from_value(json!({
        "id": T, "environments": null, "extra": null, "sessionId": T,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "old", "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000, "status": {"type": "idle"},
        "path": null, "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null,
        "source": "cli", "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": "old",
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

/// The ticket and cursor of the terminal list `fx` asks for.
fn list_ask(fx: &AppFx) -> Option<(Ticket, Option<String>)> {
    fx.commands().iter().find_map(|command| match command {
        Command::Screen {
            ticket,
            command: ScreenCommand::Chat(chat::Command::ListTerminals { cursor }),
            ..
        } => Some((*ticket, cursor.clone())),
        _ => None,
    })
}

/// The app after the resume of `thread-1` was answered, and the effects of that answer.
fn resumed() -> (CodexApp, AppFx) {
    let mut app = CodexApp::new();
    dispatch(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Listed(agents::Page {
            threads: vec![server_thread()],
            unread: vec![],
            next_cursor: None,
            append: false,
            pins: None,
        }))),
    );
    let fx = dispatch(
        &mut app,
        AppEv::Intent(Intent::OpenThread(ThreadId::new(T))),
    );
    let ticket = fx
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::Screen {
                ticket,
                command: ScreenCommand::Agents(_),
                ..
            } => Some(*ticket),
            _ => None,
        })
        .expect("a resume request");
    let fx = dispatch(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(agents::Reply::Resumed(Box::new(
                server_thread(),
            )))),
        ),
    );
    (app, fx)
}

fn page(ticket: Ticket, data: Vec<Listed>, next: Option<&str>) -> AppEv {
    AppEv::Reply(
        ticket,
        Reply::screen(ScreenReply::Chat(chat::Reply::Terminals {
            data,
            next: next.map(str::to_owned),
        })),
    )
}

fn commands_of(app: &CodexApp) -> Vec<String> {
    app.active_thread()
        .terminals()
        .iter()
        .map(|terminal| terminal.command.clone())
        .collect()
}

#[test]
fn a_resumed_thread_asks_for_its_live_terminals() {
    let (mut app, fx) = resumed();
    let (ticket, cursor) = list_ask(&fx).expect("the resume asks for the terminal list");
    assert_eq!(cursor, None);
    assert_eq!(commands_of(&app), Vec::<String>::new());
    dispatch(
        &mut app,
        page(ticket, vec![listed("c1", "43", "sleep 200")], None),
    );
    assert_eq!(commands_of(&app), ["sleep 200"]);
}

#[test]
fn a_failed_listing_leaves_no_error() {
    let (mut app, fx) = resumed();
    let (ticket, _) = list_ask(&fx).expect("the list is asked");
    dispatch(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::error(RpcError::Remote("no such thread".to_owned())),
        ),
    );
    assert_eq!(commands_of(&app), Vec::<String>::new());
    assert_eq!(app.active_thread().last_error(), None);
}

#[test]
fn a_listing_with_a_next_cursor_asks_again() {
    let (mut app, fx) = resumed();
    let (ticket, _) = list_ask(&fx).expect("the list is asked");
    let fx = dispatch(
        &mut app,
        page(
            ticket,
            vec![listed("c1", "43", "sleep 200")],
            Some("page-2"),
        ),
    );
    assert_eq!(commands_of(&app), Vec::<String>::new());
    let (ticket, cursor) = list_ask(&fx).expect("the next page is asked");
    assert_eq!(cursor.as_deref(), Some("page-2"));
    dispatch(
        &mut app,
        page(ticket, vec![listed("c2", "44", "sleep 300")], None),
    );
    assert_eq!(commands_of(&app), ["sleep 200", "sleep 300"]);
}

#[test]
fn a_terminal_listing_is_the_servers_request() {
    let request = chat::wire(
        &chat::Command::ListTerminals {
            cursor: Some("page-2".to_owned()),
        },
        &ThreadId::new(T),
        codex_app_server_protocol::RequestId::Integer(1),
    );
    let value = serde_json::to_value(&request).expect("a request");
    assert_eq!(value["method"], "thread/backgroundTerminals/list");
    assert_eq!(value["params"]["threadId"], T);
    assert_eq!(value["params"]["cursor"], "page-2");
}
