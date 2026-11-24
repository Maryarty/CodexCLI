//! The estimated cost of the thread that is shown (oracle `chatwidget/thread_usage.rs`
//! `ThreadUsageState`): when `account/tokenUsage/read` is asked for it, how an answer that comes
//! before the backend has settled its books is kept apart from a real one, and when it is asked
//! again. Pure: the clock (`Now`, milliseconds), the plan and the items the status line shows are
//! facts the caller passes; the request and its RPC are the caller's (`Request`), the figure the
//! status card and the status line show is read through [`State::estimate`]. Nothing is drawn.

use codex_app_server_protocol::ThreadUsage;
use codex_protocol::account::PlanType;

/// Milliseconds since an epoch of the caller's; only differences count.
pub type Now = i64;

/// After a turn, the usage is asked again after these delays until a figure above the baseline
/// arrives (`THREAD_USAGE_SETTLEMENT_DELAYS`).
const SETTLEMENT_DELAYS_MS: [i64; 3] = [15_000, 60_000, 120_000];
/// After a failed or misrouted answer, after these (`THREAD_USAGE_RETRY_DELAYS`).
const RETRY_DELAYS_MS: [i64; 3] = [5_000, 15_000, 60_000];

/// What the answer of the RPC was: the usage, or that the feature is off for the account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Available(ThreadUsage),
    Disabled,
}

/// What the status line and the terminal title show of the thread's cost (`SelectedThreadUsage`):
/// the settlement waits only for a figure that is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Selected {
    pub credits: bool,
    pub estimated_cost: bool,
}

/// The facts of the moment a call is made in.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub now: Now,
    pub plan: Option<PlanType>,
    /// The session signed in with the Codex backend's own auth (`has_codex_backend_auth`).
    pub backend_auth: bool,
    pub selected: Selected,
}

impl Ctx {
    /// The selection as the oracle reads it: credits are not shown for ENT26, which is billed in
    /// dollars.
    fn selected(&self) -> Selected {
        Selected {
            credits: self.selected.credits && self.plan != Some(PlanType::Ent26),
            estimated_cost: self.selected.estimated_cost,
        }
    }

    fn is_selected(&self) -> bool {
        let selected = self.selected();
        selected.credits || selected.estimated_cost
    }
}

/// The RPC the caller sends: `account/tokenUsage/read` for `thread`, answered with `request`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub thread: String,
    pub request: u64,
}

/// What an answer left.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Finished {
    /// The answer was for the request in flight of the thread shown; none else changes anything.
    pub accepted: bool,
    /// The cards that waited for a figure and have it now (their ids as `for_status` was given).
    pub cards: Vec<u64>,
    /// Another request, which a settlement that is not over asks for at once.
    pub request: Option<Request>,
}

/// ENT26 is billed directly in dollars; a missing native usage is not a zero estimate.
pub fn thread_cost_micros(usage: &ThreadUsage, plan: Option<PlanType>) -> Option<i64> {
    if plan == Some(PlanType::Ent26) {
        usage.native_usage_usd_micros
    } else {
        usage.estimated_usage_usd_micros
    }
}

/// Whether a plan's account has a thread usage to read at all.
pub fn supported(plan: Option<PlanType>) -> bool {
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    thread: Option<String>,
    estimate: Option<ThreadUsage>,
    pending: Option<u64>,
    next_request: u64,
    requested: bool,
    disabled: bool,
    /// The status cards that wait for a figure.
    cards: Vec<u64>,
    status_requested: bool,
    retry_attempts: usize,
    retry_due: Option<Now>,
    settlement_attempts: usize,
    baseline_credits: Option<i64>,
    baseline_usd: Option<i64>,
    settlement_request: Option<u64>,
    settlement_due: Option<Now>,
    /// A turn completed again by a replay does not start a settlement.
    pub replaying_turn_completion: bool,
}

impl State {
    /// The figure the status line and the cards show: none until the first answer.
    pub fn estimate(&self) -> Option<&ThreadUsage> {
        self.estimate.as_ref()
    }

    /// Nothing of another thread's stays (`clear_thread_usage_state`); request numbers go on.
    fn clear(&mut self) {
        *self = Self { next_request: self.next_request, ..Self::default() };
    }

    fn follow(&mut self, thread: &str) {
        if self.thread.as_deref() != Some(thread) {
            self.clear();
            self.thread = Some(thread.to_owned());
        }
    }

