//! What the draft keeps for modal (Vim) editing: the engine's state
//! (`core::kit::vim`), the register next to the kill ring, the
//! whole-draft undo history and the `/` query, and the `VimText` that lets
//! the engine edit the controller's text with the draft's elements kept whole
//! (`chat_composer/vim_history.rs`, `vim_register.rs`, `textarea.rs`).

use std::ops::Range;
use std::rc::Rc;

use rtwk::editing::{TextBuffer, TextEditingController, TextSelection};
use rtwk::focus::KeyPress;

use codex_tui_core::commands::parse_slash_name;
use codex_tui_core::kit::paste as paste_rules;

use codex_tui_core::kit::vim::{
    EditorAction, Persistent, Register, RegisterKind, UndoHistory, Vim, VimText,
};

use super::{ComposerDraft, Element};

/// A whole draft at one moment (text, caret, placeholders, shell mode): one
/// step of Vim's undo and redo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    text: Rc<str>,
    caret: usize,
    elements: Vec<Element>,
    shell: bool,
    /// The remote images above the text (a deleted row is one undoable edit).
    remote: Vec<String>,
}

impl Snapshot {
    /// What the snapshot costs in the history's byte budget.
    fn bytes(&self) -> usize {
        self.text.len()
            + self
                .elements
                .iter()
                .map(|e| e.label.len() + e.payload.len() + std::mem::size_of::<Element>())
                .sum::<usize>()
            + self.remote.iter().map(String::len).sum::<usize>()
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

/// The draft's modal state: what `Inner` holds for Vim.
#[derive(Default)]
pub(super) struct Modal {
    pub(super) vim: Vim,
    pub(super) history: UndoHistory<Snapshot>,
    pub(super) register_kind: RegisterKind,
    /// The first key of a chord (`g` of `g g`) waiting for its second.
    pub(super) chord: Option<KeyPress>,
    /// The `/` or `?` query being typed.
    pub(super) query: TextEditingController,
    /// How many edits put text at the start of the draft, and how many of them the composer
    /// has read (`prefix_insertion_revision`): a `!` there is shell mode.
    pub(super) prefix: (u64, u64),
}

impl ComposerDraft {
    /// Whether the draft is edited modally.
    pub fn vim_enabled(&self) -> bool {
        self.0.borrow().vim.vim.is_enabled()
    }

    /// The footer's word for the mode (`Vim: Normal`); none with Vim off.
    pub fn vim_label(&self) -> Option<&'static str> {
        self.0.borrow().vim.vim.label()
    }

    /// Turns modal editing on (Normal mode) or off (Insert), forgetting
    /// what the old mode recorded.
    pub fn set_vim_enabled(&self, on: bool) {
        let mut inner = self.0.borrow_mut();
        inner.vim.vim.set_enabled(on);
        inner.vim.history.reset();
        inner.vim.chord = None;
        inner.vim.query.clear();
    }

    /// Vim on with text entry going on: Insert mode, not Normal
    /// (`enable_vim_in_insert_mode`: a new draft of `tui.vim_mode_default`).
    pub fn start_in_vim_insert(&self, c: &TextEditingController) {
        self.set_vim_enabled(true);
        self.vim_reset_mode(c);
    }

    /// Runs `f` on the engine. The engine is out of the draft meanwhile (its
    /// host edits the draft), so a callback must not ask the draft for it.
    pub fn with_vim<R>(&self, f: impl FnOnce(&mut Vim) -> R) -> R {
        let mut vim = std::mem::take(&mut self.0.borrow_mut().vim.vim);
        let out = f(&mut vim);
        self.0.borrow_mut().vim.vim = vim;
        out
    }

    /// Runs `f` on the undo history (same rule as `with_vim`).
    pub fn with_vim_history<R>(&self, f: impl FnOnce(&mut UndoHistory<Snapshot>) -> R) -> R {
        let mut history = std::mem::take(&mut self.0.borrow_mut().vim.history);
        let out = f(&mut history);
        self.0.borrow_mut().vim.history = history;
        out
    }

    /// The field the `/` query is typed in (a handle).
    pub fn vim_query(&self) -> TextEditingController {
        self.0.borrow().vim.query.clone()
    }

    /// Whether an edit of the engine put text at the draft's start since this was last asked
    /// (the textarea's `prefix_insertion_revision` moved).
    pub fn vim_prefix_inserted(&self) -> bool {
        let mut inner = self.0.borrow_mut();
        let moved = inner.vim.prefix.0 != inner.vim.prefix.1;
        inner.vim.prefix.1 = inner.vim.prefix.0;
        moved
    }

    /// The first key of a chord pressed and not completed yet.
    pub fn vim_chord(&self) -> Option<KeyPress> {
        self.0.borrow().vim.chord.clone()
    }

    pub fn set_vim_chord(&self, key: Option<KeyPress>) {
        self.0.borrow_mut().vim.chord = key;
    }

    /// The register: the last kill or yank, and how it pastes.
    pub fn register(&self) -> Register {
        let inner = self.0.borrow();
        Register {
            text: inner.kill.as_ref().map(|k| k.0.clone()).unwrap_or_default(),
            kind: inner.vim.register_kind,
        }
    }

    /// A yank or kill of `from..to` into the register.
    fn store(&self, c: &TextEditingController, from: usize, to: usize, kind: RegisterKind) {
        self.kill(c, from, to);
        self.0.borrow_mut().vim.register_kind = kind;
    }

    /// The elements the register holds go back where its text was pasted,
    /// from `start`: a pasted placeholder and an attached image take the
    /// next number of their kind, as a new one does (`vim_register::sync`,
    /// `RegisterEvent::Pasted`); a mention keeps its label.
    fn paste_register_elements(&self, c: &TextEditingController, start: usize) {
        self.sync(c);
        let held = self
            .0
            .borrow()
            .kill
            .as_ref()
            .map(|k| k.1.clone())
            .unwrap_or_default();
        // what the new labels added to the text before the next element
        let mut shift = 0isize;
        for e in held {
            let from = start.saturating_add_signed(shift) + e.start;
            let to = from + (e.end - e.start);
            let (label, payload) = if e.binding.is_some() {
                (e.label.clone(), e.payload.clone())
            } else if e.image.is_some() {
                let images = self
                    .elements(c)
                    .iter()
                    .filter(|other| other.image.is_some())
                    .count();
                let label: Rc<str> =
                    Rc::from(renumbered(&e.label, self.remote_count() + images + 1));
                (label.clone(), label)
            } else {
                let base = e
                    .label
                    .rsplit_once(" #")
                    .filter(|(_, n)| n.parse::<usize>().is_ok())
                    .map_or(&*e.label, |(base, _)| base);
                (Rc::from(self.next_label(c, base)), e.payload.clone())
            };
            let delta = label.len() as isize - (to - from) as isize;
            if delta != 0 || *label != *e.label {
                let caret = c.selection().extent_offset;
                c.replace(from, to, &label);
                let caret = if caret >= to {
                    caret.saturating_add_signed(delta)
                } else {
                    caret
                };
                c.set_selection(TextSelection::collapsed(caret.min(c.text().len())));
            }
            let mut inner = self.0.borrow_mut();
            for other in &mut inner.elements {
                if other.start >= to {
                    other.start = other.start.saturating_add_signed(delta);
                    other.end = other.end.saturating_add_signed(delta);
                }
            }
            inner.seen = Some(c.buffer());
            let seq = inner.next_seq;
            inner.next_seq += 1;
            inner.elements.push(Element {
                start: from,
                end: from + label.len(),
                label,
                payload,
                seq,
                ..e
            });
            inner.elements.sort_by_key(|other| other.start);
            shift += delta;
        }
    }

    /// The whole draft now.
    pub fn snapshot(&self, c: &TextEditingController) -> Snapshot {
        Snapshot {
            text: c.text(),
            caret: c.selection().extent_offset,
            elements: self.elements(c),
            shell: self.shell(),
            remote: self.remote_images(),
        }
    }

    /// The draft put back as `snapshot` was.
    pub fn restore(&self, c: &TextEditingController, snapshot: &Snapshot) {
        self.clear_elements();
        self.set_shell(snapshot.shell);
        c.set_text(&snapshot.text);
        {
            let mut inner = self.0.borrow_mut();
            inner.elements = snapshot.elements.clone();
            inner.seen = Some(c.buffer());
            inner.remote.urls = snapshot.remote.clone();
            inner.remote.selected = None;
        }
        c.set_selection(TextSelection::collapsed(
            snapshot.caret.min(snapshot.text.len()),
        ));
    }

    /// A command or an insert session starts: the draft as it is becomes the
    /// step `u` goes back to (unless one is held already).
    pub fn vim_begin_edit(&self, c: &TextEditingController) {
        if !self.vim_enabled() || self.with_vim_history(|h| h.has_pending()) {
            return;
        }
        let snapshot = self.snapshot(c);
        let bytes = snapshot.bytes();
        self.with_vim_history(|h| h.begin(snapshot, bytes));
    }

    /// `edit`, an edit that no key made (an image from the clipboard, a
    /// remote image row removed): in Vim it is a step of undo of its own,
    /// and `.` does not repeat it (`begin_direct_vim_edit`,
    /// `finish_vim_edit`). Not while a command waits for its next key, a
    /// step is open or the history search is.
    pub fn vim_direct<R>(&self, c: &TextEditingController, edit: impl FnOnce() -> R) -> R {
        let own = self.vim_enabled()
            && !self.searching()
            && !self.with_vim(|v| v.is_operator_pending())
            && !self.with_vim_history(|h| h.has_pending());
        if own {
            self.vim_begin_edit(c);
            self.with_vim(Vim::forget_repeat);
        }
        let out = edit();
        if own {
            self.vim_finish_edit(c);
        }
        out
    }

    /// A Normal-mode command or an insert session ended: its step is kept
    /// unless the draft is as it was.
    pub fn vim_finish_edit(&self, c: &TextEditingController) {
        let now = self.snapshot(c);
        self.with_vim_history(|h| {
            h.commit(|before| {
                // a caret that only moved is no edit
                before.text == now.text
                    && Snapshot {
                        caret: before.caret,
                        ..now.clone()
                    } == *before
            });
        });
    }

    /// `u` (`redo`: Ctrl+R): the draft goes to the step before (after) the
    /// present one, in Normal mode. `true` when there was one.
    pub fn vim_step(&self, c: &TextEditingController, redo: bool) -> bool {
        let now = self.snapshot(c);
        let bytes = now.bytes();
        let step = self.with_vim_history(|h| {
            if redo {
                h.redo(now, bytes)
            } else {
                h.undo(now, bytes)
            }
        });
        let Some(snapshot) = step else {
            return false;
        };
        // what `.` repeats and the last search survive the swap
        let mut kept = Persistent::default();
        self.with_vim(|v| v.swap_persistent(&mut kept));
        self.restore(c, &snapshot);
        self.with_vim(|v| v.swap_persistent(&mut kept));
        let mut host = VimHost::new(c, self);
        self.with_vim(|v| v.enter_normal(&mut host));
        true
    }

    /// Answers recovered after a turn ended, joined to the draft as one
    /// edit (`append_recovered_drafts`): a command, query or text object
    /// that waited is cancelled; a `!` or `/command` the draft begins with is
    /// escaped by a `\` so the draft is a prompt; the answer follows a blank
    /// line at the end, as a placeholder when it is large (`label` names it from
    /// its size). `.` and `n` do not repeat it. The caller has flushed the
    /// paste burst.
    pub fn append_recovered_drafts(
        &self,
        c: &TextEditingController,
        drafts: &str,
        label: impl Fn(usize) -> String,
    ) {
        if self.with_vim(|v| v.is_operator_pending()) {
            let mut host = VimHost::new(c, self);
            self.with_vim(|v| {
                v.abort_search();
                v.enter_normal(&mut host);
            });
            self.vim_query().clear();
        }
        if self.vim_enabled() {
            self.vim_finish_edit(c);
        }
        let mut kept = Persistent::default();
        self.with_vim(|v| {
            v.swap_persistent(&mut kept);
            // what Replace mode overwrote is not what is under the caret now
            v.forget_recovery();
        });
        self.vim_direct(c, || {
            // a recovered answer makes the draft a prompt: no command prefix
            let text = c.text();
            let slash = !self.is_plain()
                && !text.starts_with(' ')
                && text.trim().starts_with('/')
                && parse_slash_name(text.trim_start())
                    .is_none_or(|(name, _, _)| !name.contains('/'));
            if self.shell() {
                self.set_shell(false);
                c.replace(0, 0, "\\!");
            } else if slash {
                let at = text.len() - text.trim_start().len();
                c.replace(at, at, "\\");
            }
            c.set_selection(TextSelection::collapsed(c.text().len()));
            if !c.text().is_empty() {
                c.insert("\n\n");
            }
            if let Some(chars) = paste_rules::large_paste(drafts) {
                let label = self.next_label(c, &label(chars));
                self.insert_element(c, &label, drafts);
            } else {
                c.insert(drafts);
            }
        });
        self.with_vim(|v| v.swap_persistent(&mut kept));
    }

    /// Back to Insert mode as a submit or a new draft leaves it.
    pub(super) fn vim_reset_mode(&self, c: &TextEditingController) {
        let mut host = VimHost::new(c, self);
        self.with_vim(|v| v.enter_insert(&mut host));
    }
}

/// The draft as the engine edits it: `c`'s text with the draft's elements
/// atomic and its kill ring as the register. The editor's own actions (the
/// keys of the Editor context) are the caller's (`editor`).
pub struct VimHost<'a> {
    c: &'a TextEditingController,
    draft: &'a ComposerDraft,
    editor: &'a dyn Fn(EditorAction) -> bool,
}

