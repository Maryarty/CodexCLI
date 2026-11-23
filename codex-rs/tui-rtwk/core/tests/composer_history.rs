//! The composer's history (`kit::history`): the oracle's
//! `chat_composer_history.rs` state tests and `search_batch_tests.rs`, carried
//! with the split of the oracle's one machine into the shared `Store` and one
//! composer's `History` (a reply goes to the store, then the composer polls).
//! A read the oracle sent as an `AppEvent` is a `Lookup` in `fx`.

use codex_tui_core::data::history::{Lookup, MAX_BATCH_READ_RETRIES, Store};
use codex_tui_core::kit::history::{
    HistoryEntry, PromptWalk, Response, SearchResult, WalkDirection,
};
use pretty_assertions::assert_eq;

fn entry(text: &str) -> HistoryEntry {
    HistoryEntry::new(text)
}

fn found(text: &str) -> SearchResult {
    SearchResult::Found(entry(text))
}

/// One composer over one store, as the page holds them.
struct Rig {
    store: Store,
    history: PromptWalk,
    fx: Vec<Lookup>,
}

impl Rig {
    fn new(log_id: u64, entry_count: usize) -> Self {
        let mut store = Store::new();
        store.set_metadata(log_id, entry_count);
        Rig {
            store,
            history: PromptWalk::new(),
            fx: Vec::new(),
        }
    }

    fn up(&mut self) -> Option<HistoryEntry> {
        self.history.navigate_up(&self.store, &mut self.fx)
    }

    fn down(&mut self) -> Option<HistoryEntry> {
        self.history.navigate_down(&self.store, &mut self.fx)
    }

    fn search(&mut self, query: &str, direction: WalkDirection, restart: bool) -> SearchResult {
        self.history
            .search(&self.store, query, direction, restart, &mut self.fx)
    }

    /// The next lookup the composer asked for, oldest first.
    fn asked(&mut self) -> Option<Lookup> {
        (!self.fx.is_empty()).then(|| self.fx.remove(0))
    }

    /// `on_entry_response`: the store takes the entry, the composer resumes.
    fn entry_reply(&mut self, log_id: u64, offset: usize, text: &str) -> Response {
        if !self.store.on_entry(log_id, offset, Some(text.to_string())) {
            return Response::Ignored;
        }
        self.history
            .poll(&self.store, &mut self.fx)
            .unwrap_or(Response::Ignored)
    }

    /// `on_batch_response`: `None` for a stale log and for a batch no search
    /// waits for (its rows are cached all the same).
    fn batch_reply(
        &mut self,
        log_id: u64,
        end_offset: usize,
        rows: &[(usize, &str)],
        next_older: Option<usize>,
    ) -> Option<SearchResult> {
        let rows = rows
            .iter()
            .map(|(offset, text)| (*offset, Some(text.to_string())))
            .collect();
        if !self.store.on_batch(log_id, end_offset, rows, next_older) {
            return None;
        }
        match self.history.poll(&self.store, &mut self.fx) {
            Some(Response::Search(result)) => Some(result),
            _ => None,
        }
    }

    /// `on_batch_error`.
    fn batch_error(&mut self, log_id: u64, end_offset: usize) -> Option<SearchResult> {
        if !self.store.on_batch_error(log_id, end_offset) {
            return None;
        }
        match self.history.poll(&self.store, &mut self.fx) {
            Some(Response::Search(result)) => Some(result),
            _ => None,
        }
    }
}

fn entry_lookup(log_id: u64, offset: usize) -> Lookup {
    Lookup::Entry { log_id, offset }
}

fn batch_lookup(log_id: u64, end_offset: usize) -> Lookup {
    Lookup::Batch { log_id, end_offset }
}

#[test]
fn duplicate_submissions_are_not_recorded() {
    let mut history = PromptWalk::new();

    // Empty submissions are ignored.
    history.record_local_submission(entry(""));
    assert_eq!(history.local().len(), 0);

    // First entry is recorded.
    history.record_local_submission(entry("hello"));
    assert_eq!(history.local().len(), 1);
    assert_eq!(history.local().last().unwrap(), &entry("hello"));

    // Identical consecutive entry is skipped.
    history.record_local_submission(entry("hello"));
    assert_eq!(history.local().len(), 1);

    // Different entry is recorded.
    history.record_local_submission(entry("world"));
    assert_eq!(history.local().len(), 2);
    assert_eq!(history.local().last().unwrap(), &entry("world"));
}

