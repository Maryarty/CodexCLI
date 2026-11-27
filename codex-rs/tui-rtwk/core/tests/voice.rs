//! Live call registration, stamped transport ingress and exact media receipts.

#![allow(clippy::expect_used)]

use codex_app_server_protocol as wire;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, Ingress, Intent, Now, apply_app, apply_app_from,
};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::{BackendId, ThreadId};
use codex_tui_core::protocol::{client_request, reply};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenThreadIntent, voice};
use voice::{AudioAction, AudioCommand, AudioReport, AudioResult, Peaks, Sdp, ThreadIntent};

fn ready() -> CodexApp {
    build(r#"{"step":"seed","seed":"session","thread":"thread-1"}"#)
        .expect("session")
        .app
}

fn run(app: &mut CodexApp, event: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app(app, event, &mut fx);
    app.assert_consistent();
    fx
}

fn intent(app: &mut CodexApp, intent: ThreadIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new("thread-1"),
            intent: ScreenThreadIntent::Voice(intent),
        }),
    )
}

fn host(fx: &AppFx) -> (&AudioCommand, u64) {
    let commands: Vec<_> = fx
        .commands()
        .iter()
        .filter_map(|cmd| {
            if let Command::Effect {
                request,
                ask: Ask::Audio(audio),
                ..
            } = cmd
            {
                Some((audio, *request))
            } else {
                None
            }
        })
        .collect();
    let [command] = commands.as_slice() else {
        panic!("expected one audio command: {:?}", fx.commands())
    };
    *command
}

fn media(fx: &AppFx, result: AudioResult) -> AppEv {
    let (command, request) = host(fx);
    AppEv::Effect {
        request,
        done: Done::Audio(AudioReport {
            owner: command.owner.clone(),
            result,
        }),
    }
}

fn wire(fx: &AppFx) -> (&Command, wire::ClientRequest) {
    fx.commands()
        .iter()
        .find_map(|cmd| {
            let wire = client_request(cmd, wire::RequestId::Integer(1), None)?;
            matches!(
                wire,
                wire::ClientRequest::ThreadRealtimeStart { .. }
                    | wire::ClientRequest::ThreadRealtimeStop { .. }
            )
            .then_some((cmd, wire))
        })
        .expect("voice RPC")
}

fn ack(fx: &AppFx) -> AppEv {
    let (cmd, _) = wire(fx);
    let (id, response) = reply(cmd, Ok(serde_json::json!({}))).expect("typed response");
    AppEv::Reply(id, response)
}

fn note(method: &str, params: serde_json::Value) -> AppEv {
    AppEv::Backend(BackendEv::Notification(Box::new(
        serde_json::from_value(serde_json::json!({ "method": method, "params": params }))
            .expect("pinned voice notification"),
    )))
}

fn started(thread: &str, session: &str) -> AppEv {
    note(
        "thread/realtime/started",
        serde_json::json!({
            "threadId": thread,
            "realtimeSessionId": session,
            "version": "v3",
        }),
    )
}

fn closed(thread: &str) -> AppEv {
    note(
        "thread/realtime/closed",
        serde_json::json!({ "threadId": thread }),
    )
}

fn answer(thread: &str) -> AppEv {
    note(
        "thread/realtime/sdp",
        serde_json::json!({ "threadId": thread, "sdp": "local-answer-ice" }),
    )
}

fn preparing(app: &mut CodexApp) -> AppFx {
    let prepared = intent(app, ThreadIntent::Start);
    let (cmd, _) = host(&prepared);
    assert_eq!(cmd.action, AudioAction::Prepare);
    assert_eq!(cmd.owner.thread.as_str(), "thread-1");
    assert!(
        app.dispatch_route(prepared.commands().first().expect("command"), None)
            .is_some()
    );
    prepared
}

