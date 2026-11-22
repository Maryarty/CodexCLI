//! The prompt history over `codex-message-history`: the oracle's
//! `history.jsonl` under the Codex home, read and written as the backend crate
//! does (advisory file locks, the inode as the log id, bounded batches).
//!
//! [`MessageHistory`] is the one [`HistoryFile`] of the app; the trait stays
//! so the tests of the functions above it need no disk.

use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::PoisonError;

use codex_config::types::History;
use codex_message_history::HistoryBatchCursor;
use codex_message_history::HistoryConfig;

use super::Batch;
use super::Boxed;
use super::HistoryFile;

/// The history file of one Codex home, under the user's `history` settings
/// (`persistence`, `max_bytes`).
pub struct MessageHistory {
    config: HistoryConfig,
    /// The cursor the last batch gave for the next older one. The backend
    /// continues a scan from its byte position when the file is unchanged
    /// and rescans from the start otherwise; core asks by offset only, so
    /// the cursor is kept here and used when the next ask is its offset
    /// (oracle: the cursor travels in the event).
    anchor: Mutex<Option<HistoryBatchCursor>>,
}

impl MessageHistory {
    pub fn new(codex_home: PathBuf, history: &History) -> Self {
        Self {
            config: HistoryConfig::new(codex_home, history),
            anchor: Mutex::new(None),
        }
    }

    fn cursor(&self, end_offset: usize) -> HistoryBatchCursor {
        self.anchor
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .filter(|cursor| cursor.end_offset() == end_offset)
            .unwrap_or_else(|| HistoryBatchCursor::new(end_offset))
    }
}

impl HistoryFile for MessageHistory {
    fn metadata(&self) -> Boxed<'_, (u64, usize)> {
        Box::pin(codex_message_history::history_metadata(&self.config))
    }

    fn append<'a>(&'a self, conversation: &'a str, text: &'a str) -> Boxed<'a, Result<(), String>> {
        Box::pin(async move {
            codex_message_history::append_entry(text, conversation, &self.config)
                .await
                .map_err(|error| error.to_string())
        })
    }

    fn lookup(&self, log_id: u64, offset: usize) -> Option<String> {
        codex_message_history::lookup(log_id, offset, &self.config).map(|entry| entry.text)
    }

    fn lookup_batch(&self, log_id: u64, end_offset: usize) -> Result<Batch, String> {
        let batch =
            codex_message_history::lookup_batch(log_id, self.cursor(end_offset), &self.config)
                .map_err(|error| error.to_string())?;
        *self.anchor.lock().unwrap_or_else(PoisonError::into_inner) = batch.next_older_cursor;
        Ok(Batch {
            rows: batch
                .entries
                .into_iter()
                .map(|row| (row.offset, row.entry.map(|entry| entry.text)))
                .collect(),
            next_older: batch.next_older_cursor.map(HistoryBatchCursor::end_offset),
        })
    }
}

#[cfg(test)]
mod tests {
    use codex_config::types::HistoryPersistence;
    use pretty_assertions::assert_eq;

    use super::*;

    fn history(home: &tempfile::TempDir) -> MessageHistory {
        MessageHistory::new(home.path().to_owned(), &History::default())
    }

    async fn written(file: &MessageHistory, count: usize) {
        for index in 0..count {
            file.append("thread-1", &format!("prompt {index}"))
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn an_empty_home_has_no_entries() {
        let home = tempfile::tempdir().unwrap();

        assert_eq!(history(&home).metadata().await, (0, 0));
    }

    #[tokio::test]
    async fn appended_prompts_are_counted_and_read_back_by_offset() {
        let home = tempfile::tempdir().unwrap();
        let file = history(&home);
        written(&file, 3).await;

        let (log_id, count) = file.metadata().await;

        assert_eq!(count, 3);
        assert_eq!(file.lookup(log_id, 0), Some("prompt 0".to_owned()));
        assert_eq!(file.lookup(log_id, 2), Some("prompt 2".to_owned()));
        assert_eq!(file.lookup(log_id, 3), None);
        // Another file identity is another log.
        assert_eq!(file.lookup(log_id + 1, 0), None);
    }

    #[tokio::test]
    async fn a_batch_is_newest_first_and_ends_at_the_start_of_the_file() {
        let home = tempfile::tempdir().unwrap();
        let file = history(&home);
        written(&file, 3).await;
        let (log_id, _) = file.metadata().await;

        // `end_offset` is the newest row to include: the last of three is 2.
        let batch = file.lookup_batch(log_id, 2).unwrap();

        assert_eq!(
            batch,
            Batch {
                rows: vec![
                    (2, Some("prompt 2".to_owned())),
                    (1, Some("prompt 1".to_owned())),
                    (0, Some("prompt 0".to_owned())),
                ],
                next_older: None,
            }
        );
    }

    #[tokio::test]
    async fn a_long_history_is_read_in_bounded_batches_that_chain() {
        let home = tempfile::tempdir().unwrap();
        let file = history(&home);
        written(&file, 200).await;
        let (log_id, _) = file.metadata().await;

        let first = file.lookup_batch(log_id, 199).unwrap();
        // The backend bounds a batch at 128 rows.
        assert_eq!(first.rows.len(), 128);
        assert_eq!(first.rows[0], (199, Some("prompt 199".to_owned())));
        let older = first.next_older.expect("older rows remain");
        assert_eq!(older, 71);

        let second = file.lookup_batch(log_id, older).unwrap();
        assert_eq!(second.rows.len(), 72);
        assert_eq!(second.rows[0], (71, Some("prompt 71".to_owned())));
        assert_eq!(second.next_older, None);
    }

    #[tokio::test]
    async fn a_batch_of_another_log_is_empty() {
        let home = tempfile::tempdir().unwrap();
        let file = history(&home);
        written(&file, 2).await;
        let (log_id, _) = file.metadata().await;

        assert_eq!(
            file.lookup_batch(log_id + 1, 1).unwrap(),
            Batch {
                rows: Vec::new(),
                next_older: None
            }
        );
    }

    #[tokio::test]
    async fn a_batch_of_a_missing_file_fails() {
        let home = tempfile::tempdir().unwrap();

        assert!(history(&home).lookup_batch(1, 1).is_err());
    }

    #[tokio::test]
    async fn without_persistence_a_prompt_is_not_written() {
        let home = tempfile::tempdir().unwrap();
        let file = MessageHistory::new(
            home.path().to_owned(),
            &History {
                persistence: HistoryPersistence::None,
                max_bytes: None,
            },
        );

        file.append("thread-1", "secret").await.unwrap();

        assert_eq!(file.metadata().await, (0, 0));
    }
}
