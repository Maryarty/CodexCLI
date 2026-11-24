#![allow(clippy::expect_used, clippy::unwrap_used)]

//! What an answered request leaves in the thread's history (oracle `bottom_pane/
//! approval_overlay.rs` `handle_exec_decision`, `bottom_pane/request_user_input/mod.rs`
//! `submit_answers`, `chatwidget/tests/approval_requests.rs`): the decision record of an exec,
//! network or permissions request, none for a patch or for a request of another thread, the
//! write_stdin request's terminal, and the result of a question exchange, empty answers
//! included. The words, the 80-grapheme snippet of a multi-line command and the colours are the
//! view's (`sheets/approvals/decision_cell.rs`): these tests read the data the cell is made of.

use std::collections::BTreeMap;

use codex_app_server_protocol::ServerRequest;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::{RequestKey, ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::ScreenIntent;
use codex_tui_core::screens::approvals::decided::{Actor, Decided, Grant, Outcome, Subject};
use codex_tui_core::screens::approvals::input;
use codex_tui_core::screens::approvals::{
    Decision, Intent as ApprovalIntent, Prompt, RequestAnswer,
};
use codex_tui_core::threads::{Item, Notice};
use pretty_assertions::assert_eq;
use serde_json::json;

const T: &str = "thread-1";

fn run(app: &mut CodexApp, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, ev, &mut fx);
    app.assert_consistent();
    fx
}

fn ready() -> CodexApp {
    let mut app = CodexApp::new();
    let fx = run(&mut app, AppEv::Connected);
    let ticket: Ticket = fx
        .commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a StartThread");
    run(
        &mut app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(T))),
    );
    app
}

/// A command request of `kind` with `extra` params, as the server sends it.
fn exec(kind: &str, command: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut params = json!({
        "kind": kind, "threadId": T, "turnId": "turn-1", "itemId": "item-1",
        "startedAtMs": 0, "approvalId": "approval-1", "environmentId": null,
        "command": command, "cwd": "/tmp", "reason": null,
    });
    params
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    json!({"method": "item/commandExecution/requestApproval", "id": 1, "params": params})
}

/// The request delivered, and its key.
fn ask(app: &mut CodexApp, request: serde_json::Value) -> RequestKey {
    let request: ServerRequest = serde_json::from_value(request).expect("a request");
    run(app, AppEv::Backend(BackendEv::Request(Box::new(request))));
    app.requests()
        .entries
        .last()
        .map(|entry| entry.key.clone())
        .expect("a queued request")
}

fn answer(app: &mut CodexApp, key: &RequestKey, decision: Decision) {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
            ApprovalIntent::Answer {
                key: key.clone(),
                decision,
            },
        ))),
    );
}

fn respond(app: &mut CodexApp, key: &RequestKey, answer: RequestAnswer) {
    run(
        app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Approvals(
            ApprovalIntent::Respond {
                key: key.clone(),
                answer,
            },
        ))),
    );
}

fn decisions(app: &CodexApp) -> Vec<Decided> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Decision(decided) => Some(decided.clone()),
            _ => None,
        })
        .collect()
}

/// One answered request: the answers by question id, and whether it was cancelled.
type Answered = (Vec<(String, Vec<String>)>, bool);

fn results(app: &CodexApp) -> Vec<Answered> {
    app.active_thread()
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::Notice(Notice::Questions {
                questions,
                interrupted,
            }) => Some((
                questions
                    .iter()
                    .map(|q| (q.question.clone(), q.answers.clone()))
                    .collect(),
                *interrupted,
            )),
            _ => None,
        })
        .collect()
}

fn request_of(decided: &Decided) -> (Actor, &Subject, &Outcome) {
    match decided {
        Decided::Request {
            actor,
            subject,
            outcome,
        } => (*actor, subject, outcome),
        other => panic!("not a request's record: {other:?}"),
    }
}

