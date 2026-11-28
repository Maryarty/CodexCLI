//! Live call registration, stamped transport ingress and exact media receipts.

#![allow(clippy::expect_used)]

use codex_app_server_protocol as wire;
use codex_tui_core::app::effects::{Ask, Done};
use codex_tui_core::app::{
    AppEv, AppFx, BackendEv, CodexApp, Command, Ingress, Intent, Now, apply_app, apply_app_from,
};
use codex_tui_core::fixtures::script::build;
use codex_tui_core::ids::{BackendId, ConnEpoch, ThreadId};
use codex_tui_core::protocol::{client_request, reply};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenThreadIntent, voice};
use voice::{
    AudioAction, AudioCommand, AudioReport, AudioResult, LockStop, Peaks, Sdp, ThreadIntent,
};

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
    intent_for(app, "thread-1", intent)
}

fn intent_for(app: &mut CodexApp, thread: &str, intent: ThreadIntent) -> AppFx {
    run(
        app,
        AppEv::Intent(Intent::ThreadScreen {
            thread: ThreadId::new(thread),
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
                && audio.action != AudioAction::ObserveLock
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
    preparing_for(app, "thread-1")
}

fn preparing_for(app: &mut CodexApp, thread: &str) -> AppFx {
    let prepared = intent_for(app, thread, ThreadIntent::Start);
    let (cmd, _) = host(&prepared);
    assert_eq!(cmd.action, AudioAction::Prepare);
    assert_eq!(cmd.owner.thread.as_str(), thread);
    assert!(prepared.commands().iter().any(|command| {
        matches!(command.host_ask(), Some(Ask::Audio(watch)) if watch.owner == cmd.owner && watch.action == AudioAction::ObserveLock)
    }));
    assert!(
        prepared
            .commands()
            .iter()
            .all(|command| app.dispatch_route(command, None).is_some())
    );
    prepared
}

#[test]
fn initial_desktop_lock_stops_preparation_and_old_lock_after_restart_is_ignored() {
    let mut app = ready();
    let first = preparing(&mut app);
    let lock = media(&first, AudioResult::LockStopped(LockStop::Locked));
    let stopped = run(&mut app, lock.clone());
    assert_eq!(host(&stopped).0.action, AudioAction::Close);
    assert!(display(&app).is_none());
    assert_eq!(
        app.active_thread().last_error(),
        Some(&codex_tui_core::threads::Failure::VoiceLock(
            LockStop::Locked
        ))
    );
    let next = preparing(&mut app);
    assert_ne!(host(&first).0.owner, host(&next).0.owner);
    assert!(run(&mut app, lock).commands().is_empty());
    assert!(display(&app).is_some_and(|display| display.connecting));
}

#[test]
fn desktop_lock_stops_a_parked_call_after_its_start_receipt() {
    use codex_tui_core::screens::{ScreenEv, agents};
    let mut app = ready();
    let prepare = preparing(&mut app);
    let (start, _) = offered(&mut app, &prepare);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::Current(Some(ThreadId::new(
            "thread-2",
        ))))),
    );
    let locked = run(
        &mut app,
        media(&prepare, AudioResult::LockStopped(LockStop::Locked)),
    );
    assert_eq!(host(&locked).0.owner, host(&prepare).0.owner);
    assert_eq!(host(&locked).0.action, AudioAction::Close);
    assert!(
        locked
            .commands()
            .iter()
            .all(|cmd| client_request(cmd, wire::RequestId::Integer(1), None).is_none())
    );
    let stopped = run(&mut app, ack(&start));
    assert!(
        matches!(wire(&stopped).1, wire::ClientRequest::ThreadRealtimeStop { params, .. } if params.thread_id == "thread-1")
    );
    assert_eq!(app.active_thread().id().as_str(), "thread-2");
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
    assert_eq!(params.thread_id, host(prepared).0.owner.thread.as_str());
    (
        start,
        params
            .realtime_session_id
            .expect("client's session identity"),
    )
}

