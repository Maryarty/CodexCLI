//! The thread catalog's request identity and the reply projections used by the app.

use codex_app_server_protocol::{
    ClientRequest, RequestId, SortDirection, ThreadListCwdFilter, ThreadSortKey, ThreadSourceKind,
    TurnItemsView, UserInput,
};
use codex_protocol::openai_models::ReasoningEffort;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app};
use codex_tui_core::ids::{ThreadId, TurnId};
use codex_tui_core::protocol::{client_request, reply};
use codex_tui_core::rpc::RpcError;
use codex_tui_core::threads::asks::{ListQuery, ThreadRpc};
use codex_tui_core::threads::{HISTORY_PAGE, ThreadAsk, ThreadReply, UserMessage};
use serde_json::json;

#[test]
fn configured_model_facts_survive_sparse_thread_announcements_and_submit() {
    let mut opening: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/oracle/fixtures/chat_commands_3/export_thread_start.json"
    ))
    .unwrap_or_else(|error| panic!("valid opening fixture: {error}"));
    let slug = "vendor.session-model";
    opening["model"] = slug.into();
    opening["activePermissionProfile"] = json!({"id": "team-policy"});
    opening["reasoningEffort"] = "high".into();
    opening["thread"]["model"] = serde_json::Value::Null;
    opening["thread"]["reasoningEffort"] = serde_json::Value::Null;
    let notification = serde_json::from_value(json!({
        "method":"thread/started", "params":{"thread":opening["thread"].clone()}
    }))
    .unwrap_or_else(|error| panic!("valid thread announcement: {error}"));
    let ThreadReply::Resumed(resumed) = ThreadAsk::Resume {
        thread: String::new(),
    }
    .decode(opening.clone())
    .unwrap_or_else(|error| panic!("valid resume: {error:?}")) else {
        panic!("expected resumed thread");
    };
    assert_eq!(resumed.thread.model.as_deref(), Some(slug));
    assert_eq!(
        resumed.permissions.and_then(|snapshot| snapshot.profile),
        Some("team-policy".to_owned())
    );
    assert_eq!(
        resumed
            .thread
            .reasoning_effort
            .as_ref()
            .map(ReasoningEffort::as_str),
        Some("high")
    );

    let mut app = CodexApp::new();
    let mut effects = AppFx::default();
    apply_app(&mut app, AppEv::Connected, &mut effects);
    let start = effects
        .commands()
        .iter()
        .find(|command| matches!(command, Command::StartThread { .. }));
    let Some((ticket, reply)) = start.and_then(|start| reply(start, Ok(opening))) else {
        panic!("expected the session start request");
    };
    // The server announces the new thread before the start reply is consumed.
    let mut effects = AppFx::default();
    apply_app(
        &mut app,
        AppEv::Backend(BackendEv::Notification(Box::new(notification))),
        &mut effects,
    );
    apply_app(&mut app, AppEv::Reply(ticket, reply), &mut effects);
    let thread = app.active_thread();
    let chat = &thread.screen_slots().chat;
    assert_eq!(chat.model.as_deref(), Some(slug));
    assert_eq!(chat.effort.as_deref(), Some("high"));
    let Some(header) = &chat.session_header else {
        panic!("expected opening settings");
    };
    assert_eq!(header.model.as_deref(), Some(slug));
    assert_eq!(header.effort.as_deref(), Some("high"));

    let mut effects = AppFx::default();
    apply_app(
        &mut app,
        AppEv::Intent(Intent::Submit {
            thread: Some(thread.id().clone()),
            message: UserMessage::text("continue"),
        }),
        &mut effects,
    );
    let turn = effects
        .commands()
        .iter()
        .find(|command| matches!(command, Command::StartTurn { .. }));
    let Some(ClientRequest::TurnStart { params, .. }) =
        turn.and_then(|turn| client_request(turn, RequestId::Integer(1), None))
    else {
        panic!("expected the turn request");
    };
    assert_eq!(params.thread_id, thread.id().as_str());
    assert_eq!(params.model, None);
    assert_eq!(params.effort, None);
    assert_eq!(
        app.active_thread().screen_slots().chat.model.as_deref(),
        Some(slug)
    );
}

