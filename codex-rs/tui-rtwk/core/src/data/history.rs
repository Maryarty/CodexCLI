//! The history file as the app holds it: what the file said ([`Store`], changed only by `apply`,
//! `popovers::Ev::History*`) and what the shell has to read from it ([`Lookup`]). The walk over
//! the entries (Up/Down recall, Ctrl+R search) is one composer's and lives in `kit::history`
//! (ADR-013 O3); it reads the store through `&Store` and answers a cache miss with a [`Lookup`].
//!
//! Oracle: `bottom_pane/chat_composer_history.rs` and `chat_composer_history/search_batch.rs`.

use std::collections::HashMap;

/// How often a failed batch read is asked again before the search gives up.
pub const MAX_BATCH_READ_RETRIES: u8 = 2;

/// What the shell has to read from the history file for a composer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lookup {
    /// The entry at `offset` of log `log_id` (`LookupMessageHistoryEntry`).
    Entry { log_id: u64, offset: usize },
    /// The bounded batch of entries ending at `end_offset`, newest first
    /// (`LookupMessageHistoryBatch`).
    Batch { log_id: u64, end_offset: usize },
}

/// What the history file said, shared by the composers of the app.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Store {
    pub(crate) log_id: Option<u64>,
    /// The entries in the file when the metadata was read: offsets below
    /// this are persistent, above are a composer's local entries.
    pub(crate) entry_count: usize,
    /// The text at an offset, `None` for a row that could not be read.
    pub(crate) fetched: HashMap<usize, Option<String>>,
    /// Batches that came back: where they end, and where the next older one
    /// ends (none at the start of the file).
    pub(crate) batches: HashMap<usize, Option<usize>>,
    /// How many times a batch read failed, by where it ends.
    batch_errors: HashMap<usize, u8>,
    /// Counts the metadata changes: a composer's cursors are of one epoch.
    pub(crate) epoch: u64,
    /// Counts every change: a composer polls when it moved.
    rev: u64,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    /// A store whose file holds `entries`, oldest first, all read already
    /// (the seed form `{"history": [..]}`).
    pub fn seeded(entries: Vec<String>) -> Self {
        let mut store = Self::new();
        store.set_metadata(1, entries.len());
        store.fetched = entries
            .into_iter()
            .enumerate()
            .map(|(offset, text)| (offset, Some(text)))
            .collect();
        store
    }

    /// The file was (re)opened: a new log and its entry count. What was
    /// fetched is of the old one and goes (`set_metadata`).
    pub fn set_metadata(&mut self, log_id: u64, entry_count: usize) {
        self.log_id = Some(log_id);
        self.entry_count = entry_count;
        self.fetched.clear();
        self.batches.clear();
        self.batch_errors.clear();
        self.epoch += 1;
        self.rev += 1;
    }

    /// One entry came back. `false` for an answer about another log.
    pub fn on_entry(&mut self, log_id: u64, offset: usize, entry: Option<String>) -> bool {
        if self.log_id != Some(log_id) {
            return false;
        }
        self.fetched.insert(offset, entry);
        self.rev += 1;
        true
    }

    /// A batch came back: `entries` (offset, text) and where the next older
    /// batch ends. `false` for an answer about another log.
    pub fn on_batch(
        &mut self,
        log_id: u64,
        end_offset: usize,
        entries: Vec<(usize, Option<String>)>,
        next_older: Option<usize>,
    ) -> bool {
        if self.log_id != Some(log_id) {
            return false;
        }
        for (offset, entry) in entries {
            if entry.is_some() {
                self.fetched.insert(offset, entry);
            } else {
                self.fetched.entry(offset).or_insert(None);
            }
        }
        self.batches.insert(end_offset, next_older);
        self.rev += 1;
        true
    }

    /// A batch read failed. `false` for an answer about another log.
    pub fn on_batch_error(&mut self, log_id: u64, end_offset: usize) -> bool {
        if self.log_id != Some(log_id) {
            return false;
        }
        *self.batch_errors.entry(end_offset).or_default() += 1;
        self.rev += 1;
        true
    }

    pub fn log_id(&self) -> Option<u64> {
        self.log_id
    }

    /// The persistent entries.
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// Counts every change; a composer that waits polls when it moved.
    pub fn rev(&self) -> u64 {
        self.rev
    }

    pub(crate) fn errors(&self, end_offset: usize) -> u8 {
        self.batch_errors.get(&end_offset).copied().unwrap_or(0)
    }
}