#[test]
fn voice_preference_changes_the_next_call_only_after_the_effective_write_receipt() {
    use codex_tui_core::screens::ScreenIntent;
    use voice::settings::{
        Answer, Catalog, Change, Feedback, Intent as SettingsIntent, Operation,
        RealtimeVoice as Voice, Report,
    };

    let mut app = ready();
    let thread = ThreadId::new("thread-1");
    let intent_ev = |intent| AppEv::Intent(Intent::Screen(ScreenIntent::Voice(intent)));
    let request = |fx: &AppFx| {
        fx.commands()
            .iter()
            .find_map(|command| match command.host_ask() {
                Some(Ask::VoiceSettings(request)) => Some(request.clone()),
                _ => None,
            })
            .expect("issued settings request")
    };
    let receipt = |request, answer| AppEv::Effect {
        request: 0,
        done: Done::VoiceSettings(Box::new(Report {
            request,
            result: Ok(answer),
        })),
    };
    let catalog = |current| Catalog {
        voices: vec![Voice::Juniper, Voice::Maple],
        current: Some(current),
    };
    let listed = run(
        &mut app,
        intent_ev(SettingsIntent::Read {
            thread: thread.clone(),
            operation: Operation::ReadCatalog,
        }),
    );
    let queried = request(&listed);
    assert_eq!(
        app.dispatch_route(&listed.commands()[0], None),
        Some((queried.origin.address.backend, Some(queried.origin.address)))
    );
    run(
        &mut app,
        receipt(queried.clone(), Answer::Catalog(catalog(Voice::Juniper))),
    );
    let saved = run(
        &mut app,
        intent_ev(SettingsIntent::Persist {
            origin: queried.origin,
            change: Change::Voice(Voice::Maple),
        }),
    );
    let write = request(&saved);
    let pending_call = preparing(&mut app);
    run(
        &mut app,
        receipt(
            write.clone(),
            Answer::VoiceSaved {
                catalog: catalog(Voice::Maple),
                warning: None,
            },
        ),
    );
    assert_eq!(
        app.screens()
            .voice
            .settings
            .snapshot(&thread)
            .expect("settings")
            .feedback,
        Some(Feedback::VoiceSaved(Voice::Maple))
    );
    let (start, _) = offered(&mut app, &pending_call);
    assert!(
        matches!(wire(&start).1, wire::ClientRequest::ThreadRealtimeStart { params, .. } if params.voice == Some(Voice::Juniper))
    );
    run(&mut app, ack(&start));
    run(&mut app, closed("thread-1"));
    let next = preparing(&mut app);
    let (start, _) = offered(&mut app, &next);
    assert!(
        matches!(wire(&start).1, wire::ClientRequest::ThreadRealtimeStart { params, .. } if params.voice == Some(Voice::Maple))
    );

    let backend = write.origin.address.backend;
    let epoch = write.origin.address.epoch;
    app.ingest(Ingress::transport_replaced(backend, epoch, epoch.next()))
        .expect("transport replaced");
    run(
        &mut app,
        receipt(
            write,
            Answer::VoiceSaved {
                catalog: catalog(Voice::Juniper),
                warning: None,
            },
        ),
    );
    assert!(
        app.screens()
            .voice
            .settings
            .snapshot(&thread)
            .expect("retired settings")
            .catalog
            .is_none()
    );
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

fn voice_thread_details(id: &str, paginated: bool, ephemeral: bool) -> wire::Thread {
    serde_json::from_value(serde_json::json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": "", "ephemeral": ephemeral, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": if paginated {"paginated"} else {"legacy"}, "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 0, "updatedAt": 0,
        "recencyAt": 0, "status": {"type":"idle"}, "path": null, "cwd": "/tmp/project",
        "cliVersion": "0.0.0", "originator": null, "source": "cli", "canAcceptDirectInput": true,
        "threadSource": null, "agentNickname": null, "agentRole": null, "gitInfo": null,
        "name": "Another thread", "daybreakEnabled": null, "turns": []
    }))
    .expect("a listed thread")
}