fn offered(app: &mut CodexApp, prepared: &AppFx) -> (AppFx, String) {
    let start = run(
        app,
        media(
            prepared,
            AudioResult::Prepared(Ok(Sdp::new("local-offer-ice".into()))),
        ),
    );
    let (cmd, wire) = wire(&start);
    assert!(!format!("{cmd:?}").contains("local-offer-ice"));
    let wire::ClientRequest::ThreadRealtimeStart { params, .. } = wire else {
        panic!("start")
    };
    assert!(
        matches!(params.transport, Some(wire::ThreadRealtimeStartTransport::Webrtc { sdp }) if sdp == "local-offer-ice")
    );
    assert_eq!(params.thread_id, "thread-1");
    (
        start,
        params
            .realtime_session_id
            .expect("client's session identity"),
    )
}

fn display(app: &CodexApp) -> Option<voice::Display> {
    app.active_thread().screen_slots().voice.display()
}

fn active(app: &mut CodexApp) -> String {
    let prepared = preparing(app);
    let (start, session) = offered(app, &prepared);
    run(app, started("thread-1", &session));
    let connect = run(app, answer("thread-1"));
    run(app, media(&connect, AudioResult::Connected(Ok(()))));
    run(app, ack(&start));
    assert!(display(app).is_some_and(|display| !display.connecting));
    session
}

#[test]
fn listens_only_after_exact_server_identity_rpc_acceptance_and_real_peer() {
    let mut app = ready();
    let prepared = preparing(&mut app);
    let (start, session) = offered(&mut app, &prepared);
    run(&mut app, started("thread-1", "old-attempt"));
    assert!(run(&mut app, answer("thread-1")).commands().is_empty());
    let mut foreign_fx = AppFx::default();
    apply_app_from(
        &mut app,
        BackendId(1),
        started("thread-1", &session),
        &mut foreign_fx,
    );
    assert!(run(&mut app, answer("thread-1")).commands().is_empty());
    run(&mut app, ack(&start));
    run(&mut app, started("thread-2", &session));
    assert!(display(&app).expect("connecting").connecting);
    run(&mut app, started("thread-1", &session));
    let connect = run(&mut app, answer("thread-1"));
    assert!(matches!(&host(&connect).0.action, AudioAction::Connect(_)));
    assert!(!format!("{:?}", host(&connect).0).contains("local-answer-ice"));
    assert!(display(&app).expect("connecting").connecting);
    let listening = run(&mut app, media(&connect, AudioResult::Connected(Ok(()))));
    assert!(!display(&app).expect("listening").connecting);
    assert!(listening.wake().is_some());
}

#[test]
fn mute_updates_only_after_its_media_command_and_actual_peaks_fill_both_tracks() {
    let mut app = ready();
    active(&mut app);
    let sample = run(&mut app, AppEv::Clock(Now::at_millis(1_000)));
    assert_eq!(host(&sample).0.action, AudioAction::Sample);
    let event = media(&sample, AudioResult::Sampled(Ok(Peaks([19_600, 32_768]))));
    assert!(!format!("{event:?}").contains("19600"));
    run(&mut app, event);
    let status = display(&app).expect("still listening");
    assert!(status.microphone[5] > 0);
    assert_eq!(status.speaker[5], 255);

    let mute = intent(&mut app, ThreadIntent::SetMuted(true));
    assert_eq!(host(&mute).0.action, AudioAction::SetMuted(true));
    assert!(!display(&app).expect("pending mute").muted);
    assert!(
        intent(&mut app, ThreadIntent::SetMuted(true))
            .commands()
            .is_empty()
    );
    run(
        &mut app,
        media(
            &mute,
            AudioResult::Muted {
                muted: false,
                result: Ok(()),
            },
        ),
    );
    assert!(!display(&app).expect("wrong answer").muted);
    run(
        &mut app,
        media(
            &mute,
            AudioResult::Muted {
                muted: true,
                result: Ok(()),
            },
        ),
    );
    let status = display(&app).expect("muted");
    assert!(status.muted);
    assert_eq!(status.microphone, [0; 6]);
    assert_eq!(status.speaker[5], 255);
}

