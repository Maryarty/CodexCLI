//! The writer lifecycle feeding the live composer for a retained conversation.

use codex_app_server_protocol::ErrorNotification;
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Intent, apply_app_from};
use codex_tui_core::ids::{BackendId, ThreadId, Ticket};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::{ScreenCommand, ScreenEv, agents};
use codex_tui_core::threads::ThreadAsk;
use codex_tui_core::threads::composer::{ComposerState, composer_state};

const LOCAL: BackendId = BackendId(0);
const CLOUD: BackendId = BackendId(1);

fn run(app: &mut CodexApp, from: BackendId, ev: AppEv) -> AppFx {
    let mut fx = AppFx::default();
    apply_app_from(app, from, ev, &mut fx);
    fx
}

fn event(ev: agents::Ev) -> AppEv {
    AppEv::Screen(ScreenEv::Agents(ev))
}

fn started(fx: &AppFx) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|c| match c {
            Command::StartThread { ticket, .. } => Some(*ticket),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no thread/start in {:?}", fx.commands()))
}

fn open(app: &mut CodexApp, thread: &ThreadId) -> AppFx {
    run(
        app,
        LOCAL,
        AppEv::Intent(Intent::OpenThread(thread.clone())),
    )
}

fn resume(fx: &AppFx, id: &ThreadId) -> Ticket {
    fx.commands()
        .iter()
        .find_map(|c| match c {
            Command::Screen {
                ticket,
                command:
                    ScreenCommand::Agents(agents::Command::Thread(ThreadAsk::Resume { thread })),
                ..
            } if thread == id.as_str() => Some(*ticket),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no thread/resume in {:?}", fx.commands()))
}

fn stream_lost(thread: ThreadId) -> AppEv {
    let error: ErrorNotification = serde_json::from_value(serde_json::json!({
        "threadId": thread.as_str(),
        "turnId": "",
        "willRetry": false,
        "error": {"message": "Cloud stream ended.", "codexErrorInfo": null, "additionalDetails": null}
    })).unwrap_or_else(|error| panic!("stream loss: {error}"));
    AppEv::Backend(BackendEv::StreamLost {
        thread,
        error: Box::new(error),
    })
}

fn cloud_chat() -> (CodexApp, ThreadId, ThreadId) {
    let local = ThreadId::new("local-1");
    let cloud = ThreadId::new("cloud-1");
    let mut app = CodexApp::new();
    let fx = run(&mut app, LOCAL, AppEv::Connected);
    run(
        &mut app,
        LOCAL,
        AppEv::Reply(started(&fx), Reply::thread_started(local.clone())),
    );
    run(&mut app, CLOUD, event(agents::Ev::CloudBackend(CLOUD)));
    run(&mut app, CLOUD, AppEv::Connected);
    let fx = run(&mut app, LOCAL, AppEv::Intent(Intent::CycleBackend));
    run(
        &mut app,
        CLOUD,
        AppEv::Reply(started(&fx), Reply::thread_started(cloud.clone())),
    );
    assert_eq!(app.session().active.as_ref(), Some(&cloud));
    (app, local, cloud)
}

#[test]
fn a_cloud_attach_failure_shows_the_retained_thread_after_the_connection_is_up() {
    let (mut app, local, cloud) = cloud_chat();
    run(&mut app, CLOUD, stream_lost(cloud.clone()));
    open(&mut app, &local);

    let pending = resume(&open(&mut app, &cloud), &cloud);
    run(&mut app, CLOUD, event(agents::Ev::Reconnecting));
    run(
        &mut app,
        CLOUD,
        AppEv::Reply(pending, Reply::failed("old connection")),
    );
    assert_eq!(app.session().active.as_ref(), Some(&local));

    run(&mut app, CLOUD, event(agents::Ev::Reconnected));
    let retry = resume(&open(&mut app, &cloud), &cloud);
    run(
        &mut app,
        CLOUD,
        AppEv::Reply(retry, Reply::failed("conversation no longer available")),
    );
    assert_eq!(app.session().active.as_ref(), Some(&cloud));
    assert_eq!(
        composer_state(&app.active_thread(), false),
        ComposerState::Unavailable
    );
}

#[test]
fn a_failed_history_refresh_does_not_remove_the_live_writer() {
    let (mut app, _, cloud) = cloud_chat();
    run(&mut app, CLOUD, stream_lost(cloud.clone()));
    run(
        &mut app,
        CLOUD,
        AppEv::Backend(BackendEv::StreamRestored(cloud.clone())),
    );
    let refresh = resume(&open(&mut app, &cloud), &cloud);
    run(
        &mut app,
        CLOUD,
        AppEv::Reply(refresh, Reply::failed("refresh failed")),
    );
    assert_eq!(app.session().active.as_ref(), Some(&cloud));
    assert_eq!(
        composer_state(&app.active_thread(), false),
        ComposerState::Open
    );
}

#[test]
fn an_embedded_resume_failure_keeps_its_ordinary_failure_path() {
    let thread = ThreadId::new("local-1");
    let mut app = CodexApp::new();
    let fx = run(&mut app, LOCAL, AppEv::Connected);
    run(
        &mut app,
        LOCAL,
        AppEv::Reply(started(&fx), Reply::thread_started(thread.clone())),
    );
    run(&mut app, LOCAL, event(agents::Ev::Reconnecting));
    let fx = run(&mut app, LOCAL, event(agents::Ev::Reconnected));
    let retry = resume(&fx, &thread);
    run(
        &mut app,
        LOCAL,
        AppEv::Reply(retry, Reply::failed("conversation no longer available")),
    );
    assert_eq!(
        composer_state(&app.active_thread(), false),
        ComposerState::Disabled
    );
    assert!(matches!(
        app.screens().agents.notice,
        Some(agents::Notice::Error(_))
    ));
}