/// Attach a listed app-server thread and answer its resume on the registered transport.
fn open_thread(app: &mut CodexApp, id: &str, backend: BackendId, paginated: bool) -> AppFx {
    use codex_tui_core::screens::agents::{Ev, Page, Reply as AgentsReply};
    use codex_tui_core::screens::{ScreenEv, ScreenReply};
    let thread = voice_thread_details(id, paginated, false);
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
fn desktop_media_and_devices_use_primary_services_but_voice_catalog_uses_the_thread_owner() {
    use codex_tui_core::screens::ScreenIntent;
    use voice::settings::{Intent as SettingsIntent, Operation};

    let mut app = ready();
    let secondary = BackendId(1);
    app.attach_backend(secondary);
    apply_app_from(&mut app, secondary, AppEv::Connected, &mut AppFx::default());
    open_thread(&mut app, "thread-2", secondary, true);
    let prepare = preparing_for(&mut app, "thread-2");
    assert_eq!(host(&prepare).0.owner.address.backend, secondary);
    assert!(
        prepare
            .commands()
            .iter()
            .all(|command| app.dispatch_route(command, None) == Some((BackendId::default(), None)))
    );
    let settings = |app: &mut CodexApp, operation| {
        run(
            app,
            AppEv::Intent(Intent::Screen(ScreenIntent::Voice(SettingsIntent::Read {
                thread: ThreadId::new("thread-2"),
                operation,
            }))),
        )
    };
    let local = settings(&mut app, Operation::ReadAudio);
    assert_eq!(
        app.dispatch_route(&local.commands()[0], None),
        Some((BackendId::default(), None))
    );
    let server = settings(&mut app, Operation::ReadCatalog);
    assert!(
        matches!(app.dispatch_route(&server.commands()[0], None), Some((owner, Some(address))) if owner == secondary && address.backend == secondary)
    );
    let stopped = intent_for(&mut app, "thread-2", ThreadIntent::Stop);
    let close = stopped
        .commands()
        .iter()
        .find(|cmd| {
            cmd.host_ask().is_some_and(
                |ask| matches!(ask, Ask::Audio(audio) if audio.action == AudioAction::Close),
            )
        })
        .expect("local close");
    assert_eq!(
        app.dispatch_route(close, None),
        Some((BackendId::default(), None))
    );
}

#[test]
fn replacement_of_the_primary_audio_executor_stops_secondary_calls_and_settles_devices() {
    use codex_tui_core::screens::{ScreenEv, ScreenIntent, agents};
    use codex_tui_core::threads::Failure;
    use voice::settings::{Feedback, Intent as SettingsIntent, Operation, Problem};

    let primary = BackendId::default();
    let secondary = BackendId(1);
    let thread = ThreadId::new("secondary");
    let reconnecting = || AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnecting));
    let mut app = ready();
    app.attach_backend(secondary);
    apply_app_from(&mut app, secondary, AppEv::Connected, &mut AppFx::default());
    open_thread(&mut app, thread.as_str(), secondary, true);

    let prepared = preparing_for(&mut app, thread.as_str());
    let devices = run(
        &mut app,
        AppEv::Intent(Intent::Screen(ScreenIntent::Voice(SettingsIntent::Read {
            thread: thread.clone(),
            operation: Operation::ReadAudio,
        }))),
    );
    assert_eq!(
        app.dispatch_route(&devices.commands()[0], None),
        Some((primary, None))
    );
    let lost = run(&mut app, reconnecting());
    assert_eq!(host(&lost).0.action, AudioAction::Close);
    assert_eq!(host(&lost).0.owner, host(&prepared).0.owner);
    assert!(display(&app).is_none());
    assert!(!app.screens().voice.settings.pending(&thread));
    assert_eq!(
        app.screens()
            .voice
            .settings
            .snapshot(&thread)
            .expect("read failed")
            .feedback,
        Some(Feedback::Failed(Problem::ConnectionLost))
    );
    assert_eq!(
        app.active_thread().last_error(),
        Some(&Failure::VoiceHostLost)
    );
    assert!(
        intent_for(&mut app, thread.as_str(), ThreadIntent::Start)
            .commands()
            .is_empty()
    );

    for accepted in [false, true] {
        run(
            &mut app,
            AppEv::Screen(ScreenEv::Agents(agents::Ev::Reconnected)),
        );
        let prepare = preparing_for(&mut app, thread.as_str());
        let (start, _) = offered(&mut app, &prepare);
        if accepted {
            run(&mut app, ack(&start));
        }
        let stopping = run(&mut app, reconnecting());
        let stop = if accepted {
            stopping
        } else {
            run(&mut app, ack(&start))
        };
        let (command, request) = wire(&stop);
        assert!(
            matches!(request, wire::ClientRequest::ThreadRealtimeStop { params, .. } if params.thread_id == thread.as_str())
        );
        assert!(
            matches!(app.dispatch_route(command, None), Some((owner, _)) if owner == secondary)
        );
        run(&mut app, ack(&stop));
        apply_app_from(
            &mut app,
            secondary,
            closed(thread.as_str()),
            &mut AppFx::default(),
        );
        assert!(display(&app).is_none());
    }
}