impl<'a> VimHost<'a> {
    /// A host that carries out no editor action.
    pub fn new(c: &'a TextEditingController, draft: &'a ComposerDraft) -> VimHost<'a> {
        VimHost {
            c,
            draft,
            editor: &|_| false,
        }
    }

    /// A host whose editor actions are `editor`'s.
    pub fn with_editor(
        c: &'a TextEditingController,
        draft: &'a ComposerDraft,
        editor: &'a dyn Fn(EditorAction) -> bool,
    ) -> VimHost<'a> {
        VimHost { c, draft, editor }
    }
}

impl VimText for VimHost<'_> {
    fn text(&self) -> Rc<str> {
        self.c.text()
    }

    fn cursor(&self) -> usize {
        self.c.selection().extent_offset
    }

    fn set_cursor(&mut self, pos: usize) {
        let pos = pos.min(self.c.text().len());
        self.c.set_selection(TextSelection::collapsed(pos));
        self.draft.snap_caret(self.c);
    }

    fn elements(&self) -> Vec<Range<usize>> {
        self.draft
            .elements(self.c)
            .iter()
            .map(|e| e.start..e.end)
            .collect()
    }

    fn replace(&mut self, range: Range<usize>, with: &str) {
        let caret = self.cursor();
        let (start, end) = (range.start, range.end);
        if start == 0 && !with.is_empty() {
            self.draft.0.borrow_mut().vim.prefix.0 += 1;
        }
        self.c.replace(start, end, with);
        // the controller leaves the caret after the new text; an edit moves
        // it as the engine's contract says
        let to = if caret < start {
            caret
        } else if caret <= end {
            start + with.len()
        } else {
            caret + with.len() - (end - start)
        };
        self.c
            .set_selection(TextSelection::collapsed(to.min(self.c.text().len())));
    }

    fn prev_grapheme(&self, pos: usize) -> usize {
        self.c.buffer().character_start_before(pos)
    }

    fn next_grapheme(&self, pos: usize) -> usize {
        self.c.buffer().character_end_after(pos)
    }

    fn grapheme_ends(&self, s: &str) -> Vec<usize> {
        let buffer = TextBuffer::new(s);
        let mut ends = Vec::new();
        let mut at = 0;
        while at < s.len() {
            let end = buffer.character_end_after(at);
            if end <= at {
                break;
            }
            ends.push(end);
            at = end;
        }
        ends
    }

    fn register(&self) -> Register {
        self.draft.register()
    }

    fn store_register(&mut self, range: Range<usize>, kind: RegisterKind) {
        self.draft.store(self.c, range.start, range.end, kind);
    }

    fn set_register_kind(&mut self, kind: RegisterKind) {
        self.draft.0.borrow_mut().vim.register_kind = kind;
    }

    fn paste_register_elements(&mut self, start: usize) {
        self.draft.paste_register_elements(self.c, start);
    }

    fn move_vertical(&mut self, down: bool) {
        let text = self.c.buffer();
        let caret = self.c.selection().extent_offset;
        let line = text.line_of(caret);
        let column = caret - text.line_start(line);
        let to = if down {
            let end = text.line_end(line);
            if end >= text.len() {
                text.len()
            } else {
                let next = end + 1;
                (next + column).min(text.line_end(text.line_of(next)))
            }
        } else if line == 0 {
            0
        } else {
            let prev_end = text.line_start(line) - 1;
            let prev = text.line_start(text.line_of(prev_end));
            (prev + column).min(prev_end)
        };
        self.set_cursor(to);
    }

    fn editor(&mut self, action: EditorAction) -> bool {
        (self.editor)(action)
    }
}

/// `label` (an `[Image #1]`) with its number `n`.
fn renumbered(label: &str, n: usize) -> String {
    match label.rsplit_once('#') {
        Some((before, after)) => {
            let rest = after.trim_start_matches(|c: char| c.is_ascii_digit());
            format!("{before}#{n}{rest}")
        }
        None => label.to_owned(),
    }
}
