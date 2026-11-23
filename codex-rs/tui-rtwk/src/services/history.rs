//! The composer's cross-session history and mention files.
//!
//! The prompt history is the oracle's `history.jsonl` under the Codex home
//! (`codex-message-history`: metadata, append, entry and batch reads;
//! `app/thread_routing.rs` runs them). In Flutter this is a repository behind
//! an interface: [`HistoryFile`] is the interface, [`file::MessageHistory`] is
//! the implementation over the backend crate, and the functions here run it
//! as the oracle does: the blocking reads on the blocking pool, every answer
//! one `popovers::Ev` for the app to apply. The `$` picks file is
//! [`usage::MentionFile`].

pub mod file;
pub mod usage;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use codex_tui_core::data::history::Lookup;
use codex_tui_core::screens::popovers::Ev;

/// A batch the file gave: the rows (offset, text; `None` for a row that could
/// not be read), newest first, and where the next older batch ends (none at
/// the start of the file).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Batch {
    pub rows: Vec<(usize, Option<String>)>,
    pub next_older: Option<usize>,
}

type Boxed<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The history file. Reads are blocking calls: the functions below run them
/// on the blocking pool.
pub trait HistoryFile: Send + Sync + 'static {
    /// The log id and the number of entries (`history_metadata`).
    fn metadata(&self) -> Boxed<'_, (u64, usize)>;

    /// Append a submitted prompt of `conversation` (`append_entry`).
    fn append<'a>(&'a self, conversation: &'a str, text: &'a str) -> Boxed<'a, Result<(), String>>;

    /// The text at `offset` of log `log_id`, `None` when it cannot be read.
    fn lookup(&self, log_id: u64, offset: usize) -> Option<String>;

    /// The bounded batch of entries ending at `end_offset`, newest first.
    fn lookup_batch(&self, log_id: u64, end_offset: usize) -> Result<Batch, String>;
}

/// The file was opened (oracle `history_metadata` as a thread opens): the
/// answer is `Ev::HistoryMetadata`.
pub async fn metadata(file: &dyn HistoryFile) -> Ev {
    let (log_id, entry_count) = file.metadata().await;
    Ev::HistoryMetadata {
        log_id,
        entry_count,
    }
}

/// Persist a submitted prompt in the cross-session history (oracle
/// `append_message_history_entry`): no answer; a failure is the caller's to
/// log.
pub async fn append(file: &dyn HistoryFile, conversation: &str, text: &str) -> Result<(), String> {
    file.append(conversation, text).await
}

/// Read an entry or a batch (oracle `lookup_message_history_entry` and
/// `lookup_message_history_batch`): `Ev::HistoryEntry`, `Ev::HistoryBatch` or
/// `Ev::HistoryBatchFailed`.
pub async fn lookup(file: &Arc<dyn HistoryFile>, ask: Lookup) -> Ev {
    let file = Arc::clone(file);
    match ask {
        Lookup::Entry { log_id, offset } => {
            let text = tokio::task::spawn_blocking(move || file.lookup(log_id, offset))
                .await
                .unwrap_or(None);
            Ev::HistoryEntry {
                log_id,
                offset,
                text,
            }
        }
        Lookup::Batch { log_id, end_offset } => {
            match tokio::task::spawn_blocking(move || file.lookup_batch(log_id, end_offset)).await {
                Ok(Ok(Batch { rows, next_older })) => Ev::HistoryBatch {
                    log_id,
                    end_offset,
                    rows,
                    next_older,
                },
                Ok(Err(_)) | Err(_) => Ev::HistoryBatchFailed { log_id, end_offset },
            }
        }
    }
}

#[cfg(test)]
mod tests;
