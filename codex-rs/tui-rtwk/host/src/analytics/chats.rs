//! The Chats report's loads: the recent local root chats (`thread/list`),
//! and for each a backend estimate (a business account) or its recorded task
//! usage with the descendants proven (a consumer account). Local history can
//! span accounts, so what the page shows of a title is core's to decide.
//! Oracle: `analytics/chats.rs`, `analytics/tasks.rs`.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::time::Duration;

use codex_app_server_protocol::{
    ClientRequest, SortDirection, Thread, ThreadListResponse, ThreadSortKey,
};
use codex_backend_client::{TaskUsage, TaskUsageStatus, TaskUsageThread, ThreadUsage};
use codex_tui_core::rpc::ServerError;
use codex_tui_core::screens::usage::chats::{
    Amounts, ChatList, ChatRow, Decimal, Estimate, GroupData, Status, Task, TaskGroup, TaskList,
    TaskRow,
};
use codex_tui_core::screens::usage::models::{AccountKind, thread_usage_supported};
use codex_tui_core::threads::asks::ListQuery;
use codex_tui_core::threads::{ThreadAsk, asks};
use codex_tui_rtwk::services::backend::rpc::Ids;
use codex_tui_rtwk::services::clock::{deadline_in, passed, unix_now};
use futures::StreamExt;

use super::client::{Live, Session, request_error};

const LIST_FAILED: &str = "Couldn't load analytics. Press R to retry.";

fn title(thread: Thread) -> String {
    thread
        .name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            if thread.preview.trim().is_empty() {
                "Untitled chat".into()
            } else {
                thread.preview
            }
        })
}

pub async fn business<F, Fut>(live: &Live, send: &F, ids: &Ids) -> Result<Option<ChatList>, String>
where
    F: Fn(ClientRequest) -> Fut + Sync,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let session = live.session().await?;
    if !thread_usage_supported(session.backend.account().plan_type) {
        return Ok(None);
    }
    live.ensure_identity().await?;
    let threads = tokio::time::timeout(Duration::from_secs(/*secs*/ 60), roots(send, ids))
        .await
        .map_err(|_| "Chat listing timed out. Press R to retry.".to_string());
    live.ensure_identity().await?;
    let threads = threads??;
    if threads.is_empty() {
        return Ok(Some(ChatList { rows: Vec::new() }));
    }
    let thread_ids = threads
        .iter()
        .map(|thread| thread.id.as_str())
        .collect::<Vec<_>>();
    // Repairing local history has its own budget; it must not consume
    // estimate time.
    let deadline = deadline_in(Duration::from_secs(/*secs*/ 25));
    let mut data = Vec::new();
    for pair in thread_ids.chunks(/*chunk_size*/ 200) {
        let midpoint = pair.len().min(/*other*/ 100);
        let (first, second) = tokio::join!(
            tokio::time::timeout_at(deadline, estimates(session, &pair[..midpoint])),
            tokio::time::timeout_at(deadline, estimates(session, &pair[midpoint..])),
        );
        live.ensure_identity().await?;
        for batch in [first, second].into_iter().flatten() {
            data.extend(batch?);
        }
        if passed(deadline) {
            break;
        }
    }
    live.ensure_identity().await?;
    let mut usage: HashMap<_, _> = data
        .into_iter()
        .map(|row| (row.thread_id.clone(), row))
        .collect();
    let mut rows = threads
        .into_iter()
        .map(|thread| ChatRow {
            estimate: usage.remove(&thread.id).map(estimate),
            title: title(thread),
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        Reverse(
            row.estimate
                .as_ref()
                .map(|estimate| estimate.credits_micros),
        )
    });
    Ok(Some(ChatList { rows }))
}

fn estimate(usage: ThreadUsage) -> Estimate {
    Estimate {
        credits_micros: usage.estimated_usage_credits_micros,
        usd_micros: usage.estimated_usage_usd_micros,
        groups: usage
            .groups
            .into_iter()
            .map(|group| GroupData {
                model: group.model,
                effort: group.reasoning_effort,
                speed: group.speed,
                credits_micros: group.estimated_usage_credits_micros,
            })
            .collect(),
    }
}

/// Recent local roots are shared by consumer task and business credit
/// reporting.
pub(super) async fn roots<F, Fut>(send: &F, ids: &Ids) -> Result<Vec<Thread>, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let cutoff = unix_now() - 30 * 24 * 60 * 60;
    let mut threads = Vec::new();
    let mut cursor = None;
    let mut seen_cursors = HashSet::new();
    loop {
        let page = list(
            send,
            ids,
            ListQuery {
                cursor,
                ..list_params()
            },
        )
        .await?;
        let reached_cutoff = page.data.iter().any(|thread| thread.updated_at < cutoff);
        threads.extend(page.data.into_iter().filter(|thread| {
            thread.updated_at >= cutoff && !thread.ephemeral && thread.parent_thread_id.is_none()
        }));
        match page.next_cursor {
            Some(next) if !seen_cursors.insert(next.clone()) => {
                return Err("Chat listing repeated a cursor. Press R to retry.".into());
            }
            Some(next) if !reached_cutoff => cursor = Some(next),
            _ => break,
        }
    }
    // Keep repeated rows from overlapping pages out of the backend's
    // distinct-ID batches.
    let mut seen_ids = HashSet::new();
    threads.retain(|thread| seen_ids.insert(thread.id.clone()));
    Ok(threads)
}