#[test]
fn stop_waits_for_start_ack_and_does_not_restart_after_refusal_until_closed() {
    let mut app = ready();
    let prepare = preparing(&mut app);
    let (start, _) = offered(&mut app, &prepare);
    let pending = intent(&mut app, ThreadIntent::Stop);
    assert_eq!(host(&pending).0.action, AudioAction::Close);
    assert!(
        pending
            .commands()
            .iter()
            .all(|cmd| client_request(cmd, wire::RequestId::Integer(1), None).is_none())
    );
    let stop = run(&mut app, ack(&start));
    let (cmd, request) = wire(&stop);
    assert!(
        matches!(request, wire::ClientRequest::ThreadRealtimeStop { params, .. } if params.thread_id == "thread-1")
    );
    let Command::Screen { ticket, .. } = cmd else {
        panic!("stop ticket")
    };
    run(&mut app, AppEv::Reply(*ticket, Reply::failed("refused")));
    assert!(intent(&mut app, ThreadIntent::Start).commands().is_empty());
    let retry = intent(&mut app, ThreadIntent::Stop);
    wire(&retry);
    run(&mut app, closed("thread-1"));
    assert!(intent(&mut app, ThreadIntent::Start).commands().is_empty());
    run(&mut app, ack(&retry));
    preparing(&mut app);
}

#[test]
fn stop_ack_alone_does_not_release_old_threads_untagged_terminal_events() {
    let mut app = ready();
    active(&mut app);
    let stop = intent(&mut app, ThreadIntent::Stop);
    assert_eq!(host(&stop).0.action, AudioAction::Close);
    run(&mut app, ack(&stop));
    assert!(display(&app).is_none());
    assert!(intent(&mut app, ThreadIntent::Start).commands().is_empty());
    assert!(
        run(&mut app, AppEv::Clock(Now::at_millis(2_000)))
            .commands()
            .is_empty()
    );
    run(&mut app, closed("thread-1"));
    preparing(&mut app);
}

#[test]
fn cancelled_or_replaced_start_retires_the_peer_and_rejects_late_offer() {
    let mut app = ready();
    let epoch = app.epoch_of(BackendId::default());
    let pending = preparing(&mut app);
    let replace = app
        .ingest(Ingress::transport_replaced(
            BackendId::default(),
            epoch,
            epoch.next(),
        ))
        .expect("transport replaced");
    assert_eq!(host(&replace).0.action, AudioAction::Close);
    assert!(display(&app).is_none());
    let late = run(
        &mut app,
        media(&pending, AudioResult::Prepared(Ok(Sdp::new("late".into())))),
    );
    assert_eq!(host(&late).0.action, AudioAction::Close);
    assert!(display(&app).is_none());
}

#[test]
fn unavailable_packaged_helper_never_opens_a_backend_call_or_asks_for_a_meter() {
    let mut app = ready();
    let pending = preparing(&mut app);
    let denied = run(
        &mut app,
        media(
            &pending,
            AudioResult::Prepared(Err("no audio helper".into())),
        ),
    );
    assert_eq!(host(&denied).0.action, AudioAction::Close);
    assert!(denied.wake().is_none());
    assert!(display(&app).is_none());
}

#[test]
fn displayed_thread_switch_does_not_move_or_close_the_voice_owner() {
    use codex_tui_core::screens::{ScreenEv, agents};
    let mut app = ready();
    active(&mut app);
    let show = |thread| {
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Current(Some(ThreadId::new(
            thread,
        )))))
    };
    run(&mut app, show("thread-2"));
    assert!(display(&app).is_none());
    run(&mut app, closed("thread-2"));
    run(&mut app, show("thread-1"));
    assert!(display(&app).is_some_and(|display| !display.connecting));
}

fn caption_done(thread: &str, session: &str, id: &str, text: &str) -> AppEv {
    note(
        "thread/realtime/item/completed",
        serde_json::json!({
            "threadId": thread,
            "item": {
                "id": id, "realtimeSessionId": session, "type": "transcriptSegment",
                "role": "user", "text": text,
            },
        }),
    )
}