#[test]
fn thread_catalog_keeps_launch_settings_separate_from_new_tasks() {
    let request_id = RequestId::Integer(31);
    for (chosen, expected) in [(None, "cli-model"), (Some("migration"), "migration")] {
        let ask = ThreadAsk::StartSession {
            model: chosen.map(str::to_owned),
            cwd: Some("/workspace".to_owned()),
        };
        let ClientRequest::ThreadStart {
            request_id: actual,
            params,
        } = ask.request(Some("cli-model"), request_id.clone())
        else {
            panic!("expected thread/start");
        };
        assert_eq!(actual, request_id);
        assert_eq!(params.model.as_deref(), Some(expected));
        assert_eq!(params.cwd.as_deref(), Some("/workspace"));
    }

    let ask = ThreadAsk::Start {
        cwd: Some("/agent".to_owned()),
    };
    let ClientRequest::ThreadStart { params, .. } = ask.request(Some("cli-model"), request_id)
    else {
        panic!("expected thread/start");
    };
    assert_eq!(params.model, None);
    assert_eq!(params.cwd.as_deref(), Some("/agent"));
    assert_eq!(
        ask.decode(json!({})),
        Err(RpcError::Malformed("thread/start: no thread".to_owned())),
    );
}

#[test]
fn thread_catalog_keeps_turn_input_client_id_and_safety_metadata() {
    let message = UserMessage::text("continue from here");
    let request_id = RequestId::Integer(32);
    let thread = ThreadId::new("thread-a");
    let ask = ThreadRpc::StartUserTurn {
        thread: thread.clone(),
        message: &message,
        client_id: "client message a".to_owned(),
        misalignment_override: Some(173),
        ide_prompt: None,
    };
    let ClientRequest::TurnStart {
        request_id: actual,
        params,
    } = ask.wire(request_id.clone())
    else {
        panic!("expected turn/start");
    };
    assert_eq!(actual, request_id);
    assert_eq!(params.thread_id, thread.as_str());
    assert_eq!(
        params.client_user_message_id.as_deref(),
        Some("client message a")
    );
    assert_eq!(
        params.input,
        [UserInput::Text {
            text: message.text.clone(),
            text_elements: Vec::new()
        }],
    );
    assert_eq!(
        params
            .responsesapi_client_metadata
            .as_ref()
            .and_then(|metadata| metadata.get("misalignment_override"))
            .map(String::as_str),
        Some(r#"{"timestamp":173}"#),
    );
    assert_eq!(
        ask.decode(json!({"turn":{"id":"turn-a", "items":[], "status":"inProgress"}})),
        Ok(ThreadReply::TurnStarted {
            thread,
            turn: TurnId::new("turn-a")
        }),
    );
    assert!(matches!(
        ask.decode(json!({"turn":{}})),
        Err(RpcError::Malformed(_))
    ));
}

#[test]
fn thread_catalog_preserves_listing_filters_and_history_modes() {
    let list = ThreadAsk::list(ListQuery {
        current_account_only: true,
        cursor: Some("cursor-a".to_owned()),
        limit: Some(25),
        sort_key: Some(ThreadSortKey::CreatedAt),
        source_kinds: Some(vec![ThreadSourceKind::Cli, ThreadSourceKind::VsCode]),
        model_providers: None,
        archived: Some(true),
        cwd: Some(ThreadListCwdFilter::Many(vec![
            "/a".to_owned(),
            "/b".to_owned(),
        ])),
        use_state_db_only: false,
        ..ListQuery::default()
    });
    let wire_id = RequestId::Integer(33);
    let ClientRequest::ThreadList { request_id, params } = list.wire(wire_id.clone()) else {
        panic!("expected thread/list");
    };
    assert_eq!(request_id, wire_id);
    assert!(params.current_account_only);
    assert_eq!(params.limit, Some(25));
    assert_eq!(params.cursor.as_deref(), Some("cursor-a"));
    assert_eq!(params.sort_key, Some(ThreadSortKey::CreatedAt));
    assert_eq!(params.sort_direction, None);
    assert_eq!(
        params.source_kinds,
        Some(vec![ThreadSourceKind::Cli, ThreadSourceKind::VsCode])
    );
    assert_eq!(params.model_providers, None);
    assert_eq!(params.archived, Some(true));
    assert_eq!(
        params.cwd,
        Some(ThreadListCwdFilter::Many(vec![
            "/a".to_owned(),
            "/b".to_owned()
        ]))
    );
    assert!(!params.use_state_db_only);
    assert!(!params.include_attached_threads);
    assert_eq!(params.section_id, None);

    let page = list.decode(json!({"data":[], "nextCursor":"cursor-b"}));
    assert!(
        matches!(page, Ok(ThreadReply::Listed(page)) if page.next_cursor.as_deref() == Some("cursor-b"))
    );

    let thread = "read-a".to_owned();
    let read = ThreadAsk::Read {
        thread: thread.clone(),
    };
    let active = ThreadAsk::ActiveTurn {
        thread: thread.clone(),
    };
    for (ask, turns) in [(read, false), (active, true)] {
        let ClientRequest::ThreadRead { params, .. } = ask.wire(wire_id.clone()) else {
            panic!("expected thread/read");
        };
        assert_eq!(params.thread_id, thread);
        assert_eq!(params.include_turns, turns);
    }
    let history = ThreadAsk::History {
        thread,
        cursor: Some("older".to_owned()),
    };
    let ClientRequest::ThreadTurnsList { params, .. } = history.wire(wire_id) else {
        panic!("expected thread/turns/list");
    };
    assert_eq!(params.limit, Some(HISTORY_PAGE));
    assert_eq!(params.cursor.as_deref(), Some("older"));
    assert_eq!(params.items_view, Some(TurnItemsView::Full));
    assert_eq!(params.sort_direction, Some(SortDirection::Desc));
}

/// Opening a task from the center must not pull its whole history over the
/// wire (perf-open-session): the resume asks for the thread's state only;
/// the transcript is read page by page (`thread/turns/list`) when the chat
/// shows it, as the oracle's paginated sessions do.
#[test]
fn resume_asks_for_no_turns() {
    let ask = ThreadAsk::Resume {
        thread: "t1".to_owned(),
    };
    let ClientRequest::ThreadResume { params, .. } = ask.wire(RequestId::Integer(1)) else {
        panic!("resume is thread/resume");
    };
    assert_eq!(params.thread_id, "t1");
    assert!(params.exclude_turns, "thread/resume must exclude the turns");
}

/// The edit of a past prompt (the transcript's backtrack) reads the
/// persisted turns, then reverts before one of them: both are thread RPCs,
/// made in the catalog (ADR-003).
#[test]
fn the_prompt_edit_reads_the_turns_then_reverts_before_one() {
    let ClientRequest::ThreadRead { params, .. } = (ThreadAsk::ReadTurns {
        thread: "t1".to_owned(),
    })
    .wire(RequestId::Integer(1)) else {
        panic!("a read of the turns is thread/read");
    };
    assert_eq!(params.thread_id, "t1");
    assert!(
        params.include_turns,
        "the edit resolves the prompt from the turns"
    );

    let ask = ThreadAsk::Revert {
        thread: "t1".to_owned(),
        before_turn: TurnId::new("turn-3"),
    };
    assert_eq!(ask.thread(), Some("t1"));
    let ClientRequest::ThreadRevert { params, .. } = ask.wire(RequestId::Integer(2)) else {
        panic!("a revert is thread/revert");
    };
    assert_eq!(params.thread_id, "t1");
    assert_eq!(params.before_turn_id, "turn-3");
}
