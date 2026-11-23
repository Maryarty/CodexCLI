//! A Ctrl+V the draft waits for: the host reads the clipboard's image and
//! answers with a file (what the app holds as `Pasted`); the draft that
//! asked attaches it as the next `[Image #N]`, and no other draft does.

use std::path::PathBuf;
use std::time::Duration;

use rtwk::editing::TextEditingController;

use super::ComposerDraft;

/// A second Ctrl+V within this time of the first is a repeat of it
/// (`IMAGE_PASTE_REPEAT_WINDOW`).
pub const REPEAT_WINDOW: Duration = Duration::from_millis(250);

/// What the draft remembers of its asks.
#[derive(Default)]
pub(super) struct Waiting {
    /// How many answers the host had given when the draft asked.
    asked_at: Option<u64>,
    /// When the last Ctrl+V was pressed (the time of the app's clock).
    pressed: Option<Duration>,
}

impl ComposerDraft {
    /// Ctrl+V at `now` with the host having given `answers` so far: whether
    /// the clipboard is asked for (not when the key is a repeat of the one
    /// before). The draft then waits for the answer after `answers`.
    pub fn ask_for_image(&self, now: Duration, answers: u64) -> bool {
        let mut inner = self.0.borrow_mut();
        let waiting = &mut inner.clipboard;
        let repeat = waiting
            .pressed
            .is_some_and(|before| now < before + REPEAT_WINDOW);
        waiting.pressed = Some(now);
        if repeat {
            return false;
        }
        waiting.asked_at = Some(answers);
        true
    }

    /// Whether an image the draft asked for has come (the draw may attach
    /// it): what `take_pasted_image` would give. `arrived(n)`: the host has
    /// answered with a file after its `n`-th answer.
    pub fn image_arrived(&self, arrived: impl Fn(u64) -> bool) -> bool {
        let inner = self.0.borrow();
        inner.clipboard.asked_at.is_some_and(arrived)
    }

    /// The image the draft asked for, attached at the selection as the next
    /// `[Image #N]` (the oracle's `attach_image` after `paste_image`). Whether
    /// there was one.
    pub fn take_pasted_image(
        &self,
        c: &TextEditingController,
        after: impl Fn(u64) -> Option<PathBuf>,
        image: impl Fn(usize) -> String,
    ) -> bool {
        let path: Option<PathBuf> = {
            let mut inner = self.0.borrow_mut();
            let asked = inner.clipboard.asked_at;
            let path = asked.and_then(after);
            if path.is_some() {
                inner.clipboard.asked_at = None;
            }
            path
        };
        let Some(path) = path else {
            return false;
        };
        self.attach_image(c, path, image);
        true
    }
}
