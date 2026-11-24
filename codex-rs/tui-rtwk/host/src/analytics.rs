//! The usage page's host side (S20): the account session of the local
//! sign-in, the identity checks, and the loads the page asks for (the
//! account, the reports, the allowance history, the profile, the chats).
//! Core decides what is asked and what the answers mean
//! (`screens::usage`); this reads the backend and the app server. The
//! session of a page opening lives here (`Store`) until the page asks under
//! a newer one (a refresh replaces it). Oracle: `analytics/client.rs`,
//! `chats.rs`, `tasks.rs`, `summary.rs` (`Live::profile`), `data.rs`
//! (`Load::start`'s timeouts).

mod chats;
mod client;
pub(crate) mod clock;
mod convert;
#[cfg(test)]
mod tests;

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_features::Feature;
use codex_tui_core::app::AppEv;
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::startup::launch;
use codex_tui_core::screens::usage::data::GROUPINGS;
use codex_tui_core::screens::usage::{AccountReply, Ask, AskKind, Ev, LoadError, NaiveDate};
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_tui_rtwk::services::clock::unix_now;

use crate::startup::Startup;
use client::Live;

/// The open session of the page, one at a time: the number core gave it and
/// what the host keeps of it.
#[derive(Clone, Default)]
pub struct Store(Arc<Mutex<Option<Open>>>);

type Open = (u32, Arc<Live>);

impl Store {
    /// The session `number`, opened now when the page asks under a newer one;
    /// none for an ask of a session that has been replaced (its answer would
    /// be dropped).
    fn live(&self, number: u32, config: &Arc<codex_core::config::Config>) -> Option<Arc<Live>> {
        let mut slot = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match &*slot {
            Some((open, live)) if *open == number => Some(live.clone()),
            Some((open, _)) if *open > number => None,
            _ => {
                let today = NaiveDate::from_unix_secs(unix_now());
                let live = Arc::new(Live::new(config.clone(), today));
                *slot = Some((number, live.clone()));
                Some(live)
            }
        }
    }
}

const INTERRUPTED: &str = "Request timed out. Press R to retry.";

/// `future` for at most `secs` seconds (oracle: `Load::start_with_timeout`).
async fn within<T, E>(secs: u64, future: impl Future<Output = Result<T, E>>) -> Result<T, LoadError>
where
    LoadError: From<E>,
{
    match tokio::time::timeout(Duration::from_secs(secs), future).await {
        Ok(result) => result.map_err(LoadError::from),
        Err(_) => Err(LoadError::Host(INTERRUPTED.into())),
    }
}

/// The event of `ask`: the answer under the ask's session and stamp, or the
/// page's restart when the signed-in identity changed under the session.
pub async fn run<F, Fut>(startup: Startup, ask: Ask, ids: Ids, send: F) -> Option<AppEv>
where
    F: Fn(ClientRequest) -> Fut + Sync,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    // A redemption is the account's, not a page session's: it needs no `Live`.
    if let AskKind::Redeem { key, credit_id } = &ask.what {
        return Some(crate::usage::redeem(&send, &ids, key.clone(), credit_id.clone()).await);
    }
    let config = startup.current.read().await.clone();
    let live = startup.analytics.live(ask.session, &config)?;
    let Ask {
        session,
        stamp,
        what,
    } = ask;
    let ev = match what {
        AskKind::Account => Ev::Account {
            session,
            stamp,
            result: within(30, account(&live, &ids, &send)).await,
        },
        AskKind::Report {
            section,
            days,
            group,
            model,
        } => {
            let result = match (section.report(), GROUPINGS.get(group)) {
                (Some(report), Some(grouping)) => {
                    within(30, live.history(report, days, *grouping, model.as_deref())).await
                }
                _ => Err(LoadError::Host("Invalid analytics report.".into())),
            };
            Ev::Report {
                session,
                stamp,
                section,
                result,
            }
        }
        AskKind::Profile => Ev::Profile {
            session,
            stamp,
            result: within(30, live.profile()).await,
        },
        AskKind::Plan => Ev::Plan {
            session,
            stamp,
            result: within(30, live.plan_history()).await,
        },
        AskKind::Chats => Ev::Chats {
            session,
            stamp,
            result: within(90, chats::business(&live, &send, &ids)).await,
        },
        AskKind::Tasks => Ev::Tasks {
            session,
            stamp,
            result: within(120, chats::consumer(&live, &send, &ids)).await,
        },
        // Answered above, before a session is opened for it.
        AskKind::Redeem { .. } => return None,
    };
    let ev = if live.invalidated() {
        Ev::Invalidated { session }
    } else {
        ev
    };
    Some(AppEv::Screen(ScreenEv::Usage(ev)))
}

/// The account behind the session, with the facts the page needs of the
/// configuration and the model list (oracle: `AnalyticsView::open`,
/// `Live::session`).
async fn account<F, Fut>(live: &Live, ids: &Ids, send: &F) -> Result<AccountReply, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let session = live.session().await?;
    let model_names = match send(launch::model_list(ids.next())).await {
        Ok(result) => launch::models(result)
            .into_iter()
            .map(|model| (model.model, model.display_name))
            .collect(),
        Err(_) => Vec::new(),
    };
    Ok(AccountReply {
        plan: Some(
            session
                .backend
                .account()
                .plan_type
                .unwrap_or(codex_protocol::account::PlanType::Unknown),
        ),
        label: session.backend.account().email.clone(),
        today: live.end_date(),
        plan_enabled: live
            .config()
            .features
            .enabled(Feature::AnalyticsPlanHistory),
        model_names,
        credit_groups: session.credit_groups.clone(),
        hour24: clock::hour24(),
    })
}
