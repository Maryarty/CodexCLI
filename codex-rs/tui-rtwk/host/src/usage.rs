//! The usage-limit reset's host side (oracle `app/background_requests.rs`
//! `consume_rate_limit_reset_credit`): one `account/rateLimitResetCredit/consume` request with the
//! idempotency key core chose, under the oracle's timeout. Core decides what the outcome means
//! (`screens::usage::reset`); this only asks.

use std::future::Future;
use std::time::Duration;

use codex_app_server_protocol::ClientRequest;
use codex_tui_core::app::AppEv;
use codex_tui_core::rpc::{RpcError, ServerError};
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::usage::Ev;
use codex_tui_core::screens::usage::reset::{Consume, Outcome};
use codex_tui_rtwk::services::backend::rpc::Ids;

/// `RATE_LIMIT_RESET_REQUEST_TIMEOUT`.
const TIMEOUT: Duration = Duration::from_secs(30);

/// What the server answered, or why the ask failed (oracle: the error texts of the ask).
async fn consume<F, Fut>(
    send: &F,
    ids: &Ids,
    key: String,
    credit_id: Option<String>,
) -> Result<Outcome, String>
where
    F: Fn(ClientRequest) -> Fut + Sync,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let redeem = Consume::Redeem { key, credit_id };
    let answer = tokio::time::timeout(TIMEOUT, send(redeem.request(ids.next())))
        .await
        .map_err(|_| "account/rateLimitResetCredit/consume timed out in TUI".to_string())?
        .map_err(|error| error.to_string())?;
    // a decode failure is the only one `reply` has: its text is the serde diagnostic
    redeem.reply(answer).map_err(|error| match error {
        RpcError::Malformed(text) => text,
        other => format!("{other:?}"),
    })
}

/// The event that answers the redemption of `key`.
pub async fn redeem<F, Fut>(send: &F, ids: &Ids, key: String, credit_id: Option<String>) -> AppEv
where
    F: Fn(ClientRequest) -> Fut + Sync,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let result = consume(send, ids, key.clone(), credit_id).await;
    AppEv::Screen(ScreenEv::Usage(Ev::Redeemed { key, result }))
}
