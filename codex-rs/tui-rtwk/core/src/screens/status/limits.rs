//! The rate-limit and credits rows of the `/status` card: the raw snapshots
//! the backend sent, and the rows they make (oracle `status/rate_limits.rs`
//! `compose_rate_limit_data_many`). The rows are typed (a label kind, a
//! reset time, a credit state): the words are the view's. `now` and the
//! time zone are facts of the card, never read here.

use super::helpers::{ResetTime, reset_time};

/// Maximum age before a snapshot is stale in the card (oracle
/// `RATE_LIMIT_STALE_THRESHOLD_MINUTES`).
pub const STALE_AFTER_SECS: i64 = 15 * 60;

/// One usage window of a snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Window {
    pub used_percent: i32,
    #[serde(default)]
    pub window_minutes: Option<i64>,
    /// Unix seconds.
    #[serde(default)]
    pub resets_at: Option<i64>,
}

/// The credits of an account as the backend reports them.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Credits {
    pub has_credits: bool,
    pub unlimited: bool,
    /// The balance text as sent (`"12.5"`).
    #[serde(default)]
    pub balance: Option<String>,
}

/// The effective monthly credit limit of a workspace (spend controls).
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct SpendLimit {
    /// When this usage value was read, unix seconds; the snapshot's when absent.
    #[serde(default)]
    pub captured_at: Option<i64>,
    pub remaining_percent: i32,
    pub used: String,
    pub limit: String,
    pub resets_at: i64,
}

/// One bucket of limits (oracle `RateLimitSnapshotDisplay`), raw.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Snapshot {
    /// The bucket's name; `codex` is the plan's own usage.
    #[serde(default = "codex_bucket")]
    pub name: String,
    /// Unix seconds.
    pub captured_at: i64,
    #[serde(default)]
    pub primary: Option<Window>,
    #[serde(default)]
    pub secondary: Option<Window>,
    #[serde(default)]
    pub credits: Option<Credits>,
    #[serde(default)]
    pub individual_limit: Option<SpendLimit>,
}

fn codex_bucket() -> String {
    "codex".to_string()
}

/// The limit facts of a card: the snapshots and the moment the card read them.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Limits {
    #[serde(default)]
    pub snapshots: Vec<Snapshot>,
    /// Unix seconds when the card was made (`Local::now()` of the oracle).
    #[serde(default)]
    pub now: i64,
    /// The local time zone's offset from UTC, seconds east.
    #[serde(default)]
    pub utc_offset_secs: i32,
    /// A refresh of the limits is in flight (the notices say so).
    #[serde(default)]
    pub refreshing: bool,
}

/// The kind of a window's name: `5h`, `daily`, ... or the generic one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowName {
    FiveHours,
    Daily,
    Weekly,
    Monthly,
    Annual,
    /// A length that is none of the above: the primary one is `usage`, the
    /// secondary `secondary usage`.
    Generic { secondary: bool },
}

// oracle: chatwidget/rate_limits.rs fn limit_label_for_window / get_limits_duration
pub fn window_name(window_minutes: Option<i64>, secondary: bool) -> WindowName {
    const HOUR: i64 = 60;
    const DAY: i64 = 24 * HOUR;
    let approximately = |minutes: i64, expected: i64| {
        let (minutes, expected) = (minutes as f64, expected as f64);
        minutes >= expected * 0.95 && minutes <= expected * 1.05
    };
    let Some(minutes) = window_minutes.map(|minutes| minutes.max(0)) else {
        return WindowName::Generic { secondary };
    };
    if approximately(minutes, 5 * HOUR) {
        WindowName::FiveHours
    } else if approximately(minutes, DAY) {
        WindowName::Daily
    } else if approximately(minutes, 7 * DAY) {
        WindowName::Weekly
    } else if approximately(minutes, 30 * DAY) {
        WindowName::Monthly
    } else if approximately(minutes, 365 * DAY) {
        WindowName::Annual
    } else {
        WindowName::Generic { secondary }
    }
}

/// The name of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowLabel {
    /// `{bucket} limit`: the heading of a model's own bucket.
    Bucket(String),
    /// `{Window} limit`, with the bucket's name before it when the bucket has
    /// only this window.
    Window {
        bucket: Option<String>,
        name: WindowName,
    },
    Credits,
    MonthlyCreditLimit,
}

/// What a credits row says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CreditsText {
    Unlimited,
    /// The rounded balance, `13` for `"12.5"`.
    Balance(String),
    Available,
}

/// The `used of limit credits used` line under a spend limit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpendUsed {
    pub used: String,
    pub limit: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowValue {
    /// A bar over the share left, the reset and a details line.
    Window {
        percent_used: i32,
        resets_at: Option<ResetTime>,
        details: Option<SpendUsed>,
    },
    /// A heading with no value.
    Empty,
    Credits(CreditsText),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: RowLabel,
    pub value: RowValue,
}

