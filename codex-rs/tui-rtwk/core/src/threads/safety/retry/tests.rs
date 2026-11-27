#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use codex_app_server_protocol::ThreadItemEntry;
use serde_json::json;

use super::*;
use crate::ids::{BackendId, ConnEpoch};
use crate::threads::LocalImage;

fn thread(id: &str, mode: ThreadHistoryMode, parent: Option<&str>) -> Thread {
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": parent, "attachedToThreadId": null, "parentThreadId": null,
        "preview": id, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": mode, "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1, "updatedAt": 1,
        "recencyAt": 1, "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null, "agentNickname": null,
        "agentRole": null, "gitInfo": null, "name": id, "daybreakEnabled": null, "turns": []
    }))
    .expect("thread fixture")
}

fn attempt(message: UserMessage) -> AwaitingInterrupt {
    AwaitingInterrupt::new(
        Retry {
            thread: ThreadId::new("source"),
            turn: TurnId::new("checked"),
            model: "faster".to_owned(),
            message,
        },
        Address {
            backend: BackendId(2),
            epoch: ConnEpoch(3),
        },
        Ticket::new(1),
    )
}

fn interrupted(
    message: UserMessage,
    first: InterruptSignal,
    second: InterruptSignal,
) -> ReadSource {
    let Interruption::Waiting(waiting) = attempt(message).observe(first) else {
        panic!("the first receipt cannot advance to history")
    };
    let Interruption::Waiting(waiting) = waiting.observe(first) else {
        panic!("a duplicate receipt cannot advance to history")
    };
    let Interruption::Ready(read) = waiting.observe(second) else {
        panic!("both receipts allow the history check")
    };
    read
}

fn turns(mode: ThreadHistoryMode, message: UserMessage) -> ReadTurns {
    interrupted(
        message,
        InterruptSignal::Acknowledged,
        InterruptSignal::TurnInterrupted,
    )
    .verify(&thread("source", mode, None))
    .expect("source")
}

fn turn(id: &str, status: TurnStatus, items: Vec<ThreadItem>) -> Turn {
    Turn {
        id: id.to_owned(),
        root_turn_id: None,
        items,
        items_view: TurnItemsView::Full,
        status,
        error: None,
        started_at: None,
        completed_at: None,
        duration_ms: None,
    }
}

fn page(data: Vec<Turn>, next: Option<&str>) -> ThreadTurnsListResponse {
    ThreadTurnsListResponse {
        data,
        next_cursor: next.map(str::to_owned),
        backwards_cursor: None,
    }
}

fn item(id: &str, text: &str) -> ThreadItem {
    serde_json::from_value(json!({
        "type": "userMessage", "id": id,
        "content": [{"type": "text", "text": text, "textElements": []}]
    }))
    .expect("user input")
}

fn items(message: UserMessage) -> ReadItems {
    let history = turns(ThreadHistoryMode::Paginated, message)
        .verify(page(
            vec![turn("checked", TurnStatus::Interrupted, vec![])],
            None,
        ))
        .expect("terminal turn");
    let History::NeedsItems(items) = history else {
        panic!("paged input")
    };
    items
}

fn items_page(turn: &str, item: ThreadItem, next: Option<&str>) -> ThreadItemsListResponse {
    ThreadItemsListResponse {
        data: vec![ThreadItemEntry {
            turn_id: turn.to_owned(),
            item,
            output_deferred: false,
            started_at_ms: None,
            completed_at_ms: None,
        }],
        next_cursor: next.map(str::to_owned),
        backwards_cursor: None,
    }
}

#[test]
fn only_the_terminal_latest_turn_with_a_known_predecessor_can_fork() {
    for (data, next, wanted) in [
        (vec![], None, Invalid::MissingTurn),
        (
            vec![
                turn("later", TurnStatus::Completed, vec![]),
                turn("checked", TurnStatus::Interrupted, vec![]),
            ],
            None,
            Invalid::NotLatest,
        ),
        (
            vec![turn("checked", TurnStatus::InProgress, vec![])],
            None,
            Invalid::TurnRunning,
        ),
        (
            vec![
                turn("checked", TurnStatus::Interrupted, vec![]),
                turn("previous", TurnStatus::InProgress, vec![]),
            ],
            None,
            Invalid::PredecessorRunning,
        ),
        (
            vec![turn("checked", TurnStatus::Interrupted, vec![])],
            Some("older"),
            Invalid::IncompleteTurns,
        ),
    ] {
        let history = turns(ThreadHistoryMode::Legacy, UserMessage::text("original"));
        assert!(matches!(history.verify(page(data,next)), Err(problem) if problem == wanted));
    }
}

#[test]
fn paged_input_must_be_complete_and_belong_to_the_checked_turn() {
    let message = UserMessage::text("original");
    let history = items(message.clone());
    let params = history.params();
    assert_eq!(params.thread_id, "source");
    assert_eq!(params.turn_id.as_deref(), Some("checked"));
    assert_eq!(params.limit, Some(100));
    assert_eq!(params.sort_direction, Some(SortDirection::Asc));
    assert!(matches!(
        history.verify(items_page(
            "checked",
            item("p", "original"),
            Some("overflow")
        )),
        Err(Invalid::IncompleteInput)
    ));
    assert!(matches!(
        items(message).verify(items_page("other", item("p", "original"), None)),
        Err(Invalid::WrongTurn)
    ));
}

