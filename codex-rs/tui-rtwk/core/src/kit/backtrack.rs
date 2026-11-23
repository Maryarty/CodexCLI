//! The Esc Esc machine and the prompts of a transcript it walks (the oracle's `app_backtrack.rs`,
//! `browsing.rs`): pure logic without app state (ADR-013 O3). It holds view state only (which prompt is
//! highlighted), never thread data, so whoever draws it (the UI's mode controller) owns one value of
//! it; every transition names its inputs and returns what the view must do. The edit's RPC pipeline
//! (`ThreadPart`, `Command`) is the thread's, in `screens::backtrack`.

use crate::ids::ThreadId;
use crate::threads::{Item, ItemKey, Transcript, UserPrompt};

// ---------------------------------------------------------------------------
// The prompts of a transcript

/// A prompt the backtrack can offer: the oracle's `has_visible_content`
/// (a prompt of control characters only is hidden unless it has elements or
/// images).
pub fn is_visible(prompt: &UserPrompt) -> bool {
    !super::paste::sanitize_paste(&prompt.text).trim().is_empty()
        || !prompt.elements.is_empty()
        || !prompt.images.is_empty()
}

/// Positions (in transcript order) of the prompts the backtrack offers.
/// A transcript is one thread's session, so it starts at its first item (the
/// oracle starts after the last `SessionInfoCell` of a shared cell list).
pub fn user_positions(transcript: &Transcript) -> impl Iterator<Item = usize> + '_ {
    transcript
        .iter()
        .enumerate()
        .filter_map(|(index, (_, item))| match item {
            Item::User(prompt) if is_visible(prompt) => Some(index),
            _ => None,
        })
}

pub fn user_count(transcript: &Transcript) -> usize {
    user_positions(transcript).count()
}

pub fn has_backtrack_target(transcript: &Transcript) -> bool {
    user_count(transcript) > 0
}

/// The transcript position of the `nth` offered prompt.
pub fn nth_user_position(transcript: &Transcript, nth: usize) -> Option<usize> {
    user_positions(transcript).nth(nth)
}

/// The server ids and the prompt of the `nth` offered prompt.
pub fn nth_user(transcript: &Transcript, nth: usize) -> Option<(&ItemKey, &UserPrompt)> {
    let (key, item) = transcript.get(nth_user_position(transcript, nth)?)?;
    match item {
        Item::User(prompt) => Some((key, prompt)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The machine

/// What the machine needs to know of the transcript when a key arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Targets {
    /// How many prompts the transcript offers (`user_count`).
    pub count: usize,
    /// Older history can still be loaded, so a prompt may exist beyond `count`.
    pub older_history: bool,
}

impl Targets {
    pub fn of(transcript: &Transcript, older_history: bool) -> Self {
        Self {
            count: user_count(transcript),
            older_history,
        }
    }

    fn any(self) -> bool {
        self.count > 0 || self.older_history
    }
}

/// Where the machine is (oracle `BacktrackState`: `primed`,
/// `overlay_preview_active`, `nth_user_message`, with `usize::MAX` for none).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Backtrack {
    #[default]
    Idle,
    /// The first Esc: the composer shows "Esc again to edit the previous message".
    Primed { base: Option<ThreadId> },
    /// Browsing the prompts; `nth` is none until a prompt is highlighted.
    Browsing {
        base: Option<ThreadId>,
        nth: Option<usize>,
    },
}

/// What an Esc did; the view acts on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EscStep {
    /// Not the machine's key now (the composer has a draft).
    Ignored,
    /// Primed; `hint` = show the composer hint (only when a prompt exists).
    Primed { hint: bool },
    /// Browsing began, the prompt `select` highlighted (none: nothing to show yet).
    Browse { select: Option<usize> },
    /// Nothing to edit: the chat shows its "no previous message" line; the machine is idle again.
    NoPrevious,
    /// Browsing ended: the view restores the place it had before.
    Cancelled,
}

