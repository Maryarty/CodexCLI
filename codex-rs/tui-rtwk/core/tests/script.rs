//! The JSONL event scripts: the loader, and the oracle scenario the streamed
//! answer script is ported from. A script changes the model only through
//! `ingest`, the live app's path.

use codex_app_server_protocol::ServerNotification;
use codex_tui_core::app::{AppEv, BackendEv, CodexApp, Intent, Session};
use codex_tui_core::fixtures::builders::turn_started;
use codex_tui_core::fixtures::sandbox::Sandbox;
use codex_tui_core::fixtures::script::{Input, ScriptError, build, parse};
use codex_tui_core::ids::ThreadId;
use codex_tui_core::threads::UserMessage;
use codex_tui_core::threads::{Item, MessagePhase, TurnState};
use pretty_assertions::assert_eq;

const SESSION: &str = r#"{"step":"seed","seed":"session","thread":"thread-1"}"#;

fn items(app: &CodexApp) -> Vec<Item> {
    app.active_thread()
        .transcript()
        .iter()
        .map(|(_, item)| item.clone())
        .collect()
}

#[test]
fn blank_and_comment_lines_are_skipped() {
    let text = format!("\n# a comment\n\n{SESSION}\n   \n# another\n");
    let script = parse(&text).expect("parses");
    assert_eq!(script.steps.len(), 1);
    assert_eq!(script.steps[0].0, 4, "the step keeps its line number");
}

#[test]
fn a_parse_error_names_its_line() {
    let text = format!("{SESSION}\n# comment\n{{\"step\":\"seed\",\"seed\":\"nonsense\"}}\n");
    let err = parse(&text).expect_err("fails");
    assert_eq!(err.line, 3);
    assert!(!err.message.is_empty());
    let err = parse("not json").expect_err("fails");
    assert_eq!(err.line, 1);
}

#[test]
fn key_and_resize_steps_reach_the_caller_in_order() {
    let text = format!(
        "{SESSION}\n{}\n{}\n{}\n",
        r#"{"step":"key","keys":"down"}"#,
        r#"{"step":"resize","width":60,"height":20}"#,
        r#"{"step":"key","keys":"enter"}"#,
    );
    let played = build(&text).expect("plays");
    assert_eq!(
        played.inputs,
        vec![
            Input::Key("down".into()),
            Input::Resize {
                width: 60,
                height: 20
            },
            Input::Key("enter".into()),
        ]
    );
}

#[test]
fn a_notify_step_equals_the_protocols_own_json() {
    // The builder's notification serialized is the script line: the script
    // carries the protocol's JSON, not a format of its own.
    let BackendEv::Notification(note) = turn_started("thread-1", "turn-1") else {
        panic!("a notification");
    };
    let json = serde_json::to_string(&*note).expect("serializes");
    let line = format!(r#"{{"step":"notify","message":{json}}}"#);
    let played = build(&format!("{SESSION}\n{line}\n")).expect("plays");
    assert!(matches!(
        played.app.active_thread().turn(),
        &TurnState::Running(_)
    ));
    let parsed: ServerNotification = serde_json::from_str(&json).expect("round trip");
    assert_eq!(
        serde_json::to_value(&parsed).expect("serializes"),
        serde_json::to_value(&*note).expect("serializes")
    );
}

#[test]
fn a_session_seed_makes_a_ready_thread_through_apply() {
    let played = build(SESSION).expect("plays");
    assert_eq!(played.app.session().status, Session::Ready);
    assert_eq!(
        played.app.session().active,
        Some(ThreadId::new("thread-1")),
        "the thread of the seed is active"
    );
    assert!(played.app.epoch() != Default::default(), "attached");
}

#[test]
fn a_request_step_queues_a_request() {
    let line = r#"{"step":"request","message":{"method":"item/commandExecution/requestApproval","id":1,"params":{"kind":"command","threadId":"thread-1","turnId":"turn-1","itemId":"item-1","startedAtMs":0,"approvalId":"approval-1","environmentId":null,"command":"/bin/zsh -lc 'echo hi'","cwd":"/tmp"}}}"#;
    let played = build(&format!("{SESSION}\n{line}\n")).expect("plays");
    let head = played.app.requests().head().expect("one request");
    assert_eq!(head.method, "item/commandExecution/requestApproval");
    assert_eq!(head.key.thread, ThreadId::new("thread-1"));
}

#[test]
fn a_screen_step_goes_to_the_dirs_own_seed() {
    let line = r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":[{"id":"a","preview":"Hello","model_provider":"mock_provider","timestamp":"2025-01-02T12:00:00Z"},{"id":"b","preview":"World","model_provider":"mock_provider","timestamp":"2025-01-01T13:00:00Z"}]}}}"#;
    let played = build(line).expect("plays");
    let rows = &played.app.screens().resume.rows;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, ThreadId::new("a"));
    assert_eq!(rows[1].preview, "World");
    assert_eq!(rows[0].started, "2025-01-02T12:00:00Z");
}

