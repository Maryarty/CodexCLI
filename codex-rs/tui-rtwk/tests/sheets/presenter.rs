//! The request presenter on rtwk's `AppCx::show_dialog_from` (rtwk-repin):
//! the queue's head is shown from the Navigator home's kept handle, the next
//! request is shown from `closed` with the handle then in the cell, and a
//! dead origin's `None` is not retried. The decision is the model's
//! (`ui::presenter::Presenter`, plain data); no provider is read to make it.

use codex_tui_core::app::AppEv;
use codex_tui_core::fixtures::builders::{server_request_resolved, tool_input_request};
use codex_tui_core::ids::{ConnEpoch, RequestKey, ServerRequestId, ThreadId};
use codex_tui_core::screens::approvals::order::Family;
use codex_tui_rtwk::app::UiApp;
use codex_tui_rtwk::services::backend::fake::FAKE_THREAD;
use codex_tui_rtwk::sheets::presenter::{Presenter, TYPING_IDLE_DELAY};
use rtwk::GlobalKey;

use crate::common::{Harness, ms, start};

fn key(id: &str) -> RequestKey {
    RequestKey {
        backend: Default::default(),
        epoch: ConnEpoch(1),
        thread: ThreadId::new("t"),
        id: ServerRequestId::new(id),
    }
}

/// A queue as the presenter's decision sees it.
fn queue(ids: &[&str]) -> impl Fn(&RequestKey) -> bool {
    let keys: Vec<_> = ids.iter().map(|id| key(id)).collect();
    move |k| keys.contains(k)
}

#[test]
fn the_head_is_shown_once_and_nothing_while_it_is_open() {
    let mut p = Presenter::new();
    assert_eq!(p.next(Some(key("7")), queue(&["7", "8"])), Some(key("7")));
    assert_eq!(p.shown(), Some(&key("7")));
    assert_eq!(p.next(Some(key("7")), queue(&["7", "8"])), None);
    assert_eq!(p.next(Some(key("8")), queue(&["8"])), None, "one at a time");
}

#[test]
fn a_head_that_was_answered_is_not_shown_again_while_its_dismissal_is_in_flight() {
    let mut p = Presenter::new();
    p.next(Some(key("7")), queue(&["7", "8"]));
    // `closed(Some)` runs before the model took the dismissal: 7 is still queued
    p.closed(&key("7"), Some(()), queue(&["7", "8"]));
    assert_eq!(p.shown(), None);
    assert_eq!(p.next(Some(key("7")), queue(&["7", "8"])), None);
    // the dismissal arrived: the next head shows
    assert_eq!(p.next(Some(key("8")), queue(&["8"])), Some(key("8")));
}

#[test]
fn a_none_while_the_request_is_still_queued_is_not_retried_until_an_event() {
    let mut p = Presenter::new();
    p.next(Some(key("7")), queue(&["7"]));
    p.closed(&key("7"), None, queue(&["7"]));
    // `closed` presenting again: not the same request
    assert_eq!(p.next(Some(key("7")), queue(&["7"])), None);
    assert_eq!(p.next(Some(key("7")), queue(&["7"])), None);
    // an ingested event is a new chance, once
    p.event();
    assert_eq!(p.next(Some(key("7")), queue(&["7"])), Some(key("7")));
}

#[test]
fn a_none_because_the_request_left_the_queue_frees_the_next_one() {
    let mut p = Presenter::new();
    p.next(Some(key("7")), queue(&["7", "8"]));
    p.closed(&key("7"), None, queue(&["8"]));
    assert_eq!(p.next(Some(key("8")), queue(&["8"])), Some(key("8")));
}

fn inject(h: &Harness, id: i64) {
    let Some(epoch) = h.fake.epoch() else {
        panic!("attached");
    };
    h.fake.inject(
        epoch,
        AppEv::Backend(tool_input_request(FAKE_THREAD, "turn-1", id)),
    );
}

fn shown(h: &Harness) -> Option<RequestKey> {
    h.t.read_app::<UiApp, _>(|app| app.ui.presenter.shown().cloned())
}

#[test]
fn a_queued_second_request_shows_after_the_first_closes() {
    let mut h = start();
    h.wait_idle();
    inject(&h, 7);
    inject(&h, 8);
    h.wait_for("the first dialog", |s| s.contains("7 needs an answer"));
    assert!(!h.t.screen().contains("8 needs an answer"));
    assert_eq!(shown(&h).map(|k| k.id), Some(ServerRequestId::new("7")));
    h.press("enter");
    h.wait_for("the second dialog", |s| s.contains("8 needs an answer"));
    assert!(!h.t.screen().contains("7 needs an answer"));
    assert_eq!(shown(&h).map(|k| k.id), Some(ServerRequestId::new("8")));
    h.press("enter");
    h.wait_for("no dialog", |s| !s.contains("needs an answer"));
    assert_eq!(shown(&h), None);
}

