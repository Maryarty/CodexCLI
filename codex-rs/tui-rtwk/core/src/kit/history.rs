//! The composer's history walk (`kit::history`, ADR-013 O3): shell-style Up/Down recall and Ctrl+R search over
//! one offset space of persistent (cross-session) entries and local
//! (this composer's) ones. Pure, no rtwk, no IO: what has to be read from the
//! history file leaves as [`Lookup`] values and comes back through the
//! [`Store`].
//!
//! Oracle: `bottom_pane/chat_composer_history.rs` and
//! `chat_composer_history/search_batch.rs` (`ChatComposerHistory`). The oracle
//! keeps one state machine per composer and hands it each reply; here the
//! split follows who may change what (ADR-003):
//! - [`Store`] (`screens::composer::history`): what the file said (the metadata, the entries
//!   fetched so far, the outcome of batch reads), shared by every composer, changed only by
//!   `apply` (`popovers::Ev::History*`);
//! - [`History`]: one composer's walk: its local entries, the Up/Down cursor
//!   and the Ctrl+R search with its unique matches. UI state (`ComposerDraft`),
//!   it reads the store through `&Store` and answers a cache miss with a
//!   [`Lookup`]; `poll` resumes what waited when the store changed.
//!
//! Normal navigation fetches persistent entries one at a time. A reverse
//! search probes the newest entry once and then reads bounded,
//! query-independent batches (`Lookup::Batch`); a batch fills the cache even
//! when the search that asked has moved on, and only the awaited one resumes
//! a search. A failed batch is retried [`MAX_BATCH_READ_RETRIES`] times.
//! Search deduplicates by exact text within one search session.

use std::collections::HashSet;
use std::path::PathBuf;

use super::mentions::codec::decode_history_mentions_with_at_mentions;
pub use crate::screens::composer::history::{Lookup, MAX_BATCH_READ_RETRIES, Store};

/// A placeholder or span of the entry's text (`TextElement`): the bytes
/// `start..end`, with the placeholder text when the span is atomic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextElement {
    pub start: usize,
    pub end: usize,
    pub placeholder: Option<String>,
}

/// A skill, app or plugin an entry's text mentions (`MentionBinding`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MentionBinding {
    pub sigil: char,
    pub mention: String,
    pub path: String,
}

/// A history entry that can rebuild a draft (`HistoryEntry`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistoryEntry {
    /// The text as stored (it may hold placeholder strings).
    pub text: String,
    /// The placeholders of `text`.
    pub text_elements: Vec<TextElement>,
    /// The files of the `[Image #N]` placeholders, in order.
    pub local_image_paths: Vec<PathBuf>,
    pub remote_image_urls: Vec<String>,
    pub mention_bindings: Vec<MentionBinding>,
    /// Placeholder and the payload it stands for, for large pastes.
    pub pending_pastes: Vec<(String, String)>,
}

impl HistoryEntry {
    /// A text-only entry.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// The entry of a line of the history file. The file keeps no
    /// attachments or element metadata, so this is where the mention codec
    /// recovers the bindings of a stored line (`new_with_at_mentions`).
    pub fn persisted(text: &str, at_mentions_enabled: bool) -> Self {
        let decoded = decode_history_mentions_with_at_mentions(text, at_mentions_enabled);
        Self {
            text: decoded.text,
            text_elements: decoded
                .task_mention_ranges
                .into_iter()
                .map(|range| TextElement {
                    start: range.start,
                    end: range.end,
                    placeholder: None,
                })
                .collect(),
            mention_bindings: decoded
                .mentions
                .into_iter()
                .map(|mention| MentionBinding {
                    sigil: mention.sigil,
                    mention: mention.mention,
                    path: mention.path,
                })
                .collect(),
            ..Self::default()
        }
    }

    fn is_blank(&self) -> bool {
        self.text.is_empty()
            && self.text_elements.is_empty()
            && self.local_image_paths.is_empty()
            && self.remote_image_urls.is_empty()
            && self.mention_bindings.is_empty()
            && self.pending_pastes.is_empty()
    }
}