#[test]
fn an_approved_command_is_recorded_with_the_command_the_shell_wrapper_stripped() {
    let mut app = ready();
    let key = ask(
        &mut app,
        exec("command", "bash -lc 'echo hello world'", json!({})),
    );
    assert!(
        decisions(&app).is_empty(),
        "nothing is recorded before the answer"
    );
    answer(&mut app, &key, Decision::Accept);
    assert_eq!(
        decisions(&app),
        vec![Decided::Request {
            actor: Actor::User,
            subject: Subject::Command("echo hello world".to_owned()),
            outcome: Outcome::Approved,
        }]
    );
}

#[test]
fn a_denied_and_a_canceled_command_are_told_apart() {
    let mut app = ready();
    for decision in [Decision::Decline, Decision::Cancel] {
        let key = ask(&mut app, exec("command", "rm -rf build", json!({})));
        answer(&mut app, &key, decision);
    }
    let all = decisions(&app);
    assert_eq!(
        all.iter()
            .map(|d| request_of(d).2.clone())
            .collect::<Vec<_>>(),
        vec![Outcome::Denied, Outcome::Aborted]
    );
}

#[test]
fn a_multi_line_command_is_kept_whole_for_the_cell_to_cut() {
    let mut app = ready();
    let command = "echo one\necho two\necho three";
    let key = ask(&mut app, exec("command", command, json!({})));
    answer(&mut app, &key, Decision::AcceptForSession);
    let all = decisions(&app);
    let (_, subject, outcome) = request_of(&all[0]);
    assert_eq!(subject, &Subject::Command(command.to_owned()));
    assert_eq!(outcome, &Outcome::ApprovedForSession);
}

#[test]
fn an_accepted_prefix_is_recorded_with_the_prefix() {
    let mut app = ready();
    let key = ask(
        &mut app,
        exec(
            "command",
            "git push",
            json!({"proposedExecpolicyAmendment": ["git", "push"]}),
        ),
    );
    answer(
        &mut app,
        &key,
        Decision::AcceptWithPrefix("git push".to_owned()),
    );
    let all = decisions(&app);
    assert_eq!(
        request_of(&all[0]).2,
        &Outcome::ApprovedWithPrefix("git push".to_owned())
    );
}

#[test]
fn a_network_request_is_recorded_by_its_target_and_the_host_rule() {
    let mut app = ready();
    let network = json!({
        "networkApprovalContext": {"host": "example.com", "protocol": "https"},
        "proposedNetworkPolicyAmendments": [{"host": "example.com", "action": "allow"}],
    });
    let key = ask(
        &mut app,
        exec(
            "command",
            "network-access https://example.com:8443",
            network.clone(),
        ),
    );
    answer(&mut app, &key, Decision::AcceptForSession);
    // the context alone names the target by its scheme and host
    let key = ask(&mut app, exec("command", "curl example.com", network));
    answer(
        &mut app,
        &key,
        Decision::NetworkRule {
            host: "example.com".to_owned(),
            allow: true,
        },
    );
    let all = decisions(&app);
    let target = |n: usize| match request_of(&all[n]).1 {
        Subject::Network { target } => target.clone(),
        other => panic!("not network: {other:?}"),
    };
    assert_eq!(target(0), "https://example.com:8443");
    assert_eq!(target(1), "https://example.com");
    assert_eq!(request_of(&all[0]).2, &Outcome::ApprovedForSession);
    assert_eq!(
        request_of(&all[1]).2,
        &Outcome::NetworkRule {
            host: "example.com".to_owned(),
            allow: true
        }
    );
}

#[test]
fn a_write_stdin_request_names_its_terminal_and_its_input() {
    let mut app = ready();
    let key = ask(
        &mut app,
        exec(
            "writeStdin",
            "write_stdin --session-id 42 'confirm\n'",
            json!({}),
        ),
    );
    let prompt = app
        .requests()
        .get(&key)
        .and_then(|entry| entry.prompt.as_deref())
        .cloned()
        .expect("a prompt");
    let Prompt::Exec(exec) = prompt else {
        panic!("not an exec prompt");
    };
    assert_eq!(exec.process.as_deref(), Some("42"));
    assert_eq!(exec.input.as_deref(), Some("confirm\n"));
    // a command request has neither
    let key = ask(&mut app, exec_plain());
    let Some(Prompt::Exec(plain)) = app
        .requests()
        .get(&key)
        .and_then(|e| e.prompt.as_deref())
        .cloned()
    else {
        panic!("not an exec prompt");
    };
    assert_eq!((plain.process, plain.input), (None, None));
}

