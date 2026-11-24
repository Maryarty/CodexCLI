//! The thread's billing rows of the card (oracle `status/thread_usage.rs`, `lines`): the credits
//! and dollars the backend estimates for the thread, then the shares the models, the reasoning
//! efforts and the speeds took, then the billed tokens. The figure is the card's own (the feature
//! keeps it by the card's number): the one there was when `/status` ran, and the ones that came
//! while it waited for the settled one.

use rtwk::style::Span;

use codex_app_server_protocol::ThreadUsage;
use codex_tui_core::screens::status::{Account, StatusCard, Usage, billing_slots, billing_usage};

use super::cell::{Entry, Look, Value};

/// The label column's reservations and the rows of `held`: what the card shows of the usage.
/// `held` is none for a card whose account has no thread usage, else the figure the card holds.
pub(super) fn entries(
    card: &StatusCard,
    held: Option<Option<&ThreadUsage>>,
    look: &Look,
) -> Vec<Entry> {
    let s = look.s;
    let Some(estimate) = held else {
        return Vec::new();
    };
    // the column is as wide as the rows the figure has will need, so it does not move when the
    // figure arrives
    let mut entries: Vec<Entry> = billing_slots(estimate, true)
        .into_iter()
        .map(|slot| Entry::Reserve(s.slot(slot).to_string()))
        .collect();
    let plan = match &card.account {
        Some(Account::ChatGpt { plan, .. }) => *plan,
        Some(Account::ApiKey) | None => None,
    };
    let Some(usage) = estimate.and_then(|estimate| billing_usage(estimate, plan)) else {
        return entries;
    };
    let row = |label: &str, value: Value| Entry::Row {
        label: label.to_string(),
        value,
    };
    entries.push(Entry::Blank);
    match usage {
        Usage::Native(cost) => entries.push(row(s.thread_usage, Value::Spans(vec![Span::new(cost)]))),
        Usage::Credits(billing) => {
            let mut head = vec![Span::new(s.usage_credits(&billing.credits))];
            if let Some(cost) = billing.cost {
                head.push(Span::styled(s.usage_dot, look.faint.clone()));
                head.push(Span::new(cost));
            }
            entries.push(row(s.thread_usage, Value::Spans(head)));
            let shares = |text: String| Value::Spans(vec![Span::new(text)]);
            if !billing.models.is_empty() {
                entries.push(row(
                    s.usage_models,
                    shares(s.shares(billing.models.into_iter().map(|x| (x.name, x.percent)))),
                ));
            }
            if !billing.reasoning.is_empty() {
                entries.push(row(
                    s.usage_reasoning,
                    shares(s.shares(
                        billing
                            .reasoning
                            .into_iter()
                            .map(|x| (s.effort(&x.name), x.percent)),
                    )),
                ));
            }
            if !billing.speed.is_empty() {
                entries.push(row(
                    s.usage_speed,
                    shares(s.shares(
                        billing
                            .speed
                            .into_iter()
                            .map(|x| (s.speed(&x.name), x.percent)),
                    )),
                ));
            }
            if let Some(tokens) = billing.tokens {
                entries.push(row(s.usage_billed, shares(s.billed(&tokens))));
            }
        }
    }
    entries
}