#[test]
fn catalog_and_audio_reads_requested_in_another_thread_survive_a_pending_voice_write() {
    use codex_tui_core::screens::ScreenIntent;
    use codex_tui_core::threads::{Item, NoteKey};
    use voice::settings::{
        Answer, Audio, Catalog, Change, Feedback, Intent as SettingsIntent, Operation, Problem,
        RealtimeVoice as Voice, Report,
    };

    let event = |intent| AppEv::Intent(Intent::Screen(ScreenIntent::Voice(intent)));
    let take = |fx: &AppFx| {
        fx.commands().iter().find_map(|c| match c.host_ask() {
            Some(Ask::VoiceSettings(request)) => Some(request.clone()),
            _ => None,
        })
    };
    let answer = |request, result| AppEv::Effect {
        request: 0,
        done: Done::VoiceSettings(Box::new(Report { request, result })),
    };
    let catalog = |current| Catalog {
        voices: vec![Voice::Cove, Voice::Sol],
        current: Some(current),
    };
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Settlement {
        Saved,
        Refused,
        Retired,
        ReaderReplaced,
    }
    for (operation, answer_for_b) in [
        (Operation::ReadCatalog, Answer::Catalog(catalog(Voice::Sol))),
        (Operation::ReadAudio, Answer::Audio(Audio::default())),
    ] {
        for planned in [
            Settlement::Saved,
            Settlement::Refused,
            Settlement::Retired,
            Settlement::ReaderReplaced,
        ] {
            let mut app = ready();
            let primary = BackendId::default();
            let secondary = BackendId(1);
            app.attach_backend(secondary);
            apply_app_from(&mut app, secondary, AppEv::Connected, &mut AppFx::default());
            let a = ThreadId::new("thread-1");
            let b = ThreadId::new("thread-b");
            let previous = run(
                &mut app,
                event(SettingsIntent::Read {
                    thread: a.clone(),
                    operation: Operation::ReadCatalog,
                }),
            );
            let first = take(&previous).expect("A catalog");
            run(
                &mut app,
                answer(first.clone(), Ok(Answer::Catalog(catalog(Voice::Cove)))),
            );
            let save = run(
                &mut app,
                event(SettingsIntent::Persist {
                    origin: first.origin,
                    change: Change::Voice(Voice::Sol),
                }),
            );
            let save = take(&save).expect("A write");

            open_thread(&mut app, b.as_str(), secondary, true);
            let old_reader = app.epoch_of(secondary);
            let waiting = run(
                &mut app,
                event(SettingsIntent::Read {
                    thread: b.clone(),
                    operation: operation.clone(),
                }),
            );
            assert!(take(&waiting).is_none());
            assert!(app.screens().voice.settings.pending(&b));
            if planned == Settlement::ReaderReplaced {
                app.ingest(Ingress::transport_replaced(
                    secondary,
                    old_reader,
                    old_reader.next(),
                ))
                .expect("replace the reader's connection");
            }
            let (settled, expected) = match planned {
                Settlement::Saved | Settlement::ReaderReplaced => (
                    run(
                        &mut app,
                        answer(
                            save,
                            Ok(Answer::VoiceSaved {
                                catalog: catalog(Voice::Sol),
                                warning: None,
                            }),
                        ),
                    ),
                    Feedback::VoiceSaved(Voice::Sol),
                ),
                Settlement::Refused => {
                    let problem = Problem::WriteVoice("refused".into());
                    (
                        run(&mut app, answer(save, Err(problem.clone()))),
                        Feedback::Failed(problem),
                    )
                }
                Settlement::Retired => {
                    let lost = save.origin.address.epoch;
                    let fx = app
                        .ingest(Ingress::transport_replaced(primary, lost, lost.next()))
                        .expect("the writer retired");
                    assert!(
                        run(
                            &mut app,
                            answer(
                                save,
                                Ok(Answer::VoiceSaved {
                                    catalog: catalog(Voice::Sol),
                                    warning: None
                                })
                            )
                        )
                        .commands()
                        .is_empty()
                    );
                    (fx, Feedback::Failed(Problem::ConnectionLost))
                }
            };
            let read = take(&settled).expect("B's queued request is released");
            assert_eq!(read.origin.thread, b);
            assert_eq!(read.operation, operation);
            assert_eq!(
                read.origin.address.epoch != old_reader,
                planned == Settlement::ReaderReplaced
            );
            let acknowledged = |thread: &ThreadId| {
                app.thread(thread).expect("held thread").transcript().iter().any(|(_, item)| {
                matches!(item, Item::Note(note) if note.key == NoteKey::VoiceSettings(expected.clone()))
            })
            };
            assert!(acknowledged(&a));
            assert!(!acknowledged(&b));
            let executor = if operation.local() {
                primary
            } else {
                secondary
            };
            apply_app_from(
                &mut app,
                executor,
                answer(read, Ok(answer_for_b.clone())),
                &mut AppFx::default(),
            );
            let model = &app.screens().voice.settings;
            let settings = model.snapshot(&b).expect("B owns the visible settings");
            assert!(!model.pending(&b));
            assert_eq!(
                settings.catalog.is_some(),
                operation == Operation::ReadCatalog
            );
            assert_eq!(settings.audio.is_some(), operation == Operation::ReadAudio);
        }
    }
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

fn flat(thread: &str, role: &str, text: &str, done: bool) -> AppEv {
    if done {
        note(
            "thread/realtime/transcript/done",
            serde_json::json!({"threadId": thread, "role": role, "text": text}),
        )
    } else {
        note(
            "thread/realtime/transcript/delta",
            serde_json::json!({"threadId": thread, "role": role, "delta": text}),
        )
    }
}

fn stamped(app: &mut CodexApp, physical: ConnEpoch, event: AppEv) -> Option<AppFx> {
    let backend = BackendId::default();
    let result = app.ingest(Ingress::transport_event(
        backend,
        app.epoch_of(backend),
        physical,
        event,
    ));
    app.assert_consistent();
    result
}

fn publisher(app: &mut CodexApp, physical: ConnEpoch) -> Option<AppFx> {
    app.ingest(Ingress::legacy_voice_publisher(app.epoch(), physical))
}

fn offer_for(app: &mut CodexApp, thread: &str) -> (AppFx, String) {
    let prepare = preparing_for(app, thread);
    offered(app, &prepare)
}

fn confirm_voice(app: &mut CodexApp, thread: &str, epoch: ConnEpoch, session: &str, start: &AppFx) {
    stamped(app, epoch, started(thread, session)).expect("stamped Started");
    let sdp = stamped(app, epoch, answer(thread)).expect("stamped SDP");
    run(app, media(&sdp, AudioResult::Connected(Ok(()))));
    run(app, ack(start));
    assert!(!display(app).expect("live call").connecting);
}

#[test]
fn ordered_legacy_keeps_speaker_rows_and_repeated_finals_until_stamped_closed() {
    let mut app = ready();
    let thread = "thread-2";
    let backend = BackendId::default();
    let epoch = app.epoch();
    open_thread(&mut app, thread, backend, false);
    publisher(&mut app, epoch).expect("pinned source");
    let (start, session) = offer_for(&mut app, thread);
    stamped(&mut app, epoch, started(thread, "old call"));
    stamped(&mut app, epoch, flat(thread, "user", "quarantined", true));
    run(&mut app, started(thread, &session));
    stamped(
        &mut app,
        epoch,
        flat(thread, "user", "unproved start", true),
    );
    assert!(captions(&app, thread).is_empty());

    confirm_voice(&mut app, thread, epoch, &session, &start);
    run(&mut app, flat(thread, "user", "unstamped", true));
    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "thinking", false),
    );
    stamped(&mut app, epoch, flat(thread, "user", "same phrase", false));
    stamped(
        &mut app,
        epoch,
        caption_done(thread, &session, "keyed", "canonical duplicate"),
    );
    assert_eq!(captions(&app, thread), ["thinking", "same phrase"]);
    stamped(&mut app, epoch, flat(thread, "user", "same phrase", true));
    stamped(&mut app, epoch, flat(thread, "assistant", "answer", true));
    stamped(&mut app, epoch, flat(thread, "user", "same phrase", true));
    assert_eq!(
        captions(&app, thread),
        ["answer", "same phrase", "same phrase"]
    );

    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "unfinished", false),
    );
    let stop = intent_for(&mut app, thread, ThreadIntent::Stop);
    run(&mut app, closed(thread));
    assert_eq!(
        captions(&app, thread).last().map(String::as_str),
        Some("unfinished")
    );
    stamped(&mut app, epoch, closed(thread));
    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "after close", true),
    );
    assert_eq!(
        captions(&app, thread),
        ["answer", "same phrase", "same phrase", "unfinished"]
    );
    run(&mut app, ack(&stop));
    let (replacement, new_session) = offer_for(&mut app, thread);
    assert_ne!(new_session, session);
    confirm_voice(&mut app, thread, epoch, &new_session, &replacement);
    stamped(&mut app, epoch, flat(thread, "user", "after restart", true));
    assert_eq!(
        captions(&app, thread).last().map(String::as_str),
        Some("after restart")
    );
}

