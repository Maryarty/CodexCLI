//! Restoring a lost connection to an out-of-process server, as the oracle's
//! `app/reconnect.rs::reconnect` does: attempts after 0, 1, 2, 4 and then
//! every 8 seconds, within one 120 second budget; the command center says
//! `Reconnecting` meanwhile and `Reconnect failed` when the budget ends.
//! The connection itself is the host's (`connect`); this is the policy and
//! what the app is told, so it is tested with a scripted `connect`.

use std::future::Future;
use std::time::Duration;

use codex_tui_core::app::AppEv;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::Ev as AgentsEv;
use codex_tui_core::screens::agents::Notice;
use tokio::time::Instant;

use crate::services::shell::IngressTx;
use crate::services::shell::emit;

/// The waits before the attempts; the last one repeats.
pub const DELAYS: [u64; 4] = [0, 1, 2, 4];
pub const REPEAT: u64 = 8;
/// The whole budget (oracle: 120 s for the conversation and the overview).
pub const BUDGET: Duration = Duration::from_secs(120);

fn agents(ev: AgentsEv) -> AppEv {
    AppEv::Screen(ScreenEv::Agents(ev))
}

/// Tells the app the connection is lost, then tries `connect` until it
/// answers or the budget ends. The host announces success once the new request handle is in
/// place. If the budget ends, the app is told `ReconnectFailed`.
pub async fn restore<C, F, Fut>(mut connect: F, ingress: &IngressTx, epoch: ConnEpoch) -> Option<C>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<C>>,
{
    emit(ingress, epoch, agents(AgentsEv::Reconnecting));
    let deadline = Instant::now() + BUDGET;
    for delay in DELAYS.into_iter().chain(std::iter::repeat(REPEAT)) {
        let attempt = async {
            tokio::time::sleep(Duration::from_secs(delay)).await;
            connect().await
        };
        match tokio::time::timeout_at(deadline, attempt).await {
            Ok(Some(client)) => return Some(client),
            Ok(None) => {}
            Err(_) => break,
        }
    }
    emit(
        ingress,
        epoch,
        agents(AgentsEv::Notice(Some(Notice::ReconnectFailed))),
    );
    None
}
