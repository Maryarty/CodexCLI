//! A turn's runtime metrics, the totals the completion separator prints after the duration
//! (oracle `codex_otel::RuntimeMetricsSummary`, the fields `runtime_metrics_label` reads). The
//! domain input is a sample from the WebSocket response owned by the addressed turn.

use codex_app_server_protocol::TurnRuntimeMetricsNotification;
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

impl RuntimeMetrics {
    /// The display follows the oracle's millisecond rounding for durations and
    /// keeps fractional time between tokens until the view formats it.
    pub(super) fn from_websocket(sample: &TurnRuntimeMetricsNotification) -> Self {
        let ms = |value: Option<f64>| {
            value
                .filter(|ms| ms.is_finite() && *ms > 0.0)
                .unwrap_or_default()
        };
        let duration = |value| ms(value).round() as u64;
        Self {
            responses_api_overhead_ms: duration(sample.overhead_ms),
            responses_api_inference_time_ms: duration(sample.inference_time_ms),
            responses_api_engine_iapi_ttft_ms: duration(sample.iapi_ttft_ms),
            responses_api_engine_service_ttft_ms: duration(sample.service_ttft_ms),
            responses_api_engine_iapi_tbt_ms: ms(sample.iapi_tbt_ms),
            responses_api_engine_service_tbt_ms: ms(sample.service_tbt_ms),
            ..Self::default()
        }
    }

    /// An absent reading keeps the last positive value of that field.
    pub(super) fn observe_websocket(&mut self, sample: Self) {
        let Self {
            responses_api_overhead_ms,
            responses_api_inference_time_ms,
            responses_api_engine_iapi_ttft_ms,
            responses_api_engine_service_ttft_ms,
            responses_api_engine_iapi_tbt_ms,
            responses_api_engine_service_tbt_ms,
            ..
        } = sample;
        if responses_api_overhead_ms > 0 {
            self.responses_api_overhead_ms = responses_api_overhead_ms;
        }
        if responses_api_inference_time_ms > 0 {
            self.responses_api_inference_time_ms = responses_api_inference_time_ms;
        }
        if responses_api_engine_iapi_ttft_ms > 0 {
            self.responses_api_engine_iapi_ttft_ms = responses_api_engine_iapi_ttft_ms;
        }
        if responses_api_engine_service_ttft_ms > 0 {
            self.responses_api_engine_service_ttft_ms = responses_api_engine_service_ttft_ms;
        }
        if responses_api_engine_iapi_tbt_ms > 0.0 {
            self.responses_api_engine_iapi_tbt_ms = responses_api_engine_iapi_tbt_ms;
        }
        if responses_api_engine_service_tbt_ms > 0.0 {
            self.responses_api_engine_service_tbt_ms = responses_api_engine_service_tbt_ms;
        }
    }
}
