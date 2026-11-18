//! The history search's debounce (oracle: `search_agents_overview` in
//! `agents_overview_discovery.rs`): the first page of a search waits 180 ms
//! before it asks the server, and a newer search drops the one still
//! pending. The model's text changes at once; only the request is held
//! back. A continuation page of the same search goes out without waiting.

use std::sync::Arc;
use std::time::Duration;

use codex_tui_core::app::Command;
use codex_tui_core::screens::ScreenCommand;
use codex_tui_core::screens::agents::Command as AgentsCommand;
use futures::future::AbortHandle;
use futures::future::Abortable;
use tokio::sync::watch;

use super::workspace::Boxed;

/// How long the first page of a search waits (oracle: the `sleep` of
/// `search_agents_overview`).
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(180);

/// A pause that ends when someone opens it: what a first page waits for
/// instead of the clock, where a test must not depend on the machine's
/// speed. Cloned, every copy is the same gate; once open it stays open.
#[derive(Clone, Debug)]
pub struct Gate(Arc<watch::Sender<bool>>);

impl Default for Gate {
    fn default() -> Self {
        Self::new()
    }
}

impl Gate {
    pub fn new() -> Self {
        Self(Arc::new(watch::channel(false).0))
    }

    /// Opens the gate: everything waiting on it goes on.
    pub fn release(&self) {
        self.0.send_replace(true);
    }

    async fn opened(&self) {
        let mut state = self.0.subscribe();
        // The sender is held by this gate, so the wait cannot fail.
        let _ = state.wait_for(|open| *open).await;
    }
}

/// What a first page waits for.
enum Pause {
    /// The oracle's `sleep`.
    Delay(Duration),
    Gate(Gate),
}

/// The search request still pending, if any.
pub struct Latest {
    pause: Pause,
    pending: Option<AbortHandle>,
}

impl Default for Latest {
    fn default() -> Self {
        Self::new(SEARCH_DEBOUNCE)
    }
}

impl Latest {
    pub fn new(delay: Duration) -> Self {
        Self {
            pause: Pause::Delay(delay),
            pending: None,
        }
    }

    /// The first page waits until `gate` opens, not for a time.
    pub fn paused_by(gate: &Gate) -> Self {
        Self {
            pause: Pause::Gate(gate.clone()),
            pending: None,
        }
    }

    /// `work` as it must run for `command`: a search drops the search
    /// before it (it never answers), and the first page of one waits for
    /// the delay; anything else runs as it is.
    pub fn wrap<T: Send + 'static>(
        &mut self,
        command: &Command,
        work: Boxed<Option<T>>,
    ) -> Boxed<Option<T>> {
        let Command::Screen {
            command: ScreenCommand::Agents(AgentsCommand::Search { first, .. }),
            ..
        } = command
        else {
            return work;
        };
        if let Some(previous) = self.pending.take() {
            previous.abort();
        }
        let (handle, registration) = AbortHandle::new_pair();
        self.pending = Some(handle);
        let wait = first.then(|| match &self.pause {
            Pause::Delay(delay) => Pause::Delay(*delay),
            Pause::Gate(gate) => Pause::Gate(gate.clone()),
        });
        let held = Abortable::new(
            async move {
                match wait {
                    Some(Pause::Delay(delay)) if !delay.is_zero() => {
                        tokio::time::sleep(delay).await;
                    }
                    Some(Pause::Gate(gate)) => gate.opened().await,
                    _ => {}
                }
                work.await
            },
            registration,
        );
        Box::pin(async move { held.await.unwrap_or(None) })
    }
}
