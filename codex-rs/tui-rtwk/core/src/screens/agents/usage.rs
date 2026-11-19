//! Demand-driven usage of the highlighted task: fetched on a one-minute
//! cadence (also after a failure), cached per task, discarded when the
//! account or the connection changes. Oracle: `agents_overview_usage.rs`.

use codex_app_server_protocol::ThreadUsage;
use codex_protocol::account::PlanType;

use super::Command;
use super::Ctx;
use super::Model;
use super::Usage;
use super::refresh::Out;
use super::refresh::out;

/// How long a fetched estimate is current, seconds (oracle:
/// `REFRESH_INTERVAL`).
pub const REFRESH_SECS: i64 = 60;

/// What the server answers to a thread's usage request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageOutcome {
    Available(ThreadUsage),
    /// The account has no thread usage.
    Disabled,
}

/// The plans whose usage the center shows.
fn shows_usage(plan: Option<PlanType>) -> bool {
    matches!(
        plan,
        Some(
            PlanType::Ent26
                | PlanType::Business
                | PlanType::EnterpriseCbpUsageBased
                | PlanType::EnterpriseCbpAutomation
        )
    )
}

/// The usage request of the highlighted task, if one is due (oracle:
/// `refresh_agents_overview_usage`). `None` also when the cached estimate is
/// younger than a minute; `usage_wait` says how long until it is due.
pub fn refresh_usage(model: &mut Model, at: Option<String>) -> Option<Out> {
    if model.pending_usage.is_some()
        || !model.backend_auth
        || !shows_usage(model.plan)
        || model.usage_disabled
        || matches!(
            model.notice,
            Some(super::Notice::Reconnecting | super::Notice::ReconnectFailed)
        )
    {
        return None;
    }
    let thread = at.filter(|id| model.has_task(id))?;
    if usage_wait(model, &thread).is_some() {
        return None;
    }
    model.usage_seq += 1;
    let id = model.usage_seq;
    model.pending_usage = Some((thread.clone(), id));
    Some(out(
        Command::Usage {
            thread: thread.clone(),
        },
        Ctx::Usage { thread, id },
    ))
}

/// Seconds until the highlighted task's cached estimate goes stale; `None`
/// when there is none cached or it is stale.
pub fn usage_wait(model: &Model, thread: &str) -> Option<i64> {
    let age = model.now - model.usage.get(thread)?.fetched_at?;
    (age < REFRESH_SECS).then_some(REFRESH_SECS - age)
}

/// Everything known of usage is discarded: the account changed, or the
/// connection did (oracle: `AccountUpdated`, `Lagged`, `begin_reconnect`).
pub(super) fn reset(model: &mut Model) {
    model.usage.clear();
    model.pending_usage = None;
    model.usage_disabled = false;
}

/// The estimates are dropped for another backend; tokens stay (oracle:
/// `reset_agents_overview_usage_for_backend_switch`).
pub fn reset_for_backend_switch(model: &mut Model) {
    model.usage.retain(|_, usage| {
        usage.estimate = None;
        usage.fetched_at = None;
        usage.tokens.is_some()
    });
}

/// The answer of a usage request (`result`: `None` when it failed).
pub(super) fn finish(model: &mut Model, thread: &str, id: u64, result: Option<super::Reply>) {
    if model.pending_usage.as_ref() != Some(&(thread.to_owned(), id)) {
        return;
    }
    model.pending_usage = None;
    let known = model.threads.iter().any(|held| held.id == thread)
        || model.search_matches.iter().any(|held| held.id == thread);
    if !known {
        return;
    }
    let now = model.now;
    let usage = model.usage.entry(thread.to_owned()).or_default();
    usage.fetched_at = Some(now);
    match result {
        Some(super::Reply::Usage(UsageOutcome::Available(mut estimate)))
            if estimate.thread_id == thread =>
        {
            if let Some(previous) = &usage.estimate {
                // A settling estimate that reads zero keeps what was known.
                let zero_credits = estimate.estimated_usage_credits_micros == 0
                    && previous.estimated_usage_credits_micros > 0;
                let zero_cost = estimate.estimated_usage_usd_micros == Some(0)
                    && previous
                        .estimated_usage_usd_micros
                        .is_some_and(|cost| cost > 0);
                if zero_credits {
                    estimate.estimated_usage_credits_micros =
                        previous.estimated_usage_credits_micros;
                }
                if zero_cost {
                    estimate.estimated_usage_usd_micros = previous.estimated_usage_usd_micros;
                }
                if (zero_credits || zero_cost) && estimate.groups.is_empty() {
                    estimate.groups.clone_from(&previous.groups);
                }
            }
            usage.estimate = Some(estimate);
        }
        Some(super::Reply::Usage(UsageOutcome::Disabled)) => {
            model.usage_disabled = true;
            for usage in model.usage.values_mut() {
                usage.estimate = None;
            }
        }
        _ => {}
    }
}

impl Usage {
    /// The tokens as the screen shows them: the notifications' counts, else
    /// the estimate's groups when every group has the count (oracle:
    /// `usage_lines`).
    pub fn token_counts(&self) -> (Option<i64>, Option<i64>) {
        let sum =
            |count: fn(&codex_app_server_protocol::ThreadUsageBreakdownGroup) -> Option<i64>| {
                let groups = &self.estimate.as_ref()?.groups;
                if groups.is_empty() {
                    return None;
                }
                groups.iter().try_fold(0_i64, |total, group| {
                    count(group)
                        .filter(|tokens| *tokens >= 0)
                        .map(|tokens| total.saturating_add(tokens))
                })
            };
        let input = self
            .tokens
            .map(|(input, _)| input)
            .or_else(|| sum(|group| group.input_tokens));
        let output = self
            .tokens
            .map(|(_, output)| output)
            .or_else(|| sum(|group| group.output_tokens));
        (input, output)
    }
}
