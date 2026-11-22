//! A turn's runtime metrics, the totals the completion separator prints after the duration
//! (oracle `codex_otel::RuntimeMetricsSummary`, the fields `runtime_metrics_label` reads). The
//! host converts the session telemetry and sends the totals so far per turn.

use serde::Deserialize;

/// A count of events and the time they took.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Totals {
    pub count: u64,
    pub duration_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct RuntimeMetrics {
    pub tool_calls: Totals,
    pub api_calls: Totals,
    pub streaming_events: Totals,
    pub websocket_calls: Totals,
    pub websocket_events: Totals,
    pub responses_api_overhead_ms: u64,
    pub responses_api_inference_time_ms: u64,
    pub responses_api_engine_iapi_ttft_ms: u64,
    pub responses_api_engine_service_ttft_ms: u64,
    pub responses_api_engine_iapi_tbt_ms: f64,
    pub responses_api_engine_service_tbt_ms: f64,
}

// The two time-between-tokens fields are finite milliseconds from the telemetry, never NaN.
impl Eq for RuntimeMetrics {}