    fn available(&self, thread: Option<&str>, ctx: &Ctx) -> bool {
        ctx.backend_auth && !self.disabled && thread.is_some() && supported(ctx.plan)
    }

    /// When the earliest request that is due is, for the caller's timer (the early redraw).
    pub fn next_due(&self) -> Option<Now> {
        [self.settlement_due, self.retry_due].into_iter().flatten().min()
    }

    /// The thread is shown (oracle `ensure_thread_usage_requested`): its usage is asked once.
    pub fn ensure_requested(&mut self, thread: &str, ctx: &Ctx) -> Option<Request> {
        self.follow(thread);
        if self.requested {
            return None;
        }
        self.request(thread, ctx)
    }

    /// A turn of the thread ended (`refresh_thread_usage_after_turn`): the usage is asked, and
    /// asked again after the delays until it is above what it was.
    pub fn after_turn(&mut self, thread: &str, ctx: &Ctx) -> Option<Request> {
        if self.replaying_turn_completion
            || !ctx.is_selected()
            || !self.available(Some(thread), ctx)
        {
            return None;
        }
        self.settlement_attempts = 0;
        self.settlement_request = None;
        self.baseline_credits = Some(
            self.estimate
                .as_ref()
                .map(|estimate| estimate.estimated_usage_credits_micros)
                .unwrap_or_default(),
        );
        self.baseline_usd = self
            .estimate
            .as_ref()
            .and_then(|estimate| thread_cost_micros(estimate, ctx.plan));
        self.schedule_settlement(ctx.now);
        self.request(thread, ctx)
    }

    /// `/status` of the thread (`request_thread_usage_for_status`): `card` waits for the figure.
    pub fn for_status(&mut self, thread: &str, card: u64, ctx: &Ctx) -> Option<Request> {
        if !self.available(Some(thread), ctx) {
            return None;
        }
        self.follow(thread);
        self.cards.push(card);
        self.status_requested = true;
        self.retry_attempts = 0;
        self.request(thread, ctx)
    }

    /// The caller's tick (`refresh_thread_usage_if_settlement_due`): the request that is due, if
    /// one is; a thread whose figure nothing shows any more stops polling.
    pub fn tick(&mut self, thread: &str, ctx: &Ctx) -> Option<Request> {
        let settlement = self.settlement_due.is_some_and(|due| ctx.now >= due);
        let retry = self.retry_due.is_some_and(|due| ctx.now >= due);
        if !settlement && !retry {
            return None;
        }
        if !ctx.is_selected() {
            self.cancel_polling();
            return None;
        }
        if settlement {
            self.settlement_due = None;
        }
        if retry {
            self.retry_due = None;
        }
        self.request(thread, ctx)
    }

    /// The status line no longer shows the cost (`cancel_thread_usage_polling`); a card that
    /// waits keeps the request going.
    pub fn cancel_polling(&mut self) {
        if self.status_requested {
            return;
        }
        self.pending = None;
        self.requested = false;
        self.retry_attempts = 0;
        self.retry_due = None;
        self.end_settlement();
    }

    fn end_settlement(&mut self) {
        self.settlement_attempts = 0;
        self.baseline_credits = None;
        self.baseline_usd = None;
        self.settlement_request = None;
        self.settlement_due = None;
    }

    fn request(&mut self, thread: &str, ctx: &Ctx) -> Option<Request> {
        if !self.available(Some(thread), ctx) || self.pending.is_some() {
            return None;
        }
        let request = self.next_request;
        self.next_request = self.next_request.wrapping_add(1);
        self.pending = Some(request);
        self.requested = true;
        self.thread = Some(thread.to_owned());
        if self.baseline_credits.is_some() {
            self.settlement_request = Some(request);
        }
        Some(Request { thread: thread.to_owned(), request })
    }

    fn schedule_settlement(&mut self, now: Now) {
        let Some(delay) = SETTLEMENT_DELAYS_MS.get(self.settlement_attempts) else {
            self.baseline_credits = None;
            self.baseline_usd = None;
            self.settlement_request = None;
            return;
        };
        self.settlement_attempts += 1;
        self.settlement_due = Some(now + delay);
    }

    fn schedule_retry(&mut self, ctx: &Ctx) {
        if !ctx.is_selected() {
            return;
        }
        let Some(delay) = RETRY_DELAYS_MS.get(self.retry_attempts) else { return };
        self.retry_attempts += 1;
        self.retry_due = Some(ctx.now + delay);
    }

