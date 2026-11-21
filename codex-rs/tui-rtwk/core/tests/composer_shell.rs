//! The user shell command of the composer (`!cmd`, oracle
//! `chatwidget/input_submission.rs` `submit_shell_command`): the page asks the
//! app with `chat::Intent::Shell`, the app asks the thread (ADR-003), the
//! thread's effect makes the `thread/shellCommand` request, a turn running or
//! not.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::{ClientRequest, RequestId};
use codex_tui_core::app::{AppEv, AppFx, CodexApp, Command, Intent, apply_app};
use codex_tui_core::fixtures::builders::turn_started;
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::ThreadId;
use codex_tui_core::protocol::client_request;
use codex_tui_core::screens::{ScreenCommand, ScreenIntent, chat};

const A: &str = "thread-1";

fn app() -> CodexApp {
    let script = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;
    build(&format!("{script}\n")).expect("plays").app
}

fn send(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn shell(command: &str) -> AppEv {
    AppEv::Intent(Intent::Screen(ScreenIntent::Chat(chat::Intent::Shell {
        thread: Some(ThreadId::new(A)),
        command: command.to_string(),
    })))
}

fn the_request(fx: &AppFx) -> ClientRequest {
    let [command @ Command::Screen { thread, .. }] = fx.commands() else {
        panic!("one thread command expected: {:?}", fx.commands());
    };
    assert_eq!(thread, &ThreadId::new(A));
    client_request(command, RequestId::Integer(7), None).expect("a request")
}

#[test]
fn a_shell_intent_makes_the_threads_shell_command_request() {
    let mut app = app();
    let fx = send(&mut app, shell("ls -la"));
    let [
        Command::Screen {
            command: ScreenCommand::Chat(chat::Command::Shell(command)),
            ..
        },
    ] = fx.commands()
    else {
        panic!("a screen command expected: {:?}", fx.commands());
    };
    assert_eq!(command, "ls -la");
    let ClientRequest::ThreadShellCommand { params, .. } = the_request(&fx) else {
        panic!("thread/shellCommand expected");
    };
    assert_eq!(params.thread_id, A);
    assert_eq!(params.command, "ls -la");
    assert_eq!(params.timeout_ms, None);
}

#[test]
fn a_shell_command_runs_while_a_turn_runs() {
    // `bang_shell_enter_while_task_running_submits_run_user_shell_command`:
    // not queued behind the turn, not steered into it.
    let mut app = app();
    send(&mut app, AppEv::Backend(turn_started(A, "turn-1")));
    let fx = send(&mut app, shell("echo hi"));
    let ClientRequest::ThreadShellCommand { params, .. } = the_request(&fx) else {
        panic!("thread/shellCommand expected");
    };
    assert_eq!(params.command, "echo hi");
}

#[test]
fn an_empty_command_asks_for_nothing() {
    let mut app = app();
    let fx = send(&mut app, shell("   "));
    assert!(fx.commands().is_empty(), "{:?}", fx.commands());
}
