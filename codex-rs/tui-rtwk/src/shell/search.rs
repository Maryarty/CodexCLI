//! The history search's debounce (oracle: `search_agents_overview` in
//! `agents_overview_discovery.rs`): the first page of a search waits 180 ms
//! before it asks the server, and a newer search drops the one still
//! pending. The model's text changes at once; only the request is held
//! back. A continuation page of the same search goes out without waiting.

use std::time::Duration;

use codex_tui_core::app::Command;
use codex_tui_core::screens::ScreenCommand;
use codex_tui_core::screens::agents::Command as AgentsCommand;
use futures::future::AbortHandle;
use futures::future::Abortable;

use super::workspace::Boxed;

/// How long the first page of a search waits (oracle: the `sleep` of
/// `search_agents_overview`).
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(180);

/// The search request still pending, if any.
pub struct Latest {
    delay: Duration,
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
            delay,
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
        let wait = if *first { self.delay } else { Duration::ZERO };
        let held = Abortable::new(
            async move {
                if !wait.is_zero() {
                    tokio::time::sleep(wait).await;
                }
                work.await
            },
            registration,
        );
        Box::pin(async move { held.await.unwrap_or(None) })
    }
}