fn captions(app: &CodexApp, thread: &str) -> Vec<String> {
    use codex_tui_core::threads::Item;
    app.thread(&ThreadId::new(thread))
        .expect("the thread is held")
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::VoiceTranscript { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Attach a listed app-server thread and answer its resume on the registered transport.
fn open_thread(app: &mut CodexApp, id: &str, backend: BackendId, paginated: bool) -> AppFx {
    use codex_tui_core::screens::agents::{Ev, Page, Reply as AgentsReply};
    use codex_tui_core::screens::{ScreenEv, ScreenReply};
    let thread: wire::Thread = serde_json::from_value(serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "", "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": if paginated {"paginated"} else {"legacy"}, "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
        "recencyAt": 0, "status": {"type":"idle"}, "path": null, "cwd": "/tmp/project",
        "cliVersion": "0.0.0", "originator": null, "source": "cli", "canAcceptDirectInput": true,
        "threadSource": null, "agentNickname": null, "agentRole": null, "gitInfo": null,
        "name": "Another thread", "daybreakEnabled": null, "turns": []
    }))
    .expect("a listed thread");
    run(
        app,
        AppEv::Screen(ScreenEv::Agents(Ev::ListedBy(
            backend,
            Page {
                threads: vec![thread.clone()],
                unread: Vec::new(),
                next_cursor: None,
                append: false,
                pins: None,
            },
        ))),
    );
    let resume = run(app, AppEv::Intent(Intent::OpenThread(ThreadId::new(id))));
    let ticket = resume
        .commands()
        .iter()
        .find_map(
            |command| match client_request(command, wire::RequestId::Integer(1), None) {
                Some(wire::ClientRequest::ThreadResume { params, .. })
                    if params.thread_id == id =>
                {
                    match command {
                        Command::Screen { ticket, .. } => Some(*ticket),
                        _ => None,
                    }
                }
                _ => None,
            },
        )
        .expect("a resume request for the detached thread");
    let resumed = run(
        app,
        AppEv::Reply(
            ticket,
            Reply::screen(ScreenReply::Agents(AgentsReply::Resumed(Box::new(thread)))),
        ),
    );
    assert_eq!(app.session().active, Some(ThreadId::new(id)));
    resumed
}

#[test]
fn caption_uses_the_exact_thread_call_and_physical_address() {
    let mut app = ready();
    let session = active(&mut app);
    let backend = BackendId::default();
    let epoch = app.epoch_of(backend);
    let receive = |app: &mut CodexApp, physical, event| {
        app.ingest(Ingress::transport_event(backend, epoch, physical, event))
    };

    open_thread(&mut app, "thread-2", BackendId(1), true);
    receive(
        &mut app,
        epoch,
        caption_done("thread-2", &session, "a", "misrouted"),
    );
    receive(
        &mut app,
        epoch,
        caption_done("thread-1", "old-call", "b", "foreign call"),
    );
    for _ in 0..2 {
        receive(
            &mut app,
            epoch,
            caption_done("thread-1", &session, "c", "hello from background"),
        );
    }
    assert_eq!(captions(&app, "thread-1"), ["hello from background"]);
    assert!(captions(&app, "thread-2").is_empty());

    app.ingest(Ingress::transport_replaced(backend, epoch, epoch.next()))
        .expect("new physical transport");
    open_thread(&mut app, "thread-1", backend, true);
    let prepared = preparing(&mut app);
    let (_, current_session) = offered(&mut app, &prepared);
    assert_ne!(current_session, session);
    assert!(
        receive(
            &mut app,
            epoch,
            caption_done("thread-1", &current_session, "late", "retired transport"),
        )
        .is_none()
    );
    receive(
        &mut app,
        epoch.next(),
        caption_done("thread-1", &session, "old", "retired session"),
    );
    receive(
        &mut app,
        epoch.next(),
        caption_done("thread-1", &current_session, "fresh", "new transport"),
    );
    assert_eq!(
        captions(&app, "thread-1"),
        ["hello from background", "new transport"]
    );
}

fn timeline(fx: &AppFx) -> Option<(&Command, wire::ThreadTimelineListParams)> {
    fx.commands().iter().find_map(|command| {
        match client_request(command, wire::RequestId::Integer(1), None) {
            Some(wire::ClientRequest::ThreadTimelineList { params, .. }) => Some((command, params)),
            _ => None,
        }
    })
}

