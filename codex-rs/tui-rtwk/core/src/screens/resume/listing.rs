//! The picker's listing cycle: which sources are being paged, each source's
//! cursor and in-flight request, and the generation and search tokens that
//! make a late reply recognisable (R1: results carry their ids, a stale one
//! is dropped). Oracle: `resume_picker/page_loading.rs` (`PaginationState`)
//! and `resume_picker/catalog.rs` (`ThreadCatalog`): a source that is slow or
//! exhausted does not hold another one back, and resetting a listing
//! invalidates every pending page without mixing the sources' cursors.

use std::collections::BTreeMap;

use crate::ids::BackendId;

/// Whether `thread/list` trusts the indexed metadata or repairs it from
/// rollout files (oracle: `PageLoadMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageMode {
    /// Only the store's indexed metadata.
    StateDbOnly,
    /// The store's normal listing, which may repair.
    #[default]
    StoreDefault,
}

/// One backend's place in the listing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    /// The server's cursor of the page to read next.
    pub cursor: Option<String>,
    /// The request in flight for this source.
    pub pending: Option<u64>,
    /// No more pages.
    pub exhausted: bool,
    /// A page failed: the source is not read again until the listing restarts.
    pub failed: bool,
    pub mode: PageMode,
}

/// A page to ask for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub backend: BackendId,
    pub cursor: Option<String>,
    pub mode: PageMode,
    /// The request number, kept with the reply's ticket.
    pub request: u64,
    /// The search token the page serves, when a search asked for it.
    pub search: Option<u64>,
}

impl Source {
    /// The request for this source's next page, when it has one to ask for.
    fn ask(
        &mut self,
        backend: BackendId,
        next_request: &mut u64,
        search: Option<u64>,
    ) -> Option<Request> {
        if self.pending.is_some() || self.exhausted || self.failed {
            return None;
        }
        let request = *next_request;
        *next_request += 1;
        self.pending = Some(request);
        Some(Request {
            backend,
            cursor: self.cursor.clone(),
            mode: self.mode,
            request,
            search,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    /// The listing cycle. Every restart (a filter, the sort, the status, a
    /// reopen) starts a new one; a reply of an older cycle is dropped.
    pub generation: u64,
    next_request: u64,
    next_search: u64,
    pub sources: BTreeMap<BackendId, Source>,
    /// The search whose pages are being read (oracle: `SearchState::Active`).
    pub search: Option<u64>,
    /// The mode the first page of each source is read in.
    pub initial_mode: PageMode,
}

impl Listing {
    /// A new cycle over `backends`: every source starts from its first page.
    /// The first requests are returned.
    pub fn restart(&mut self, backends: &[BackendId], search: Option<u64>) -> Vec<Request> {
        self.generation += 1;
        self.sources.clear();
        for backend in backends {
            self.sources.insert(
                *backend,
                Source {
                    mode: self.initial_mode,
                    ..Source::default()
                },
            );
        }
        let mut out = Vec::new();
        for backend in backends {
            out.extend(self.begin(*backend, search));
        }
        out
    }

    /// The next page of `backend`, unless one is in flight or none is left.
    pub fn begin(&mut self, backend: BackendId, search: Option<u64>) -> Option<Request> {
        let source = self.sources.get_mut(&backend)?;
        source.ask(backend, &mut self.next_request, search)
    }

    /// The next page of every source that has one.
    pub fn begin_all(&mut self, search: Option<u64>) -> Vec<Request> {
        let next_request = &mut self.next_request;
        self.sources
            .iter_mut()
            .filter_map(|(backend, source)| source.ask(*backend, next_request, search))
            .collect()
    }

    /// The reply of `request` of this `generation`: the source it was for,
    /// when it is the one in flight (a stale or repeated reply is `None`).
    pub fn finish(&mut self, backend: BackendId, generation: u64, request: u64) -> Option<&mut Source> {
        if generation != self.generation {
            return None;
        }
        let source = self.sources.get_mut(&backend)?;
        if source.pending != Some(request) {
            return None;
        }
        source.pending = None;
        Some(source)
    }

    pub fn is_loading(&self) -> bool {
        self.sources.values().any(|source| source.pending.is_some())
    }

    /// Whether a page is in flight or a source has one left (oracle:
    /// `next_cursor.is_some()`, the list's `↓ more`).
    pub fn has_more(&self) -> bool {
        self.sources
            .values()
            .any(|source| source.pending.is_some() || (!source.exhausted && !source.failed))
    }

    /// Whether a page could be asked for now.
    pub fn can_page(&self) -> bool {
        !self.is_loading()
            && self
                .sources
                .values()
                .any(|source| !source.exhausted && !source.failed)
    }

    pub fn next_search_token(&mut self) -> u64 {
        let token = self.next_search;
        self.next_search += 1;
        token
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: BackendId = BackendId(0);
    const B: BackendId = BackendId(1);

    #[test]
    fn a_restart_invalidates_the_pending_pages_of_the_old_cycle() {
        let mut listing = Listing::default();
        let old = listing.restart(&[A], None);
        let old_generation = listing.generation;
        let new = listing.restart(&[A], None);
        assert!(
            listing
                .finish(A, old_generation, old[0].request)
                .is_none(),
            "an old cycle's reply is dropped"
        );
        assert!(listing.finish(A, listing.generation, new[0].request).is_some());
    }

    #[test]
    fn sources_page_independently() {
        let mut listing = Listing::default();
        let first = listing.restart(&[A, B], None);
        assert_eq!(first.len(), 2);
        let generation = listing.generation;
        let a = listing.finish(A, generation, first[0].request).unwrap();
        a.exhausted = true;
        assert!(listing.is_loading(), "B is still in flight");
        assert!(listing.begin(A, None).is_none(), "A has no more pages");
        assert!(listing.begin(B, None).is_none(), "B is in flight");
        let b = listing.finish(B, generation, first[1].request).unwrap();
        b.cursor = Some("next".to_owned());
        let again = listing.begin_all(None);
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].cursor.as_deref(), Some("next"));
    }

    #[test]
    fn a_repeated_reply_is_not_taken_twice() {
        let mut listing = Listing::default();
        let first = listing.restart(&[A], None);
        let generation = listing.generation;
        assert!(listing.finish(A, generation, first[0].request).is_some());
        assert!(listing.finish(A, generation, first[0].request).is_none());
    }
}
