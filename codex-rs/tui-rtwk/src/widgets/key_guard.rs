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

/// Two handles are equal when they are the same guard: a widget that holds
/// one rebuilds only when it is handed another.
impl PartialEq for KeyGuard {
    fn eq(&self, other: &KeyGuard) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for KeyGuard {}

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
    pub fn held(&self, key: Option<GuardKey>, now: Duration) {
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