fn timeline_answer(command: &Command, data: serde_json::Value, next: Option<&str>) -> AppEv {
    let (ticket, reply) = reply(
        command,
        Ok(serde_json::json!({
            "data": data, "nextCursor": next, "activeRealtimeSessionAtPageStart": null,
        })),
    )
    .expect("canonical timeline response");
    AppEv::Reply(ticket, reply)
}

fn read_replay(app: &mut CodexApp, backend: BackendId, paginated: bool) -> AppFx {
    let resumed = open_thread(app, "thread-2", backend, paginated);
    let turns = resumed
        .commands()
        .iter()
        .find(|command| {
            matches!(
                client_request(command, wire::RequestId::Integer(1), None),
                Some(wire::ClientRequest::ThreadTurnsList { params, .. }) if params.thread_id == "thread-2"
            )
        })
        .expect("ordinary history request");
    let (ticket, history) = reply(
        turns,
        Ok(serde_json::json!({ "data": [], "nextCursor": null, "backwardsCursor": null })),
    )
    .expect("ordinary history response");
    run(app, AppEv::Reply(ticket, history))
}

#[test]
fn timeline_replay_stays_on_local_or_remote_thread_owner_and_skips_cloud() {
    for owner in [BackendId::default(), BackendId(1)] {
        let mut app = ready();
        let read = read_replay(&mut app, owner, true);
        let (command, params) = timeline(&read).expect("typed timeline read");
        assert_eq!(params.thread_id, "thread-2");
        let (backend, address) = app
            .dispatch_route(command, None)
            .expect("registered address");
        assert_eq!(backend, owner);
        assert_eq!(address.expect("physical address").backend, owner);

        let answer = timeline_answer(
            command,
            serde_json::json!([{
                "type": "realtime", "position": 1,
                "item": {
                    "id": "saved", "realtimeSessionId": "older-call", "type": "transcriptSegment",
                    "role": "assistant", "text": "saved voice answer"
                }
            }]),
            None,
        );
        let other = BackendId(owner.0 + 1);
        assert!(
            app.ingest(Ingress::of(other, app.epoch_of(other), answer.clone()))
                .is_none()
        );
        assert!(captions(&app, "thread-2").is_empty());
        run(&mut app, answer);
        assert_eq!(captions(&app, "thread-2"), ["saved voice answer"]);
    }

    let mut app = ready();
    let cloud = BackendId(1);
    run(
        &mut app,
        AppEv::Screen(codex_tui_core::screens::ScreenEv::Agents(
            codex_tui_core::screens::agents::Ev::CloudBackend(cloud),
        )),
    );
    assert!(timeline(&read_replay(&mut app, cloud, true)).is_none());
    let mut legacy = ready();
    assert!(timeline(&read_replay(&mut legacy, BackendId::default(), false)).is_none());
}

#[test]
fn timeline_stops_automatic_pages_and_keeps_explicit_older_history_available() {
    use codex_tui_core::threads::HistoryState;
    let mut app = ready();
    let mut read = read_replay(&mut app, BackendId::default(), true);
    let mut seen = 0;
    while let Some((command, _)) = timeline(&read) {
        seen += 1;
        assert!(seen <= 8, "more pages need an explicit history action");
        let data = serde_json::json!([{
            "type": "turnStarted", "position": 100 - seen, "turnId": "another-turn", "startedAt": null,
        }]);
        let answer = timeline_answer(command, data, Some(&format!("cursor-{seen}")));
        read = run(&mut app, answer);
    }
    assert_eq!(seen, 8);
    assert_eq!(app.active_thread().history_state(), HistoryState::Partial);
    read = run(
        &mut app,
        AppEv::Intent(Intent::LoadOlderHistory(ThreadId::new("thread-2"))),
    );
    let (command, params) = timeline(&read).expect("explicit continuation");
    assert_eq!(params.cursor.as_deref(), Some("cursor-8"));
    let answer = timeline_answer(command, serde_json::json!([]), Some("spurious-empty-page"));
    assert!(timeline(&run(&mut app, answer)).is_none());
    assert_eq!(app.active_thread().history_state(), HistoryState::Complete);
}
