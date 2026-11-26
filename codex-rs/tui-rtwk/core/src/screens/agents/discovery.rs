//! Demand-driven discovery: the pages of the list, the rows fetched but not
//! shown, the refill of vacancies left by removals, and the history search.
//! Oracle: `agents_overview_discovery.rs` (`SourcePage::fill`, `next_batch`,
//! `HistorySearch`, `search_agents_overview`,
//! `apply_agents_overview_search_page`), `agents_overview.rs`
//! (`refill_count`, `apply_agents_overview_thread_refresh`),
//! `agents_overview_view.rs` (`load_search_matches`).

use std::collections::HashSet;

use crate::ids::BackendId;
use codex_app_server_protocol::Thread;

use super::Command;
use super::Ctx;
use super::History;
use super::Lens;
use super::Model;
use super::Notice;
use super::PAGE_SIZE;
use super::Page;
use super::carried::is_overview_thread;
use super::carried::matches_query;
use super::refresh::Out;
use super::refresh::out;

/// The rows a page may add: not removed, not already shown or buffered.
fn fresh(model: &Model, buffered: &[Thread], rows: Vec<Thread>) -> Vec<Thread> {
    let known: HashSet<&str> = model
        .threads
        .iter()
        .chain(buffered)
        .map(|held| held.id.as_str())
        .collect();
    let mut kept: Vec<Thread> = Vec::new();
    for thread in rows {
        if !known.contains(thread.id.as_str()) && !model.removed.contains(&thread.id) {
            kept.push(thread);
        }
    }
    // A page that repeats an id keeps its first row only.
    let mut seen = HashSet::new();
    kept.retain(|thread| seen.insert(thread.id.clone()));
    kept
}

/// `has_more` follows the cursor and the buffer.
pub(super) fn sync(model: &mut Model) {
    model.paging.has_more = model.paging.more();
}

/// Shows up to `want` buffered rows (oracle: `next_batch` takes `limit`
/// rows, newest first).
fn show_buffered(model: &mut Model, want: usize) {
    let take = want.min(model.paging.buffer.len());
    let shown: Vec<Thread> = model.paging.buffer.drain(..take).collect();
    show_rows(model, shown);
}

fn show_rows(model: &mut Model, shown: impl IntoIterator<Item = Thread>) -> usize {
    let before = model.threads.len();
    for thread in shown {
        if model.hidden.contains(&thread.id) || model.removed.contains(&thread.id) {
            continue;
        }
        model.search_matches.retain(|held| held.id != thread.id);
        model.threads.push(super::refresh::normalize(thread));
    }
    model.threads.len() - before
}

fn page_request(model: &mut Model, want: usize) -> Out {
    model.paging.loading = true;
    model.paging.want = want;
    out(
        Command::List {
            cursor: model.paging.cursor.clone(),
        },
        Ctx::List { append: true },
    )
}

/// "Show more": the next rows, from the buffer when it holds a page, else
/// from the next page (oracle: `ShowMoreAgentsOverview`).
pub(super) fn show_more(model: &mut Model) -> Vec<Out> {
    if model.paging.loading {
        return Vec::new();
    }
    let page = PAGE_SIZE as usize;
    if model.paging.cursor.is_none() || model.paging.buffer.len() >= page {
        show_buffered(model, page);
        sync(model);
        return Vec::new();
    }
    vec![page_request(model, page)]
}

/// The vacancies removals left are filled from the buffer or the next page,
/// without growing the window (oracle: `refill_count`).
pub fn refill(model: &mut Model) -> Vec<Out> {
    let vacancies = model.paging.refill;
    if vacancies == 0 || model.paging.loading || !model.paging.more() {
        return Vec::new();
    }
    let page = PAGE_SIZE as usize;
    let want = vacancies.min(page);
    if model.paging.cursor.is_none() || model.paging.buffer.len() >= want {
        let before = model.threads.len();
        show_buffered(model, want);
        model.paging.refill = vacancies.saturating_sub(model.threads.len() - before);
        sync(model);
        return Vec::new();
    }
    vec![page_request(model, want)]
}