#[test]
fn verified_fork_and_start_preserve_input_and_inherit_permissions() {
    let message = UserMessage {
        images: vec![LocalImage {
            path: PathBuf::from("/tmp/diagram.png"),
            range: 0..10,
        }],
        remote_images: vec!["https://example.test/diagram.png".to_owned()],
        ..UserMessage::text("[Image #1] Explain this diagram.")
    };
    let read = turns(ThreadHistoryMode::Legacy, message.clone());
    let params = read.params();
    assert_eq!(params.limit, Some(5));
    assert_eq!(params.sort_direction, Some(SortDirection::Desc));
    let saved = vec![
        item("original-item", "already captured"),
        item("steer", "Keep it brief."),
    ];
    let History::Complete(fork) = read
        .verify(page(
            vec![
                turn("checked", TurnStatus::Interrupted, saved),
                turn("previous", TurnStatus::Completed, vec![]),
            ],
            Some("older"),
        ))
        .expect("verified history")
    else {
        panic!("legacy input")
    };
    let params = fork.params();
    assert_eq!(params.thread_id, "source");
    assert_eq!(params.before_turn_id.as_deref(), Some("checked"));
    assert!(params.last_turn_id.is_none());
    assert!(params.defer_goal_continuation);
    assert!(params.permissions.is_none());
    assert!(params.sandbox.is_none());
    assert!(params.approval_policy.is_none());
    assert_eq!(fork.origin().address.backend, BackendId(2));

    let wrong = Box::new(thread("child", ThreadHistoryMode::Legacy, Some("other")));
    assert!(matches!(
        fork.clone().forked(wrong),
        Err(Invalid::WrongFork)
    ));
    let child = Box::new(thread("child", ThreadHistoryMode::Legacy, Some("source")));
    let start = fork.forked(child).expect("fork from source");
    assert_eq!(start.origin().turn.as_str(), "checked");
    assert_eq!(start.child().id, "child");
    let (_, saved, params) = start.start("start-receipt".to_owned());
    assert_eq!(params.thread_id, "child");
    assert_eq!(
        params.client_user_message_id.as_deref(),
        Some("start-receipt")
    );
    assert_eq!(params.model.as_deref(), Some("faster"));
    assert_eq!(params.effort, Some(ReasoningEffort::Low));
    assert!(params.permissions.is_none());
    assert!(params.sandbox_policy.is_none());
    assert!(params.approval_policy.is_none());
    assert!(params.responsesapi_client_metadata.is_none());
    assert_eq!(saved, message);
    assert_eq!(params.input.len(), 5);
    assert!(matches!(&params.input[0], UserInput::Image { .. }));
    assert!(
        matches!(&params.input[1], UserInput::LocalImage { path, .. } if path == &PathBuf::from("/tmp/diagram.png"))
    );
    assert!(matches!(&params.input[2], UserInput::Text { text, .. } if text == &message.text));
    assert!(matches!(&params.input[4], UserInput::Text { text, .. } if text == "Keep it brief."));
}

#[test]
fn a_saved_visible_answer_prevents_a_retry_when_the_live_update_was_delayed() {
    let answer = serde_json::from_value(json!({
        "type": "agentMessage", "id": "answer", "text": "The answer is ready.",
        "phase": "final_answer"
    }))
    .expect("agent item");
    let read = turns(ThreadHistoryMode::Legacy, UserMessage::text("original"));
    assert!(matches!(
        read.verify(page(
            vec![turn("checked", TurnStatus::Completed, vec![answer])],
            None
        )),
        Err(Invalid::ResponseStarted)
    ));
}

#[test]
fn the_interrupt_reply_and_terminal_event_are_both_required_in_either_order() {
    for (first, second) in [
        (
            InterruptSignal::Acknowledged,
            InterruptSignal::TurnInterrupted,
        ),
        (
            InterruptSignal::TurnInterrupted,
            InterruptSignal::Acknowledged,
        ),
    ] {
        let ready = interrupted(UserMessage::text("original"), first, second);
        assert_eq!(ready.origin().turn.as_str(), "checked");
    }
}

#[test]
fn a_side_or_foreign_source_cannot_supply_the_history_check() {
    let make = || {
        interrupted(
            UserMessage::text("original"),
            InterruptSignal::Acknowledged,
            InterruptSignal::TurnInterrupted,
        )
    };
    let other = thread("other", ThreadHistoryMode::Legacy, None);
    assert!(matches!(make().verify(&other), Err(Invalid::WrongThread)));
    let mut side = thread("source", ThreadHistoryMode::Paginated, None);
    side.attached_to_thread_id = Some("parent".to_owned());
    assert!(matches!(
        make().verify(&side),
        Err(Invalid::IneligibleThread)
    ));
}
