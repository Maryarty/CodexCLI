//! What the composer's draft did to the sparkle (UI state, one per thread in
//! `ThreadUi`, never seen by `apply`): the one short Astra flourish of a
//! fresh task with an untouched prompt. Ordinary draft content ends the
//! opportunity for good; a slash command keeps it while it is typed, and it
//! comes back when the command ran or was erased. Oracle:
//! `bottom_pane/chat_composer/sparkle.rs`.
//!
//! Flutter writes this as a `ChangeNotifier` the field's owner calls and the
//! overlay (`footer/sparkle.rs`) listens to, as an `AnimationController`'s
//! status is told to the widgets that paint it: the composer reports what
//! the user did (`typed`, `pasted`, `text_changed`), the page what the line
//! became (`line_ran`), and the painter asks what to show. The clock is the
//! app's: the callers hand the time over, nothing here reads one.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use rtwk::foundation::ValueNotifier;

use codex_tui_core::commands::{
    CommandFlags, ServiceTierCommand, find_slash_command, has_slash_command_prefix,
};

/// How often a visible sparkle is painted again (`FRAME_TICK`).
pub const FRAME_TICK: Duration = Duration::from_millis(150);
/// How long a visible sparkle lasts (`IDLE_TIMEOUT`).
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(15);
/// The last of those it fades over (`IDLE_FADE`).
pub const IDLE_FADE: Duration = Duration::from_secs(1);

/// What the draft has been (`SparkleDraft`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Draft {
    /// Nothing was typed: the opportunity is whole.
    #[default]
    Untouched,
    /// A slash command is being typed: the dots wait, the opportunity stays.
    Command,
    /// Something else was: no sparkle in this thread.
    Dismissed,
}

/// Where the sparkle is in its life (`Phase`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    /// No Astra model has offered it yet.
    #[default]
    Unarmed,
    /// Offered, not shown yet.
    Waiting,
    /// Shown since this time of the app's clock; the deadline runs from it
    /// whatever hides the dots meanwhile.
    Visible(Duration),
    /// Over: it never comes back.
    Finished,
}

/// What the composer says of itself and its key, before the field handles it.
pub struct Key {
    /// The key is a plain text character (no Ctrl, Alt or Super).
    pub plain_char: Option<char>,
    /// The composer holds nothing: no text, no shell mode, no attachment.
    pub empty: bool,
    /// The caret as a byte offset of `text`.
    pub cursor: usize,
    pub text: String,
    /// The key opens or belongs to the history search or is the `?` of the
    /// shortcut overlay: it does not count (`before_sparkle_key`).
    pub aside: bool,
    /// A Vim `/` query is open, or Normal mode makes the key a command.
    pub vim_query: bool,
    pub vim_normal: bool,
}

struct Inner {
    draft: Draft,
    /// The command as the user typed it, for what the field does not show yet.
    command_input: String,
    phase: Phase,
    /// The text as the last change left it (a caret move is none).
    last_text: String,
    flags: CommandFlags,
    tiers: Vec<ServiceTierCommand>,
    shell: bool,
}

/// The sparkle of one thread's composer: a handle, clones share it.
#[derive(Clone)]
pub struct SparkleController {
    inner: Rc<RefCell<Inner>>,
    ticks: Rc<ValueNotifier<u32>>,
}

impl Default for SparkleController {
    fn default() -> Self {
        SparkleController {
            inner: Rc::new(RefCell::new(Inner {
                draft: Draft::Untouched,
                command_input: String::new(),
                phase: Phase::Unarmed,
                last_text: String::new(),
                flags: CommandFlags::default(),
                tiers: Vec::new(),
                shell: false,
            })),
            ticks: Rc::new(ValueNotifier::new(0)),
        }
    }
}

