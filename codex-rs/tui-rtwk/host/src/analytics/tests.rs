//! The host's side of the usage page: the root listing and the descendant
//! proof over a scripted app server, the decoding of the backend's payloads,
//! and which session of the page the host keeps. (The loads that need the
//! backend's account session run in the binary's scenario.)

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use codex_app_server_protocol::{ClientRequest, Thread, ThreadListParams};
use codex_tui_core::rpc::ServerError;
use codex_tui_rtwk::services::backend::rpc::Ids;
use serde_json::{Value, json};

use super::{chats, convert};

fn now() -> i64 {
    codex_tui_rtwk::services::clock::unix_now()
}

fn thread(id: &str, parent: Option<&str>, updated_at: i64, ephemeral: bool) -> Value {
    json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": parent,
        "preview": id, "ephemeral": ephemeral, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": updated_at, "recencyAt": updated_at,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": null,
        "daybreakEnabled": null, "turns": []
    })
}

fn page(threads: Vec<Value>, next: Option<&str>) -> Value {
    json!({"data": threads, "nextCursor": next, "backwardsCursor": null})
}

/// An app server that answers the listing with `pages` in order and keeps
/// the requests it was sent.
struct Server {
    pages: Mutex<VecDeque<Value>>,
    asked: Mutex<Vec<ThreadListParams>>,
}

impl Server {
    fn new(pages: Vec<Value>) -> Arc<Self> {
        Arc::new(Self {
            pages: Mutex::new(pages.into()),
            asked: Mutex::default(),
        })
    }

    fn send(
        self: &Arc<Self>,
    ) -> impl Fn(ClientRequest) -> std::future::Ready<Result<Value, ServerError>> + Sync {
        let server = self.clone();
        move |request| {
            let ClientRequest::ThreadList { params, .. } = request else {
                panic!("only thread/list is asked: {request:?}");
            };
            server.asked.lock().unwrap().push(params);
            std::future::ready(server.pages.lock().unwrap().pop_front().ok_or_else(|| {
                ServerError {
                    code: None,
                    data: None,
                    message: "no more pages".into(),
                }
            }))
        }
    }

    fn asked(&self) -> Vec<ThreadListParams> {
        self.asked.lock().unwrap().clone()
    }
}

fn ids_of(threads: &[Thread]) -> Vec<&str> {
    threads.iter().map(|thread| thread.id.as_str()).collect()
}

#[tokio::test]
async fn the_roots_are_the_recent_persistent_top_level_chats_of_this_account() {
    let recent = now() - 60;
    let old = now() - 40 * 24 * 60 * 60;
    let server = Server::new(vec![page(
        vec![
            thread("root", None, recent, false),
            thread("scratch", None, recent, true),
            thread("child", Some("root"), recent, false),
            thread("ancient", None, old, false),
        ],
        // The cutoff was reached: the next page is not read.
        Some("more"),
    )]);
    let roots = chats::roots(&server.send(), &Ids::default())
        .await
        .expect("roots");
    assert_eq!(ids_of(&roots), ["root"]);
    let asked = server.asked();
    assert_eq!(asked.len(), 1);
    assert_eq!(
        (
            asked[0].current_account_only,
            asked[0].archived,
            asked[0].limit
        ),
        (true, Some(false), Some(100))
    );
}

#[tokio::test]
async fn the_roots_follow_the_cursors_and_drop_a_repeated_chat() {
    let recent = now() - 60;
    let server = Server::new(vec![
        page(vec![thread("a", None, recent, false)], Some("one")),
        page(
            vec![
                thread("a", None, recent, false),
                thread("b", None, recent, false),
            ],
            None,
        ),
    ]);
    let roots = chats::roots(&server.send(), &Ids::default())
        .await
        .expect("roots");
    assert_eq!(ids_of(&roots), ["a", "b"]);
    assert_eq!(server.asked()[1].cursor.as_deref(), Some("one"));
}

#[tokio::test]
async fn a_repeated_cursor_ends_the_listing_with_an_error() {
    let recent = now() - 60;
    let server = Server::new(vec![
        page(vec![thread("a", None, recent, false)], Some("loop")),
        page(vec![thread("b", None, recent, false)], Some("loop")),
    ]);
    let error = chats::roots(&server.send(), &Ids::default())
        .await
        .expect_err("a repeated cursor");
    assert_eq!(error, "Chat listing repeated a cursor. Press R to retry.");
}

#[tokio::test]
async fn a_refused_listing_shows_the_report_error_not_the_servers_text() {
    let server = Server::new(Vec::new());
    let error = chats::roots(&server.send(), &Ids::default())
        .await
        .expect_err("no page");
    assert_eq!(error, "Couldn't load analytics. Press R to retry.");
}

fn root() -> Thread {
    serde_json::from_value(thread("root", None, now(), false)).expect("a thread")
}

#[tokio::test]
async fn a_tasks_descendants_are_read_live_and_archived_and_sorted() {
    let recent = now();
    let server = Server::new(vec![
        page(
            vec![
                thread("kid-b", Some("root"), recent, false),
                thread("grandkid", Some("kid-b"), recent, false),
            ],
            None,
        ),
        page(vec![thread("kid-a", Some("root"), recent, false)], None),
    ]);
    let query = chats::descendants(&server.send(), &Ids::default(), &root())
        .await
        .expect("descendants");
    assert_eq!(query.thread_id, "root");
    assert_eq!(query.descendant_thread_ids, ["grandkid", "kid-a", "kid-b"]);
    assert_eq!(
        query.created_at.as_deref(),
        Some("2023-11-14T22:13:20+00:00")
    );
    let asked = server.asked();
    assert_eq!(
        asked
            .iter()
            .map(|params| (
                params.ancestor_thread_id.as_deref(),
                params.archived,
                params.current_account_only
            ))
            .collect::<Vec<_>>(),
        [
            (Some("root"), Some(false), false),
            (Some("root"), Some(true), false)
        ]
    );
}