/// Result of one step of an incremental search.
///
/// `Pending`: a read was asked for and the search stays open until the
/// store has it (`poll`). `AtBoundary`: the current match stays, there is no
/// further unique one that way (not a miss). `Unavailable` ends a failed read
/// without claiming the query matches nothing.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchResult {
    Found(HistoryEntry),
    Pending,
    AtBoundary,
    NotFound,
    Unavailable,
}

/// What `poll` made of what came into the store.
#[derive(Clone, Debug, PartialEq)]
pub enum Response {
    /// The entry navigation waited for.
    Found(HistoryEntry),
    /// The next state of the search that waited.
    Search(SearchResult),
    Ignored,
}

/// One composer's walk through the history.
#[derive(Clone, Debug, Default)]
pub struct History {
    /// The store epoch the cursors below belong to.
    epoch: u64,
    /// What this composer sent or cleared, newest last, with the whole draft
    /// state (elements, images, pastes).
    local_history: Vec<HistoryEntry>,
    /// Local entries seeded from a resumed transcript's replay.
    replay_seeded_history: Vec<HistoryEntry>,
    /// Where Up/Down stands in the combined space; none: not browsing.
    history_cursor: Option<isize>,
    pending_navigation_direction: Option<Direction>,
    /// The text the last recall put in the draft: the next Up/Down walks
    /// only while the draft still is that text.
    last_history_text: Option<String>,
    search: Option<SearchState>,
    /// Whether a stored line's `[@name](thread://id)` links come back as
    /// task mentions (`set_at_mention_restore_enabled`).
    at_mention_restore_enabled: bool,
}

/// One active search query.
///
/// Two cursors: `selected_offset` (the raw offset the scan goes on from) and
/// `selected_match_index` (into `unique_matches`, so the matches found
/// already are revisited without a rescan). `seen_texts` keys on exact text:
/// the user sees and accepts text, not records.
#[derive(Clone, Debug)]
struct SearchState {
    query: String,
    query_lower: String,
    selected_offset: Option<usize>,
    unique_matches: Vec<UniqueMatch>,
    selected_match_index: Option<usize>,
    seen_texts: HashSet<String>,
    awaiting: Option<Awaiting>,
    /// Where the next older batch ends, kept so a match does not send the
    /// next Older back to the prefix scan.
    next_older: Option<usize>,
    exhausted_older: bool,
    exhausted_newer: bool,
}

#[derive(Clone, Debug)]
struct UniqueMatch {
    offset: usize,
    entry: HistoryEntry,
}

/// The read a search waits for, with the boundary rule it was asked under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Awaiting {
    Entry {
        offset: usize,
        direction: Direction,
        boundary_if_exhausted: bool,
    },
    Batch {
        end_offset: usize,
        boundary_if_exhausted: bool,
        read_failures: u8,
        /// The store's failures of this batch when it was asked: a failure
        /// is one that came after.
        errors_seen: u8,
    },
}

