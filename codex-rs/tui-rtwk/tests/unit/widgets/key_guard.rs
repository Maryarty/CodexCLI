use codex_tui_rtwk::widgets::key_guard::*;
use rtwk::focus::KeyPress;
use rtwk::prelude::Mods;
use rtwk_core::input::KeyCode;
use rtwk_core::input::KeyEvent;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

const SECOND: Duration = Duration::from_secs(1);

fn armed(key: GuardKey) -> KeyGuard {
    let guard = KeyGuard::default();
    guard.arm(key, Duration::ZERO);
    guard
}

// activation_key_tests.rs: held_enter_press_after_activation_is_ignored
#[test]
fn a_press_of_the_guarded_key_is_held_back() {
    let guard = armed(GuardKey::Enter);
    assert!(guard.press(Some(GuardKey::Enter), Duration::from_millis(10)));
    assert!(guard.is_armed());
}

// activation_key_tests.rs: another key ends the guard
#[test]
fn another_key_ends_the_guard() {
    let guard = armed(GuardKey::Enter);
    assert!(!guard.press(Some(GuardKey::Char('x')), Duration::from_millis(10)));
    assert!(!guard.is_armed());
    assert!(!guard.press(Some(GuardKey::Enter), Duration::from_millis(20)));
}

// a terminal that reports no repeats repeats with presses: each press
// prolongs the guard; one after a second of silence is a new key
#[test]
fn presses_prolong_the_guard_and_a_second_of_silence_ends_it() {
    let guard = armed(GuardKey::Enter);
    for tick in 1..=5 {
        assert!(guard.press(
            Some(GuardKey::Enter),
            SECOND * tick - Duration::from_millis(1)
        ));
    }
    assert!(!guard.press(Some(GuardKey::Enter), SECOND * 7));
    assert!(!guard.is_armed());
}

#[test]
fn repeats_prolong_and_another_repeat_ends() {
    let guard = armed(GuardKey::Enter);
    guard.held(Some(GuardKey::Enter), SECOND * 2);
    assert!(guard.press(Some(GuardKey::Enter), SECOND * 2 + SECOND / 2));
    guard.held(Some(GuardKey::Char('j')), SECOND * 3);
    assert!(!guard.is_armed());
}

#[test]
fn a_release_of_the_guarded_key_ends_the_guard() {
    let guard = armed(GuardKey::Char('1'));
    guard.release(Some(GuardKey::Enter));
    assert!(guard.is_armed());
    guard.release(Some(GuardKey::Char('1')));
    assert!(!guard.is_armed());
    assert!(!guard.press(Some(GuardKey::Char('1')), Duration::from_millis(1)));
}

#[test]
fn clones_share_the_guard() {
    let guard = KeyGuard::default();
    let other = guard.clone();
    guard.arm(GuardKey::Enter, Duration::ZERO);
    assert!(other.press(Some(GuardKey::Enter), Duration::from_millis(1)));
}

#[test]
fn only_enter_and_characters_are_guard_keys() {
    assert_eq!(GuardKey::of_press(&KeyPress::ENTER), Some(GuardKey::Enter));
    assert_eq!(
        GuardKey::of_press(&KeyPress::char('y')),
        Some(GuardKey::Char('y'))
    );
    assert_eq!(GuardKey::of_press(&KeyPress::UP), None);
    assert_eq!(
        GuardKey::of_press(&KeyPress::char('y').with_mods(Mods::CTRL)),
        None
    );
}