#[test]
fn a_new_log_voids_the_walk_and_the_search_but_keeps_what_the_composer_sent() {
    // The oracle clears a composer's local entries when a second thread is
    // configured; here every thread has its own composer, so what it sent
    // stays and only the cursors of the old log go (`set_metadata`).
    let mut rig = Rig::new(1, 0);
    rig.history
        .record_local_submission(entry("thread-owned draft"));
    assert_eq!(rig.up(), Some(entry("thread-owned draft")));
    assert!(rig.history.is_navigating());

    rig.store.set_metadata(2, 0);
    assert_eq!(rig.up(), Some(entry("thread-owned draft")));
    assert!(rig.history.is_navigating());
    // not the second Up of the old walk: it began again from the newest
    assert_eq!(rig.up(), None);
}

#[test]
fn navigation_with_async_fetch() {
    let mut rig = Rig::new(1, 3);
    rig.history.record_local_submission(entry("latest"));

    // First Up should recall current-session local history.
    assert!(rig.history.should_handle_navigation(&rig.store, "", 0));
    assert_eq!(Some(entry("latest")), rig.up());

    // Next Up should request offset 2 and await async data.
    assert!(rig.up().is_none()); // don't replace the text yet
    assert_eq!(rig.asked(), Some(entry_lookup(1, 2)));

    // Inject the async response.
    assert_eq!(
        Response::Found(entry("latest")),
        rig.entry_reply(1, 2, "latest")
    );

    // Next Up should move to offset 1.
    assert!(rig.up().is_none()); // don't replace the text yet
    assert_eq!(rig.asked(), Some(entry_lookup(1, 1)));
    assert_eq!(
        Response::Found(entry("older")),
        rig.entry_reply(1, 1, "older")
    );
    assert_eq!(rig.asked(), None);
}

#[test]
fn down_walks_back_to_an_empty_draft() {
    let mut rig = Rig::new(1, 2);
    rig.store.on_entry(1, 0, Some("a".into()));
    rig.store.on_entry(1, 1, Some("b".into()));
    assert_eq!(rig.down(), None); // not browsing
    assert_eq!(rig.up(), Some(entry("b")));
    assert_eq!(rig.up(), Some(entry("a")));
    assert_eq!(rig.up(), None); // the oldest
    assert_eq!(rig.down(), Some(entry("b")));
    // past the newest: an empty entry, the draft is to be cleared
    assert_eq!(rig.down(), Some(entry("")));
    assert!(!rig.history.is_navigating());
}

#[test]
fn search_matches_local_history_and_stops_at_boundaries() {
    let mut rig = Rig::new(1, 0);
    for text in ["git status", "cargo test -p codex-tui", "git diff"] {
        rig.history.record_local_submission(entry(text));
    }
    use WalkDirection::{Newer, Older};

    assert_eq!(found("git diff"), rig.search("git", Older, true));
    assert_eq!(found("git status"), rig.search("git", Older, false));
    assert_eq!(SearchResult::AtBoundary, rig.search("git", Older, false));
    assert_eq!(SearchResult::AtBoundary, rig.search("git", Older, false));
    assert_eq!(found("git diff"), rig.search("git", Newer, false));
    assert_eq!(SearchResult::AtBoundary, rig.search("git", Newer, false));
}

#[test]
fn search_skips_duplicate_local_matches() {
    let mut rig = Rig::new(1, 0);
    for text in [
        "git status",
        "cargo test -p codex-tui",
        "git status",
        "git diff",
    ] {
        rig.history.record_local_submission(entry(text));
    }
    use WalkDirection::{Newer, Older};

    assert_eq!(found("git diff"), rig.search("git", Older, true));
    assert_eq!(found("git status"), rig.search("git", Older, false));
    assert_eq!(SearchResult::AtBoundary, rig.search("git", Older, false));
    assert_eq!(found("git diff"), rig.search("git", Newer, false));
    assert_eq!(found("git status"), rig.search("git", Older, false));
}