impl PartialEq for SparkleController {
    fn eq(&self, other: &SparkleController) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl SparkleController {
    /// Notifies when the draft or the phase changes: whoever paints the dots
    /// listens.
    pub fn listenable(&self) -> Rc<ValueNotifier<u32>> {
        self.ticks.clone()
    }

    fn moved(&self) {
        self.ticks.set_value(self.ticks.value().wrapping_add(1));
    }

    pub fn draft(&self) -> Draft {
        self.inner.borrow().draft
    }

    pub fn phase(&self) -> Phase {
        self.inner.borrow().phase
    }

    /// What the slash commands are now (their gates and the tiers the model
    /// adds), and whether the draft is in shell mode: what decides whether a
    /// typed line is a command.
    pub fn know(&self, flags: CommandFlags, tiers: Vec<ServiceTierCommand>, shell: bool) {
        let mut inner = self.inner.borrow_mut();
        inner.flags = flags;
        inner.tiers = tiers;
        inner.shell = shell;
    }

    /// Whether the thread is offered the sparkle now: an eligible one whose
    /// opportunity is whole is armed (`mark_fresh_task_for_sparkle`,
    /// `select_sparkle_model`), an ineligible one stops what shows
    /// (`stop_ineligible_sparkle`). The widget that paints says it as it
    /// builds, so nothing is notified: that build is the answer.
    pub fn offer(&self, eligible: bool) {
        let mut inner = self.inner.borrow_mut();
        if !eligible {
            if matches!(inner.phase, Phase::Waiting | Phase::Visible(_)) {
                inner.phase = Phase::Finished;
            }
            return;
        }
        let open = matches!(inner.draft, Draft::Untouched | Draft::Command);
        if open && inner.phase == Phase::Unarmed {
            inner.phase = Phase::Waiting;
        }
    }

    /// The first frame the dots can show in: the deadline runs from `now`.
    pub fn begin(&self, now: Duration) {
        let mut inner = self.inner.borrow_mut();
        if inner.phase == Phase::Waiting {
            inner.phase = Phase::Visible(now);
        }
    }

    fn finish(&self) -> bool {
        let mut inner = self.inner.borrow_mut();
        if matches!(inner.phase, Phase::Waiting | Phase::Visible(_)) {
            inner.phase = Phase::Finished;
            true
        } else {
            false
        }
    }

    /// Ends what waits or shows (`stop_visible_sparkle`).
    pub fn stop_visible(&self) {
        if self.finish() {
            self.moved();
        }
    }

    /// Ordinary content: the opportunity is gone (`dismiss_sparkle`).
    pub fn dismiss(&self) {
        let changed = {
            let mut inner = self.inner.borrow_mut();
            let was = inner.draft;
            inner.draft = Draft::Dismissed;
            inner.command_input.clear();
            was != Draft::Dismissed
        };
        if self.finish() || changed {
            self.moved();
        }
    }

    /// A line was sent. A command that took it keeps the opportunity; a
    /// message (sent, queued or refused for a thread its parent owns) ends it
    /// (`after_sparkle_key`).
    pub fn line_ran(&self, command: bool) {
        if !command {
            self.dismiss();
            return;
        }
        let was = {
            let mut inner = self.inner.borrow_mut();
            if inner.draft == Draft::Command {
                inner.draft = Draft::Untouched;
                inner.command_input.clear();
                true
            } else {
                false
            }
        };
        if was {
            self.moved();
        }
    }

    /// A key, before the field handles it (`before_sparkle_key`): any key
    /// but the `/` of a command ends what shows; a plain character is
    /// classified here, for the text it makes is told by `text_changed`.
    pub fn before_key(&self, key: &Key) {
        if key.aside {
            return;
        }
        let command = self.draft() == Draft::Command;
        let slash = key.plain_char == Some('/') && key.empty && self.slash_enabled();
        if !command && !slash {
            self.stop_visible();
        }
        if key.vim_query || key.vim_normal {
            return;
        }
        let Some(ch) = key.plain_char else { return };
        match self.draft() {
            Draft::Untouched if slash => {
                let mut inner = self.inner.borrow_mut();
                inner.draft = Draft::Command;
                inner.command_input = ch.to_string();
                drop(inner);
                self.moved();
            }
            Draft::Command => {
                let mut projected = self.inner.borrow().command_input.clone();
                // the caret is where it is while the field shows what was typed
                let at = if key.text == projected {
                    key.cursor.min(projected.len())
                } else {
                    projected.len()
                };
                projected.insert(at, ch);
                if self.is_command(&projected) {
                    self.inner.borrow_mut().command_input = projected;
                } else {
                    self.dismiss();
                }
            }
            Draft::Untouched => self.dismiss(),
            Draft::Dismissed => {}
        }
    }

    /// A paste, before the field takes it (`note_sparkle_paste`); `empty` is
    /// whether the composer held nothing.
    pub fn pasted(&self, text: &str, current: &str, cursor: usize, empty: bool) {
        let command_paste = empty && self.is_command(text);
        if self.draft() != Draft::Command && !command_paste {
            self.stop_visible();
        }
        if text.is_empty() {
            return;
        }
        match self.draft() {
            Draft::Untouched if command_paste => {
                let mut inner = self.inner.borrow_mut();
                inner.draft = Draft::Command;
                inner.command_input = text.to_string();
                drop(inner);
                self.moved();
            }
            Draft::Command => {
                let mut projected = self.inner.borrow().command_input.clone();
                let at = if current == projected {
                    cursor.min(projected.len())
                } else {
                    projected.len()
                };
                projected.insert_str(at, text);
                if self.is_command(&projected) {
                    self.inner.borrow_mut().command_input = projected;
                } else {
                    self.dismiss();
                }
            }
            Draft::Untouched => self.dismiss(),
            Draft::Dismissed => {}
        }
    }

    /// The composer's text is `text` now, by whatever means (typing, an edit
    /// key, history, a clear: `note_sparkle_replaced_text`). While the
    /// history search is open its previews are not the draft, and nothing is
    /// counted until it ends.
    pub fn text_changed(&self, text: &str, searching: bool) {
        if searching {
            return;
        }
        {
            let mut inner = self.inner.borrow_mut();
            if inner.last_text == text {
                return;
            }
            inner.last_text = text.to_string();
        }
        self.replaced(text);
    }

    /// The draft is `text` now, though it may be what it was: what the
    /// sparkle tracks of it is judged again (`note_sparkle_replaced_text`).
    pub fn replaced(&self, text: &str) {
        match self.draft() {
            Draft::Command if text.is_empty() => {
                let mut inner = self.inner.borrow_mut();
                inner.draft = Draft::Untouched;
                inner.command_input.clear();
                drop(inner);
                self.moved();
            }
            Draft::Command if self.is_command(text) => {
                self.inner.borrow_mut().command_input = text.to_string();
            }
            Draft::Untouched if text.is_empty() => {}
            Draft::Untouched | Draft::Command => self.dismiss(),
            Draft::Dismissed => {}
        }
    }

    /// The composer as a frame finds it (`render_sparkle_at`): a draft with
    /// content that is no command in the making ends the opportunity, whatever
    /// way it came by. Nothing is judged while the history search is open
    /// (its previews are not the draft).
    pub fn settle(&self, text: &str, empty: bool, searching: bool) {
        if searching || empty {
            return;
        }
        let command = self.draft() == Draft::Command && self.is_command(text);
        if !command {
            self.dismiss();
        }
    }

    fn slash_enabled(&self) -> bool {
        true
    }

    /// Whether `text` is a command the user is typing or has typed
    /// (`is_sparkle_command`).
    pub fn is_command(&self, text: &str) -> bool {
        let inner = self.inner.borrow();
        let Some(body) = text.strip_prefix('/') else {
            return false;
        };
        if inner.shell || text.contains(['\n', '\r']) {
            return false;
        }
        let split = body.find(char::is_whitespace).unwrap_or(body.len());
        let (name, rest) = body.split_at(split);
        if rest.is_empty() {
            return name.is_empty()
                || has_slash_command_prefix(name, inner.flags, &inner.tiers)
                || find_slash_command(name, inner.flags, &inner.tiers).is_some();
        }
        find_slash_command(name, inner.flags, &inner.tiers)
            .is_some_and(|command| rest.trim().is_empty() || command.supports_inline_args())
    }
}