/// The prompt chosen by Enter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chosen {
    pub thread: ThreadId,
    pub nth: usize,
}

impl Backtrack {
    pub fn is_primed(&self) -> bool {
        matches!(self, Self::Primed { .. })
    }

    pub fn is_browsing(&self) -> bool {
        matches!(self, Self::Browsing { .. })
    }

    /// The highlighted prompt while browsing.
    pub fn selected(&self) -> Option<usize> {
        match self {
            Self::Browsing { nth, .. } => *nth,
            _ => None,
        }
    }

    /// Esc in the main view (oracle `handle_backtrack_esc_key`,
    /// `open_backtrack_preview`) and while browsing (`cancel_transcript_browsing`).
    pub fn esc(
        &mut self,
        thread: Option<&ThreadId>,
        composer_empty: bool,
        targets: Targets,
    ) -> EscStep {
        match self {
            Self::Browsing { .. } => {
                self.reset();
                EscStep::Cancelled
            }
            _ if !composer_empty => EscStep::Ignored,
            Self::Idle => {
                *self = Self::Primed {
                    base: thread.cloned(),
                };
                EscStep::Primed {
                    hint: targets.count > 0,
                }
            }
            Self::Primed { .. } => self.begin(thread, targets),
        }
    }

    /// Esc in the detailed transcript, where one Esc starts browsing (oracle
    /// `begin_overlay_backtrack_preview`).
    pub fn begin_from_detailed(&mut self, thread: Option<&ThreadId>, targets: Targets) -> EscStep {
        self.begin(thread, targets)
    }

    fn begin(&mut self, thread: Option<&ThreadId>, targets: Targets) -> EscStep {
        if !targets.any() {
            self.reset();
            return EscStep::NoPrevious;
        }
        *self = Self::Browsing {
            base: thread.cloned(),
            nth: None,
        };
        let select = self.older(targets.count);
        EscStep::Browse { select }
    }

    /// Left: the next older prompt, the newest first (oracle
    /// `step_backtrack_and_highlight`). None when there is no prompt.
    pub fn older(&mut self, count: usize) -> Option<usize> {
        let last = count.checked_sub(1)?;
        let Self::Browsing { nth, .. } = self else {
            return None;
        };
        let next = match *nth {
            None => last,
            Some(0) => 0,
            Some(n) => (n - 1).min(last),
        };
        *nth = Some(next);
        Some(next)
    }

    /// Right: the next newer prompt (oracle `step_forward_backtrack_and_highlight`).
    pub fn newer(&mut self, count: usize) -> Option<usize> {
        let last = count.checked_sub(1)?;
        let Self::Browsing { nth, .. } = self else {
            return None;
        };
        let next = nth.map_or(last, |n| (n + 1).min(last));
        *nth = Some(next);
        Some(next)
    }

    /// Enter: the chosen prompt, when the thread is still the one browsing
    /// began on and the prompt exists (oracle `backtrack_selection`). The
    /// machine is idle after a choice; with none it stays where it is.
    pub fn confirm(&mut self, thread: Option<&ThreadId>, count: usize) -> Option<Chosen> {
        let Self::Browsing {
            base: Some(base),
            nth: Some(nth),
        } = self
        else {
            return None;
        };
        if thread != Some(&*base) || *nth >= count {
            return None;
        }
        let selection = Chosen {
            thread: base.clone(),
            nth: *nth,
        };
        self.reset();
        Some(selection)
    }

    /// Any other input between the two Escs ends the pair (oracle
    /// `cancel_primed_browsing_for_event`).
    pub fn interrupt(&mut self) {
        if self.is_primed() {
            self.reset();
        }
    }

    /// The oldest loaded prompt (or none) is highlighted: older history is
    /// wanted (oracle `browsing_needs_history`).
    pub fn needs_history(&self) -> bool {
        matches!(
            self,
            Self::Browsing {
                nth: None | Some(0),
                ..
            }
        )
    }

    pub fn reset(&mut self) {
        *self = Self::Idle;
    }
}
