use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use codex_app_server_protocol::ClientInfo;
use codex_app_server_protocol::InitializeCapabilities;
use codex_app_server_protocol::JSONRPCMessage;
use codex_app_server_protocol::ThreadStartParams;
use codex_app_server_protocol::TurnStartParams;
use codex_app_server_protocol::TurnStartResponse;
use codex_app_server_protocol::UserInput;
use codex_features::Feature;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

const READ_TIMEOUT: Duration = Duration::from_secs(20);
const METHOD: &str = "turn/runtimeMetrics";

#[test_case::test_case(true, true, false; "enabled")]
#[test_case::test_case(false, true, false; "feature_disabled")]
#[test_case::test_case(true, false, false; "legacy_client")]
#[test_case::test_case(true, true, true; "method_opt_out")]
#[tokio::test]
async fn websocket_runtime_metrics_follow_the_owning_turn(
    feature: bool,
    experimental_api: bool,
    opt_out: bool,
) -> Result<()> {
    skip_if_no_network!(Ok(()));
    let sample = |iapi_ttft| {
        json!({
            "type": "responsesapi.websocket_timing",
            "timing_metrics": {
                "responses_duration_excl_engine_and_client_tool_time_ms": 14,
                "engine_service_total_ms": -1,
                "engine_iapi_ttft_total_ms": iapi_ttft,
                "engine_service_ttft_total_ms": "120",
                "engine_iapi_tbt_across_engine_calls_ms": 0,
                "engine_service_tbt_across_engine_calls_ms": 2.5
            }
        })
    };
    let connection = |response_id: &str, iapi_ttft| {
        let warmup_id = format!("{response_id}-warmup");
        vec![
            vec![
                responses::ev_response_created(&warmup_id),
                sample(999),
                responses::ev_completed(&warmup_id),
            ],
            vec![
                sample(888),
                responses::ev_response_created(response_id),
                sample(iapi_ttft),
                json!({
                    "type": "responsesapi.websocket_timing",
                    "timing_metrics": { "engine_service_total_ms": -1 }
                }),
                responses::ev_assistant_message(response_id, "Done"),
                responses::ev_completed(response_id),
            ],
        ]
    };
    let websocket = responses::start_websocket_server(vec![
        connection("first", 120),
        connection("second", 220),
    ])
    .await;
    let mut release_first = Some(websocket.gate_response_event(&sample(120)));
    let home = TempDir::new()?;
    MockResponsesConfig::new(&websocket.uri().replacen("ws://", "http://", 1))
        .with_provider_config("supports_websockets = true")
        .with_features(&BTreeMap::from([(Feature::RuntimeMetrics, feature)]))
        .write(home.path())?;
    let mut app = TestAppServer::builder()
        .with_codex_home(home.path())
        .build()
        .await?;
    let initialized = timeout(
        READ_TIMEOUT,
        app.initialize_with_capabilities(
            ClientInfo {
                name: "runtime-metrics-test".to_string(),
                title: None,
                version: "0.0.0".to_string(),
            },
            Some(InitializeCapabilities {
                experimental_api,
                opt_out_notification_methods: opt_out.then(|| vec![METHOD.to_string()]),
                ..Default::default()
            }),
        ),
    )
    .await??;
    assert!(matches!(initialized, JSONRPCMessage::Response(_)));

    let mut expected = BTreeMap::new();
    for label in ["first", "second"] {
        let thread = timeout(READ_TIMEOUT, app.start_thread(ThreadStartParams::default()))
            .await??
            .thread;
        let request = app
            .send_turn_start_request(TurnStartParams {
                thread_id: thread.id.clone(),
                input: vec![UserInput::Text {
                    text: label.to_string(),
                    text_elements: Vec::new(),
                }],
                ..Default::default()
            })
            .await?;
        let response: TurnStartResponse =
            timeout(READ_TIMEOUT, app.read_response(request)).await??;
        expected.insert(thread.id, (response.turn.id, label));
        if label == "first" {
            // The next thread has a distinct WebSocket while this response is still streaming.
            timeout(READ_TIMEOUT, websocket.wait_for_request(0, 1)).await?;
        }
    }

    let expected_visible = feature && experimental_api && !opt_out;
    let mut lifecycle = BTreeMap::<String, Vec<String>>::new();
    let mut completed = Vec::new();
    timeout(READ_TIMEOUT, async {
        while completed.len() < 2 {
            let JSONRPCMessage::Notification(notification) = app.read_next_message().await? else {
                continue;
            };
            let method = notification.method.as_str();
            if !matches!(method, "turn/started" | METHOD | "turn/completed") {
                continue;
            }
            let params = notification.params.unwrap_or(Value::Null);
            let thread_id = params["threadId"].as_str().expect("thread ID");
            let (turn_id, label) = &expected[thread_id];
            if method == METHOD {
                assert_eq!(
                    params,
                    json!({
                        "threadId": thread_id,
                        "turnId": turn_id,
                        "overheadMs": 14.0,
                        "iapiTtftMs": if *label == "first" { 120.0 } else { 220.0 },
                        "iapiTbtMs": 0.0,
                        "serviceTbtMs": 2.5
                    })
                );
            } else {
                assert_eq!(params["turn"]["id"], *turn_id);
            }
            lifecycle
                .entry(thread_id.to_string())
                .or_default()
                .push(method.to_string());
            if method == "turn/completed" {
                assert_eq!(params["turn"]["status"], "completed");
                completed.push(*label);
                if *label == "second" {
                    release_first
                        .take()
                        .expect("second completes once")
                        .send(())
                        .expect("first sample is waiting");
                }
            }
        }
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    assert_eq!(completed, ["second", "first"]);
    let expected_lifecycle = if expected_visible {
        vec!["turn/started", METHOD, "turn/completed"]
    } else {
        vec!["turn/started", "turn/completed"]
    };
    for methods in lifecycle.into_values() {
        assert_eq!(methods, expected_lifecycle);
    }
    assert_eq!(websocket.handshakes().len(), 2);
    for handshake in websocket.handshakes() {
        assert_eq!(
            handshake.header("x-responsesapi-include-timing-metrics"),
            feature.then(|| "true".to_string())
        );
    }
    websocket.shutdown().await;
    Ok(())
}
