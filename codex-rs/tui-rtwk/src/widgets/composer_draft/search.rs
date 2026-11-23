//! Reverse history search (Ctrl+R) in the draft: the search's own state
//! (`core::kit::history::HistorySearch`), the draft as it was when the
//! search opened, and the previews that stand in the draft's place while the
//! query is typed (`chat_composer/history_search.rs`,
//! `history_search_draft.rs`).

use rtwk::editing::TextEditingController;

use codex_tui_core::data::history::{Lookup, Store};
use codex_tui_core::kit::history::SearchResult;
use codex_tui_core::kit::vim::UndoHistory;
use codex_tui_core::kit::history::{WalkDirection, HistorySearch, SearchPreview, SearchStatus};

use super::{ComposerDraft, Snapshot};

/// An open search.
pub(super) struct Session {
    search: HistorySearch,
    /// The draft Ctrl+R found: Esc brings it back, with its caret and its
    /// placeholders (`original_draft`).
    original: Snapshot,
    /// Vim's undo steps of that draft, kept aside while the previews come
    /// and go (`original_vim_history`).
    vim_history: UndoHistory<Snapshot>,
}

impl ComposerDraft {
    /// Whether reverse history search is open.
    pub fn searching(&self) -> bool {
        self.0.borrow().search.is_some()
    }

    /// How many times the search opened, stepped, was edited or ended: what
    /// the footer shows is of this revision.
    pub fn search_rev(&self) -> u64 {
        self.0.borrow().search_rev
    }

    /// The query as the footer shows it and where the search stands.
    pub fn search_state(&self) -> Option<(String, SearchStatus)> {
        let inner = self.0.borrow();
        let session = inner.search.as_ref()?;
        Some((session.search.display_query(), session.search.status))
    }

    /// The query as it is matched.
    pub fn search_query(&self) -> Option<String> {
        let inner = self.0.borrow();
        Some(inner.search.as_ref()?.search.query.clone())
    }

    /// Ctrl+R: the search opens with nothing previewed (an empty composer
    /// is not replaced by the latest prompt), the draft kept aside and the
    /// walk of Up and Down over (`begin_history_search`).
    pub fn search_begin(&self, c: &TextEditingController) {
        let original = self.snapshot(c);
        let mut inner = self.0.borrow_mut();
        let vim_history = std::mem::take(&mut inner.vim.history);
        inner.prompts.reset_search();
        inner.search_rev += 1;
        inner.search = Some(Session {
            search: HistorySearch::new(),
            original,
            vim_history,
        });
    }

    /// The query edited by `edit` and the search restarted from the newest
    /// entry; an empty query puts the draft back and stays open
    /// (`update_history_search_query`). A read the search needs is asked
    /// for in `fx`.
    pub fn search_edit(
        &self,
        c: &TextEditingController,
        store: &Store,
        fx: &mut Vec<Lookup>,
        edit: impl FnOnce(&mut String),
    ) {
        let Some(mut query) = self.search_query() else {
            return;
        };
        edit(&mut query);
        self.search_with(c, |search, prompts| {
            search.set_query(query, prompts, store, fx)
        });
    }

    /// Up and Ctrl+R (older), Down and Ctrl+S (newer): the next match; at
    /// the end of the matches the current one stays
    /// (`history_search_in_direction`).
    pub fn search_step(
        &self,
        c: &TextEditingController,
        store: &Store,
        dir: WalkDirection,
        fx: &mut Vec<Lookup>,
    ) {
        self.search_with(c, |search, prompts| search.step(dir, prompts, store, fx));
    }

    /// Enter: a match ends the search and stays in the draft, caret at its
    /// end; without one the search stays open. `true` when it ended.
    pub fn search_accept(&self, c: &TextEditingController) -> bool {
        {
            let mut inner = self.0.borrow_mut();
            let matched = inner
                .search
                .as_ref()
                .is_some_and(|session| session.search.status == SearchStatus::Match);
            if !matched {
                return false;
            }
            inner.search = None;
            inner.search_rev += 1;
            inner.prompts.reset_search();
        }
        self.rest_at_end(c);
        true
    }

    /// Esc and Ctrl+C: the search ends and the draft is back as it was,
    /// Vim's undo steps with it, and the walk of Up and Down starts over
    /// (`cancel_history_search`). `false` when none was open.
    pub fn search_cancel(&self, c: &TextEditingController) -> bool {
        let Some(session) = self.0.borrow_mut().search.take() else {
            return false;
        };
        self.0.borrow_mut().search_rev += 1;
        self.restore(c, &session.original);
        let mut inner = self.0.borrow_mut();
        inner.vim.history = session.vim_history;
        inner.prompts.reset_navigation();
        true
    }

    /// Runs `step` on the search, then shows what it found in the draft's
    /// place: the entry on a match, the original draft otherwise
    /// (`apply_history_search_result`).
    fn search_with(
        &self,
        c: &TextEditingController,
        step: impl FnOnce(&mut HistorySearch, &mut super::Prompts) -> SearchPreview,
    ) {
        let shown = {
            let mut guard = self.0.borrow_mut();
            let inner = &mut *guard;
            let Some(session) = inner.search.as_mut() else {
                return;
            };
            let preview = step(&mut session.search, &mut inner.prompts);
            let original = session.original.clone();
            inner.search_rev += 1;
            (preview, original)
        };
        self.show(c, shown.0, &shown.1);
    }

    /// A read the search waited for came in (`apply_history_search_result`
    /// from `on_history_entry_response`).
    pub(super) fn search_result(&self, c: &TextEditingController, result: SearchResult) {
        let shown = {
            let mut inner = self.0.borrow_mut();
            let Some(session) = inner.search.as_mut() else {
                return;
            };
            let preview = session.search.apply(result);
            let original = session.original.clone();
            inner.search_rev += 1;
            (preview, original)
        };
        self.show(c, shown.0, &shown.1);
    }

    fn show(&self, c: &TextEditingController, preview: SearchPreview, original: &Snapshot) {
        match preview {
            SearchPreview::Show(entry) => self.recall(c, entry),
            SearchPreview::Original => self.restore(c, original),
            SearchPreview::Keep => {}
        }
    }
}
