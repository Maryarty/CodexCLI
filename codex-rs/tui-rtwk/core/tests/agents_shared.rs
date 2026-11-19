//! The shared overview's two oracle tests of `app/agents_overview_tests.rs`
//! (codex-tui 0.170.0): `shared_overview_keeps_rows_and_replays_changes_over_stale_reads`
//! (526) and `shared_overview_seeds_once_and_retains_locally_resumed_history`
//! (697). The oracle holds reads as one refresh result and replays queued
//! notifications over it; here a task's read is a chain (`Read`, `Turns`)
//! answered by RPC, so the same facts are asserted over the answers:
//! a notification that came while a read was in flight wins over it, an
//! archived or deleted task is never revived by a read, a read that finds
//! nothing or fails keeps the row, a revert or a lag drops what an older
//! read brings. The oracle's `DynamicToolThreadStarted` registration (a
//! tool response starting a thread) has no counterpart: the port has no
//! dynamic tools.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use codex_app_server_protocol::ReasoningSummaryTextDeltaNotification;
use codex_app_server_protocol::ServerNotification;
use codex_app_server_protocol::Thread;
use codex_app_server_protocol::ThreadArchivedNotification;
use codex_app_server_protocol::ThreadClosedNotification;
use codex_app_server_protocol::ThreadDeletedNotification;
use codex_app_server_protocol::ThreadNameUpdatedNotification;
use codex_app_server_protocol::ThreadRevertedNotification;
use codex_app_server_protocol::ThreadStartedNotification;
use codex_app_server_protocol::ThreadStatus;
use codex_tui_core::app::AppFx;
use codex_tui_core::rpc::RpcError;
use codex_tui_core::screens::agents::{
    Command, Ctx, Ev, Model, Out, Page, Reply, apply, apply_answer, observe,
};
use serde_json::json;

fn id(n: u128) -> String {
    format!("00000000-0000-0000-0000-{n:012}")
}

/// The oracle's `overview_thread`: an idle root named `name`.
fn thread(id: &str, name: &str) -> Thread {
    serde_json::from_value(json!({
        "id": id, "environments": null, "extra": null, "sessionId": id,
        "forkedFromId": null, "attachedToThreadId": null, "parentThreadId": null,
        "preview": name, "ephemeral": false, "section": null, "sectionEnteredAt": null,
        "projectId": null, "historyMode": "legacy", "modelProvider": "openai",
        "model": null, "reasoningEffort": null, "createdAt": 1_700_000_000,
        "updatedAt": 1_700_000_000, "recencyAt": 1_700_000_000,
        "status": {"type": "idle"}, "path": null,
        "cwd": "/tmp/project", "cliVersion": "0.0.0", "originator": null, "source": "cli",
        "canAcceptDirectInput": true, "threadSource": null,
        "agentNickname": null, "agentRole": null, "gitInfo": null, "name": name,
        "daybreakEnabled": null, "turns": []
    }))
    .expect("a thread")
}

fn page(threads: Vec<Thread>) -> Page {
    Page {
        threads,
        unread: Vec::new(),
        next_cursor: None,
        append: false,
        pins: None,
    }
}

fn list(model: &mut Model, threads: Vec<Thread>) {
    apply(model, Ev::Listed(page(threads)), &mut AppFx::default());
}

fn row<'a>(model: &'a Model, id: &str) -> Option<&'a Thread> {
    model.threads.iter().find(|held| held.id == id)
}

fn read_reply(thread: Thread) -> Result<Reply, RpcError> {
    Ok(Reply::Read {
        thread: Box::new(thread),
        unread: false,
    })
}

fn reads(outs: &[Out], thread: &str) -> usize {
    outs.iter()
        .filter(|out| matches!(&out.command, Command::Read { thread: t } if t == thread))
        .count()
}

