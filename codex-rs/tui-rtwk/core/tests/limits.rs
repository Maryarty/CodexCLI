#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The account's limits as the thread tells them (oracle `chatwidget/rate_limits.rs`,
//! `usage_notice.rs`, `thread_usage.rs`): the one-shot warnings at 50, 75, 90 and 95 percent, the
//! window the composer's notice names, and the estimated cost of the thread that is shown. The
//! words are the view's: these tests read data.

use codex_app_server_protocol::ThreadUsage;
use codex_protocol::account::PlanType;
use codex_tui_core::screens::status::{
    Sample, Source, Used, Warning, WarningLimits as Notices, WindowName, warning_threshold,
};
use codex_tui_core::features::thread_usage::model::{Ctx, Outcome, Selected, State};

const FIVE_HOURS: i64 = 300;
const WEEK: i64 = 7 * 24 * 60;

fn used(percent: i32, window_minutes: i64) -> Used {
    Used {
        percent,
        window_minutes: Some(window_minutes),
        resets_at: None,
    }
}

fn sample(primary: Option<Used>, secondary: Option<Used>) -> Sample {
    Sample {
        codex: true,
        plan: Some(PlanType::Pro),
        primary,
        secondary,
        ..Sample::default()
    }
}

fn remaining(warnings: &[Warning]) -> Vec<i32> {
    warnings
        .iter()
        .map(|warning| warning.remaining_percent)
        .collect()
}

#[test]
fn a_threshold_warns_once_and_only_the_highest_one_passed_at_a_time() {
    let mut limits = Notices::default();
    assert!(
        limits
            .take(&sample(Some(used(60, FIVE_HOURS)), None), Source::Read)
            .is_empty()
    );
    // 75 is the first threshold a Pro window warns at
    let due = limits.take(&sample(Some(used(80, FIVE_HOURS)), None), Source::Read);
    assert_eq!(remaining(&due), [25]);
    assert_eq!(due[0].window, WindowName::FiveHours);
    assert!(
        limits
            .take(
                &sample(Some(used(80, FIVE_HOURS)), None),
                Source::Rolling { read_out: false }
            )
            .is_empty()
    );
    // a jump over two thresholds says the highest
    assert_eq!(
        remaining(&limits.take(&sample(Some(used(96, FIVE_HOURS)), None), Source::Read)),
        [5]
    );
    // the percentage falling and rising again does not repeat a warning
    limits.take(&sample(Some(used(10, FIVE_HOURS)), None), Source::Read);
    assert!(
        limits
            .take(&sample(Some(used(96, FIVE_HOURS)), None), Source::Read)
            .is_empty()
    );
}

#[test]
fn the_two_windows_warn_apart_and_the_secondary_one_first() {
    let mut limits = Notices::default();
    let due = limits.take(
        &sample(Some(used(92, FIVE_HOURS)), Some(used(76, WEEK))),
        Source::Read,
    );
    assert_eq!(remaining(&due), [25, 10]);
    assert_eq!(due[0].window, WindowName::Weekly);
    assert_eq!(due[1].window, WindowName::FiveHours);
}

#[test]
fn plus_and_team_warn_earlier_on_the_five_hour_window_only() {
    assert_eq!(
        warning_threshold(Some(PlanType::Plus), Some(FIVE_HOURS)),
        50
    );
    assert_eq!(
        warning_threshold(Some(PlanType::Team), Some(FIVE_HOURS)),
        50
    );
    assert_eq!(warning_threshold(Some(PlanType::Plus), Some(WEEK)), 75);
    assert_eq!(warning_threshold(Some(PlanType::Pro), Some(FIVE_HOURS)), 75);
    assert_eq!(warning_threshold(Some(PlanType::Plus), None), 75);
    let mut limits = Notices::default();
    let mut plus = sample(Some(used(55, FIVE_HOURS)), None);
    plus.plan = Some(PlanType::Plus);
    assert_eq!(remaining(&limits.take(&plus, Source::Read)), [50]);
}

#[test]
fn a_window_at_a_hundred_percent_or_credits_that_can_be_spent_do_not_warn() {
    let mut limits = Notices::default();
    assert!(
        limits
            .take(
                &sample(Some(used(100, FIVE_HOURS)), Some(used(95, WEEK))),
                Source::Read
            )
            .is_empty()
    );

    let mut credits = sample(Some(used(96, FIVE_HOURS)), None);
    credits.credits_usable = Some(true);
    assert!(limits.take(&credits, Source::Read).is_empty());
    // a workspace limit that was reached takes the credits' place
    credits.reached = Some(true);
    assert_eq!(remaining(&limits.take(&credits, Source::Read)), [5]);
}

