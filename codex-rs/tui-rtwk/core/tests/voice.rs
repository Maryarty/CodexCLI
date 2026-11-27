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

fn active(app: &mut CodexApp) {
    let prepared = preparing(app);
    let (start, session) = offered(app, &prepared);
    run(app, started("thread-1", &session));
    let connect = run(app, answer("thread-1"));
    run(app, media(&connect, AudioResult::Connected(Ok(()))));
    run(app, ack(&start));
    assert!(display(app).is_some_and(|display| !display.connecting));
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