fn exec_plain() -> serde_json::Value {
    exec("command", "ls", json!({}))
}

#[test]
fn a_permissions_answer_records_what_was_granted() {
    let mut app = ready();
    let request = json!({
        "method": "item/permissions/requestApproval", "id": 1,
        "params": {
            "threadId": T, "turnId": "turn-1", "itemId": "item-1", "startedAtMs": 0,
            "cwd": "/tmp", "reason": null, "environmentId": null,
            "permissions": {"network": {"enabled": true}},
        },
    });
    let key = ask(&mut app, request);
    answer(&mut app, &key, Decision::GrantForSession);
    assert_eq!(decisions(&app), vec![Decided::Permissions(Grant::Session)]);
}

#[test]
fn a_patch_leaves_no_record() {
    let mut app = ready();
    let request = json!({
        "method": "item/fileChange/requestApproval", "id": 1,
        "params": {
            "threadId": T, "turnId": "turn-1", "itemId": "item-1", "startedAtMs": 0,
            "reason": "tidy", "grantRoot": null,
        },
    });
    let key = ask(&mut app, request);
    answer(&mut app, &key, Decision::Accept);
    assert!(decisions(&app).is_empty());
}

#[test]
fn a_question_exchange_is_recorded_with_every_question_in_its_order() {
    let mut app = ready();
    let request = json!({
        "method": "item/tool/requestUserInput", "id": 1,
        "params": {
            "threadId": T, "turnId": "turn-1", "itemId": "item-1",
            "questions": [
                {"id": "q1", "header": "H1", "question": "First?", "isOther": false,
                 "isSecret": false, "options": [{"label": "Yes", "description": "d"}]},
                {"id": "q2", "header": "H2", "question": "Second?", "isOther": false,
                 "isSecret": false, "options": null},
            ],
        },
    });
    let key = ask(&mut app, request);
    let Some(Prompt::UserInput(asked)) = app
        .requests()
        .get(&key)
        .and_then(|e| e.prompt.as_deref())
        .cloned()
    else {
        panic!("not a user-input prompt");
    };
    assert_eq!(asked.questions.len(), 2);
    // q1 chosen with a note, q2 left empty: an empty list is no answer
    let drafts = [
        input::Draft {
            choice: Some(0),
            committed: true,
            notes: "because".to_owned(),
        },
        input::Draft::default(),
    ];
    respond(&mut app, &key, input::answer(&asked.questions, &drafts));
    assert_eq!(
        results(&app),
        vec![(
            vec![
                (
                    "First?".to_owned(),
                    vec!["Yes".to_owned(), "user_note: because".to_owned()]
                ),
                ("Second?".to_owned(), vec![]),
            ],
            false
        )]
    );
}

#[test]
fn an_empty_answer_leaves_every_question_unanswered() {
    let mut app = ready();
    let request = json!({
        "method": "item/tool/requestUserInput", "id": 1,
        "params": {
            "threadId": T, "turnId": "turn-1", "itemId": "item-1", "isBlocking": false,
            "questions": [
                {"id": "q1", "header": "H1", "question": "First?", "isOther": false,
                 "isSecret": false, "options": null},
            ],
        },
    });
    let key = ask(&mut app, request);
    respond(&mut app, &key, input::empty_answer());
    assert_eq!(
        results(&app),
        vec![(vec![("First?".to_owned(), vec![])], false)]
    );
    assert_eq!(
        input::empty_answer(),
        RequestAnswer::UserInput(BTreeMap::new())
    );
}