#[test]
fn a_rolling_update_without_credits_is_judged_by_the_credits_the_last_read_saw() {
    let mut limits = Notices::default();
    let mut read = sample(Some(used(10, FIVE_HOURS)), None);
    read.credits_usable = Some(true);
    limits.take(&read, Source::Read);
    assert!(
        limits
            .take(
                &sample(Some(used(96, FIVE_HOURS)), None),
                Source::Rolling { read_out: false }
            )
            .is_empty()
    );
}

#[test]
fn a_snapshot_of_another_bucket_neither_warns_nor_changes_the_notice() {
    let mut limits = Notices::default();
    let mut other = sample(Some(used(96, FIVE_HOURS)), None);
    other.codex = false;
    assert!(limits.take(&other, Source::Read).is_empty());
    assert_eq!(limits.notice(), None);
}

#[test]
fn the_notice_names_the_window_with_the_least_left_and_gives_way_to_credits() {
    let mut limits = Notices::default();
    limits.take(
        &sample(Some(used(80, FIVE_HOURS)), Some(used(90, WEEK))),
        Source::Read,
    );
    assert_eq!(limits.notice(), Some((used(90, WEEK), true)));
    // equal use: the primary window has priority
    limits.take(
        &sample(Some(used(90, FIVE_HOURS)), Some(used(90, WEEK))),
        Source::Read,
    );
    assert_eq!(limits.notice(), Some((used(90, FIVE_HOURS), false)));
    let mut credits = sample(Some(used(90, FIVE_HOURS)), None);
    credits.credits_usable = Some(true);
    limits.take(&credits, Source::Read);
    assert_eq!(limits.notice(), None);
}

#[test]
fn a_rolling_update_does_not_lower_a_window_that_was_past_its_threshold() {
    let mut limits = Notices::default();
    limits.take(&sample(Some(used(90, FIVE_HOURS)), None), Source::Read);
    limits.take(
        &sample(Some(used(10, FIVE_HOURS)), None),
        Source::Rolling { read_out: false },
    );
    assert_eq!(limits.notice().map(|(window, _)| window.percent), Some(90));
    // a full read does
    limits.take(&sample(Some(used(10, FIVE_HOURS)), None), Source::Read);
    assert_eq!(limits.notice(), None);
}

#[test]
fn a_read_that_a_rolling_update_overtook_does_not_replace_the_notice() {
    let mut limits = Notices::default();
    limits.take(
        &sample(Some(used(90, FIVE_HOURS)), None),
        Source::Rolling { read_out: true },
    );
    limits.take(&sample(Some(used(10, FIVE_HOURS)), None), Source::Read);
    assert_eq!(limits.notice().map(|(window, _)| window.percent), Some(90));
    // the next read is the newer one
    limits.take(&sample(Some(used(10, FIVE_HOURS)), None), Source::Read);
    assert_eq!(limits.notice(), None);
}

#[test]
fn a_reached_limit_blocks_the_notice_until_a_read_says_otherwise() {
    let mut limits = Notices::default();
    let mut reached = sample(Some(used(100, FIVE_HOURS)), None);
    reached.reached = Some(false);
    limits.take(&reached, Source::Rolling { read_out: false });
    assert!(limits.blocked());
    limits.take(
        &sample(Some(used(20, FIVE_HOURS)), None),
        Source::Rolling { read_out: false },
    );
    assert!(
        limits.blocked(),
        "a sparse update keeps what the read learned"
    );
    limits.take(&sample(Some(used(20, FIVE_HOURS)), None), Source::Read);
    assert!(!limits.blocked());
}

// the estimated cost of the thread

fn ctx(now: i64) -> Ctx {
    Ctx {
        now,
        plan: Some(PlanType::Business),
        backend_auth: true,
        status_items: Selected {
            credits: true,
            estimated_cost: false,
        },
    }
}

fn usage(thread: &str, credits: i64) -> ThreadUsage {
    ThreadUsage {
        thread_id: thread.to_owned(),
        estimated_usage_credits_micros: credits,
        estimated_usage_usd_micros: None,
        native_usage_usd_micros: None,
        groups: Vec::new(),
    }
}

#[test]
fn the_usage_is_asked_once_for_an_enterprise_thread_and_never_for_another_plan() {
    let mut state = State::default();
    let request = state.ensure_requested("t1", &ctx(0)).expect("asked");
    assert_eq!(request.thread, "t1");
    assert_eq!(state.ensure_requested("t1", &ctx(0)), None);

    let mut other = State::default();
    let mut pro = ctx(0);
    pro.plan = Some(PlanType::Pro);
    assert_eq!(other.ensure_requested("t1", &pro), None);
    let mut keyless = ctx(0);
    keyless.backend_auth = false;
    assert_eq!(other.ensure_requested("t1", &keyless), None);
}

