#![allow(clippy::expect_used, clippy::unwrap_used)]

//! The usage-limit reset flow (oracle `chatwidget/usage.rs`, `reset_credits.rs`): the pure model
//! of the idempotency key, the redemption in flight, how each outcome ends it and what the hint
//! at startup waits for. The words and the popups are the view's: these tests read data.

use codex_tui_core::screens::status::{CreditStatus, ResetCredit, ResetCredits, ResetKind};
use codex_tui_core::screens::usage::reset::{
    MenuEntry, Next, Outcome, Settled, State, Step, options,
};

fn credit(id: &str, status: CreditStatus, expires_at: Option<i64>) -> ResetCredit {
    ResetCredit {
        id: id.to_owned(),
        kind: ResetKind::CodexRateLimits,
        status,
        granted_at: 0,
        expires_at,
        title: None,
        description: None,
    }
}

fn credits(count: i64) -> ResetCredits {
    ResetCredits {
        available_count: count,
        credits: None,
    }
}

fn step(state: &State) -> Step {
    state
        .redemption
        .as_ref()
        .expect("a redemption")
        .step
        .clone()
}

fn key_of(next: Next) -> String {
    match next {
        Next::Redeem { key, .. } => key,
        other => panic!("expected a redemption, got {other:?}"),
    }
}

#[test]
fn a_retry_reuses_the_idempotency_key_and_a_new_confirmation_makes_a_new_one() {
    let mut state = State::default();
    let key = key_of(state.redeem(Some("c1".into())));
    assert_eq!(state.redeemed(&key, Err("boom".into())), Next::Nothing);
    assert_eq!(step(&state), Step::Settled(Settled::Failed));
    assert_eq!(
        state.retry(),
        Next::Redeem {
            key: key.clone(),
            credit_id: Some("c1".into())
        }
    );
    assert_eq!(step(&state), Step::Asking);
    state.redeemed(&key, Err("boom".into()));
    let fresh = key_of(state.redeem(None));
    assert_ne!(fresh, key);
}

#[test]
fn a_redemption_in_flight_is_not_started_twice_and_a_wrong_key_changes_nothing() {
    let mut state = State::default();
    let key = key_of(state.redeem(None));
    assert_eq!(state.redeem(None), Next::Nothing);
    assert_eq!(state.retry(), Next::Nothing);
    assert_eq!(state.redeemed("another", Ok(Outcome::Reset)), Next::Nothing);
    assert_eq!(step(&state), Step::Asking);
    assert!(state.redeeming());
    assert_eq!(
        state.redeemed(&key, Ok(Outcome::NothingToReset)),
        Next::Nothing
    );
    assert_eq!(step(&state), Step::Settled(Settled::NothingToReset));
    // an answer that is not waited for any more is dropped
    assert_eq!(state.redeemed(&key, Ok(Outcome::Reset)), Next::Nothing);
    assert_eq!(step(&state), Step::Settled(Settled::NothingToReset));
}

#[test]
fn a_reset_reads_the_credits_again_and_tells_the_count_that_is_left() {
    let mut state = State::default();
    let key = key_of(state.redeem(None));
    assert_eq!(state.redeemed(&key, Ok(Outcome::Reset)), Next::Read);
    assert_eq!(step(&state), Step::Refreshing);
    assert_eq!(state.available, None);
    state.after_reset_read(Ok(Some(credits(2))));
    assert_eq!(
        step(&state),
        Step::Settled(Settled::Reset { left: Some(2) })
    );
    assert_eq!(state.available, Some(2));
}

#[test]
fn a_reset_whose_count_cannot_be_read_still_says_it_happened() {
    let mut state = State::default();
    let key = key_of(state.redeem(None));
    assert_eq!(
        state.redeemed(&key, Ok(Outcome::AlreadyRedeemed)),
        Next::Read
    );
    state.after_reset_read(Err(()));
    assert_eq!(step(&state), Step::Settled(Settled::Reset { left: None }));
    assert_eq!(state.available, None);
}

#[test]
fn no_credit_is_stale_for_a_chosen_credit_and_zero_for_the_default_one() {
    let mut state = State::default();
    state.available = Some(1);
    let key = key_of(state.redeem(Some("c1".into())));
    state.redeemed(&key, Ok(Outcome::NoCredit));
    assert_eq!(step(&state), Step::Settled(Settled::Stale));
    assert_eq!(state.available, None);

    let key = key_of(state.redeem(None));
    state.redeemed(&key, Ok(Outcome::NoCredit));
    assert_eq!(step(&state), Step::Settled(Settled::NoCredit));
    assert_eq!(state.available, Some(0));
}

#[test]
fn the_menu_waits_for_its_own_read_and_a_stale_answer_is_ignored() {
    let mut state = State::default();
    assert_eq!(state.menu_entry(true), MenuEntry::Check);
    let first = state.open_menu().expect("a read is asked");
    let second = state.open_menu().expect("a read is asked");
    state.menu_read(first, Ok(Some(credits(3))));
    assert_eq!(state.available, None);
    state.menu_read(second, Ok(Some(credits(1))));
    assert_eq!(state.available, Some(1));
    assert_eq!(state.menu_entry(true), MenuEntry::Available(1));
    // a count known to be positive needs no read; zero does
    assert_eq!(state.open_menu(), None);
    state.available = Some(0);
    assert!(state.open_menu().is_some());
    assert_eq!(state.menu_entry(true), MenuEntry::NoneAvailable);
}

#[test]
fn the_startup_hint_is_told_once_and_only_when_a_reset_can_be_used() {
    let mut state = State::default();
    let request = state.start_hint();
    assert_eq!(state.hint_read(request + 1, Ok(Some(credits(2)))), None);
    assert_eq!(state.hint_read(request, Ok(Some(credits(2)))), Some(2));
    assert_eq!(state.available, Some(2));
    assert_eq!(state.hint_read(request, Ok(Some(credits(2)))), None);

    let request = state.start_hint();
    assert_eq!(state.hint_read(request, Ok(Some(credits(0)))), None);
    assert_eq!(state.available, Some(0));
    let request = state.start_hint();
    assert_eq!(state.hint_read(request, Err(())), None);
}

#[test]
fn clear_drops_what_was_asked_of_the_old_account_but_keeps_the_numbers_going() {
    let mut state = State::default();
    let hint = state.start_hint();
    let key = key_of(state.redeem(None));
    state.clear();
    assert_eq!(state.redemption, None);
    assert_eq!(state.hint_read(hint, Ok(Some(credits(1)))), None);
    assert_ne!(key_of(state.redeem(None)), key);
}

#[test]
fn the_picker_lists_available_credits_soonest_expiry_first_up_to_the_count() {
    let summary = ResetCredits {
        available_count: 2,
        credits: Some(vec![
            credit("never", CreditStatus::Available, None),
            credit("late", CreditStatus::Available, Some(300)),
            credit("used", CreditStatus::Redeemed, Some(1)),
            credit("soon", CreditStatus::Available, Some(100)),
        ]),
    };
    let ids: Vec<_> = options(&summary)
        .into_iter()
        .map(|option| option.credit_id)
        .collect();
    assert_eq!(ids, [Some("soon".to_owned()), Some("late".to_owned())]);

    // the details were not sent: one default row, which redeems the backend's next credit
    let rows = options(&credits(3));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].credit_id, None);
}