impl SearchState {
    fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            query_lower: query.to_lowercase(),
            selected_offset: None,
            unique_matches: Vec::new(),
            selected_match_index: None,
            seen_texts: HashSet::new(),
            awaiting: None,
            next_older: None,
            exhausted_older: false,
            exhausted_newer: false,
        }
    }

    fn is_exhausted(&self, direction: Direction) -> bool {
        match direction {
            Direction::Older => self.exhausted_older,
            Direction::Newer => self.exhausted_newer,
        }
    }

    fn mark_exhausted(&mut self, direction: Direction) {
        match direction {
            Direction::Older => self.exhausted_older = true,
            Direction::Newer => self.exhausted_newer = true,
        }
    }

    fn record_match(&mut self, offset: usize, entry: &HistoryEntry) {
        if let Some(index) = self.unique_matches.iter().position(|m| m.offset == offset) {
            self.select_match(index);
            return;
        }
        self.seen_texts.insert(entry.text.clone());
        // newest to oldest offset
        let at = self.unique_matches.partition_point(|m| m.offset > offset);
        self.unique_matches.insert(
            at,
            UniqueMatch {
                offset,
                entry: entry.clone(),
            },
        );
        self.select_match(at);
    }

    fn select_match(&mut self, index: usize) {
        let Some(found) = self.unique_matches.get(index) else {
            return;
        };
        self.selected_offset = Some(found.offset);
        self.selected_match_index = Some(index);
        self.awaiting = None;
        self.exhausted_older = false;
        self.exhausted_newer = false;
    }
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether stored `[@name](thread://id)` links come back as task
    /// mentions; turning it on or off drops the cursors and the search, whose
    /// entries were decoded the other way.
    pub fn set_at_mention_restore_enabled(&mut self, enabled: bool) {
        if self.at_mention_restore_enabled == enabled {
            return;
        }
        self.at_mention_restore_enabled = enabled;
        self.history_cursor = None;
        self.last_history_text = None;
        self.search = None;
    }

    /// The cursors of an older log are void (`set_metadata`: "every
    /// configuration resets navigation cursors and search state"). The
    /// replay seeds stay: the oracle's metadata always comes before the
    /// replay of the thread, here the file may be read after it, and the
    /// seeds are this thread's own prompts either way.
    fn sync(&mut self, store: &Store) {
        if self.epoch != store.epoch {
            self.epoch = store.epoch;
            self.history_cursor = None;
            self.pending_navigation_direction = None;
            self.last_history_text = None;
            self.search = None;
        }
    }

    /// This composer's entries, oldest first.
    pub fn local(&self) -> &[HistoryEntry] {
        &self.local_history
    }

    /// A submission (or a cleared draft) of this session, so Up can bring it
    /// back with its whole draft state. Empty ones are ignored, an equal
    /// repeat is collapsed, and the walk and the search start over: a new
    /// newest entry moves every offset. `true` when it was recorded.
    pub fn record_local_submission(&mut self, entry: HistoryEntry) -> bool {
        if entry.is_blank() {
            return false;
        }
        self.history_cursor = None;
        self.pending_navigation_direction = None;
        self.last_history_text = None;
        self.search = None;
        if self.local_history.last() == Some(&entry) {
            return false;
        }
        self.local_history.push(entry);
        true
    }

    /// A submission of a replayed transcript: recorded as a local one, and
    /// remembered so the file's copy of it is not offered twice.
    pub fn record_replayed_submission(&mut self, entry: HistoryEntry) {
        if self.record_local_submission(entry.clone()) {
            self.replay_seeded_history.push(entry);
        }
    }

    /// The text the draft shows after a recall changed it (a leading `!`
    /// left for shell mode, say), so the walk goes on from it
    /// (`record_recalled_text`).
    pub fn record_recalled_text(&mut self, text: String) {
        self.last_history_text = Some(text);
    }

    pub fn is_navigating(&self) -> bool {
        self.history_cursor.is_some()
    }

    /// The text of the last recall.
    pub fn last_recalled_text(&self) -> Option<&str> {
        self.last_history_text.as_deref()
    }

    /// The next Up starts from the newest entry again; the search ends too:
    /// the walk and the search keep different cursors.
    pub fn reset_navigation(&mut self) {
        self.history_cursor = None;
        self.pending_navigation_direction = None;
        self.last_history_text = None;
        self.search = None;
    }

    /// Only the search ends (it was accepted, or its query emptied): the
    /// next one starts with a fresh cache of unique matches.
    pub fn reset_search(&mut self) {
        self.search = None;
    }

    /// Whether the search waits for a read.
    pub fn is_waiting(&self) -> bool {
        self.search.as_ref().is_some_and(|s| s.awaiting.is_some())
            || (self.history_cursor.is_some() && self.pending_navigation_direction.is_some())
    }

    /// Whether Up/Down walk the history now: on an empty draft always (when
    /// there is any history), otherwise only on the text the last recall
    /// put there with the caret at one of its ends, so a multi-line draft
    /// keeps its vertical movement (`should_handle_navigation`).
    pub fn should_handle_navigation(&self, store: &Store, text: &str, cursor: usize) -> bool {
        if store.entry_count == 0 && self.local_history.is_empty() {
            return false;
        }
        if text.is_empty() {
            return true;
        }
        if cursor != 0 && cursor != text.len() {
            return false;
        }
        matches!(&self.last_history_text, Some(prev) if prev == text)
    }

    /// Up: one entry older in the combined space. A local or cached entry is
    /// returned at once; a persistent one that is not in the store is asked
    /// for (`fx`) and `None` returned until `poll` has it. Ends a search.
    pub fn navigate_up(&mut self, store: &Store, fx: &mut Vec<Lookup>) -> Option<HistoryEntry> {
        self.sync(store);
        self.search = None;
        let total = self.total_entries(store);
        if total == 0 {
            return None;
        }
        let next = match self.history_cursor {
            None => total as isize - 1,
            Some(0) => return None, // already at the oldest
            Some(at) => at - 1,
        };
        self.history_cursor = Some(next);
        self.populate_history_at_index(store, next as usize, Direction::Older, fx)
    }

    /// Down: one entry newer, or past the newest an empty entry (the draft
    /// is to be cleared). Ends a search.
    pub fn navigate_down(&mut self, store: &Store, fx: &mut Vec<Lookup>) -> Option<HistoryEntry> {
        self.sync(store);
        self.search = None;
        let total = self.total_entries(store);
        if total == 0 {
            return None;
        }
        let next = match self.history_cursor {
            None => return None, // not browsing
            Some(at) if at as usize + 1 >= total => None,
            Some(at) => Some(at + 1),
        };
        match next {
            Some(at) => {
                self.history_cursor = Some(at);
                self.populate_history_at_index(store, at as usize, Direction::Newer, fx)
            }
            None => {
                self.history_cursor = None;
                self.pending_navigation_direction = None;
                self.last_history_text = None;
                Some(HistoryEntry::new(""))
            }
        }
    }

    /// The step of the search `query` asks for. `restart` after opening the
    /// search or editing the query: the matches found are dropped and the
    /// scan starts at the newest entry. Without it the step is relative to
    /// the current match, which stays at a boundary. Called while a read is
    /// still awaited it keeps answering `Pending`, so a late reply cannot
    /// race a newer key and put an unexpected entry in the draft.
    pub fn search(
        &mut self,
        store: &Store,
        query: &str,
        direction: Direction,
        restart: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        self.sync(store);
        let total = self.total_entries(store);
        if total == 0 {
            self.search = Some(SearchState::new(query));
            return SearchResult::NotFound;
        }
        let query_changed = self.search.as_ref().is_none_or(|s| s.query != query);
        if !query_changed
            && !restart
            && self.search.as_ref().and_then(|s| s.awaiting).is_some()
        {
            return SearchResult::Pending;
        }
        if query_changed || restart || self.search.is_none() {
            self.search = Some(SearchState::new(query));
        } else if let Some(search) = self.search.as_mut() {
            search.awaiting = None;
        }
        let boundary_if_exhausted = !restart
            && self
                .search
                .as_ref()
                .and_then(|s| s.selected_offset)
                .is_some();
        if !restart
            && !query_changed
            && let Some(result) = self.select_cached_unique_match(direction)
        {
            return result;
        }
        if boundary_if_exhausted
            && self
                .search
                .as_ref()
                .is_some_and(|s| s.is_exhausted(direction))
        {
            return SearchResult::AtBoundary;
        }
        let start = self.search_start_offset(total, direction, query_changed || restart);
        let Some(start) = start else {
            return self.exhausted_search_result(direction, boundary_if_exhausted);
        };
        let result = self.advance_search_from(store, start, direction, boundary_if_exhausted, fx);
        if matches!(result, SearchResult::NotFound) {
            self.exhausted_search_result(direction, boundary_if_exhausted)
        } else {
            result
        }
    }

    /// Resumes what waited for the store: a search for an entry or batch it
    /// has now, or the Up/Down walk for the entry it asked. `None` when
    /// nothing waited or what it waits for is not in yet. Call it when the
    /// store's `rev` moved.
    pub fn poll(&mut self, store: &Store, fx: &mut Vec<Lookup>) -> Option<Response> {
        self.sync(store);
        if let Some(awaiting) = self.search.as_ref().and_then(|s| s.awaiting) {
            return match awaiting {
                Awaiting::Entry {
                    offset,
                    direction,
                    boundary_if_exhausted,
                } if store.fetched.contains_key(&offset) => Some(Response::Search(
                    self.resume_after_entry(store, offset, direction, boundary_if_exhausted, fx),
                )),
                Awaiting::Batch {
                    end_offset,
                    boundary_if_exhausted,
                    read_failures,
                    errors_seen,
                } => {
                    if let Some(next_older) = store.batches.get(&end_offset) {
                        Some(Response::Search(self.resume_after_batch(
                            store,
                            end_offset,
                            *next_older,
                            boundary_if_exhausted,
                            fx,
                        )))
                    } else if store.errors(end_offset) > errors_seen {
                        Some(Response::Search(self.after_batch_error(
                            store,
                            end_offset,
                            boundary_if_exhausted,
                            read_failures,
                            fx,
                        )))
                    } else {
                        None
                    }
                }
                Awaiting::Entry { .. } => None,
            };
        }
        let (cursor, direction) = (
            self.history_cursor?,
            self.pending_navigation_direction?,
        );
        let offset = usize::try_from(cursor).ok()?;
        let fetched = store.fetched.get(&offset)?;
        self.pending_navigation_direction = None;
        let Some(text) = fetched else {
            return Some(Response::Ignored);
        };
        let entry = HistoryEntry::persisted(text, self.at_mention_restore_enabled);
        if self.persistent_entry_duplicates_local(&entry)
            && let Some(next) = self.next_history_offset(store, offset, direction)
        {
            self.history_cursor = Some(next as isize);
            return Some(
                self.populate_history_at_index(store, next, direction, fx)
                    .map(Response::Found)
                    .unwrap_or(Response::Ignored),
            );
        }
        self.record_recalled_text(entry.text.clone());
        Some(Response::Found(entry))
    }

    // ---------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------

    fn total_entries(&self, store: &Store) -> usize {
        store.entry_count + self.local_history.len()
    }

    fn search_start_offset(
        &self,
        total: usize,
        direction: Direction,
        restart: bool,
    ) -> Option<usize> {
        let selected = self.search.as_ref().and_then(|s| s.selected_offset);
        match direction {
            Direction::Older => {
                if restart {
                    total.checked_sub(1)
                } else {
                    selected.and_then(|offset| offset.checked_sub(1))
                }
            }
            Direction::Newer => {
                if restart {
                    Some(0)
                } else {
                    selected
                        .and_then(|offset| offset.checked_add(1))
                        .filter(|offset| *offset < total)
                }
            }
        }
    }

    fn advance_search_after(
        &mut self,
        store: &Store,
        offset: usize,
        direction: Direction,
        boundary_if_exhausted: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        let Some(next) = self.next_history_offset(store, offset, direction) else {
            return self.exhausted_search_result(direction, boundary_if_exhausted);
        };
        let result = self.advance_search_from(store, next, direction, boundary_if_exhausted, fx);
        if matches!(result, SearchResult::NotFound) {
            self.exhausted_search_result(direction, boundary_if_exhausted)
        } else {
            result
        }
    }

    fn advance_search_from(
        &mut self,
        store: &Store,
        mut offset: usize,
        direction: Direction,
        boundary_if_exhausted: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        let total = self.total_entries(store);
        while offset < total {
            if let Some(entry) = self.entry_at_cached_offset(store, offset) {
                if self.search_matches(&entry) && self.search_result_is_unique(&entry) {
                    return self.search_match(offset, entry);
                }
            } else if !store.fetched.contains_key(&offset) && offset < store.entry_count {
                if direction == Direction::Older
                    && let Some(end_offset) = self.search.as_ref().and_then(|s| s.next_older)
                    && end_offset == offset
                {
                    return self.request_older_search_batch(
                        store,
                        end_offset,
                        boundary_if_exhausted,
                        fx,
                    );
                }
                if let Some(log_id) = store.log_id {
                    if let Some(search) = self.search.as_mut() {
                        search.awaiting = Some(Awaiting::Entry {
                            offset,
                            direction,
                            boundary_if_exhausted,
                        });
                    }
                    fx.push(Lookup::Entry { log_id, offset });
                    return SearchResult::Pending;
                }
            }
            let Some(next) = self.next_history_offset(store, offset, direction) else {
                return SearchResult::NotFound;
            };
            offset = next;
        }
        SearchResult::NotFound
    }

    fn entry_at_cached_offset(&self, store: &Store, offset: usize) -> Option<HistoryEntry> {
        if offset >= store.entry_count {
            self.local_history.get(offset - store.entry_count).cloned()
        } else {
            store
                .fetched
                .get(&offset)
                .and_then(|text| text.as_deref())
                .map(|text| HistoryEntry::persisted(text, self.at_mention_restore_enabled))
        }
    }

    fn search_matches(&self, entry: &HistoryEntry) -> bool {
        let Some(search) = self.search.as_ref() else {
            return false;
        };
        search.query.is_empty() || entry.text.to_lowercase().contains(&search.query_lower)
    }

    fn search_result_is_unique(&self, entry: &HistoryEntry) -> bool {
        self.search
            .as_ref()
            .is_none_or(|s| !s.seen_texts.contains(entry.text.as_str()))
    }

    fn search_match(&mut self, offset: usize, entry: HistoryEntry) -> SearchResult {
        self.history_cursor = Some(offset as isize);
        self.record_recalled_text(entry.text.clone());
        if let Some(search) = self.search.as_mut() {
            search.selected_offset = Some(offset);
            search.record_match(offset, &entry);
            search.awaiting = None;
            search.exhausted_older = false;
            search.exhausted_newer = false;
        }
        SearchResult::Found(entry)
    }

    fn select_cached_unique_match(&mut self, direction: Direction) -> Option<SearchResult> {
        let next = {
            let search = self.search.as_ref()?;
            let selected = search.selected_match_index?;
            match direction {
                Direction::Older => {
                    let next = selected + 1;
                    (next < search.unique_matches.len()).then_some(next)?
                }
                Direction::Newer => selected.checked_sub(1)?,
            }
        };
        let found = self.search.as_ref()?.unique_matches[next].clone();
        self.history_cursor = Some(found.offset as isize);
        self.record_recalled_text(found.entry.text.clone());
        if let Some(search) = self.search.as_mut() {
            search.select_match(next);
        }
        Some(SearchResult::Found(found.entry))
    }

    fn exhausted_search_result(
        &mut self,
        direction: Direction,
        boundary_if_exhausted: bool,
    ) -> SearchResult {
        if let Some(search) = self.search.as_mut() {
            search.awaiting = None;
            if boundary_if_exhausted {
                search.mark_exhausted(direction);
            }
        }
        if boundary_if_exhausted {
            SearchResult::AtBoundary
        } else {
            SearchResult::NotFound
        }
    }

    fn populate_history_at_index(
        &mut self,
        store: &Store,
        mut at: usize,
        direction: Direction,
        fx: &mut Vec<Lookup>,
    ) -> Option<HistoryEntry> {
        loop {
            if let Some(entry) = self.entry_at_cached_offset(store, at) {
                if at < store.entry_count && self.persistent_entry_duplicates_local(&entry) {
                    let Some(next) = self.next_history_offset(store, at, direction) else {
                        self.pending_navigation_direction = None;
                        return None;
                    };
                    self.history_cursor = Some(next as isize);
                    at = next;
                    continue;
                }
                self.pending_navigation_direction = None;
                self.record_recalled_text(entry.text.clone());
                return Some(entry);
            }
            if at >= store.entry_count {
                return None;
            }
            if let Some(log_id) = store.log_id {
                self.pending_navigation_direction = Some(direction);
                fx.push(Lookup::Entry {
                    log_id,
                    offset: at,
                });
            }
            return None;
        }
    }

    fn next_history_offset(
        &self,
        store: &Store,
        offset: usize,
        direction: Direction,
    ) -> Option<usize> {
        match direction {
            Direction::Older => offset.checked_sub(1),
            Direction::Newer => offset
                .checked_add(1)
                .filter(|next| *next < self.total_entries(store)),
        }
    }

    fn persistent_entry_duplicates_local(&self, entry: &HistoryEntry) -> bool {
        self.replay_seeded_history
            .iter()
            .any(|local| local.text == entry.text && local.mention_bindings == entry.mention_bindings)
    }

    // The entry the search awaited is in the store (`on_entry_response`).
    fn resume_after_entry(
        &mut self,
        store: &Store,
        offset: usize,
        direction: Direction,
        boundary_if_exhausted: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        if let Some(entry) = self.entry_at_cached_offset(store, offset)
            && self.search_matches(&entry)
            && self.search_result_is_unique(&entry)
        {
            return self.search_match(offset, entry);
        }
        match direction {
            Direction::Older => {
                self.advance_older_search_after_entry_miss(store, offset, boundary_if_exhausted, fx)
            }
            Direction::Newer => {
                self.advance_search_after(store, offset, direction, boundary_if_exhausted, fx)
            }
        }
    }

    /// Switches an older search from the probe of the newest entry to bounded
    /// batches: the first probe stays a single read so a match on the newest
    /// entry costs no batch, every later miss is amortized.
    fn advance_older_search_after_entry_miss(
        &mut self,
        store: &Store,
        offset: usize,
        boundary_if_exhausted: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        let Some(next) = offset.checked_sub(1) else {
            return self.exhausted_search_result(Direction::Older, boundary_if_exhausted);
        };
        self.advance_older_search_with_batches_from(store, next, boundary_if_exhausted, fx)
    }

    /// Scans the cached offsets from `end_offset` down and asks for the first
    /// uncached older range.
    fn advance_older_search_with_batches_from(
        &mut self,
        store: &Store,
        mut offset: usize,
        boundary_if_exhausted: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        loop {
            if let Some(entry) = self.entry_at_cached_offset(store, offset) {
                if self.search_matches(&entry) && self.search_result_is_unique(&entry) {
                    return self.search_match(offset, entry);
                }
            } else if !store.fetched.contains_key(&offset) && offset < store.entry_count {
                return self.request_older_search_batch(store, offset, boundary_if_exhausted, fx);
            }
            let Some(next) = offset.checked_sub(1) else {
                return self.exhausted_search_result(Direction::Older, boundary_if_exhausted);
            };
            offset = next;
        }
    }

    fn request_older_search_batch(
        &mut self,
        store: &Store,
        end_offset: usize,
        boundary_if_exhausted: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        let Some(log_id) = store.log_id else {
            return self.exhausted_search_result(Direction::Older, boundary_if_exhausted);
        };
        if let Some(search) = self.search.as_mut() {
            search.awaiting = Some(Awaiting::Batch {
                end_offset,
                boundary_if_exhausted,
                read_failures: 0,
                errors_seen: store.errors(end_offset),
            });
        }
        fx.push(Lookup::Batch { log_id, end_offset });
        SearchResult::Pending
    }

    // The batch the search awaited is in the store (`on_batch_response`):
    // its rows are cached now, so the scan goes on from where the batch
    // ended, newest row first, and asks for the next uncached range. The
    // batch says where the next older one ends; that is kept so a match in
    // the middle of it does not send the next Older back to a prefix scan.
    fn resume_after_batch(
        &mut self,
        store: &Store,
        end_offset: usize,
        next_older: Option<usize>,
        boundary_if_exhausted: bool,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        if let Some(search) = self.search.as_mut() {
            search.next_older = next_older;
        }
        self.advance_older_search_with_batches_from(store, end_offset, boundary_if_exhausted, fx)
    }

    // A batch read failed: ask again up to the bound, then give up without
    // claiming the query matches nothing (`on_batch_error`).
    fn after_batch_error(
        &mut self,
        store: &Store,
        end_offset: usize,
        boundary_if_exhausted: bool,
        read_failures: u8,
        fx: &mut Vec<Lookup>,
    ) -> SearchResult {
        if read_failures < MAX_BATCH_READ_RETRIES
            && let Some(log_id) = store.log_id
        {
            if let Some(search) = self.search.as_mut() {
                search.awaiting = Some(Awaiting::Batch {
                    end_offset,
                    boundary_if_exhausted,
                    read_failures: read_failures + 1,
                    errors_seen: store.errors(end_offset),
                });
            }
            fx.push(Lookup::Batch { log_id, end_offset });
            return SearchResult::Pending;
        }
        if boundary_if_exhausted {
            if let Some(search) = self.search.as_mut() {
                search.awaiting = None;
            }
            SearchResult::AtBoundary
        } else {
            self.search = None;
            SearchResult::Unavailable
        }
    }
}