#[test]
fn an_answer_for_another_thread_or_request_is_not_accepted() {
    let mut state = State::default();
    let request = state.ensure_requested("t1", &ctx(0)).unwrap();
    let wrong_request = state.finish(
        "t1",
        Some("t1"),
        request.request + 1,
        Ok(Outcome::Available(usage("t1", 5))),
        &ctx(0),
    );
    assert!(!wrong_request.accepted);
    let not_shown = state.finish(
        "t1",
        Some("t2"),
        request.request,
        Ok(Outcome::Available(usage("t1", 5))),
        &ctx(0),
    );
    assert!(!not_shown.accepted);
    let done = state.finish(
        "t1",
        Some("t1"),
        request.request,
        Ok(Outcome::Available(usage("t1", 5))),
        &ctx(0),
    );
    assert!(done.accepted);
    assert_eq!(
        state
            .estimate()
            .map(|usage| usage.estimated_usage_credits_micros),
        Some(5)
    );
}

#[test]
fn a_zero_after_a_positive_figure_is_kept_as_the_positive_one() {
    let mut state = State::default();
    let first = state.ensure_requested("t1", &ctx(0)).unwrap();
    state.finish(
        "t1",
        Some("t1"),
        first.request,
        Ok(Outcome::Available(usage("t1", 7))),
        &ctx(0),
    );
    let second = state
        .after_turn("t1", &ctx(1_000))
        .expect("asked after the turn");
    state.finish(
        "t1",
        Some("t1"),
        second.request,
        Ok(Outcome::Available(usage("t1", 0))),
        &ctx(1_000),
    );
    assert_eq!(
        state
            .estimate()
            .map(|usage| usage.estimated_usage_credits_micros),
        Some(7)
    );
}

#[test]
fn a_turn_settles_when_the_figure_rises_and_asks_again_on_the_delays_until_then() {
    let mut state = State::default();
    let first = state.ensure_requested("t1", &ctx(0)).unwrap();
    state.finish(
        "t1",
        Some("t1"),
        first.request,
        Ok(Outcome::Available(usage("t1", 10))),
        &ctx(0),
    );

    let request = state
        .after_turn("t1", &ctx(1_000))
        .expect("asked after the turn");
    // the backend has not settled: the same figure, the next ask is due after 15 s
    let finished = state.finish(
        "t1",
        Some("t1"),
        request.request,
        Ok(Outcome::Available(usage("t1", 10))),
        &ctx(1_000),
    );
    assert!(finished.request.is_none());
    assert_eq!(state.next_due(), Some(16_000));
    assert_eq!(state.tick("t1", &ctx(15_999)), None);
    let again = state.tick("t1", &ctx(16_000)).expect("due");
    state.finish(
        "t1",
        Some("t1"),
        again.request,
        Ok(Outcome::Available(usage("t1", 25))),
        &ctx(16_000),
    );
    // settled: nothing is due any more
    assert_eq!(state.next_due(), None);
}

#[test]
fn a_failed_ask_is_retried_after_5_15_and_60_seconds_and_then_not_again() {
    let mut state = State::default();
    let mut now = 0;
    let mut request = state.ensure_requested("t1", &ctx(now)).unwrap();
    for delay in [5_000, 15_000, 60_000] {
        state.finish(
            "t1",
            Some("t1"),
            request.request,
            Err("down".into()),
            &ctx(now),
        );
        assert_eq!(state.next_due(), Some(now + delay));
        now += delay;
        request = state.tick("t1", &ctx(now)).expect("due");
    }
    state.finish(
        "t1",
        Some("t1"),
        request.request,
        Err("down".into()),
        &ctx(now),
    );
    assert_eq!(state.next_due(), None);
}

#[test]
fn a_disabled_feature_clears_the_figure_and_stops_asking() {
    let mut state = State::default();
    let request = state.ensure_requested("t1", &ctx(0)).unwrap();
    state.finish(
        "t1",
        Some("t1"),
        request.request,
        Ok(Outcome::Disabled),
        &ctx(0),
    );
    assert_eq!(state.estimate(), None);
    assert_eq!(state.ensure_requested("t1", &ctx(0)), None);
}

#[test]
fn a_status_card_waits_for_the_figure_and_is_handed_it() {
    let mut state = State::default();
    let request = state.for_status("t1", 42, &ctx(0)).expect("asked");
    let done = state.finish(
        "t1",
        Some("t1"),
        request.request,
        Ok(Outcome::Available(usage("t1", 9))),
        &ctx(0),
    );
    assert_eq!(done.cards, [42]);
}