#[tokio::test]
async fn a_server_that_ignores_the_ancestor_filter_is_not_trusted() {
    let recent = now();
    // A chat with no parent came back: the filter is unsupported.
    let server = Server::new(vec![page(vec![thread("other", None, recent, false)], None)]);
    let error = chats::descendants(&server.send(), &Ids::default(), &root())
        .await
        .expect_err("unsupported");
    assert_eq!(error, "Task descendant discovery is unsupported.");

    // A chain that does not reach the root is incomplete.
    let server = Server::new(vec![
        page(vec![thread("lost", Some("elsewhere"), recent, false)], None),
        page(Vec::new(), None),
    ]);
    let error = chats::descendants(&server.send(), &Ids::default(), &root())
        .await
        .expect_err("incomplete");
    assert_eq!(error, "Task descendant discovery is incomplete.");

    // A cycle among the descendants never reaches it either.
    let server = Server::new(vec![
        page(
            vec![
                thread("x", Some("y"), recent, false),
                thread("y", Some("x"), recent, false),
            ],
            None,
        ),
        page(Vec::new(), None),
    ]);
    let error = chats::descendants(&server.send(), &Ids::default(), &root())
        .await
        .expect_err("a cycle");
    assert_eq!(error, "Task descendant cycle.");
}

#[test]
fn the_plan_history_keeps_what_the_backend_sent() {
    let history: codex_backend_client::PlanLimitHistory = serde_json::from_value(json!({
        "data_as_of": "2026-11-20T10:00:00Z",
        "coverage_start": null,
        "coverage_complete": true,
        "boundary_tolerance_seconds": 5,
        "periods": [{
            "id": "p1", "window_minutes": 300, "plan_type": "plus",
            "starts_at": "2026-11-20T05:00:00Z", "ends_at": "2026-11-20T10:00:00Z",
            "accounting_complete": true, "used_basis_points": 1250.0,
            "breakdowns": [
                {"dimension": "model", "rows": [{"key": "gpt", "basis_points": 800.0}]},
                {"dimension": "a_new_one", "rows": []}
            ]
        }]
    }))
    .expect("a history");
    let data = convert::plan_history(history);
    assert!(data.approximate, "approximate unless the server says not");
    let period = &data.periods[0];
    assert_eq!(
        (period.window_minutes, period.used_basis_points),
        (300, Some(1250.0))
    );
    let breakdowns = period.breakdowns.as_ref().expect("breakdowns");
    assert_eq!(
        breakdowns
            .iter()
            .map(|breakdown| (breakdown.dimension, breakdown.rows.len()))
            .collect::<Vec<_>>(),
        [
            (codex_tui_core::screens::usage::plan::Dimension::Model, 1),
            (codex_tui_core::screens::usage::plan::Dimension::Unknown, 0)
        ]
    );
}

#[test]
fn the_profile_keeps_unreported_values_unreported() {
    let profile: codex_backend_client::AccountProfile = serde_json::from_value(json!({
        "profile": {"display_name": "Ada", "username": null},
        "metadata": {"stats_as_of": "2026-11-20", "stats_error": null},
        "stats": {
            "lifetime_tokens": 12, "peak_daily_tokens": null,
            "longest_running_turn_sec": null, "current_streak_days": 2,
            "longest_streak_days": null,
            "daily_usage_buckets": [{"start_date": "2026-11-19", "tokens": 7}],
            "fast_mode_usage_percentage": 12.5,
            "top_invocations": [{"type": "skill", "skill_name": "x", "usage_count": 3}]
        }
    }))
    .expect("a profile");
    let profile = convert::profile(profile);
    assert_eq!(
        profile.identity.and_then(|identity| identity.display_name),
        Some("Ada".to_string())
    );
    assert_eq!(profile.stats.lifetime_tokens, Some(12));
    assert_eq!(profile.stats.peak_daily_tokens, None);
    assert_eq!(profile.stats.fast_mode_usage_percentage, Some(12.5));
    assert_eq!(profile.stats.most_used_reasoning_effort, None);
    assert_eq!(
        profile
            .stats
            .daily_usage_buckets
            .map(|buckets| buckets.len()),
        Some(1)
    );
    assert_eq!(
        profile.stats.top_invocations.map(|rows| rows.len()),
        Some(1)
    );
}

#[tokio::test]
async fn the_page_keeps_one_session_and_drops_the_asks_of_a_replaced_one() {
    let dir = tempfile::tempdir().expect("a directory");
    let config = codex_core::config::ConfigBuilder::default()
        .codex_home(dir.path().to_path_buf())
        .fallback_cwd(Some(dir.path().to_path_buf()))
        .build()
        .await
        .expect("a config");
    let config = Arc::new(config);
    let store = super::Store::default();
    let first = store.live(1, &config).expect("the first session");
    assert!(Arc::ptr_eq(&first, &store.live(1, &config).expect("kept")));
    let second = store.live(2, &config).expect("a refresh opens the next");
    assert!(!Arc::ptr_eq(&first, &second));
    assert!(store.live(1, &config).is_none(), "an ask of the old one");
    assert!(Arc::ptr_eq(&second, &store.live(2, &config).expect("kept")));
}