    /// The RPC of `request` ended (oracle `finish_thread_usage_refresh`). `shown` is the thread
    /// the chat shows now; an answer for another one, or for a request that is not the one in
    /// flight, is not accepted.
    pub fn finish(
        &mut self,
        thread: &str,
        shown: Option<&str>,
        request: u64,
        result: Result<Outcome, String>,
        ctx: &Ctx,
    ) -> Finished {
        if shown != Some(thread)
            || self.thread.as_deref() != Some(thread)
            || self.pending != Some(request)
        {
            return Finished::default();
        }
        self.pending = None;
        self.status_requested = false;
        let post_turn = self.settlement_request == Some(request);
        let mut updated = false;
        match result {
            Ok(Outcome::Disabled) => {
                self.disabled = true;
                self.estimate = None;
                updated = true;
                self.retry_due = None;
                self.settlement_due = None;
                self.baseline_credits = None;
                self.baseline_usd = None;
                self.settlement_request = None;
            }
            // an answer for another thread than the one asked is no answer
            Ok(Outcome::Available(usage)) if usage.thread_id != thread => {
                self.schedule_retry(ctx);
            }
            Ok(Outcome::Available(usage)) => {
                updated = true;
                self.retry_attempts = 0;
                self.retry_due = None;
                self.settle(usage, post_turn, ctx);
            }
            Err(_) => self.schedule_retry(ctx),
        }
        let mut cards = Vec::new();
        if updated && !self.cards.is_empty() && self.baseline_credits.is_none() {
            cards = std::mem::take(&mut self.cards);
        } else if updated && !self.cards.is_empty() {
            // the cards get this figure and keep waiting for the settled one
            cards = self.cards.clone();
        }
        let again = self.baseline_credits.is_some()
            && self.settlement_request.is_none()
            && ctx.is_selected();
        let request = if again { self.request(thread, ctx) } else { None };
        Finished { accepted: true, cards, request }
    }

    /// A figure arrived: a zero after a positive one is the backend not having settled, and keeps
    /// the one before; the settlement ends when a shown figure is above its baseline.
    fn settle(&mut self, mut usage: ThreadUsage, post_turn: bool, ctx: &Ctx) {
        let previous = self.estimate.as_ref();
        let previous_credits = previous
            .map(|estimate| estimate.estimated_usage_credits_micros)
            .filter(|micros| *micros > 0);
        let zero_credits = usage.estimated_usage_credits_micros == 0 && previous_credits.is_some();
        if let (Some(previous), true) = (previous_credits, zero_credits) {
            usage.estimated_usage_credits_micros = previous;
        }
        let previous_usd = previous
            .and_then(|estimate| estimate.estimated_usage_usd_micros)
            .filter(|micros| *micros > 0);
        let zero_cost = usage.estimated_usage_usd_micros == Some(0) && previous_usd.is_some();
        if let (Some(previous), true) = (previous_usd, zero_cost) {
            usage.estimated_usage_usd_micros = Some(previous);
        }
        if (zero_credits || zero_cost)
            && usage.groups.is_empty()
            && let Some(previous) = self.estimate.as_ref()
        {
            usage.groups.clone_from(&previous.groups);
        }
        let credits = usage.estimated_usage_credits_micros;
        let cost = thread_cost_micros(&usage, ctx.plan);
        self.estimate = Some(usage);
        let Some(baseline) = self.baseline_credits else { return };
        if !post_turn {
            self.baseline_credits = Some(baseline.max(credits));
            self.baseline_usd = match (self.baseline_usd, cost) {
                (Some(previous), Some(current)) => Some(previous.max(current)),
                (previous, current) => previous.or(current),
            };
            return;
        }
        let mut selected = ctx.selected();
        if !self.cards.is_empty() {
            selected.credits = ctx.plan != Some(PlanType::Ent26);
            selected.estimated_cost |= ctx.plan == Some(PlanType::Ent26)
                || self.baseline_usd.is_some()
                || cost.is_some();
        }
        let credits_settled = !selected.credits || credits > baseline;
        let cost_settled = !selected.estimated_cost
            || cost.is_some_and(|current| {
                self.baseline_usd.map_or(current > 0, |previous| current > previous)
            });
        if credits_settled && cost_settled {
            self.baseline_credits = None;
            self.baseline_usd = None;
            self.settlement_request = None;
            self.settlement_due = None;
        } else if self.settlement_due.is_none() {
            self.schedule_settlement(ctx.now);
        }
    }
}