#[test]
fn repeated_boundary_search_does_not_refetch_persistent_history() {
    let mut rig = Rig::new(1, 3);
    use WalkDirection::Older;

    assert_eq!(SearchResult::Pending, rig.search("needle", Older, true));
    assert_eq!(rig.asked(), Some(entry_lookup(1, 2)));
    assert_eq!(
        Response::Search(found("needle latest")),
        rig.entry_reply(1, 2, "needle latest")
    );

    assert_eq!(SearchResult::Pending, rig.search("needle", Older, false));
    assert_eq!(rig.asked(), Some(entry_lookup(1, 1)));
    assert_eq!(
        Response::Search(SearchResult::Pending),
        rig.entry_reply(1, 1, "not a match")
    );
    assert_eq!(rig.asked(), Some(batch_lookup(1, 0)));
    assert_eq!(
        Some(SearchResult::AtBoundary),
        rig.batch_reply(1, 0, &[(0, "also not a match")], None)
    );
    assert_eq!(rig.asked(), None);

    assert_eq!(SearchResult::AtBoundary, rig.search("needle", Older, false));
    assert_eq!(rig.asked(), None);
}

#[test]
fn search_fetches_persistent_history_until_match() {
    let mut rig = Rig::new(1, 3);
    use WalkDirection::Older;

    assert_eq!(SearchResult::Pending, rig.search("older", Older, true));
    assert_eq!(rig.asked(), Some(entry_lookup(1, 2)));
    assert_eq!(
        Response::Search(SearchResult::Pending),
        rig.entry_reply(1, 2, "latest")
    );
    assert_eq!(rig.asked(), Some(batch_lookup(1, 1)));
    assert_eq!(
        Some(found("OLDER command")),
        rig.batch_reply(1, 1, &[(1, "OLDER command")], Some(0))
    );
}

#[test]
fn search_skips_duplicate_persistent_matches() {
    let mut rig = Rig::new(1, 4);
    use WalkDirection::{Newer, Older};

    assert_eq!(SearchResult::Pending, rig.search("needle", Older, true));
    assert_eq!(rig.asked(), Some(entry_lookup(1, 3)));
    assert_eq!(
        Response::Search(found("needle same")),
        rig.entry_reply(1, 3, "needle same")
    );

    assert_eq!(SearchResult::Pending, rig.search("needle", Older, false));
    assert_eq!(rig.asked(), Some(entry_lookup(1, 2)));
    assert_eq!(
        Response::Search(SearchResult::Pending),
        rig.entry_reply(1, 2, "needle same")
    );
    assert_eq!(rig.asked(), Some(batch_lookup(1, 1)));
    assert_eq!(
        Some(found("needle older")),
        rig.batch_reply(1, 1, &[(1, "not a match"), (0, "needle older")], None)
    );
    assert_eq!(SearchResult::AtBoundary, rig.search("needle", Older, false));
    assert_eq!(found("needle same"), rig.search("needle", Newer, false));
}

#[test]
fn search_is_case_insensitive_and_empty_query_finds_latest() {
    let mut rig = Rig::new(1, 0);
    rig.history.record_local_submission(entry("Build Release"));
    assert_eq!(
        found("Build Release"),
        rig.search("release", WalkDirection::Older, true)
    );
    assert_eq!(
        found("Build Release"),
        rig.search("", WalkDirection::Older, true)
    );
}

#[test]
fn reset_navigation_resets_cursor() {
    let mut rig = Rig::new(1, 3);
    rig.store.on_entry(1, 1, Some("command2".into()));
    rig.store.on_entry(1, 2, Some("command3".into()));

    assert_eq!(Some(entry("command3")), rig.up());
    assert_eq!(Some(entry("command2")), rig.up());

    rig.history.reset_navigation();
    assert!(!rig.history.is_navigating());
    assert!(rig.history.last_recalled_text().is_none());

    assert_eq!(Some(entry("command3")), rig.up());
}