#[test]
fn shared_overview_keeps_rows_and_replays_changes_over_stale_reads() {
    let mut m = Model::default();
    let [retained, archived, deleted, created] = [id(1), id(2), id(3), id(4)];
    list(
        &mut m,
        vec![
            thread(&retained, "Old name"),
            thread(&archived, "Archived"),
            thread(&deleted, "Deleted"),
        ],
    );
    // A read of each is in flight (the seed) when the changes arrive.
    let mut outs = Vec::new();
    for notification in [
        ServerNotification::ThreadNameUpdated(ThreadNameUpdatedNotification {
            thread_id: retained.clone(),
            thread_name: Some("New name".into()),
        }),
        ServerNotification::ThreadClosed(ThreadClosedNotification {
            thread_id: retained.clone(),
        }),
        ServerNotification::ThreadArchived(ThreadArchivedNotification {
            thread_id: archived.clone(),
        }),
        ServerNotification::ThreadDeleted(ThreadDeletedNotification {
            thread_id: deleted.clone(),
        }),
    ] {
        outs.extend(observe(&mut m, &notification));
    }
    assert_eq!(reads(&outs, &retained), 1, "one chain, the rest re-asked");
    // A creation notification adds a row without a read of a list.
    let made = thread(&created, "Created in another client");
    observe(
        &mut m,
        &ServerNotification::ThreadStarted(ThreadStartedNotification {
            thread: made,
        }),
    );
    assert!(row(&m, &created).is_some());
    assert!(row(&m, &archived).is_none());
    assert!(row(&m, &deleted).is_none());

    // A read supersedes what it read; the notifications that came after it
    // was asked supersede the read.
    let mut fresh = thread(&retained, "Old name");
    fresh.preview = "Fresh preview from the read".to_owned();
    let _ = apply_answer(
        &mut m,
        Ctx::Read {
            thread: retained.clone(),
        },
        read_reply(fresh),
    );
    let held = row(&m, &retained).expect("retained row");
    assert_eq!(held.preview, "Fresh preview from the read");
    assert_eq!(held.name.as_deref(), Some("New name"));
    assert_eq!(held.status, ThreadStatus::NotLoaded);
    // An archived or deleted task is not revived by a read that was asked
    // before.
    for gone in [&archived, &deleted] {
        let _ = apply_answer(
            &mut m,
            Ctx::Read {
                thread: gone.clone(),
            },
            read_reply(thread(gone, "Zombie")),
        );
        assert!(row(&m, gone).is_none());
    }
    let expected = m.threads.clone();
    // A read that fails keeps the rows.
    let _ = apply_answer(
        &mut m,
        Ctx::Read {
            thread: retained.clone(),
        },
        Err(RpcError::Remote("temporarily unavailable".into())),
    );
    assert_eq!(m.threads, expected);
    let _ = apply_answer(
        &mut m,
        Ctx::Turns {
            thread: retained.clone(),
        },
        Err(RpcError::Remote("temporarily unavailable".into())),
    );
    assert_eq!(m.threads, expected);

    // A revert and a lag drop what was derived from the history; an older
    // read of it arriving after must not bring it back, and fresh activity
    // survives it.
    let reasoning = || {
        ServerNotification::ReasoningSummaryTextDelta(ReasoningSummaryTextDeltaNotification {
            thread_id: retained.clone(),
            turn_id: "t".into(),
            item_id: "new-reasoning".into(),
            summary_index: 0,
            delta: "**Checking the revised task**".into(),
        })
    };
    let reverted = ServerNotification::ThreadReverted(ThreadRevertedNotification {
        thread_id: retained.clone(),
    });
    for lag in [false, true] {
        // The chain is at its `Turns` question when the invalidation comes.
        m.last_messages
            .insert(retained.clone(), "Removed answer".into());
        m.activity.entry(retained.clone()).or_default();
        let outs = observe(
            &mut m,
            &ServerNotification::ThreadNameUpdated(ThreadNameUpdatedNotification {
                thread_id: retained.clone(),
                thread_name: Some("New name".into()),
            }),
        );
        assert_eq!(reads(&outs, &retained), 1);
        let outs = apply_answer(
            &mut m,
            Ctx::Read {
                thread: retained.clone(),
            },
            read_reply(thread(&retained, "New name")),
        );
        assert!(
            outs.iter()
                .any(|out| matches!(out.command, Command::Turns { .. })),
            "the chain asks the turns"
        );
        if lag {
            apply(&mut m, Ev::Lagged, &mut AppFx::default());
        } else {
            observe(&mut m, &reverted);
        }
        assert!(!m.activity.contains_key(&retained));
        assert!(m.last_messages.is_empty());
        observe(&mut m, &reasoning());
        let _ = apply_answer(
            &mut m,
            Ctx::Turns {
                thread: retained.clone(),
            },
            Ok(Reply::Turns {
                prompt: None,
                last_message: Some("Removed answer".into()),
            }),
        );
        assert!(m.last_messages.is_empty(), "the old turns do not return");
        assert!(m.activity.contains_key(&retained), "fresh activity stays");
        // Drain the chain a change queued, so the next round starts clean.
        while m.reading.contains(&retained) {
            let _ = apply_answer(
                &mut m,
                Ctx::Read {
                    thread: retained.clone(),
                },
                Err(RpcError::Remote("done".into())),
            );
            let _ = apply_answer(
                &mut m,
                Ctx::Turns {
                    thread: retained.clone(),
                },
                Err(RpcError::Remote("done".into())),
            );
        }
    }
}

#[test]
fn shared_overview_seeds_once_and_retains_locally_resumed_history() {
    // The seed lists the recent tasks only (days 3..=12); older history is
    // not listed.
    let recent: Vec<String> = (3..=12).map(id).collect();
    let seed = || {
        page(
            recent
                .iter()
                .map(|id| thread(id, &format!("Task {id}")))
                .collect(),
        )
    };
    let mut m = Model::default();
    apply(&mut m, Ev::Listed(seed()), &mut AppFx::default());
    let held: std::collections::HashSet<&str> =
        m.threads.iter().map(|held| held.id.as_str()).collect();
    assert_eq!(held.len(), 10);
    assert!(
        !held.contains(id(1).as_str()),
        "older history is not seeded"
    );

    // A creation notification adds a session without attaching a local view.
    let created = id(13);
    observe(
        &mut m,
        &ServerNotification::ThreadStarted(ThreadStartedNotification {
            thread: thread(&created, "Created"),
        }),
    );
    assert!(row(&m, &created).is_some());

    // Opening older history is a local addition, even without a turn.
    let older = id(1);
    let _ = apply_answer(
        &mut m,
        Ctx::Resume {
            thread: older.clone(),
        },
        Ok(Reply::Resumed(Box::new(thread(&older, "Task 1")))),
    );
    // A refresh that does not reach it keeps it, and the created one.
    apply(&mut m, Ev::Listed(seed()), &mut AppFx::default());
    assert!(row(&m, &older).is_some(), "the resumed history is retained");
    assert!(row(&m, &created).is_some(), "the created task is retained");
    assert_eq!(m.threads.len(), 12);

    // A fresh app has no in-memory additions.
    let mut restarted = Model::default();
    apply(&mut restarted, Ev::Listed(seed()), &mut AppFx::default());
    assert!(row(&restarted, &older).is_none());
    assert!(row(&restarted, &created).is_none());
    assert_eq!(restarted.threads.len(), 10);
}