/// The server answered 7 first: the dialog closes with `None`, the model
/// (not a provider read in the same flush) decides 8 is next.
#[test]
fn a_request_resolved_by_the_server_closes_with_none_and_the_next_shows() {
    let mut h = start();
    h.wait_idle();
    inject(&h, 7);
    inject(&h, 8);
    h.wait_for("the first dialog", |s| s.contains("7 needs an answer"));
    let epoch = h.fake.epoch().expect("attached");
    h.fake.inject(
        epoch,
        AppEv::Backend(server_request_resolved(FAKE_THREAD, 7)),
    );
    h.wait_for("the second dialog", |s| s.contains("8 needs an answer"));
    assert_eq!(shown(&h).map(|k| k.id), Some(ServerRequestId::new("8")));
}

/// A home replaced (its navigator's key changed) while a request is open:
/// the open dialog goes with it (`None`), and the request, still queued, is
/// not retried by `closed`; the next event shows it from the new home.
#[test]
fn a_replaced_home_closes_the_dialog_and_the_next_event_shows_it_again_from_the_new_home() {
    let mut h = start();
    h.wait_idle();
    inject(&h, 7);
    h.wait_for("the dialog", |s| s.contains("7 needs an answer"));
    h.t.update_app::<UiApp>(|app, _| app.ui.nav_key = GlobalKey::with_label("navigator-2"));
    h.t.pump_for(ms(200));
    assert!(
        !h.t.screen().contains("needs an answer"),
        "{}",
        h.t.screen()
    );
    assert_eq!(shown(&h), None);
    assert!(h.t.take_errors().is_empty(), "a live home: no debug check");
    inject(&h, 9);
    h.wait_for("the first request again", |s| {
        s.contains("7 needs an answer")
    });
}

/// The kept handle's memo node is gone: nothing is shown, `closed` gets
/// `None` and does not show it again from the dead place (a debug build
/// reports one debug check; a release build none).
#[test]
fn a_dead_origin_shows_nothing_and_its_none_is_not_retried() {
    let mut h = start();
    h.wait_idle();
    let stale =
        h.t.read_app::<UiApp, _>(|app| app.ui.presenter.place().get());
    assert!(stale.is_some(), "the home left its handle");
    h.t.update_app::<UiApp>(|app, _| app.ui.nav_key = GlobalKey::with_label("navigator-2"));
    h.t.pump_for(ms(200));
    h.t.update_app::<UiApp>(move |app, _| app.ui.presenter.place().set_root(stale));
    h.t.pump_for(ms(50));
    inject(&h, 7);
    h.settle();
    h.settle();
    assert!(
        !h.t.screen().contains("needs an answer"),
        "{}",
        h.t.screen()
    );
    assert_eq!(shown(&h), None);
    let errors = h.t.take_errors();
    assert_eq!(
        errors.len(),
        usize::from(cfg!(debug_assertions)),
        "one debug check, once, not at every flush: {errors:?}"
    );
    assert!(
        h.t.read_app::<UiApp, _>(|app| app.ui.presenter.place().get().is_none()),
        "a dead handle is dropped: requests wait for the next home"
    );
}

mod typing_idle {
    use std::time::Duration;

    use super::*;

    fn at(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Oracle `bottom_pane/mod.rs` `approval_prompt_delay_remaining` (and its test at 3262): an
    /// approval is held until a second after the last key, to the millisecond.
    #[test]
    fn an_approval_waits_the_idle_delay_after_the_last_key() {
        let mut p = Presenter::new();
        assert_eq!(
            p.wait(Family::Approval, at(0)),
            None,
            "nothing typed, nothing held"
        );
        p.typed(at(100));
        assert_eq!(p.wait(Family::Approval, at(100)), Some(TYPING_IDLE_DELAY));
        assert_eq!(p.wait(Family::Approval, at(600)), Some(at(500)));
        let just_before = at(100) + TYPING_IDLE_DELAY - Duration::from_millis(1);
        assert_eq!(p.wait(Family::Approval, just_before), Some(at(1)));
        assert_eq!(p.wait(Family::Approval, at(100) + TYPING_IDLE_DELAY), None);
        assert_eq!(p.wait(Family::Approval, at(60_000)), None);
    }

    /// Oracle 3280-3292: more typing pushes the moment out again.
    #[test]
    fn typing_on_moves_the_moment_out() {
        let mut p = Presenter::new();
        p.typed(at(0));
        p.typed(at(800));
        assert_eq!(p.wait(Family::Approval, at(1000)), Some(at(800)));
        assert_eq!(p.wait(Family::Approval, at(1800)), None);
    }

    /// Oracle 2098-2123: a form, a question and a verification are not delayed, only the
    /// approval overlay is.
    #[test]
    fn only_an_approval_is_held() {
        let mut p = Presenter::new();
        p.typed(at(0));
        for family in [
            Family::Elicitation,
            Family::UserInput,
            Family::UserVerification,
            Family::Other,
        ] {
            assert_eq!(p.wait(family, at(10)), None, "{family:?}");
        }
        assert_eq!(p.wait(Family::Approval, at(10)), Some(at(990)));
    }
}