#[test]
fn a_screen_step_for_an_unknown_dir_names_the_line() {
    let text = format!(
        "{SESSION}\n{}\n",
        r#"{"step":"screen","dir":"nowhere","ev":{}}"#
    );
    let Err(err): Result<_, ScriptError> = build(&text) else {
        panic!("an unknown dir fails");
    };
    assert_eq!(err.line, 2);
}

#[test]
fn a_screen_step_for_a_dir_without_seeds_fails_on_its_line() {
    let Err(err): Result<_, ScriptError> =
        build(r#"{"step":"screen","dir":"chat","ev":{"anything":1}}"#)
    else {
        panic!("a dir with no seedable events fails");
    };
    assert_eq!(err.line, 1);
}

#[test]
fn a_bad_event_in_a_screen_step_names_the_line() {
    let Err(err): Result<_, ScriptError> =
        build(r#"{"step":"screen","dir":"resume","ev":{"listed":{"rows":"no"}}}"#)
    else {
        panic!("a bad event fails");
    };
    assert_eq!(err.line, 1);
}

#[test]
fn a_sandbox_applies_an_intent_and_drops_its_effects() {
    let mut sandbox = Sandbox::play(SESSION).expect("plays");
    sandbox.send(AppEv::Intent(Intent::Submit {
        thread: Some(ThreadId::new("thread-1")),
        message: UserMessage::text("hi"),
    }));
    // The model took the submit (a ticket was minted for it); the RPC it
    // asked for went nowhere: the sandbox has no backend.
    assert!(sandbox.app().active_thread().last_submit().is_some());
}

/// Ported from `live_app_server_turn_completion_repairs_dropped_message_deltas`
/// (codex-tui `chatwidget/tests/app_server.rs`): the transport kept the first
/// delta and dropped the rest; `turn/completed` carries the whole message in
/// its summary items and the transcript is repaired from it.
///
#[test]
fn live_app_server_turn_completion_repairs_dropped_message_deltas() {
    let script = include_str!("../../src/pages/session/fixtures/streamed_answer.jsonl");
    let played = build(script).expect("plays");
    assert_eq!(
        items(&played.app),
        vec![
            Item::User(codex_tui_core::threads::UserPrompt {
                text: "hello".into(),
                ..Default::default()
            }),
            Item::Agent {
                text: "The transport kept this.\nAnd dropped this.".into(),
                // The completed turn's summary says it is the final answer (oracle replay
                // `phase: phase.clone()`).
                phase: Some(MessagePhase::FinalAnswer),
                questions: None,
            },
        ]
    );
    assert!(!played.app.active_thread().is_running(), "the turn ended");
}

#[test]
fn a_script_error_in_play_names_the_line() {
    // No session seed: the notification is for a thread the model does not
    // have, which `ingest` accepts (R9); a second session seed after a
    // failure of the first is the error: the model is already connected.
    let Err(err): Result<_, ScriptError> = build(&format!("{SESSION}\n{SESSION}\n")) else {
        panic!("a second session seed fails");
    };
    assert_eq!(err.line, 2);
}

mod replies {
    use super::{SESSION, items};
    use codex_tui_core::fixtures::builders::{history_turn, turns_page};
    use codex_tui_core::fixtures::script::{Answer, Replies, build, parse};
    use codex_tui_core::threads::Item;
    use pretty_assertions::assert_eq;
    use serde_json::json;

    fn line(method: &str, result: &serde_json::Value) -> String {
        json!({"step": "reply", "method": method, "result": result}).to_string()
    }

    // The n-th request of a method gets the n-th reply of it; other methods have their own queue.
    #[test]
    fn replies_queue_by_method_in_script_order() {
        let text = format!(
            "{SESSION}\n{}\n{}\n{}\n",
            line("thread/turns/list", &json!({"page": 1})),
            line("mcpServerStatus/list", &json!({"data": []})),
            line("thread/turns/list", &json!({"page": 2})),
        );
        let mut replies = parse(&text).expect("parses").replies;
        assert_eq!(replies.waiting("thread/turns/list"), 2);
        assert_eq!(
            replies.next("thread/turns/list"),
            Some(Answer::Result(json!({"page": 1})))
        );
        assert_eq!(
            replies.next("thread/turns/list"),
            Some(Answer::Result(json!({"page": 2})))
        );
        assert_eq!(replies.next("thread/turns/list"), None, "none left");
        assert_eq!(replies.waiting("mcpServerStatus/list"), 1);
        assert_eq!(replies.next("account/read"), None, "unknown method");
    }

    // A reply is not an event of the timeline: it is not a step and play ignores it.
    #[test]
    fn a_reply_is_not_a_step_and_changes_no_model_state() {
        let text = format!("{SESSION}\n{}\n", line("thread/turns/list", &json!({})));
        let script = parse(&text).expect("parses");
        assert_eq!(script.steps.len(), 1);
        let played = build(&text).expect("plays");
        assert_eq!(items(&played.app), Vec::<Item>::new());
    }

    #[test]
    fn a_reply_may_refuse() {
        let text = r#"{"step":"reply","method":"thread/turns/list","error":{"code":-32600,"message":"gone"}}"#;
        let mut replies = parse(text).expect("parses").replies;
        assert_eq!(
            replies.next("thread/turns/list"),
            Some(Answer::Error {
                code: Some(-32600),
                message: "gone".into(),
                data: None
            })
        );
        let bare = r#"{"step":"reply","method":"m","error":{"message":"x"}}"#;
        assert_eq!(
            parse(bare).expect("parses").replies.next("m"),
            Some(Answer::Error {
                code: None,
                message: "x".into(),
                data: None
            })
        );
    }

    #[test]
    fn a_reply_has_exactly_one_of_result_and_error() {
        let neither = r#"{"step":"reply","method":"m"}"#;
        assert_eq!(parse(neither).expect_err("fails").line, 1);
        let both = r#"{"step":"reply","method":"m","result":{},"error":{"message":"x"}}"#;
        let err = parse(both).expect_err("fails");
        assert!(err.message.contains("exactly one"), "{}", err.message);
        let no_method = r#"{"step":"reply","result":{}}"#;
        assert_eq!(parse(no_method).expect_err("fails").line, 1);
    }

    #[test]
    fn the_builders_make_a_page_the_protocol_reads_back() {
        let page = turns_page(
            vec![
                history_turn("t1", "hello", "hi"),
                history_turn("t2", "more", "ok"),
            ],
            Some("older"),
            Some("newer"),
        );
        let read: codex_app_server_protocol::ThreadTurnsListResponse =
            serde_json::from_value(page.clone()).expect("a response");
        assert_eq!(read.data.len(), 2);
        assert_eq!(read.data[0].items.len(), 2);
        assert_eq!(read.next_cursor.as_deref(), Some("older"));
        let mut replies = Replies::default();
        replies.push("thread/turns/list", Answer::Result(page));
        assert!(!replies.is_empty());
    }
}