#[test]
fn should_handle_navigation_when_cursor_is_at_line_boundaries() {
    let store = Store::new();
    let mut history = PromptWalk::new();
    history.record_local_submission(entry("hello"));
    history.record_recalled_text("hello".to_string());

    assert!(history.should_handle_navigation(&store, "hello", 0));
    assert!(history.should_handle_navigation(&store, "hello", "hello".len()));
    assert!(!history.should_handle_navigation(&store, "hello", 1));
    assert!(!history.should_handle_navigation(&store, "other", 0));
}

#[test]
fn a_replayed_prompt_is_not_offered_twice() {
    // `persistent_entry_duplicates_local`: the file's copy of a prompt the
    // composer was seeded with from a resumed transcript is skipped.
    let mut rig = Rig::new(1, 2);
    rig.store.on_entry(1, 0, Some("first".into()));
    rig.store.on_entry(1, 1, Some("replayed".into()));
    rig.history.record_replayed_submission(entry("replayed"));
    // the local copy is at offset 2, the file's at 1: Up skips the latter
    assert_eq!(rig.up(), Some(entry("replayed")));
    assert_eq!(rig.up(), Some(entry("first")));
    assert_eq!(rig.up(), None);
}

#[test]
fn a_replayed_prompt_stays_known_when_the_log_is_read_after_the_replay() {
    // The oracle configures the thread, then replays it; the file is read
    // by the host on its own time, so its metadata may come second.
    let mut store = Store::new();
    let mut history = PromptWalk::new();
    let mut fx = Vec::new();
    history.record_replayed_submission(entry("replayed"));
    store.set_metadata(1, 2);
    store.on_entry(1, 0, Some("first".into()));
    store.on_entry(1, 1, Some("replayed".into()));
    assert_eq!(
        history.navigate_up(&store, &mut fx),
        Some(entry("replayed"))
    );
    assert_eq!(history.navigate_up(&store, &mut fx), Some(entry("first")));
}

// search_batch_tests.rs

const LOG: u64 = 42;

fn rig_of(entry_count: usize) -> Rig {
    Rig::new(LOG, entry_count)
}

/// `start_older_search`: the newest entry is probed by itself, then the batch
/// below it is asked for. Returns where that batch ends.
fn start_older_search(rig: &mut Rig, query: &str, newest: usize) -> usize {
    assert_eq!(
        rig.search(query, WalkDirection::Older, true),
        SearchResult::Pending
    );
    assert_eq!(rig.asked(), Some(entry_lookup(LOG, newest)));
    assert_eq!(
        rig.entry_reply(LOG, newest, "unrelated entry"),
        Response::Search(SearchResult::Pending)
    );
    assert_eq!(rig.asked(), Some(batch_lookup(LOG, newest - 1)));
    newest - 1
}

#[test]
fn search_batch_late_data_is_cache_only_after_cancel() {
    let mut rig = rig_of(5);
    start_older_search(&mut rig, "cached", 4);
    rig.history.reset_search();
    assert_eq!(
        rig.batch_reply(LOG, 3, &[(3, "cached match")], Some(2)),
        None
    );
    assert_eq!(
        rig.search("cached", WalkDirection::Older, true),
        found("cached match")
    );
    assert_eq!(rig.asked(), None);
}

#[test]
fn search_batch_late_data_after_a_query_edit_is_not_the_new_searchs_answer() {
    let mut rig = rig_of(5);
    start_older_search(&mut rig, "old", 4);
    // the query changed: the new search waits for offset 3 by itself
    assert_eq!(
        rig.search("new", WalkDirection::Older, true),
        SearchResult::Pending
    );
    assert_eq!(rig.asked(), Some(entry_lookup(LOG, 3)));
    // The oracle's late batch is of a search that is gone: it only fills the
    // cache (`None`). Here the cache is what the awaited entry is read from,
    // so a batch that holds offset 3 resolves the new search with the very
    // row the entry read would have brought; its text decides the match.
    assert_eq!(
        rig.batch_reply(LOG, 3, &[(3, "new current match")], Some(2)),
        Some(found("new current match"))
    );
    // the entry read that comes after finds nothing waiting
    assert_eq!(
        rig.entry_reply(LOG, 3, "new current match"),
        Response::Ignored
    );
}

