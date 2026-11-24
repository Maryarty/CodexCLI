//! The estimated cost of the thread in use as a feature slice (ADR-012, ADR-013; oracle
//! `chatwidget/thread_usage.rs` `ThreadUsageState`, `app/history_ui.rs`
//! `finish_thread_usage_refresh`, `status/thread_usage.rs`): the credits and the dollars the
//! backend estimates for the thread, which the status line, the terminal title and the `/status`
//! card show. The figure is read with `account/usage/read` naming the thread.
//!
//! What is kept is what the asking needs (`model`): the answer, the request in flight, the
//! settlement after a turn (a figure the backend has not settled yet is asked again after 15 s,
//! 60 s and 120 s until it is above what it was) and the retries of a failed answer. When it asks
//! is decided by what is shown, not by a configuration the core would have to read (ADR-013):
//! the widgets that show a figure raise an interest (`UsageInterest`: the credits, the cost, a
//! card), the first loads and the last lets go. The moments of the settlement and the retries
//! are a declared wake (`wake_at`), never a tick.
//!
//! The figure is read live by what shows it (`State::estimate_of`); a `/status` card that is in
//! the transcript shows the figure of its thread as it is when it is drawn, where the oracle
//! patches the card in place (`last_thread_usage_status_cell`): the same until a later turn
//! settles a new figure.

pub mod model;

use codex_app_server_protocol::{ClientRequest, GetAccountTokenUsageParams, RequestId};

use super::{AppCx, AppView, Feature, Never};
use crate::app::Now;
use crate::ids::ThreadId;
use crate::rpc::RpcError;
use crate::screens::status::{Account, UsageInterest};
use codex_app_server_protocol::{GetAccountTokenUsageResponse, ServerNotification, ThreadUsage};

pub use model::{Outcome, Selected, supported, thread_cost_micros};

/// The marker type of the feature.
pub struct ThreadUsageCost;

/// The one RPC: `account/usage/read` of one thread.
mod rpcs {
    use super::{GetAccountTokenUsageParams, GetAccountTokenUsageResponse, Outcome};

    crate::rpc_list! {
        /// What the slice asks the server.
        pub enum Read => Outcome;
        /// `account/usage/read` with the thread: the answer holds the thread's usage when its
        /// billing route is there (`fetch_thread_usage`), none when it is not.
        Thread(String), Self::Thread(thread)
            => GetAccountTokenUsage(Some(GetAccountTokenUsageParams {
                thread_id: Some(thread.clone()),
            }))
            -> GetAccountTokenUsageResponse, |response| response
                .thread_usage
                .map_or(Outcome::Disabled, Outcome::Available);
    }
}

pub use rpcs::Read;

/// What an ask was for: the thread and the number the machine gave the request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ctx {
    thread: String,
    request: u64,
}

/// What the clock asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    /// A moment of the settlement or of a retry has come (`refresh_thread_usage_if_settlement_due`).
    Due,
}

/// What the feature makes of the stream: the end of a turn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A turn of `thread` ended (`refresh_thread_usage_after_turn`).
    TurnEnded { thread: String },
}

/// The feature's state: the machine, and what is shown now.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    machine: model::State,
    /// The figures a widget shows now (`SelectedThreadUsage`).
    selected: Selected,
    /// A `/status` card shows the thread's usage.
    card: bool,
}

impl State {
    /// The estimate for `thread`: the figure of the answer that is kept, none until the first
    /// one and none for another thread's.
    pub fn estimate_of(&self, thread: &ThreadId) -> Option<&ThreadUsage> {
        let estimate = self.machine.estimate()?;
        (self.machine.thread() == Some(thread.as_str())).then_some(estimate)
    }

    /// Whether the feature is off for the account's plan or the thread (an answer without a
    /// usage): no row is shown then.
    pub fn is_disabled(&self) -> bool {
        self.machine.is_disabled()
    }
}

/// The facts of the moment, as the machine reads them: the clock in milliseconds, the plan, and
/// whether the session is signed in with the backend's own auth.
fn ctx(state: &State, view: AppView<'_>) -> model::Ctx {
    let facts = view.facts();
    let (plan, backend_auth) = match facts.status().account.as_ref() {
        Some(Account::ChatGpt { plan, .. }) => (*plan, true),
        Some(Account::ApiKey) | None => (None, false),
    };
    model::Ctx {
        now: facts.now().millis(),
        plan,
        backend_auth,
        status_items: state.selected,
    }
}

fn is_shown(state: &State) -> bool {
    state.selected.credits || state.selected.estimated_cost
}

