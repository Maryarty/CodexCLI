//! The thread's billing rows of the `/status` card (oracle `status/thread_usage.rs`
//! `StatusThreadUsage::{push_labels, lines}`) as typed values: the credits and the dollars, the
//! shares the models, the reasoning efforts and the speeds took of them, and the billed tokens.
//! Every word is the view's; the shares are worked out here (the order the oracle sorts them in,
//! the percentages that round to the nearest).

use std::collections::BTreeMap;

use codex_app_server_protocol::{ThreadUsage, ThreadUsageBreakdownGroup};
use codex_protocol::account::PlanType;

use super::helpers::{format_credit_micros, format_estimated_usd_micros, thread_cost_micros};

const REASONING_ORDER: [&str; 8] = [
    "None",
    "Minimal",
    "Light",
    "Medium",
    "High",
    "Extra High",
    "Max",
    "Ultra",
];
const SPEED_ORDER: [&str; 3] = ["Fast mode", "Ultrafast", "Standard"];

/// What the backend estimates a thread cost, as the views read it: through the methods and
/// `slots`/`usage` below, never the protocol record behind it.
#[derive(Clone, Debug, PartialEq)]
pub struct Figure(ThreadUsage);

impl From<ThreadUsage> for Figure {
    fn from(usage: ThreadUsage) -> Self {
        Self(usage)
    }
}

impl Figure {
    /// The credits the thread used, in millionths.
    pub fn credits_micros(&self) -> i64 {
        self.0.estimated_usage_credits_micros
    }

    /// The thread's cost in millionths of a dollar for an account of `plan`: the native usage on
    /// Ent26, the estimate on the others.
    pub fn cost_micros(&self, plan: Option<PlanType>) -> Option<i64> {
        thread_cost_micros(&self.0, plan)
    }
}

/// The rows a card's label column is made for (`push_labels`): the oracle sizes it by the rows a
/// figure will have, so it does not move when a figure arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Usage,
    Models,
    Reasoning,
    Speed,
    BilledTokens,
}

/// The labels the rows of `estimate` have, and (`reserve`) the one a card that waits for a figure
/// holds back so its column keeps its width.
pub fn slots(estimate: Option<&Figure>, reserve: bool) -> Vec<Slot> {
    let mut slots = Vec::new();
    let mut put = |slot: Slot| {
        if !slots.contains(&slot) {
            slots.push(slot);
        }
    };
    if reserve {
        put(Slot::BilledTokens);
    }
    let Some(Figure(estimate)) = estimate else {
        return slots;
    };
    put(Slot::Usage);
    if !estimate.groups.is_empty() {
        put(Slot::Models);
        put(Slot::Reasoning);
        put(Slot::Speed);
        if estimate
            .groups
            .iter()
            .any(|group| group.input_tokens.is_some() || group.output_tokens.is_some())
        {
            put(Slot::BilledTokens);
        }
    }
    slots
}

/// A reasoning effort as the breakdown names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effort {
    None,
    Minimal,
    Light,
    Medium,
    High,
    ExtraHigh,
    Max,
    Ultra,
    /// The group names none.
    Other,
    /// A name the terminal does not know: shown as the server spells it.
    Named(String),
}

/// A speed as the breakdown names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Speed {
    Fast,
    Ultrafast,
    Standard,
    Other,
    Named(String),
}

/// What `name` took of the credits, in percent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Share<K> {
    pub name: K,
    pub percent: i64,
}

/// The tokens the groups billed; a count is none when a group did not say it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BilledTokens {
    pub input: Option<i64>,
    pub cached: Option<i64>,
    pub output: Option<i64>,
}

/// The credit-weighted rows of a thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Billing {
    /// The credits as `5.2` (the view adds `credits`).
    pub credits: String,
    /// The estimate in dollars as `~$1.82`, when the backend has one.
    pub cost: Option<String>,
    pub models: Vec<Share<String>>,
    pub reasoning: Vec<Share<Effort>>,
    pub speed: Vec<Share<Speed>>,
    /// The billed tokens row, when the groups hold tokens.
    pub tokens: Option<BilledTokens>,
}

/// What the card shows of a thread's usage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Usage {
    /// ENT26 is billed in dollars: the cost alone (credit-weighted breakdowns do not describe
    /// native dollar usage).
    Native(String),
    Credits(Billing),
}

/// The rows of `estimate` for an account of `plan` (`StatusThreadUsage::lines`); none when the
/// estimate has nothing to show.
pub fn usage(Figure(estimate): &Figure, plan: Option<PlanType>) -> Option<Usage> {
    if plan == Some(PlanType::Ent26) {
        return thread_cost_micros(estimate, plan)
            .and_then(format_estimated_usd_micros)
            .map(Usage::Native);
    }
    let sum = |count: fn(&ThreadUsageBreakdownGroup) -> Option<i64>| {
        estimate.groups.iter().try_fold(0_i64, |total, group| {
            count(group).map(|tokens| total.saturating_add(tokens))
        })
    };
    let tokens = BilledTokens {
        input: sum(|group| group.input_tokens),
        cached: sum(|group| group.cached_input_tokens),
        output: sum(|group| group.output_tokens),
    };
    Some(Usage::Credits(Billing {
        credits: format_credit_micros(estimate.estimated_usage_credits_micros),
        cost: estimate
            .estimated_usage_usd_micros
            .and_then(format_estimated_usd_micros),
        models: shares(&estimate.groups, Dimension::Model)
            .into_iter()
            .map(|(name, percent)| Share { name, percent })
            .collect(),
        reasoning: shares(&estimate.groups, Dimension::Reasoning)
            .into_iter()
            .map(|(name, percent)| Share {
                name: effort(&name),
                percent,
            })
            .collect(),
        speed: shares(&estimate.groups, Dimension::Speed)
            .into_iter()
            .map(|(name, percent)| Share {
                name: speed(&name),
                percent,
            })
            .collect(),
        tokens: (!estimate.groups.is_empty() && (tokens.input.is_some() || tokens.output.is_some()))
            .then_some(tokens),
    }))
}