async fn list<F, Fut>(send: &F, ids: &Ids, params: ListQuery) -> Result<ThreadListResponse, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let value = send(ThreadAsk::list(params).wire(ids.next()))
        .await
        .map_err(|_| LIST_FAILED.to_string())?;
    asks::listed(value).map_err(|_| LIST_FAILED.to_string())
}

fn list_params() -> ListQuery {
    ListQuery {
        current_account_only: true,
        limit: Some(100),
        sort_key: Some(ThreadSortKey::UpdatedAt),
        sort_direction: Some(SortDirection::Desc),
        model_providers: Some(Vec::new()),
        source_kinds: None,
        archived: Some(false),
        cursor: None,
        cwd: None,
        use_state_db_only: false,
        ..ListQuery::default()
    }
}

/// Isolate unavailable estimates within the existing concurrency slot; keep
/// healthy rows.
async fn estimates(session: &Session, ids: &[&str]) -> Result<Vec<ThreadUsage>, String> {
    let mut pending = if ids.is_empty() {
        Vec::new()
    } else {
        vec![ids]
    };
    let mut rows = Vec::new();
    // At most 15 requests per batch; leave remaining estimates unavailable
    // during an outage.
    for _ in 0..15 {
        let Some(ids) = pending.pop() else { break };
        match session
            .backend
            .request(|client| async move { client.get_threads_usage(ids).await })
            .await
        {
            Ok(batch) => rows.extend(batch),
            Err(error) if error.status().is_some_and(|status| status.as_u16() == 503) => {
                if ids.len() > 1 {
                    let midpoint = ids.len().div_ceil(/*rhs*/ 2);
                    pending.push(&ids[midpoint..]);
                    pending.push(&ids[..midpoint]);
                }
            }
            Err(error) => return Err(request_error(error)),
        }
    }
    Ok(rows)
}

pub async fn consumer<F, Fut>(live: &Live, send: &F, ids: &Ids) -> Result<Option<TaskList>, String>
where
    F: Fn(ClientRequest) -> Fut + Sync,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let session = live.session().await?;
    if session.kind != AccountKind::Consumer {
        return Ok(None);
    }
    live.ensure_identity().await?;
    let roots = tokio::time::timeout(Duration::from_secs(/*secs*/ 60), roots(send, ids))
        .await
        .map_err(|_| "Chat listing timed out. Press R to retry.".to_string());
    live.ensure_identity().await?;
    let mut roots = roots??;
    roots.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| a.id.cmp(&b.id))
    });
    let truncated = roots.len() > 100;
    roots.truncate(/*len*/ 100);
    let discoveries = roots
        .iter()
        .map(|root| descendants(send, ids, root))
        .collect::<Vec<_>>();
    let mut pending = futures::stream::iter(discoveries).buffer_unordered(/*n*/ 2);
    let mut queries = Vec::new();
    // Keep complete groups collected before the deadline; never query
    // partial descendants.
    let deadline = deadline_in(Duration::from_secs(/*secs*/ 25));
    while let Ok(Some(result)) = tokio::time::timeout_at(deadline, pending.next()).await {
        if let Ok(query) = result {
            queries.push(query);
        }
    }
    drop(pending);
    live.ensure_identity().await?;
    let mut seen = HashSet::new();
    let mut batches = Vec::new();
    let mut batch = Vec::new();
    let mut count = 0;
    for query in queries {
        let members = std::iter::once(&query.thread_id).chain(&query.descendant_thread_ids);
        if members.clone().any(|id| !seen.insert(id.clone())) {
            return Err("Task descendant groups overlap.".into());
        }
        let size = 1 + query.descendant_thread_ids.len();
        if count + size > 1_000 || batch.len() == 100 {
            batches.push(std::mem::take(&mut batch));
            count = 0;
        }
        count += size;
        batch.push(query);
    }
    if !batch.is_empty() {
        batches.push(batch);
    }
    let requests = batches
        .into_iter()
        .map(|threads| async move {
            session
                .backend
                .request(|client| {
                    let threads = threads.clone();
                    async move { client.get_task_usage(&threads).await }
                })
                .await
        })
        .collect::<Vec<_>>();
    let expected_batches = requests.len();
    let mut completed_batches = 0;
    let mut pending = futures::stream::iter(requests).buffer_unordered(/*n*/ 2);
    let mut usage = HashMap::new();
    let mut freshness = Vec::new();
    let deadline = deadline_in(Duration::from_secs(/*secs*/ 25));
    while let Ok(Some(result)) = tokio::time::timeout_at(deadline, pending.next()).await {
        completed_batches += 1;
        live.ensure_identity().await?;
        match result {
            Ok(response) => {
                freshness.push(
                    response
                        .data_as_of
                        .as_deref()
                        .map(chrono::DateTime::parse_from_rfc3339)
                        .transpose()
                        .map_err(|_| "Invalid task usage timestamp.")?
                        .map(|time| time.timestamp()),
                );
                usage.extend(
                    response
                        .threads
                        .into_iter()
                        .map(|task| (task.thread_id.clone(), task)),
                );
            }
            Err(error)
                if error
                    .status()
                    .is_some_and(|status| matches!(status.as_u16(), 404 | 503)) =>
            {
                freshness.push(/*value*/ None);
            }
            Err(error) => return Err(request_error(error)),
        }
    }
    if completed_batches < expected_batches {
        freshness.push(/*value*/ None);
    }
    drop(pending);
    live.ensure_identity().await?;
    Ok(Some(TaskList {
        truncated,
        updated_at: if freshness.iter().all(Option::is_some) {
            freshness.into_iter().flatten().min()
        } else {
            None
        },
        rows: roots
            .into_iter()
            .map(|thread| TaskRow {
                task: usage.remove(&thread.id).map(task),
                title: title(thread),
            })
            .collect(),
    }))
}