/// A page asked for by "Show more" or a refill arrived for its originating backend.
pub(super) fn on_more_page(model: &mut Model, owner: BackendId, page: Option<Page>) {
    let visible = owner == model.displayed;
    let mut paging = if visible {
        std::mem::take(&mut model.paging)
    } else {
        model.parked.remove(&owner).unwrap_or_default()
    };
    // An answer nobody recorded a count for shows a page.
    let want = match std::mem::take(&mut paging.want) {
        0 => PAGE_SIZE as usize,
        want => want,
    };
    paging.loading = false;
    match page {
        // The server repeating the cursor ends nothing: it is a failure that
        // a retry may cure (oracle: `SourcePage::fill`).
        Some(page) if page.next_cursor.is_none() || page.next_cursor != paging.cursor => {
            for id in &page.unread {
                model.unread.insert(id.clone());
            }
            let rows = fresh(
                model,
                &paging.buffer,
                page.threads
                    .into_iter()
                    .filter(is_overview_thread)
                    .collect(),
            );
            paging
                .buffer
                .extend(rows.into_iter().filter(|row| model.observe(owner, &row.id)));
            paging.cursor = page.next_cursor;
            paging.failed = false;
        }
        _ => paging.failed = true,
    }
    let take = want.min(paging.buffer.len());
    let added = show_rows(model, paging.buffer.drain(..take));
    paging.refill = paging.refill.saturating_sub(added);
    paging.has_more = paging.more();
    if visible {
        if paging.failed {
            model.notice = Some(Notice::LoadFailed);
        } else if matches!(model.notice, Some(Notice::LoadFailed)) {
            model.notice = None;
        }
        model.paging = paging;
    } else {
        model.parked.insert(owner, paging);
    }
}

/// The search text changed: a new history search starts, keeping how many
/// matches were wanted (oracle: `restart_history_search`).
pub(super) fn restart_history(model: &mut Model, lens: &Lens) -> Vec<Out> {
    let offline = matches!(
        model.notice,
        Some(Notice::Reconnecting | Notice::ReconnectFailed)
    );
    model.search_matches.clear();
    model.search_seq += 1;
    model.history = (!model.search.is_empty() && !offline).then(|| History {
        id: model.search_seq,
        target: model
            .history
            .as_ref()
            .map_or(PAGE_SIZE as usize, |held| held.target),
        ..History::default()
    });
    load_matches(model, PAGE_SIZE as usize, lens)
}

/// Asks for more matches: `additional` more than are visible (0: only up to
/// the target) (oracle: `load_search_matches`).
pub(super) fn load_matches(model: &mut Model, additional: usize, lens: &Lens) -> Vec<Out> {
    if model.history.is_none() {
        return Vec::new();
    }
    let visible = {
        let rows = model.rows(None);
        model.visible_indices(&rows, lens).len()
    };
    let Some(history) = model.history.as_mut() else {
        return Vec::new();
    };
    if !history.has_more() || history.loading {
        return Vec::new();
    }
    if additional > 0 {
        history.target = visible + additional;
        history.failed = false;
    }
    if history.failed || visible >= history.target {
        return Vec::new();
    }
    let limit = history.target - visible;
    let id = history.id;
    let page = PAGE_SIZE as usize;
    if history.exhausted || history.buffer.len() >= page.min(limit) {
        take_matches(model, limit);
        return Vec::new();
    }
    history.loading = true;
    let first = !history.started;
    history.started = true;
    vec![out(
        Command::Search {
            cursor: history.cursor.clone(),
            first,
        },
        Ctx::Search { id, limit },
    )]
}

/// Moves up to `limit` buffered matches to the rows the center shows.
fn take_matches(model: &mut Model, limit: usize) {
    let Some(history) = model.history.as_mut() else {
        return;
    };
    let take = limit.min(history.buffer.len());
    let shown: Vec<Thread> = history.buffer.drain(..take).collect();
    for thread in shown {
        let known = model.threads.iter().any(|held| held.id == thread.id)
            || model.search_matches.iter().any(|held| held.id == thread.id);
        if !known && !model.hidden.contains(&thread.id) && !model.removed.contains(&thread.id) {
            model.search_matches.push(super::refresh::normalize(thread));
        }
    }
}

/// A page of the history search arrived (oracle:
/// `apply_agents_overview_search_page`).
pub(super) fn on_search_page(
    model: &mut Model,
    id: u64,
    limit: usize,
    page: Option<Page>,
) -> Vec<Out> {
    let term = model.search.to_lowercase();
    let Some(history) = model.history.as_mut().filter(|held| held.id == id) else {
        return Vec::new();
    };
    match page {
        Some(page) => {
            history.exhausted = page.next_cursor.is_none();
            history.cursor = page.next_cursor;
            let rows: Vec<Thread> = page
                .threads
                .into_iter()
                .filter(|thread| is_overview_thread(thread) && matches_query(thread, &term))
                .collect();
            for id in page.unread {
                model.unread.insert(id);
            }
            let Some(history) = model.history.as_mut() else {
                return Vec::new();
            };
            history.buffer.extend(rows);
            // Nothing matched and more pages remain: scan on.
            if history.buffer.is_empty() && !history.exhausted {
                return vec![out(
                    Command::Search {
                        cursor: history.cursor.clone(),
                        first: false,
                    },
                    Ctx::Search { id, limit },
                )];
            }
            history.loading = false;
            history.failed = false;
            take_matches(model, limit);
        }
        None => {
            history.loading = false;
            history.failed = true;
        }
    }
    Vec::new()
}