fn ask(cx: &mut AppCx<'_>, request: model::Request) {
    let ctx = Ctx {
        thread: request.thread.clone(),
        request: request.request,
    };
    cx.ask(None, Read::Thread(request.thread), ctx);
}

impl Feature for ThreadUsageCost {
    const NAME: &'static str = "thread_usage";

    type State = State;
    type ThreadPart = ();
    type Target = Never<Self>;
    type Intent = Intent;
    type ThreadIntent = Never<Self>;
    type Command = Read;
    type Reply = Outcome;
    type Ctx = Ctx;
    type Note = Never<Self>;
    type Fact = Never<Self>;
    type Wish = Never<Self>;
    type Interest = UsageInterest;
    type Event = Event;

    /// The moment of a settlement or a retry: the request that is due goes out.
    fn intent(state: &mut State, intent: Intent, cx: &mut AppCx<'_>) {
        let Intent::Due = intent;
        let Some(thread) = state.machine.thread().map(str::to_owned) else {
            return;
        };
        let machine_ctx = ctx(state, cx.view());
        if let Some(request) = state.machine.tick(&thread, &machine_ctx) {
            ask(cx, request);
        }
    }

    /// `finish_thread_usage_refresh`: the answer of the request in flight of the thread shown is
    /// kept; a settlement that is not over asks again at once.
    fn answered(
        state: &mut State,
        ctx_: Ctx,
        result: Result<Outcome, RpcError>,
        cx: &mut AppCx<'_>,
    ) {
        let machine_ctx = ctx(state, cx.view());
        let shown = cx.view().active().map(ThreadId::to_string);
        let finished = state.machine.finish(
            &ctx_.thread,
            shown.as_deref(),
            ctx_.request,
            result.map_err(|error| format!("{error:?}")),
            &machine_ctx,
        );
        if let Some(request) = finished.request {
            ask(cx, request);
        }
    }

    fn project(notification: &ServerNotification) -> Option<Event> {
        let ServerNotification::TurnCompleted(done) = notification else {
            return None;
        };
        Some(Event::TurnEnded {
            thread: done.thread_id.clone(),
        })
    }

    /// `refresh_thread_usage_after_turn`, for the thread shown.
    fn on_event(state: &mut State, event: Event, cx: &mut AppCx<'_>) {
        let Event::TurnEnded { thread } = event;
        if cx.view().active().map(ThreadId::as_str) != Some(thread.as_str()) {
            return;
        }
        let machine_ctx = ctx(state, cx.view());
        if let Some(request) = state.machine.after_turn(&thread, &machine_ctx) {
            ask(cx, request);
        }
    }

    /// `ensure_thread_usage_requested`: a thread that is shown has its usage asked once, when a
    /// figure of it is shown.
    fn on_shown(state: &mut State, thread: &ThreadId, cx: &mut AppCx<'_>) {
        if !is_shown(state) && !state.card {
            return;
        }
        let machine_ctx = ctx(state, cx.view());
        if let Some(request) = state.machine.ensure_requested(thread.as_str(), &machine_ctx) {
            ask(cx, request);
        }
    }

    /// A widget shows a figure (the status line, the title, a `/status` card): the usage of the
    /// thread in use is asked; the last that lets go stops the polling.
    fn on_interest(
        state: &mut State,
        interest: &UsageInterest,
        open: bool,
        cx: &mut AppCx<'_>,
    ) {
        match interest {
            UsageInterest::Credits => state.selected.credits = open,
            UsageInterest::EstimatedCost => state.selected.estimated_cost = open,
            UsageInterest::Card => state.card = open,
        }
        if !open {
            if !is_shown(state) {
                state.machine.cancel_polling();
            }
            return;
        }
        let Some(thread) = cx.view().active().map(ToString::to_string) else {
            return;
        };
        let machine_ctx = ctx(state, cx.view());
        let request = match interest {
            // `request_thread_usage_for_status`: a card waits for a figure of its own.
            UsageInterest::Card => state.machine.for_status(&thread, 0, &machine_ctx),
            UsageInterest::Credits | UsageInterest::EstimatedCost => {
                state.machine.ensure_requested(&thread, &machine_ctx)
            }
        };
        if let Some(request) = request {
            ask(cx, request);
        }
    }

    /// The settlement and the retries are moments the clock is asked to say.
    fn wake_at(state: &State) -> Option<(Now, Intent)> {
        state
            .machine
            .next_due()
            .map(|millis| (Now::at_millis(millis), Intent::Due))
    }

    fn wire(command: &Read, _thread: &ThreadId, id: RequestId) -> ClientRequest {
        command.request(id)
    }

    fn decode(command: &Read, result: serde_json::Value) -> Result<Outcome, RpcError> {
        command.reply(result)
    }
}
