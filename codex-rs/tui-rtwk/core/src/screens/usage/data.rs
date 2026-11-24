//! Report loading state and the number words of the account dashboard.
//! Carried from the oracle's `analytics/data.rs`: its `Load<T>` held a spawned
//! task and a oneshot receiver; here a load is a value in the model, started
//! with a stamp and settled by the event that carries the stamp (`apply`
//! drops any other), so replacing a load drops the old answer (R1).

use super::models::AccountAnalyticsGrouping as Grouping;

pub const GROUPINGS: [Grouping; 7] = [
    Grouping::Surface,
    Grouping::Feature,
    Grouping::Model,
    Grouping::TaskStart,
    Grouping::Speed,
    Grouping::Reasoning,
    Grouping::TokenType,
];

/// What a grouping is called (the view's words; oracle: `GROUP_LABELS`, the
/// Overview's chart views and `Product` for the first grouping of the
/// reports that read it so). `NAMES` is `GROUPINGS` in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupName {
    Surface,
    Feature,
    Model,
    TurnStart,
    Speed,
    Reasoning,
    TokenType,
    Product,
    Daily,
    Weekly,
    Cumulative,
}

pub const GROUP_NAMES: [GroupName; 7] = [
    GroupName::Surface,
    GroupName::Feature,
    GroupName::Model,
    GroupName::TurnStart,
    GroupName::Speed,
    GroupName::Reasoning,
    GroupName::TokenType,
];

/// What a load says while it is not ready (the view's words; oracle:
/// `Load::message`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Loading,
    Unavailable,
    /// Why the load failed.
    Error(LoadError),
}

/// Why a load failed: the host's own message (a request that failed) or what
/// a normaliser found wrong in the reply (the page words each variant).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    Host(String),
    Report(super::normalize::Error),
    Tokens(super::tokens::Error),
    Plan(super::plan::Error),
}

/// A message of the host (a failed request, a timeout).
impl From<String> for LoadError {
    fn from(message: String) -> Self {
        Self::Host(message)
    }
}

impl From<super::normalize::Error> for LoadError {
    fn from(error: super::normalize::Error) -> Self {
        Self::Report(error)
    }
}

impl From<super::tokens::Error> for LoadError {
    fn from(error: super::tokens::Error) -> Self {
        Self::Tokens(error)
    }
}

impl From<super::plan::Error> for LoadError {
    fn from(error: super::plan::Error) -> Self {
        Self::Plan(error)
    }
}

/// What a report was asked for (ADR-013: the key of a request record; the reports are a cache by
/// it): the section's report over `days` days ending at the model's end date, grouped by the
/// `group`-th grouping, filtered to `model` (the token report's `m` control).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReportKey {
    pub section: super::Section,
    pub days: u32,
    pub group: usize,
    pub model: Option<String>,
}

/// The number of one request: a reply settles the load only when it carries
/// the stamp the load was started with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Stamp(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Load<T> {
    Loading(Stamp),
    Ready(T),
    Unavailable,
    Error(LoadError),
}

// Not derived: a derive would ask `T: Default`.
#[allow(clippy::derivable_impls)]
impl<T> Default for Load<T> {
    fn default() -> Self {
        Load::Unavailable
    }
}

impl<T> Load<T> {
    pub fn ready(&self) -> Option<&T> {
        if let Self::Ready(value) = self {
            Some(value)
        } else {
            None
        }
    }

    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading(_))
    }

    pub fn stamp(&self) -> Option<Stamp> {
        match self {
            Self::Loading(stamp) => Some(*stamp),
            _ => None,
        }
    }

    pub fn note(&self) -> Option<Note> {
        match self {
            Self::Ready(_) => None,
            Self::Loading(_) => Some(Note::Loading),
            Self::Unavailable => Some(Note::Unavailable),
            Self::Error(message) => Some(Note::Error(message.clone())),
        }
    }

    /// The reply of the request stamped `stamp` (the oracle's `poll`):
    /// `Some(data)` is ready, `None` unavailable, an error is shown as its
    /// text. A reply of another stamp changes nothing.
    pub fn settle(&mut self, stamp: Stamp, result: Result<Option<T>, LoadError>) -> bool {
        if self.stamp() != Some(stamp) {
            return false;
        }
        *self = match result {
            Ok(Some(data)) => Self::Ready(data),
            Ok(None) => Self::Unavailable,
            Err(message) => Self::Error(message),
        };
        true
    }
}

/// Keep tiny refunds visible while avoiding noise on ordinary credit amounts.
pub fn amount(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if value.abs() < 0.000001 {
        return format!("{value:.2e}");
    }
    let rounded = if value.abs() < 0.01 {
        format!("{value:.6}")
    } else {
        format!("{value:.2}")
    };
    let (whole, fraction) = rounded.split_once('.').unwrap_or((&rounded, ""));
    let mut grouped = String::new();
    for (index, ch) in whole.chars().enumerate() {
        if index > 0
            && ch.is_ascii_digit()
            && (whole.len() - index).is_multiple_of(/*rhs*/ 3)
            && !grouped.ends_with('-')
        {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let fraction = fraction.trim_end_matches('0');
    if !fraction.is_empty() {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    grouped
}

/// Compact headline and axis labels to one decimal, preserving small signed adjustments.
pub fn compact_amount(value: f64) -> String {
    if value.abs() < 1_000.0 || !value.is_finite() {
        return amount(value);
    }
    let units = ["K", "M", "B", "T"];
    let mut scaled = value / 1_000.0;
    let mut unit = 0;
    // Promote before rounding would produce a label such as 1000K.
    while scaled.abs() >= 999.95 && unit + 1 < units.len() {
        scaled /= 1_000.0;
        unit += 1;
    }
    let rounded = format!("{:.1}", (scaled * 10.0).round() / 10.0);
    format!(
        "{}{}",
        rounded.trim_end_matches('0').trim_end_matches('.'),
        units[unit]
    )
}

/// Align ordinary credit amounts to two decimals without hiding tiny adjustments.
pub fn credit_amount(value: f64) -> String {
    let formatted = amount(value);
    if value != 0.0 && value.abs() < 0.01 {
        return formatted;
    }
    match formatted.split_once('.') {
        None => format!("{formatted}.00"),
        Some((_, fraction)) if fraction.len() == 1 => format!("{formatted}0"),
        Some(_) => formatted,
    }
}

pub fn credits(micros: i64) -> String {
    let magnitude = micros.unsigned_abs();
    if magnitude > 0 && magnitude < 10_000 {
        let fractional = format!("{magnitude:06}");
        return format!(
            "{}0.{}",
            if micros < 0 { "-" } else { "" },
            fractional.trim_end_matches('0')
        );
    }
    let cents = (magnitude + 5_000) / 10_000;
    let mut whole = (cents / 100).to_string();
    let digits = whole.len();
    for index in (1..digits).rev() {
        if (digits - index).is_multiple_of(/*rhs*/ 3) {
            whole.insert(index, ',');
        }
    }
    format!(
        "{}{whole}.{:02}",
        if micros < 0 { "-" } else { "" },
        cents % 100
    )
}