#[test]
fn replacement_requires_a_new_publisher_fact_for_a_legacy_thread() {
    let mut app = ready();
    let backend = BackendId::default();
    let epoch = app.epoch();
    let thread = "thread-2";
    open_thread(&mut app, thread, backend, false);
    publisher(&mut app, epoch).expect("pinned source");
    let (first, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, epoch, &session, &first);
    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "before replacement", false),
    );
    assert_eq!(captions(&app, thread), ["before replacement"]);

    app.ingest(Ingress::transport_replaced(backend, epoch, epoch.next()))
        .expect("new physical transport");
    assert!(captions(&app, thread).is_empty());
    assert!(publisher(&mut app, epoch).is_none());
    assert!(stamped(&mut app, epoch, closed(thread)).is_none());
    assert!(
        stamped(
            &mut app,
            epoch,
            flat(thread, "user", "retired packet", true)
        )
        .is_none()
    );

    let physical = epoch.next();
    open_thread(&mut app, thread, backend, false);
    let (second, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, physical, &session, &second);
    stamped(
        &mut app,
        physical,
        flat(thread, "user", "unknown publisher", true),
    );
    assert!(captions(&app, thread).is_empty());

    publisher(&mut app, physical).expect("new publisher proof");
    stamped(
        &mut app,
        physical,
        flat(thread, "user", "mid-call proof", true),
    );
    assert!(captions(&app, thread).is_empty());
    stamped(&mut app, physical, closed(thread));

    let (third, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, physical, &session, &third);
    stamped(
        &mut app,
        physical,
        flat(thread, "user", "new publisher", true),
    );
    assert_eq!(captions(&app, thread), ["new publisher"]);
}

