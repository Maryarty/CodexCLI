//! The account profile of the Overview (oracle: `analytics/summary.rs`,
//! `codex_backend_client::AccountProfile`): what the backend reports about
//! the account's token use, kept as the optional fields it sends so the view
//! can tell "unreported" (a dash) from zero. The host decodes the backend's
//! type into this one; the seed form is the same shape.

use super::date::NaiveDate;

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct Profile {
    #[serde(default, rename = "profile")]
    pub identity: Option<Identity>,
    #[serde(default)]
    pub metadata: Option<Metadata>,
    #[serde(default)]
    pub stats: Stats,
}

// The numbers come from JSON numbers, so none is NaN.
impl Eq for Profile {}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Identity {
    pub display_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Metadata {
    /// `YYYY-MM-DD`.
    pub stats_as_of: Option<String>,
    pub stats_error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
pub struct Stats {
    pub lifetime_tokens: Option<i64>,
    pub peak_daily_tokens: Option<i64>,
    pub longest_running_turn_sec: Option<i64>,
    pub current_streak_days: Option<i64>,
    pub longest_streak_days: Option<i64>,
    pub daily_usage_buckets: Option<Vec<Bucket>>,
    pub fast_mode_usage_percentage: Option<f64>,
    pub most_used_reasoning_effort: Option<String>,
    pub most_used_reasoning_effort_percentage: Option<f64>,
    pub unique_skills_used: Option<u64>,
    pub total_skills_used: Option<u64>,
    pub total_threads: Option<u64>,
    pub top_invocations: Option<Vec<Invocation>>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Bucket {
    /// `YYYY-MM-DD`.
    pub start_date: String,
    pub tokens: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Invocation {
    #[serde(rename = "type")]
    pub kind: InvocationKind,
    pub plugin_name: Option<String>,
    pub skill_name: Option<String>,
    pub usage_count: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationKind {
    Plugin,
    Skill,
    #[serde(other)]
    Unknown,
}

impl Metadata {
    /// The day the statistics are as of, when the backend sent one.
    pub fn as_of(&self) -> Option<NaiveDate> {
        self.stats_as_of.as_deref().and_then(NaiveDate::parse_ymd)
    }
}

/// `1,142` (oracle: `summary_panel.rs` `count_value`).
pub fn count_value(value: Option<u64>) -> String {
    let Some(value) = value else {
        return "\u{2014}".into();
    };
    let digits = value.to_string();
    digits
        .chars()
        .enumerate()
        .fold(String::new(), |mut result, (index, ch)| {
            if index > 0 && (digits.len() - index).is_multiple_of(3) {
                result.push(',');
            }
            result.push(ch);
            result
        })
}

/// `37%`, or a dash outside 0..=100 (oracle: `summary_panel.rs` `percent`).
pub fn percent(value: Option<f64>) -> String {
    value
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        .map(|value| format!("{value:.0}%"))
        .unwrap_or_else(|| "\u{2014}".into())
}

/// `44h 7m` (oracle: `summary_panel.rs` `duration`).
pub fn duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    let (hours, minutes) = (seconds / 3600, seconds % 3600 / 60);
    match (hours, minutes) {
        (0, 0) => format!("{seconds}s"),
        (0, minutes) => format!("{minutes}m"),
        (hours, 0) => format!("{hours}h"),
        (hours, minutes) => format!("{hours}h {minutes}m"),
    }
}
