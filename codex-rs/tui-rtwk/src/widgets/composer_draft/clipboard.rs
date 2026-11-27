//! A clipboard read belongs to the draft that asked for it. A first `!` that
//! is still held by the paste detector belongs after a successfully read image.

use std::time::Duration;

use codex_tui_core::ids::{ClipboardRead, ThreadId};
use codex_tui_core::kit::paste_burst::FlushResult;

use super::ComposerDraft;

/// A second Ctrl+V within this time of the first is a repeat of it
/// (`IMAGE_PASTE_REPEAT_WINDOW`).
pub const REPEAT_WINDOW: Duration = Duration::from_millis(250);

#[derive(Default)]
pub(super) struct Waiting {
    issued: u64,
    pending: Option<ClipboardRead>,
    pressed: Option<Duration>,
    prefix: Option<String>,
}

impl Waiting {
    pub(super) fn retire(&mut self) {
        self.pending = None;
        self.prefix = None;
    }
}

impl ComposerDraft {
    pub fn ask_for_image(
        &self,
        thread: &ThreadId,
        now: Duration,
        hold_bang: bool,
    ) -> Option<ClipboardRead> {
        let mut inner = self.0.borrow_mut();
        let repeat = inner
            .clipboard
            .pressed
            .is_some_and(|before| now < before + REPEAT_WINDOW);
        inner.clipboard.pressed = Some(now);
        if repeat {
            return None;
        }
        if hold_bang
            && !inner.shell
            && matches!(inner.burst.clone().flush_pending(), FlushResult::Typed('!'))
        {
            inner.burst.flush_pending();
            inner.clipboard.prefix = Some("!".to_owned());
        }
        let waiting = &mut inner.clipboard;
        waiting.issued += 1;
        let read = ClipboardRead {
            thread: thread.clone(),
            number: waiting.issued,
        };
        waiting.pending = Some(read.clone());
        Some(read)
    }

    pub fn pending_image_read(&self) -> Option<ClipboardRead> {
        self.0.borrow().clipboard.pending.clone()
    }

    /// The first bang typed after the read began also belongs after the image.
    pub fn hold_image_bang(&self) -> bool {
        let mut inner = self.0.borrow_mut();
        if inner.shell || inner.clipboard.pending.is_none() || inner.clipboard.prefix.is_some() {
            return false;
        }
        inner.clipboard.prefix = Some("!".to_owned());
        true
    }

    /// Enter while the held prefix is being prepared is a newline in the paste.
    pub fn pending_image_newline(&self) -> bool {
        let mut inner = self.0.borrow_mut();
        let Some(prefix) = &mut inner.clipboard.prefix else {
            return false;
        };
        prefix.push('\n');
        true
    }

    /// Another edit should see the held prefix in the field before handling its key.
    pub fn take_image_prefix(&self) -> Option<String> {
        self.0.borrow_mut().clipboard.prefix.take()
    }

    pub fn complete_image_read(&self) -> Option<String> {
        let mut inner = self.0.borrow_mut();
        inner.clipboard.pending = None;
        inner.clipboard.prefix.take()
    }
}
