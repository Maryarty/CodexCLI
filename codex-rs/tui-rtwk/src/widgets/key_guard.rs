//! `KeyGuard`: the key that changed what is shown does not act again on the
//! screen it opened (a held Enter must not activate the next menu's
//! highlighted row). Oracle: `onboarding/auth/activation_key_guard.rs`
//! (`ActivationKeyGuard` with `reported_repeats_only` false): the key is
//! held back until it is released, another key is pressed, or a second
//! passes without it (a terminal that reports no repeats repeats with
//! presses).
//!
//! In Flutter it is the `HardwareKeyboard` state a `Focus.onKeyEvent`
//! consults: an object of the `State`, shared by the nodes' callbacks, with
//! the time handed in (the app's clock, a test's fake clock moves it).

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::focus::KeyPress;
use rtwk::prelude::Mods;
use rtwk_core::input::{KeyCode, KeyEvent};

/// How long a guarded key stays guarded without being seen again.
pub const GUARD: Duration = Duration::from_secs(1);

/// The key a guard holds back: Enter or a character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardKey {
    Enter,
    Char(char),
}

impl GuardKey {
    /// The key of a press: Enter or a character with no modifier but Shift.
    pub fn of_press(key: &KeyPress) -> Option<GuardKey> {
        if key.mods != Mods::NONE && key.mods != Mods::SHIFT {
            return None;
        }
        match key.code {
            KeyCode::Enter => Some(GuardKey::Enter),
            KeyCode::Char => key.ch.map(GuardKey::Char),
            _ => None,
        }
    }

    /// The key of a raw event (a repeat or a release, which the focus path
    /// does not tell from a press): by its code or its text.
    pub fn of_event(event: &KeyEvent) -> Option<GuardKey> {
        KeyPress::from_event(event)
            .as_ref()
            .and_then(GuardKey::of_press)
            .or(match (event.code, event.text.chars().next()) {
                (KeyCode::Enter, _) => Some(GuardKey::Enter),
                (KeyCode::Char, Some(c)) => Some(GuardKey::Char(c)),
                _ => None,
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Guarded {
    key: GuardKey,
    until: Duration,
}

/// The guard; clones share it (the callbacks of several focus nodes).
#[derive(Clone, Debug, Default)]
pub struct KeyGuard(Rc<Cell<Option<Guarded>>>);

impl KeyGuard {
    /// `key` just acted: it is held back from `now` on.
    pub fn arm(&self, key: GuardKey, now: Duration) {
        self.0.set(Some(Guarded {
            key,
            until: now + GUARD,
        }));
    }

    /// Whether the guard holds a key.
    pub fn is_armed(&self) -> bool {
        self.0.get().is_some()
    }

    /// Whether the press of `key` at `now` is held back; the guard is
    /// prolonged by it, and ended by any other press.
    pub fn press(&self, key: Option<GuardKey>, now: Duration) -> bool {
        let Some(guard) = self.0.get() else {
            return false;
        };
        if key == Some(guard.key) && now <= guard.until {
            self.0.set(Some(Guarded {
                key: guard.key,
                until: guard.until.max(now + GUARD),
            }));
            return true;
        }
        self.0.set(None);
        false
    }

    /// A repeat of `key`: the guard is prolonged while it is the guarded
    /// key, ended by another.
    pub fn repeat(&self, key: Option<GuardKey>, now: Duration) {
        match self.0.get() {
            Some(guard) if key == Some(guard.key) => self.0.set(Some(Guarded {
                key: guard.key,
                until: guard.until.max(now + GUARD),
            })),
            Some(_) => self.0.set(None),
            None => {}
        }
    }

    /// `key` was released: its guard ends.
    pub fn release(&self, key: Option<GuardKey>) {
        if self.0.get().is_some_and(|guard| Some(guard.key) == key) {
            self.0.set(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        guard.repeat(Some(GuardKey::Enter), SECOND * 2);
        assert!(guard.press(Some(GuardKey::Enter), SECOND * 2 + SECOND / 2));
        guard.repeat(Some(GuardKey::Char('j')), SECOND * 3);
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
}
