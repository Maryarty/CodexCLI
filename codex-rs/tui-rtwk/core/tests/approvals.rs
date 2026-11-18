#![allow(clippy::expect_used, clippy::unwrap_used)]

//! State tests of S13: the request a script's `screen` step presents becomes
//! a prompt of the model with its decisions; an answer is a model fact. The
//! expected decisions are the oracle's (`bottom_pane/approval_overlay.rs`:
//! `exec_options`, `permissions_options`, `patch_options`,
//! `elicitation_options`, and `ApprovalRequest::default_available_decisions`).

use codex_tui_core::app::{AppEv, Intent};
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::ids::{RequestKey, ServerRequestId, ThreadId};
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::approvals::{Decision, Intent as ApprovalIntent, Prompt};
use pretty_assertions::assert_eq;

fn play(request: &str) -> Sandbox {
    let script = format!(
        r#"{{"step":"seed","seed":"session","thread":"thread-1"}}
{{"step":"request","message":{request}}}
{{"step":"screen","dir":"approvals","ev":{{"kind":"request","request":{request}}}}}"#
    );
    Sandbox::play(&script).expect("plays")
}

fn exec(extra: &str) -> String {
    format!(
        r#"{{"method":"item/commandExecution/requestApproval","id":1,"params":{{"kind":"command","threadId":"thread-1","turnId":"turn-1","itemId":"call-1","startedAtMs":0,"approvalId":"call-1","environmentId":null,"command":"bash -lc 'echo hello world'","cwd":"/tmp"{extra}}}}}"#
    )
}

fn prompt(sandbox: &Sandbox) -> Prompt {
    let model = &sandbox.app().screens.approvals;
    model
        .prompt(&ThreadId::new("thread-1"), &ServerRequestId::new("1"))
        .expect("a prompt")
        .clone()
}

#[test]
fn exec_prefix_option_emits_execpolicy_amendment() {
    let sandbox = play(&exec(
        r#","proposedExecpolicyAmendment":["echo","hello","world"]"#,
    ));
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptWithPrefix("echo hello world".into()),
            Decision::Cancel
        ]
    );
}

#[test]
fn the_shell_wrapper_is_stripped_from_the_command() {
    let sandbox = play(&exec(""));
    let Prompt::Exec(exec) = prompt(&sandbox) else {
        panic!("an exec prompt");
    };
    assert_eq!(exec.command, "echo hello world");
}

#[test]
fn generic_exec_options_can_offer_allow_for_session() {
    let sandbox = play(&exec(
        r#","availableDecisions":["accept","acceptForSession","cancel"]"#,
    ));
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptForSession,
            Decision::Cancel
        ]
    );
}

#[test]
fn network_exec_options_hide_execpolicy_amendment() {
    let sandbox = play(&exec(
        r#","networkApprovalContext":{"host":"example.com","protocol":"https"},"proposedExecpolicyAmendment":["curl"]"#,
    ));
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptForSession,
            Decision::Cancel
        ]
    );
}

#[test]
fn permissions_options_use_expected_decisions() {
    let sandbox = play(
        r#"{"method":"item/permissions/requestApproval","id":1,"params":{"threadId":"thread-1","turnId":"turn-1","itemId":"test","environmentId":null,"startedAtMs":0,"cwd":"/tmp","reason":"need workspace access","permissions":{"network":{"enabled":true},"fileSystem":{"read":["/tmp/readme.txt"],"write":["/tmp/out.txt"]}}}}"#,
    );
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::GrantForTurn,
            Decision::GrantForTurnStrict,
            Decision::GrantForSession,
            Decision::DenyPermissions
        ]
    );
}

#[test]
fn patch_options_use_expected_decisions() {
    let sandbox = play(
        r#"{"method":"item/fileChange/requestApproval","id":1,"params":{"threadId":"thread-1","turnId":"turn-1","itemId":"call-1","startedAtMs":0}}"#,
    );
    assert_eq!(
        prompt(&sandbox).decisions(),
        vec![
            Decision::Accept,
            Decision::AcceptForSession,
            Decision::Cancel
        ]
    );
}

#[test]
fn an_answer_is_a_model_fact_and_the_prompt_goes() {
    let mut sandbox = play(&exec(""));
    let key = RequestKey {
        epoch: sandbox.app().epoch(),
        thread: ThreadId::new("thread-1"),
        id: ServerRequestId::new("1"),
    };
    sandbox.send(AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
        ApprovalIntent::Answer {
            key,
            decision: Decision::Accept,
        },
    ))));
    let model = &sandbox.app().screens.approvals;
    assert!(
        model
            .prompt(&ThreadId::new("thread-1"), &ServerRequestId::new("1"))
            .is_none()
    );
    assert_eq!(model.answered.len(), 1);
    assert_eq!(model.answered[0].decision, Decision::Accept);
}

#[test]
fn a_bad_seed_is_an_error_with_its_line() {
    let err = Sandbox::play(
        r#"{"step":"screen","dir":"approvals","ev":{"kind":"request","request":{"method":"nope"}}}"#,
    )
    .err()
    .expect("fails");
    assert_eq!(err.line, 1);
}
