//! The history functions over a scripted file (oracle `thread_routing.rs`
//! lookups: a failed row is `None`, a failed batch is its own event).

use std::sync::Arc;
use std::sync::Mutex;

use pretty_assertions::assert_eq;

use super::*;

/// A history file in memory: 3 entries under log 7; row 2 cannot be read, a
/// batch ending at 99 fails.
#[derive(Default)]
struct Scripted {
    appended: Mutex<Vec<(String, String)>>,
}

impl HistoryFile for Scripted {
    fn metadata(&self) -> Boxed<'_, (u64, usize)> {
        Box::pin(async { (7, 3) })
    }

    fn append<'a>(&'a self, conversation: &'a str, text: &'a str) -> Boxed<'a, Result<(), String>> {
        Box::pin(async move {
            if text == "refused" {
                return Err("disk full".to_owned());
            }
            self.appended
                .lock()
                .unwrap()
                .push((conversation.to_owned(), text.to_owned()));
            Ok(())
        })
    }

    fn lookup(&self, log_id: u64, offset: usize) -> Option<String> {
        assert_eq!(log_id, 7);
        match offset {
            0 => Some("first".to_owned()),
            1 => Some("second".to_owned()),
            _ => None,
        }
    }

    fn lookup_batch(&self, log_id: u64, end_offset: usize) -> Result<Batch, String> {
        assert_eq!(log_id, 7);
        match end_offset {
            3 => Ok(Batch {
                rows: vec![(2, None), (1, Some("second".to_owned()))],
                next_older: Some(1),
            }),
            1 => Ok(Batch {
                rows: vec![(0, Some("first".to_owned()))],
                next_older: None,
            }),
            _ => Err("cannot read the history".to_owned()),
        }
    }
}

fn file() -> Arc<dyn HistoryFile> {
    Arc::new(Scripted::default())
}

#[tokio::test]
async fn opening_a_thread_reads_the_log_id_and_the_entry_count() {
    assert_eq!(
        metadata(&*file()).await,
        Ev::HistoryMetadata {
            log_id: 7,
            entry_count: 3
        }
    );
}

#[tokio::test]
async fn an_entry_read_answers_its_text_and_none_for_a_row_that_cannot_be_read() {
    let file = file();

    assert_eq!(
        lookup(
            &file,
            Lookup::Entry {
                log_id: 7,
                offset: 1
            }
        )
        .await,
        Ev::HistoryEntry {
            log_id: 7,
            offset: 1,
            text: Some("second".to_owned())
        }
    );
    assert_eq!(
        lookup(
            &file,
            Lookup::Entry {
                log_id: 7,
                offset: 2
            }
        )
        .await,
        Ev::HistoryEntry {
            log_id: 7,
            offset: 2,
            text: None
        }
    );
}

#[tokio::test]
async fn a_batch_read_answers_its_rows_and_where_the_next_older_one_ends() {
    let file = file();

    assert_eq!(
        lookup(
            &file,
            Lookup::Batch {
                log_id: 7,
                end_offset: 3
            }
        )
        .await,
        Ev::HistoryBatch {
            log_id: 7,
            end_offset: 3,
            rows: vec![(2, None), (1, Some("second".to_owned()))],
            next_older: Some(1),
        }
    );
    assert_eq!(
        lookup(
            &file,
            Lookup::Batch {
                log_id: 7,
                end_offset: 1
            }
        )
        .await,
        Ev::HistoryBatch {
            log_id: 7,
            end_offset: 1,
            rows: vec![(0, Some("first".to_owned()))],
            next_older: None,
        }
    );
}

#[tokio::test]
async fn a_batch_read_that_fails_is_its_own_event() {
    assert_eq!(
        lookup(
            &file(),
            Lookup::Batch {
                log_id: 7,
                end_offset: 99
            }
        )
        .await,
        Ev::HistoryBatchFailed {
            log_id: 7,
            end_offset: 99
        }
    );
}

#[tokio::test]
async fn a_submitted_prompt_is_appended_for_its_conversation_and_a_failure_is_returned() {
    let scripted = Arc::new(Scripted::default());
    let file: Arc<dyn HistoryFile> = scripted.clone();

    append(&*file, "thread-1", "hello").await.unwrap();
    let failed = append(&*file, "thread-1", "refused").await;

    assert_eq!(
        scripted.appended.lock().unwrap().as_slice(),
        [("thread-1".to_owned(), "hello".to_owned())]
    );
    assert_eq!(failed, Err("disk full".to_owned()));
}
