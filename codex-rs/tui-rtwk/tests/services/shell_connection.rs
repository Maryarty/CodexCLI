//! The connection of the command center to an out-of-process server: the
//! reconnect policy and what the app is told, and the older-server words.
//! Oracle: `app/reconnect.rs` (`reconnect`, `begin_reconnect`),
//! `app/server_version_notice.rs`, `update_versions.rs`. The server is a
//! scripted `connect` (the host's real one is IO); time is paused.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::disallowed_methods)]

use std::cell::Cell;
use std::time::Duration;

use codex_tui_core::app::AppEv;
use codex_tui_core::ids::ConnEpoch;
use codex_tui_core::screens::ScreenEv;
use codex_tui_core::screens::agents::{Ev, Notice};
use codex_tui_rtwk::services::agents::version::{Kind, kind, notice};
use codex_tui_rtwk::services::backend::reconnect::restore;
use pretty_assertions::assert_eq;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::Instant;

const EPOCH: ConnEpoch = ConnEpoch(1);

/// `update_versions::tests::stable_clients_only_warn_for_older_releases`.
#[test]
fn stable_clients_only_warn_for_older_releases() {
    for (client, server, expected) in [
        ("0.152.1", "0.152.0", Some(Kind::Older)),
        ("0.153.0", "0.152.1", Some(Kind::Older)),
        ("0.153.0", "0.153.0-alpha.10.1", Some(Kind::Older)),
        ("0.156.0", "0.155.0-alpha.12", Some(Kind::Older)),
        ("0.153.0", "0.153.0", None),
        ("0.153.0", "0.154.0", None),
        ("0.153.0", "0.154.0-alpha.1", None),
        ("0.153.0", "0.0.0", None),
        ("0.153.0", "0.0.0-alpha.1", None),
        ("0.153.0", "0.152.0+dev", None),
    ] {
        assert_eq!(kind(client, server), expected, "{client} vs {server}");
    }
}

/// `prerelease_clients_compare_within_the_same_release_line`.
#[test]
fn prerelease_clients_compare_within_the_same_release_line() {
    for (newer, older) in [
        ("0.155.0-alpha.23", "0.155.0-alpha.22"),
        ("0.153.0-alpha.10", "0.153.0-alpha.9"),
        ("0.153.0-alpha.10.1", "0.153.0-alpha.10"),
        ("0.155.0", "0.155.0-alpha.23"),
    ] {
        assert_eq!(kind(newer, older), Some(Kind::Older));
        assert_eq!(kind(older, newer), None);
        assert_eq!(kind(newer, newer), None);
    }
}

/// `prerelease_clients_on_other_release_lines_and_local_clients_warn_for_mismatches`
/// and `unknown_or_malformed_versions_do_not_produce_notices`.
#[test]
fn other_lines_and_local_clients_differ_and_malformed_ones_say_nothing() {
    for (client, server) in [
        ("0.155.0-alpha.23", "0.156.0"),
        ("0.0.0", "0.153.0"),
        ("0.153.0+dev", "0.153.0"),
        ("0.153.0-alpha.10", "0.0.0"),
    ] {
        assert_eq!(kind(client, server), Some(Kind::Different));
        assert_eq!(kind(client, client), None);
    }
    for version in [
        "unknown",
        "0.153",
        "0.153.0.1",
        " 0.153.0",
        "0.153.0-alpha.01",
    ] {
        assert_eq!(kind(version, "0.153.0"), None);
        assert_eq!(kind("0.153.0", version), None);
    }
}

/// `server_version_overview_notice_updates_and_clears`: the words, and the
/// `/daemon` guidance only for the local daemon.
#[test]
fn the_notice_words_follow_the_oracle() {
    assert_eq!(
        notice("0.153.0", "0.151.0", false).as_deref(),
        Some("Service v0.151.0 < Codex CLI v0.153.0")
    );
    assert_eq!(
        notice("0.153.0", "0.152.1", true).as_deref(),
        Some("Service v0.152.1 < Codex CLI v0.153.0 \u{b7} /daemon")
    );
    assert_eq!(
        notice("0.0.0", "0.153.0", false).as_deref(),
        Some("Service v0.153.0 \u{2260} Codex CLI v0.0.0")
    );
    assert_eq!(notice("0.153.0", "0.153.0", true), None);
}

fn agents(ev: &AppEv) -> &Ev {
    match ev {
        AppEv::Screen(ScreenEv::Agents(ev)) => ev,
        other => panic!("not an agents event: {other:?}"),
    }
}

/// The app is told `Reconnecting` first, the attempts come after 0, 1 and
/// 2 seconds (3 s in all). The host announces success after installing the handle.
#[tokio::test(start_paused = true)]
async fn the_attempts_follow_the_oracles_delays_and_the_app_is_told() {
    let (tx, mut rx) = unbounded_channel();
    let start = Instant::now();
    let attempts = Cell::new(0);
    let at = std::cell::RefCell::new(Vec::new());
    let client = restore(
        || {
            attempts.set(attempts.get() + 1);
            at.borrow_mut().push(start.elapsed());
            let ok = attempts.get() == 3;
            async move { ok.then_some("client") }
        },
        &tx,
        EPOCH,
    )
    .await;
    assert_eq!(client, Some("client"));
    assert_eq!(
        *at.borrow(),
        vec![
            Duration::ZERO,
            Duration::from_secs(1),
            Duration::from_secs(3)
        ]
    );
    let first = rx.try_recv().expect("told first");
    assert_eq!(first.epoch, EPOCH);
    assert_eq!(agents(&first.ev), &Ev::Reconnecting);
    assert!(rx.try_recv().is_err());
}

/// The budget is 120 s whatever the attempts do; then the notice is
/// `Reconnect failed`, and `Reconnected` was never sent.
#[tokio::test(start_paused = true)]
async fn the_budget_ends_in_reconnect_failed() {
    let (tx, mut rx) = unbounded_channel();
    let start = Instant::now();
    let client: Option<()> = restore(|| async { None }, &tx, EPOCH).await;
    assert_eq!(client, None);
    assert_eq!(start.elapsed(), Duration::from_secs(120));
    assert_eq!(agents(&rx.try_recv().unwrap().ev), &Ev::Reconnecting);
    assert_eq!(
        agents(&rx.try_recv().unwrap().ev),
        &Ev::Notice(Some(Notice::ReconnectFailed))
    );
    assert!(rx.try_recv().is_err());
}

/// An attempt that hangs is cut at the budget, not waited for.
#[tokio::test(start_paused = true)]
async fn a_hanging_attempt_is_cut_at_the_budget() {
    let (tx, _rx) = unbounded_channel();
    let start = Instant::now();
    let client: Option<()> = restore(std::future::pending::<Option<()>>, &tx, EPOCH).await;
    assert_eq!(client, None);
    assert_eq!(start.elapsed(), Duration::from_secs(120));
}
