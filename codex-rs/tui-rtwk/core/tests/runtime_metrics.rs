#![allow(clippy::expect_used)]

use codex_app_server_protocol::{ServerNotification, TurnRuntimeMetricsNotification};
use codex_tui_core::app::{AppEv, AppFx, BackendEv, CodexApp, Command, Ingress, Intent, apply_app};
use codex_tui_core::fixtures::builders;
use codex_tui_core::ids::{BackendId, ConnEpoch, ThreadId, TurnId};
use codex_tui_core::rpc::Reply;
use codex_tui_core::screens::cells::RuntimeMetrics;
use codex_tui_core::screens::{ScreenEv, agents};
use codex_tui_core::threads::{Item, ThreadModel, TurnTiming};
use pretty_assertions::assert_eq;

const LOCAL: BackendId = BackendId(0);
const CLOUD: BackendId = BackendId(1);

fn run(app: &mut CodexApp, event: AppEv) -> AppFx {
    let mut effects = AppFx::default();
    apply_app(app, event, &mut effects);
    effects
}

fn start(app: &mut CodexApp, event: AppEv, backend: BackendId, thread: &str) {
    let ticket = run(app, event)
        .commands()
        .iter()
        .find_map(|command| match command {
            Command::StartThread {
                ticket,
                backend: owner,
                ..
            } if *owner == backend => Some(*ticket),
            _ => None,
        })
        .expect("start routed on the known backend");
    run(
        app,
        AppEv::Reply(ticket, Reply::thread_started(ThreadId::new(thread))),
    );
}

fn notify(app: &mut CodexApp, backend: BackendId, ev: BackendEv) {
    let epoch = app.epoch_of(backend);
    assert!(
        app.ingest(Ingress::of(backend, epoch, AppEv::Backend(ev)))
            .is_some()
    );
}

fn sample(thread: &str, turn: &str, iapi: Option<f64>, service: Option<f64>) -> BackendEv {
    BackendEv::Notification(Box::new(ServerNotification::TurnRuntimeMetrics(
        TurnRuntimeMetricsNotification {
            thread_id: thread.to_owned(),
            turn_id: turn.to_owned(),
            overhead_ms: None,
            inference_time_ms: None,
            iapi_ttft_ms: iapi,
            service_ttft_ms: None,
            iapi_tbt_ms: None,
            service_tbt_ms: service,
        },
    )))
}

fn thread<'a>(app: &'a CodexApp, id: &str) -> &'a ThreadModel {
    app.thread(&ThreadId::new(id)).expect("retained thread")
}

fn totals<'a>(app: &'a CodexApp, id: &str, turn: &str) -> Option<&'a RuntimeMetrics> {
    thread(app, id)
        .screen_slots()
        .cells
        .metrics(&TurnId::new(turn))
}