/// What the card's limit section shows (oracle `StatusRateLimitData`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data {
    Available(Vec<Row>),
    /// Rows from a snapshot older than the threshold.
    Stale(Vec<Row>),
    /// The refresh finished with nothing to show.
    Unavailable,
    /// No snapshot yet.
    Missing,
}

impl Limits {
    // oracle: status/rate_limits.rs fn compose_rate_limit_data_many
    pub fn data(&self) -> Data {
        if self.snapshots.is_empty() {
            return Data::Missing;
        }
        let mut rows = Vec::new();
        let mut stale = false;
        // The plan's own usage before the model-specific buckets.
        let mut ordered: Vec<&Snapshot> = self.snapshots.iter().collect();
        ordered.sort_by_key(|snapshot| !snapshot.name.eq_ignore_ascii_case("codex"));
        for snapshot in ordered {
            let credits = snapshot.credits.as_ref().and_then(credits_text);
            // A bucket with nothing visible leaves no empty heading.
            if snapshot.primary.is_none()
                && snapshot.secondary.is_none()
                && credits.is_none()
                && snapshot.individual_limit.is_none()
            {
                continue;
            }
            stale |= self.now - snapshot.captured_at > STALE_AFTER_SECS;
            if let Some(limit) = &snapshot.individual_limit {
                stale |=
                    self.now - limit.captured_at.unwrap_or(snapshot.captured_at) > STALE_AFTER_SECS;
            }
            let bucket = bucket_name(&snapshot.name);
            let prefixed = !bucket.eq_ignore_ascii_case("codex");
            let windows =
                usize::from(snapshot.primary.is_some()) + usize::from(snapshot.secondary.is_some());
            let combined = prefixed && windows == 1;
            if prefixed && !combined {
                rows.push(Row {
                    label: RowLabel::Bucket(bucket.clone()),
                    value: RowValue::Empty,
                });
            }
            for (window, secondary) in [(snapshot.primary, false), (snapshot.secondary, true)] {
                let Some(window) = window else { continue };
                rows.push(Row {
                    label: RowLabel::Window {
                        bucket: combined.then(|| bucket.clone()),
                        name: window_name(window.window_minutes, secondary),
                    },
                    value: RowValue::Window {
                        percent_used: window.used_percent,
                        resets_at: window.resets_at.map(|at| self.reset(at, snapshot.captured_at)),
                        details: None,
                    },
                });
            }
            if let Some(text) = credits {
                rows.push(Row {
                    label: RowLabel::Credits,
                    value: RowValue::Credits(text),
                });
            }
            if let Some(limit) = &snapshot.individual_limit
                && let (Some(used), Some(max)) =
                    (credit_amount(&limit.used), credit_amount(&limit.limit))
            {
                rows.push(Row {
                    label: RowLabel::MonthlyCreditLimit,
                    value: RowValue::Window {
                        percent_used: 100 - limit.remaining_percent.clamp(0, 100),
                        resets_at: Some(self.reset(limit.resets_at, snapshot.captured_at)),
                        details: Some(SpendUsed { used, limit: max }),
                    },
                });
            }
        }
        if rows.is_empty() {
            Data::Unavailable
        } else if stale {
            Data::Stale(rows)
        } else {
            Data::Available(rows)
        }
    }

    fn reset(&self, at: i64, captured_at: i64) -> ResetTime {
        reset_time(at, captured_at, self.utc_offset_secs)
    }
}

/// The bucket's name as shown (oracle `model_display_name`).
fn bucket_name(name: &str) -> String {
    if name.eq_ignore_ascii_case("gpt-reserve") {
        "Luna Reserve".to_string()
    } else {
        name.to_string()
    }
}

// oracle: status/rate_limits.rs fn credit_status_row
fn credits_text(credits: &Credits) -> Option<CreditsText> {
    if credits.unlimited {
        return Some(CreditsText::Unlimited);
    }
    if !credits.has_credits {
        return None;
    }
    Some(
        credits
            .balance
            .as_deref()
            .and_then(credit_balance)
            .map_or(CreditsText::Available, CreditsText::Balance),
    )
}

// oracle: status/rate_limits.rs fn format_credit_balance
fn credit_balance(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(int_value) = trimmed.parse::<i64>()
        && int_value > 0
    {
        return Some(int_value.to_string());
    }
    if let Ok(value) = trimmed.parse::<f64>()
        && value.is_finite()
        && value > 0.0
    {
        return Some((value.round() as i64).to_string());
    }
    None
}

// oracle: status/rate_limits.rs fn format_credit_amount (`format_with_separators`)
fn credit_amount(raw: &str) -> Option<String> {
    let value = raw.trim().parse::<f64>().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    Some(with_separators(value.round() as i64))
}

fn with_separators(value: i64) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}
