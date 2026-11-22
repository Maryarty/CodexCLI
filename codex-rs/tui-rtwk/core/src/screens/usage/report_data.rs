//! Projects endpoint-specific backend models into inputs for display normalization.
//! This representation is never used to deserialize HTTP responses.
//! Optional fields select the inputs used by each report; missing backend values stay missing.


/// Report values projected for normalization after endpoint-specific decoding.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsData {
    pub data: Vec<AnalyticsRecord>,
    pub units: Option<String>,
    pub data_freshness_ts: Option<String>,
    pub series: Option<Vec<AnalyticsSeries>>,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsSeries {
    pub key: String,
    pub label: String,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsRecord {
    pub date: String,
    pub values: Option<std::collections::BTreeMap<String, f64>>,
    pub product_surface_usage_values: Option<std::collections::BTreeMap<String, f64>>,
    pub premium_usage_values: Option<PremiumUsage>,
    pub attribution: Option<Vec<UsageAttribution>>,
    pub credit_amount: Option<f64>,
    pub product_surface: Option<String>,
    pub totals: Option<AnalyticsCount>,
    pub models: Option<Vec<AnalyticsModel>>,
    pub groups: Option<Vec<AnalyticsTokenGroup>>,
    pub clients: Option<Vec<AnalyticsClient>>,
    pub plugin_usage_overviews: Option<Vec<AnalyticsTool>>,
    pub skill_usage_overviews: Option<Vec<AnalyticsTool>>,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct PremiumUsage {
    pub credit_usage_credits: std::collections::BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct UsageAttribution {
    pub thread_source: Option<String>,
    pub turn_trigger: Option<String>,
    pub model: Option<String>,
    pub surface: Option<String>,
    pub value: f64,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsCount {
    pub turns: f64,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsModel {
    pub model: String,
    pub credits: Option<f64>,
    pub uncached_text_input_tokens: Option<f64>,
    pub cached_text_input_tokens: Option<f64>,
    pub text_output_tokens: Option<f64>,
    pub text_total_tokens: Option<f64>,
    pub turns: Option<f64>,
    pub on_demand_credits: Option<f64>,
    pub speed: Option<String>,
}

/// Enterprise model groups carry text token counts independently of billing amounts.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsTokenGroup {
    pub dimensions: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub is_other: bool,
    pub uncached_text_input_tokens: Option<f64>,
    pub cached_text_input_tokens: Option<f64>,
    pub text_output_tokens: Option<f64>,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsClient {
    pub client_id: String,
    pub turns: f64,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
pub struct AnalyticsTool {
    pub display_name: String,
    pub invocation_counts: f64,
}