fn readings(app: &CodexApp, id: &str) -> Vec<(u64, f64)> {
    thread(app, id)
        .transcript()
        .iter()
        .filter_map(|(_, item)| match item {
            Item::WebSocketTiming(metrics) => Some((
                metrics.responses_api_engine_iapi_ttft_ms,
                metrics.responses_api_engine_service_tbt_ms,
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn addressed_websocket_timing_keeps_live_rows_and_scopes_the_latest_fields() {
    let mut app = CodexApp::new();
    let local = app.attach();
    start(&mut app, AppEv::Connected, LOCAL, "local-1");
    let cloud = app.attach_backend(CLOUD);
    run(
        &mut app,
        AppEv::Screen(ScreenEv::Agents(agents::Ev::CloudBackend(CLOUD))),
    );
    start(
        &mut app,
        AppEv::Intent(Intent::CycleBackend),
        CLOUD,
        "cloud-1",
    );
    assert_eq!(app.active_thread().id(), &ThreadId::new("cloud-1"));

    // Both backends can independently issue the same opaque turn ID.
    notify(&mut app, LOCAL, builders::turn_started("local-1", "turn"));
    notify(&mut app, CLOUD, builders::turn_started("cloud-1", "turn"));
    notify(
        &mut app,
        CLOUD,
        sample("local-1", "turn", Some(777.0), None),
    );
    notify(
        &mut app,
        LOCAL,
        sample("local-1", "WRONG", Some(777.0), None),
    );
    notify(
        &mut app,
        LOCAL,
        sample("unknown", "turn", Some(777.0), None),
    );
    assert!(app.thread(&ThreadId::new("unknown")).is_none());
    assert_eq!(app.screens().agents.owner_of("unknown"), CLOUD);

    notify(
        &mut app,
        LOCAL,
        sample("local-1", "turn", Some(120.0), Some(50.0)),
    );
    notify(
        &mut app,
        CLOUD,
        sample("cloud-1", "turn", Some(220.0), Some(70.0)),
    );
    notify(
        &mut app,
        LOCAL,
        sample("local-1", "turn", Some(80.0), Some(0.0)),
    );
    notify(
        &mut app,
        LOCAL,
        sample("local-1", "turn", Some(f64::NAN), Some(-1.0)),
    );

    assert_eq!(readings(&app, "local-1"), [(120, 50.0), (80, 0.0)]);
    assert_eq!(readings(&app, "cloud-1"), [(220, 70.0)]);
    let transcript = thread(&app, "local-1").transcript();
    assert_eq!(
        transcript.timing_after(1),
        None,
        "running turn has no separator"
    );

    notify(&mut app, LOCAL, builders::turn_completed("local-1", "turn"));
    notify(
        &mut app,
        LOCAL,
        sample("local-1", "turn", Some(777.0), Some(777.0)),
    );
    notify(&mut app, LOCAL, builders::turn_completed("local-1", "turn"));
    assert_eq!(readings(&app, "local-1"), [(120, 50.0), (80, 0.0)]);
    let metrics = totals(&app, "local-1", "turn").expect("local turn totals");
    assert_eq!(metrics.responses_api_engine_iapi_ttft_ms, 80);
    assert_eq!(metrics.responses_api_engine_service_tbt_ms, 50.0);
    assert_eq!(
        thread(&app, "local-1").transcript().timing_after(1),
        Some(TurnTiming {
            duration_ms: None,
            completed_at: None
        })
    );

    // Replacing the physical handle detaches its prior live turn.
    assert!(
        app.ingest(Ingress::transport_replaced(CLOUD, cloud, cloud.next()))
            .is_some()
    );
    assert!(
        app.ingest(Ingress::transport_event(
            CLOUD,
            cloud,
            cloud,
            AppEv::Backend(sample("cloud-1", "turn", Some(777.0), None)),
        ))
        .is_none()
    );
    let current =
        |event| Ingress::transport_event(CLOUD, cloud, cloud.next(), AppEv::Backend(event));
    assert!(
        app.ingest(current(sample("cloud-1", "turn", Some(777.0), None)))
            .is_some()
    );
    assert!(
        app.ingest(current(builders::turn_started(
            "cloud-1",
            "replacement-turn"
        )))
        .is_some()
    );
    assert!(
        app.ingest(current(sample(
            "cloud-1",
            "replacement-turn",
            Some(180.0),
            None
        )))
        .is_some()
    );
    let cloud_metrics = totals(&app, "cloud-1", "turn").expect("original cloud totals");
    assert_eq!(cloud_metrics.responses_api_engine_iapi_ttft_ms, 220);
    assert_eq!(cloud_metrics.responses_api_engine_service_tbt_ms, 70.0);
    let replacement =
        totals(&app, "cloud-1", "replacement-turn").expect("replacement's own totals");
    assert_eq!(replacement.responses_api_engine_iapi_ttft_ms, 180);
    assert_eq!(replacement.responses_api_engine_service_tbt_ms, 0.0);

    let next_local: ConnEpoch = app.attach();
    assert_ne!(next_local, local);
    assert!(
        app.ingest(Ingress::of(
            LOCAL,
            local,
            AppEv::Backend(sample("local-1", "turn", Some(777.0), None))
        ))
        .is_none()
    );
    assert_eq!(readings(&app, "local-1"), [(120, 50.0), (80, 0.0)]);
}

#[test]
fn failed_without_final_answer_and_old_backend_keep_their_own_completion_behavior() {
    let mut app = CodexApp::new();
    app.attach();
    start(&mut app, AppEv::Connected, LOCAL, "local-1");
    notify(&mut app, LOCAL, builders::turn_started("local-1", "failed"));
    notify(
        &mut app,
        LOCAL,
        sample("local-1", "failed", Some(120.0), None),
    );
    notify(&mut app, LOCAL, builders::turn_failed("local-1", "failed"));
    notify(
        &mut app,
        LOCAL,
        sample("local-1", "failed", Some(999.0), None),
    );
    assert_eq!(readings(&app, "local-1"), [(120, 0.0)]);
    assert_eq!(thread(&app, "local-1").transcript().timing_after(0), None);
    notify(
        &mut app,
        LOCAL,
        builders::turn_started("local-1", "no-samples"),
    );
    notify(
        &mut app,
        LOCAL,
        builders::turn_completed("local-1", "no-samples"),
    );
    assert!(totals(&app, "local-1", "no-samples").is_none());
}