// ---- Ctrl+R: the query and where the search stands (UI state, one per thread's draft) ----

/// Where reverse history search stands (`HistorySearchStatus`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchStatus {
    /// The query is empty (or a read failed): the draft shows as it was.
    #[default]
    Idle,
    /// A persistent read is outstanding.
    Searching,
    /// A match is previewed.
    Match,
    NoMatch,
}

/// What the draft shows after a search step.
#[derive(Clone, Debug, PartialEq)]
pub enum Preview {
    /// The entry that was found.
    Show(HistoryEntry),
    /// The draft as it was when the search opened.
    Original,
    /// What is shown stays.
    Keep,
}

/// Reverse history search: the query and where it stands
/// (`chat_composer/history_search.rs`); the walk over the entries is the
/// composer's [`History`]. The composer owns it with the draft it will
/// restore (UI state, one per thread's draft).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistorySearch {
    pub query: String,
    pub status: SearchStatus,
}

/// Which way the search walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Older,
    Newer,
}

impl HistorySearch {
    /// Opens without previewing anything: Ctrl+R on an empty composer must not
    /// replace it with the latest prompt.
    pub fn new() -> Self {
        Self::default()
    }

    /// The query as the footer shows it: a newline and a tab as marks
    /// (`display_query`); matching keeps the query as it is.
    pub fn display_query(&self) -> String {
        self.query.replace('\n', "\u{21b5}").replace('\t', "\u{21e5}")
    }