#[test]
fn ordered_publisher_keeps_paginated_captions_canonical_and_discards_flat_duplicates() {
    let mut app = ready();
    let backend = BackendId::default();
    let epoch = app.epoch();
    let thread = "thread-2";
    open_thread(&mut app, thread, backend, true);
    publisher(&mut app, epoch).expect("pinned source");
    let (start, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, epoch, &session, &start);
    stamped(&mut app, epoch, flat(thread, "user", "one answer", true));
    for _ in 0..2 {
        stamped(
            &mut app,
            epoch,
            caption_done(thread, &session, "keyed", "one answer"),
        );
    }
    assert_eq!(captions(&app, thread), ["one answer"]);
}

#[test]
fn a_notification_gap_discards_legacy_previews_and_locks_that_call() {
    let mut app = ready();
    let epoch = app.epoch();
    let thread = "thread-2";
    open_thread(&mut app, thread, BackendId::default(), false);
    publisher(&mut app, epoch).expect("pinned source");
    let (start, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, epoch, &session, &start);
    stamped(&mut app, epoch, flat(thread, "assistant", "partial", false));
    assert_eq!(captions(&app, thread), ["partial"]);
    stamped(&mut app, epoch, AppEv::Backend(BackendEv::Lagged(1)));
    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "uncertain final", true),
    );
    assert!(captions(&app, thread).is_empty());
    stamped(&mut app, epoch, closed(thread));
    let (start, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, epoch, &session, &start);
    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "next call", true),
    );
    assert_eq!(captions(&app, thread), ["next call"]);
}