fn amounts(amounts: codex_backend_client::TaskUsageAmounts) -> Amounts {
    Amounts {
        weekly_percent: amounts.weekly_limit_percent,
        five_hour_percent: amounts.five_hour_limit_percent,
        balance: amounts
            .balance_usage_credits
            .map(|credits| Decimal::new(credits.as_str())),
    }
}

fn task(usage: TaskUsage) -> Task {
    Task {
        status: match usage.data_status {
            TaskUsageStatus::Available => Status::Available,
            TaskUsageStatus::Partial => Status::Partial,
            TaskUsageStatus::Unavailable => Status::Unavailable,
        },
        amounts: amounts(usage.amounts),
        groups: usage
            .groups
            .into_iter()
            .map(|group| TaskGroup {
                model: group.model,
                effort: group.reasoning_effort,
                speed: group.speed,
                product: group.product_experience,
                amounts: amounts(group.amounts),
            })
            .collect(),
    }
}

pub(super) async fn descendants<F, Fut>(
    send: &F,
    ids: &Ids,
    root: &Thread,
) -> Result<TaskUsageThread, String>
where
    F: Fn(ClientRequest) -> Fut,
    Fut: Future<Output = Result<serde_json::Value, ServerError>>,
{
    let mut members = HashSet::new();
    let mut parents = HashMap::new();
    for archived in [false, true] {
        let mut cursor = None;
        let mut seen = HashSet::new();
        loop {
            let page = list(
                send,
                ids,
                ListQuery {
                    current_account_only: false,
                    ancestor_thread_id: Some(root.id.clone()),
                    archived: Some(archived),
                    cursor,
                    limit: Some(100),
                    ..list_params()
                },
            )
            .await?;
            for thread in page.data {
                let parent = thread
                    .parent_thread_id
                    .filter(|_| thread.id != root.id)
                    .ok_or("Task descendant discovery is unsupported.")?;
                parents.insert(thread.id.clone(), parent);
                members.insert(thread.id);
            }
            if members.len() >= 1_000 {
                return Err("Task exceeds the reporting limit.".into());
            }
            match page.next_cursor {
                Some(next) if seen.insert(next.clone()) => cursor = Some(next),
                Some(_) => return Err("Task listing repeated a cursor.".into()),
                None => break,
            }
        }
    }
    // Older servers can ignore the ancestor filter. Prove every returned
    // chain reaches this root.
    for id in &members {
        let mut current = id;
        let mut chain = HashSet::new();
        while current != &root.id {
            if !chain.insert(current) {
                return Err("Task descendant cycle.".into());
            }
            current = parents
                .get(current)
                .ok_or("Task descendant discovery is incomplete.")?;
        }
    }
    let mut descendant_thread_ids = members.into_iter().collect::<Vec<_>>();
    descendant_thread_ids.sort();
    Ok(TaskUsageThread {
        thread_id: root.id.clone(),
        created_at: (root.created_at > 0)
            .then(|| chrono::DateTime::from_timestamp(root.created_at, /*nsecs*/ 0))
            .flatten()
            .map(|time| time.to_rfc3339()),
        descendant_thread_ids,
    })
}