    /// The query changed (`update_history_search_query`): the draft goes
    /// back to its original first, the search restarts from the newest
    /// entry, and an empty query ends the walk and stays open.
    pub fn set_query(
        &mut self,
        query: String,
        history: &mut History,
        store: &Store,
        fx: &mut Vec<Lookup>,
    ) -> Preview {
        self.query = query;
        self.status = SearchStatus::Searching;
        if self.query.is_empty() {
            history.reset_search();
            self.status = SearchStatus::Idle;
            return Preview::Original;
        }
        let result = history.search(store, &self.query, Direction::Older, true, fx);
        match self.apply(result) {
            Preview::Show(entry) => Preview::Show(entry),
            _ => Preview::Original,
        }
    }

    /// The next older or newer match (`history_search_in_direction`); at the
    /// end of the matches the current one stays (no "no match" flicker).
    pub fn step(
        &mut self,
        direction: Direction,
        history: &mut History,
        store: &Store,
        fx: &mut Vec<Lookup>,
    ) -> Preview {
        if self.query.is_empty() {
            history.reset_search();
            self.status = SearchStatus::Idle;
            return Preview::Original;
        }
        let result = history.search(store, &self.query, direction, false, fx);
        self.apply(result)
    }

    /// What a step or a read that came in says (`apply_history_search_result`):
    /// `Found` previews, `Pending` waits, `AtBoundary` keeps the match,
    /// `NotFound` puts the original back with "no match" and `Unavailable`
    /// with nothing said.
    pub fn apply(&mut self, result: SearchResult) -> Preview {
        match result {
            SearchResult::Found(entry) => {
                self.status = SearchStatus::Match;
                Preview::Show(entry)
            }
            SearchResult::Pending => {
                self.status = SearchStatus::Searching;
                Preview::Keep
            }
            SearchResult::AtBoundary => {
                self.status = SearchStatus::Match;
                Preview::Keep
            }
            SearchResult::NotFound => {
                self.status = SearchStatus::NoMatch;
                Preview::Original
            }
            SearchResult::Unavailable => {
                self.status = SearchStatus::Idle;
                Preview::Original
            }
        }
    }
}