#[test]
fn search_batch_rejects_stale_log_metadata() {
    let mut rig = rig_of(5);
    start_older_search(&mut rig, "stale", 4);
    rig.store.set_metadata(43, 5);
    assert_eq!(
        rig.batch_reply(LOG, 3, &[(3, "stale match")], Some(2)),
        None
    );
    // the store kept nothing of it: offset 3 is still to be read
    assert_eq!(
        rig.search("stale", WalkDirection::Older, true),
        SearchResult::Pending
    );
    assert_eq!(rig.asked(), Some(entry_lookup(43, 4)));
}

#[test]
fn search_batch_read_failure_stops_after_bounded_retries() {
    let mut rig = rig_of(5);
    let end = start_older_search(&mut rig, "retry", 4);
    for _ in 0..MAX_BATCH_READ_RETRIES {
        assert_eq!(rig.batch_error(LOG, end), Some(SearchResult::Pending));
        assert_eq!(rig.asked(), Some(batch_lookup(LOG, end)));
    }
    assert_eq!(rig.batch_error(LOG, end), Some(SearchResult::Unavailable));
    assert_eq!(rig.asked(), None);
    // the search is gone: the next step begins again, from the cache
    assert_eq!(
        rig.search("retry", WalkDirection::Older, false),
        SearchResult::Pending
    );
    assert_eq!(rig.asked(), Some(entry_lookup(LOG, 3)));
}

#[test]
fn a_failure_nobody_waits_for_is_not_the_next_searchs_answer() {
    let mut rig = rig_of(5);
    let end = start_older_search(&mut rig, "retry", 4);
    rig.history.reset_search();
    assert_eq!(rig.batch_error(LOG, end), None);
    // the next search is not failed by it: offset 4 is read, so the probe
    // is of offset 3, and the batch below that starts with a fresh count
    assert_eq!(
        rig.search("again", WalkDirection::Older, true),
        SearchResult::Pending
    );
    assert_eq!(rig.asked(), Some(entry_lookup(LOG, 3)));
    assert_eq!(
        rig.entry_reply(LOG, 3, "unrelated entry"),
        Response::Search(SearchResult::Pending)
    );
    assert_eq!(rig.asked(), Some(batch_lookup(LOG, 2)));
    for _ in 0..MAX_BATCH_READ_RETRIES {
        assert_eq!(rig.batch_error(LOG, 2), Some(SearchResult::Pending));
        assert_eq!(rig.asked(), Some(batch_lookup(LOG, 2)));
    }
    assert_eq!(rig.batch_error(LOG, 2), Some(SearchResult::Unavailable));
}

#[test]
fn search_batch_match_preserves_cursor_for_next_older_search() {
    let mut rig = rig_of(7);
    let end = start_older_search(&mut rig, "needle", 6);
    assert_eq!(
        rig.batch_reply(
            LOG,
            end,
            &[
                (5, "unrelated newer"),
                (4, "needle first"),
                (3, "unrelated older")
            ],
            Some(2),
        ),
        Some(found("needle first"))
    );
    assert_eq!(
        rig.search("needle", WalkDirection::Older, false),
        SearchResult::Pending
    );
    assert_eq!(rig.asked(), Some(batch_lookup(LOG, 2)));
    assert_eq!(rig.asked(), None);
    assert_eq!(
        rig.batch_reply(LOG, 2, &[(2, "needle second")], None),
        Some(found("needle second"))
    );
}

#[test]
fn search_batch_absent_1024_uses_one_single_and_eight_batches() {
    let mut rig = rig_of(1_024);
    let mut end = start_older_search(&mut rig, "absent", 1_023);
    let mut batches = 0;
    loop {
        let start = end.saturating_sub(127);
        let rows: Vec<(usize, &str)> = (start..=end)
            .rev()
            .map(|offset| (offset, "unrelated entry"))
            .collect();
        let next_older = start.checked_sub(1);
        let expected = if next_older.is_some() {
            SearchResult::Pending
        } else {
            SearchResult::NotFound
        };
        assert_eq!(rig.batch_reply(LOG, end, &rows, next_older), Some(expected));
        batches += 1;
        let Some(next) = next_older else {
            break;
        };
        assert_eq!(rig.asked(), Some(batch_lookup(LOG, next)));
        end = next;
    }
    assert_eq!((1 + batches, batches), (9, 8));
    assert_eq!(rig.asked(), None);
}