#[test]
fn a_new_ephemeral_chat_receives_flat_captions_even_with_a_paginated_hint() {
    use codex_tui_core::threads::ThreadReply;

    let mut app = CodexApp::new();
    let epoch = app.epoch();
    let thread = "temporary";
    publisher(&mut app, epoch).expect("pinned source");
    let boot = run(&mut app, AppEv::Connected);
    let ticket = boot
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .expect("a new session request");
    run(
        &mut app,
        AppEv::Reply(
            ticket,
            Reply::thread(ThreadReply::SessionStarted {
                thread: ThreadId::new(thread),
                details: Some(Box::new(voice_thread_details(thread, true, true))),
                sources: Vec::new(),
                permissions: None,
            }),
        ),
    );
    assert_eq!(app.session().active, Some(ThreadId::new(thread)));
    let (start, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, epoch, &session, &start);
    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "unsaved dialogue", true),
    );
    assert_eq!(captions(&app, thread), ["unsaved dialogue"]);
}

#[test]
fn ordered_close_seals_both_partial_speakers_without_repeating_a_done_item() {
    use codex_tui_core::screens::{ScreenEv, chat};

    let mut app = ready();
    let epoch = app.epoch();
    let thread = "thread-2";
    open_thread(&mut app, thread, BackendId::default(), false);
    publisher(&mut app, epoch).expect("pinned source");
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Chat(chat::Ev::Status(Box::new(
            chat::StatusFacts {
                animations: false,
                ..chat::StatusFacts::default()
            },
        )))),
    );
    let (start, session) = offer_for(&mut app, thread);
    confirm_voice(&mut app, thread, epoch, &session, &start);

    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "already completed", true),
    );
    stamped(&mut app, epoch, flat(thread, "assistant", "first ", false));
    stamped(&mut app, epoch, flat(thread, "user", "a correction", false));
    stamped(&mut app, epoch, flat(thread, "assistant", "answer", false));
    assert_eq!(
        captions(&app, thread),
        ["already completed", "first answer"]
    );

    stamped(&mut app, epoch, closed(thread));
    stamped(&mut app, epoch, flat(thread, "user", "a correction", true));
    stamped(
        &mut app,
        epoch,
        flat(thread, "assistant", "first answer", true),
    );
    assert_eq!(
        captions(&app, thread),
        ["already completed", "a correction", "first answer"]
    );
}