fn effort(name: &str) -> Effort {
    match name {
        "None" => Effort::None,
        "Minimal" => Effort::Minimal,
        "Light" => Effort::Light,
        "Medium" => Effort::Medium,
        "High" => Effort::High,
        "Extra High" => Effort::ExtraHigh,
        "Max" => Effort::Max,
        "Ultra" => Effort::Ultra,
        "Other" => Effort::Other,
        other => Effort::Named(other.to_owned()),
    }
}

fn speed(name: &str) -> Speed {
    match name {
        "Fast mode" => Speed::Fast,
        "Ultrafast" => Speed::Ultrafast,
        "Standard" => Speed::Standard,
        "Other" => Speed::Other,
        other => Speed::Named(other.to_owned()),
    }
}

#[derive(Clone, Copy)]
enum Dimension {
    Model,
    Reasoning,
    Speed,
}

/// `grouped_usage`: the credits per name of the dimension, biggest model first and the others in
/// their natural order, each as a percent of the total. Sorted by the oracle's English names.
fn shares(groups: &[ThreadUsageBreakdownGroup], dimension: Dimension) -> Vec<(String, i64)> {
    let mut totals = BTreeMap::<String, i64>::new();
    for group in groups {
        if group.estimated_usage_credits_micros < 0 {
            continue;
        }
        let name = match dimension {
            Dimension::Model => group
                .model
                .as_deref()
                .map_or_else(|| "Other".to_string(), model_name),
            Dimension::Reasoning => match group.reasoning_effort.as_deref() {
                Some("none") => "None",
                Some("minimal") => "Minimal",
                Some("low") => "Light",
                Some("medium") => "Medium",
                Some("high") => "High",
                Some("xhigh") => "Extra High",
                Some("max") => "Max",
                Some("ultra") => "Ultra",
                Some(other) => other,
                None => "Other",
            }
            .to_string(),
            Dimension::Speed => match group.speed.as_deref() {
                Some("fast") => "Fast mode",
                Some("ultrafast") => "Ultrafast",
                Some("standard") => "Standard",
                Some(other) => other,
                None => "Other",
            }
            .to_string(),
        };
        let total = totals.entry(name).or_default();
        *total = total.saturating_add(group.estimated_usage_credits_micros);
    }
    let total_credits = totals
        .values()
        .fold(0_i64, |total, credits| total.saturating_add(*credits));
    let position = |order: &[&str], name: &str| {
        order
            .iter()
            .position(|value| *value == name)
            .unwrap_or(usize::MAX)
    };
    let mut totals = totals.into_iter().collect::<Vec<_>>();
    totals.sort_by(|(left_name, left_credits), (right_name, right_credits)| match dimension {
        Dimension::Model => right_credits
            .cmp(left_credits)
            .then_with(|| left_name.cmp(right_name)),
        Dimension::Reasoning => position(&REASONING_ORDER, left_name)
            .cmp(&position(&REASONING_ORDER, right_name))
            .then_with(|| left_name.cmp(right_name)),
        Dimension::Speed => position(&SPEED_ORDER, left_name)
            .cmp(&position(&SPEED_ORDER, right_name))
            .then_with(|| left_name.cmp(right_name)),
    });
    totals
        .into_iter()
        .map(|(name, credits)| {
            let percent = if total_credits <= 0 {
                0
            } else {
                (i128::from(credits) * 100 + i128::from(total_credits) / 2)
                    / i128::from(total_credits)
            };
            (name, i64::try_from(percent).unwrap_or(i64::MAX))
        })
        .collect()
}

/// `format_model_display_name`: `gpt-5.4` is `GPT-5.4`, `gpt-5-mini` is `GPT-5 Mini`, `gpt-oai`
/// keeps its `OAI`.
fn model_name(model: &str) -> String {
    let model = if model.eq_ignore_ascii_case("gpt-reserve") {
        "Luna Reserve"
    } else {
        model
    };
    let Some((prefix, remainder)) = model.split_once('-') else {
        return model.to_string();
    };
    if !prefix.eq_ignore_ascii_case("gpt") || remainder.is_empty() {
        return model.to_string();
    }
    let mut segments = remainder.split('-');
    let first = segments.next().unwrap_or_default();
    let mut name = String::from("GPT-");
    name.push_str(first);
    let numeric = first.chars().next().is_some_and(|c| c.is_ascii_digit());
    for segment in segments {
        name.push(if numeric { ' ' } else { '-' });
        if segment.eq_ignore_ascii_case("oai") {
            name.push_str("OAI");
            continue;
        }
        let mut characters = segment.chars();
        if let Some(head) = characters.next() {
            name.extend(head.to_uppercase());
            name.push_str(characters.as_str());
        }
    }
    name
}
