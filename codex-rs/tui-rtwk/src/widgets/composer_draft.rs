//! What the composer's draft holds beyond its text (UI state, one per thread
//! in `ThreadUi`, never seen by `apply`): the atomic elements of the text
//! (the `[Pasted Content N chars]` placeholders of large pastes and the
//! payload each stands for) and the kill ring of `Ctrl+K` / `Ctrl+Y`.
//!
//! The text itself, its caret, selection and undo are the
//! `TextEditingController`'s. As a Flutter app keeps the spans of a
//! `TextEditingController` subclass in step with its value, the draft keeps
//! its elements in step with the controller's text: `sync` maps them across
//! each change (an edit before an element moves it, an edit that cuts into
//! one breaks it and drops its payload), and the keys that would step or
//! delete into an element take it whole (`Composer`'s `EditingKeys`).
//! Oracle: `bottom_pane/chat_composer.rs` (`pending_pastes`,
//! `next_large_paste_placeholder`, `expand_pending_pastes`),
//! `chat_composer/paste_input.rs` (`apply_paste`) and `bottom_pane/textarea.rs`
//! (elements, `kill_buffer`).

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use rtwk::editing::{TextBuffer, TextEditingController, TextSelection};

use codex_tui_core::data::history::{Lookup, Store};
use codex_tui_core::data::mention::Mention;
use codex_tui_core::data::task::valid_thread_path;
use codex_tui_core::kit::history::{
    HistoryEntry, MentionBinding, PromptWalk as Prompts, TextElement,
};
use codex_tui_core::kit::mentions::rebind::find_next_mention_token_range;
use codex_tui_core::kit::mentions::session::ComposerBinding;
use codex_tui_core::kit::mentions::tool_mentions;
use codex_tui_core::kit::paste as paste_rules;
use codex_tui_core::kit::paste_burst::PasteBurst;
use codex_tui_core::threads::{LocalImage, UserMessage};

use crate::services::images;

mod clipboard;
mod mentions;
mod remote;
mod restricted;
mod search;
mod vim;

pub use mentions::Mentions;
pub use vim::{Snapshot, VimHost};

/// One atomic element of the text: the bytes `start..end` show `label`
/// and stand for `payload` on submit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub start: usize,
    pub end: usize,
    pub label: Rc<str>,
    pub payload: Rc<str>,
    /// The file of an `[Image #N]` placeholder (its payload is its label,
    /// the image goes with the turn), `None` for a paste.
    pub image: Option<Rc<PathBuf>>,
    /// When it was attached: images are numbered in this order
    /// (`AttachmentState::relabel_local_images`).
    pub seq: u64,
    /// What a picked `$skill`, `@plugin` or task mention stands for
    /// (`ComposerMentionBinding`), `None` for a paste or an image.
    pub binding: Option<ComposerBinding>,
}

#[derive(Default)]
struct Inner {
    elements: Vec<Element>,
    /// The next `Element::seq`.
    next_seq: u64,
    /// The text the elements are measured on.
    seen: Option<TextBuffer>,
    /// What the last `Ctrl+K` / `Ctrl+W` killed (`textarea.rs`
    /// `kill_buffer`), put back by `Ctrl+Y`, with the elements it held.
    kill: Option<(String, Vec<Element>)>,
    /// The prompts history of this composer: what it sent or cleared, the
    /// walk of Up and Down and the Ctrl+R search over them and the file's
    /// (`ChatComposerHistory`; the file is the app's `Store`).
    prompts: Prompts,
    /// Shell mode (`is_bash_mode`): a `!` typed at the draft's start left
    /// the text and became the prompt; a submit sends it back in front.
    shell: bool,
    /// The draft of a plain-text composer (`ChatComposerConfig::plain_text`,
    /// the answers inside question sheets): no shell mode, no image paths.
    plain: bool,
    /// The separator space a `$skill` completion inserted, while the next
    /// key may still replace it with trailing punctuation
    /// (`skill_completion_space`).
    skill_space: Option<usize>,
    /// The paste-burst detector (`draft.paste_burst`): keys that arrive as
    /// fast as a paste are held and go in as one paste.
    burst: PasteBurst,
    /// Modal (Vim) editing: the engine, the register's kind, the undo
    /// history and the search query.
    vim: vim::Modal,
    /// Reverse history search (Ctrl+R), while it is open.
    search: Option<search::Session>,
    /// The mention popups (`@`, `$`): rows, highlight and scroll.
    mentions: Mentions,
    /// The remote images above the text and the row selected.
    remote: remote::Remote,
    /// The Ctrl+V the draft waits an answer for.
    clipboard: clipboard::Waiting,
    /// Counts the changes of that search: a page that shows its footer asks
    /// whether it is the one it drew (`ComposerDraft::search_rev`).
    search_rev: u64,
}

/// The draft's elements and kill ring, shared by every clone (a handle,
/// like the controller it goes with).
#[derive(Clone, Default)]
pub struct ComposerDraft(Rc<RefCell<Inner>>);

impl PartialEq for ComposerDraft {
    fn eq(&self, other: &ComposerDraft) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for ComposerDraft {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComposerDraft")
            .field("elements", &self.0.borrow().elements)
            .finish_non_exhaustive()
    }
}

impl ComposerDraft {
    /// The draft of a plain-text composer (`ChatComposerConfig::plain_text`):
    /// `!` stays literal text (shell mode cannot be entered) and a pasted
    /// path of an image file is text too; the blockquote paste and the large
    /// paste placeholders stay.
    pub fn plain() -> ComposerDraft {
        ComposerDraft(Rc::new(RefCell::new(Inner {
            plain: true,
            ..Inner::default()
        })))
    }

    /// Whether this is a plain-text composer's draft.
    pub fn is_plain(&self) -> bool {
        self.0.borrow().plain
    }

    /// The mention popups of this draft (a handle).
    pub fn mentions(&self) -> Mentions {
        self.0.borrow().mentions.clone()
    }

    /// A mention completion `text` (`… $skill `, `… @file `) was put in
    /// before the caret with a separator space of its own. As the oracle's
    /// `advance_past_completion_separator`: a plain space already after the
    /// caret, with nothing but blanks behind it, is the separator (the
    /// caret steps over it); otherwise the new space stays, and for a skill
    /// the next key may swap it for punctuation.
    pub fn completed(&self, c: &TextEditingController, text: &str) {
        let word = text.trim_end().rsplit(' ').next().unwrap_or_default();
        if !text.ends_with(' ') || !word.starts_with(['$', '@']) {
            return;
        }
        let caret = c.selection().extent_offset;
        let Some(added) = caret
            .checked_sub(1)
            .filter(|at| c.text().get(*at..caret) == Some(" "))
        else {
            return;
        };
        let rest = &c.text()[caret..];
        let mut chars = rest.chars();
        let existing = chars.next().filter(|ch| {
            ch.is_whitespace() && !"\n\r\u{b}\u{c}\u{85}\u{2028}\u{2029}".contains(*ch)
        });
        if let Some(sep) = existing
            && chars.next().is_none_or(char::is_whitespace)
        {
            c.replace(added, caret, "");
            c.set_selection(TextSelection::collapsed(added + sep.len_utf8()));
        } else if word.starts_with('$') {
            self.0.borrow_mut().skill_space = Some(added);
        }
    }

    /// The completion's space, kept only for the key that asks (every key
    /// but a punctuation mark right after it forgets it).
    pub fn take_skill_space(&self) -> Option<usize> {
        self.0.borrow_mut().skill_space.take()
    }

    /// Keeps the completion's space for the next key (a held first
    /// character of a burst does not use it up).
    pub fn keep_skill_space(&self, at: Option<usize>) {
        self.0.borrow_mut().skill_space = at;
    }

    /// Runs `f` on the draft's paste-burst detector (`f` must not call back
    /// into the draft).
    pub fn burst<R>(&self, f: impl FnOnce(&mut PasteBurst) -> R) -> R {
        f(&mut self.0.borrow_mut().burst)
    }

    /// Whether keys are held as a paste right now (`is_in_paste_burst`):
    /// the `?` and Left shortcuts of an empty prompt wait for it.
    pub fn in_burst(&self) -> bool {
        self.0.borrow().burst.is_active()
    }

    /// The elements as they stand in `c`'s text, start first.
    pub fn elements(&self, c: &TextEditingController) -> Vec<Element> {
        self.sync(c);
        self.0.borrow().elements.clone()
    }

    /// Maps the elements across the changes of `c`'s text since they were
    /// last measured: an element after a change moves with it, one a change
    /// cut into is gone (its payload with it). Called by the controller's
    /// listener and before every use, so an edit made while no composer
    /// listened (a draft put in by the app) is mapped too.
    pub fn sync(&self, c: &TextEditingController) {
        let now = c.buffer();
        let mut inner = self.0.borrow_mut();
        let Some(seen) = inner.seen.replace(now.clone()) else {
            return;
        };
        if TextBuffer::ptr_eq(&seen, &now) || inner.elements.is_empty() {
            return;
        }
        let (start, old_end, new_end) = now
            .change_since(&seen)
            .unwrap_or_else(|| changed(&seen.text(), &now.text()));
        inner.elements.retain_mut(|e| {
            if e.end <= start {
                true
            } else if e.start >= old_end {
                e.start = e.start + new_end - old_end;
                e.end = e.end + new_end - old_end;
                true
            } else {
                false
            }
        });
        // a text that no longer holds an element's label there (a reset to
        // another draft) drops it
        let text = now.text();
        inner
            .elements
            .retain(|e| text.get(e.start..e.end) == Some(&*e.label));
    }

    /// Forgets every element and the text they were measured on (a submit
    /// or a draft the app replaced).
    pub fn clear(&self) {
        let mut inner = self.0.borrow_mut();
        inner.elements.clear();
        inner.seen = None;
    }

    /// The paste rule of `apply_paste` (core's `large_paste`): a large
    /// paste goes in as a placeholder element at the selection, named by
    /// `label` from its size, and `true` says it was taken; a shorter one is
    /// left to the field; a path of an image file is attached, named by
    /// `image` from its number.
    pub fn paste(
        &self,
        c: &TextEditingController,
        pasted: &str,
        label: impl Fn(usize) -> String,
        image: impl Fn(usize) -> String,
    ) -> bool {
        let original = pasted;
        let sanitized = paste_rules::sanitize_paste(pasted);
        let pasted = sanitized.as_str();
        // a multiline paste after `> ` goes in quoted, and the caret leaves
        // the quote (`apply_paste`); shell input stays literal
        let quoted = (!self.shell())
            .then(|| {
                let at = c.selection().start();
                let before = &c.text()[..at];
                let line = before.rsplit('\n').next().unwrap_or_default();
                paste_rules::blockquote_paste(line, pasted)
            })
            .flatten();
        let text = quoted.as_deref().unwrap_or(pasted);
        // a small paste that puts a `!` at the draft's start is shell mode
        // (`insert_str_at_target`: `ShellPrefixHandling::Interpret`)
        let at_start = c.selection().start() == 0;
        if let Some(chars) = paste_rules::large_paste(text) {
            let label = self.next_label(c, &label(chars));
            self.insert_element(c, &label, text);
        } else if text.chars().nth(1).is_some()
            && !self.is_plain()
            && let Some(path) = images::pasted_image(text)
        {
            // a dragged image file: attached, a space after it
            // (`handle_paste_image_path`)
            self.attach_image(c, path, image);
            c.insert(" ");
        } else if at_start && text.starts_with('!') && !self.shell() && !self.is_plain() {
            c.insert(text);
            self.absorb_shell_prefix(c);
        } else if quoted.is_some() || text != original {
            // what the field would insert is not what the draft takes
            c.insert(text);
        } else {
            return false;
        }
        if quoted.is_some() {
            c.insert("\n\n");
        }
        true
    }

    /// Puts an element showing `label` for `payload` in place of the
    /// selection, the caret after it.
    pub fn insert_element(&self, c: &TextEditingController, label: &str, payload: &str) {
        self.insert_with(c, label, payload, None, None);
    }

    /// Puts the mention `text` (`$skill`, `@plugin`) in place of the
    /// selection as one atomic element, the caret after it; `binding` is
    /// what it stands for on submit (`insert_selected_mention`).
    pub fn insert_mention(
        &self,
        c: &TextEditingController,
        text: &str,
        binding: Option<ComposerBinding>,
    ) {
        self.insert_with(c, text, text, None, binding);
    }

    /// Attaches the image file `path` at the selection as the next
    /// `[Image #N]` (`AttachmentState::attach_image`).
    pub fn attach_image(
        &self,
        c: &TextEditingController,
        path: PathBuf,
        image: impl Fn(usize) -> String,
    ) {
        let images = self
            .elements(c)
            .iter()
            .filter(|e| e.image.is_some())
            .count();
        let label = image(self.remote_count() + images + 1);
        self.insert_with(c, &label, &label, Some(Rc::new(path)), None);
    }

    fn insert_with(
        &self,
        c: &TextEditingController,
        label: &str,
        payload: &str,
        image: Option<Rc<PathBuf>>,
        binding: Option<ComposerBinding>,
    ) {
        self.sync(c);
        let selection = c.selection();
        let start = selection.start();
        c.insert(label);
        self.sync(c);
        let mut inner = self.0.borrow_mut();
        let at = inner.elements.partition_point(|e| e.start < start);
        let seq = inner.next_seq;
        inner.next_seq += 1;
        inner.elements.insert(
            at,
            Element {
                start,
                end: start + label.len(),
                label: Rc::from(label),
                payload: Rc::from(payload),
                image,
                seq,
                binding,
            },
        );
    }

    /// Numbers the images again in the order they were attached, `[Image
    /// #1]`.. with no gap (after one was deleted, killed or yanked back:
    /// `relabel_local_images`); the caret stays where it was in the text.
    pub fn relabel_images(&self, c: &TextEditingController, image: impl Fn(usize) -> String) {
        self.sync(c);
        let elements = self.0.borrow().elements.clone();
        let mut by_seq: Vec<usize> = (0..elements.len())
            .filter(|&i| elements[i].image.is_some())
            .collect();
        by_seq.sort_by_key(|&i| elements[i].seq);
        let mut want: Vec<Option<String>> = vec![None; elements.len()];
        let remote = self.remote_count();
        for (rank, &i) in by_seq.iter().enumerate() {
            let label = image(remote + rank + 1);
            if *elements[i].label != *label {
                want[i] = Some(label);
            }
        }
        if want.iter().all(Option::is_none) {
            return;
        }
        let selection = c.selection();
        // the field's listeners read the elements while the text changes:
        // none, rather than the old ones
        {
            let mut inner = self.0.borrow_mut();
            inner.elements.clear();
            inner.seen = None;
        }
        for (i, label) in want.iter().enumerate().rev() {
            if let Some(label) = label {
                c.replace(elements[i].start, elements[i].end, label);
            }
        }
        let mut delta = 0isize;
        let mut moved = elements;
        // where the text before each element's old end moved to
        let mut shifts = Vec::new();
        for (e, label) in moved.iter_mut().zip(&want) {
            let old_len = e.end - e.start;
            e.start = e.start.saturating_add_signed(delta);
            let old_end = e.end;
            if let Some(label) = label {
                delta += isize::try_from(label.len()).unwrap_or(0)
                    - isize::try_from(old_len).unwrap_or(0);
                e.label = Rc::from(label.as_str());
                e.payload = e.label.clone();
            }
            e.end = e.start + e.label.len();
            shifts.push((old_end, delta));
        }
        {
            let mut inner = self.0.borrow_mut();
            inner.elements = moved;
            inner.seen = Some(c.buffer());
        }
        let after = |offset: usize| {
            let by = shifts
                .iter()
                .rev()
                .find(|(end, _)| *end <= offset)
                .map_or(0, |(_, delta)| *delta);
            offset.saturating_add_signed(by)
        };
        c.set_selection(TextSelection::new(
            after(selection.base_offset),
            after(selection.extent_offset),
        ));
    }

    /// `next_large_paste_placeholder`: `[Pasted Content N chars]`, then
    /// ` #2`, ` #3`, ... past the highest suffix of that size still in the
    /// draft.
    fn next_label(&self, c: &TextEditingController, base: &str) -> String {
        let base = base.to_owned();
        let prefix = format!("{base} #");
        let top = self
            .elements(c)
            .iter()
            .filter_map(|e| {
                if *e.label == *base {
                    Some(1)
                } else {
                    e.label.strip_prefix(&prefix)?.parse::<usize>().ok()
                }
            })
            .max()
            .unwrap_or(0);
        if top == 0 {
            base
        } else {
            format!("{base} #{}", top + 1)
        }
    }

    /// The text with every element's label replaced by its payload: what a
    /// submit sends (`expand_pending_pastes`).
    pub fn expanded(&self, c: &TextEditingController) -> String {
        self.expand(c).0
    }

    /// [`submission_with`](Self::submission_with) where no catalog lists a tool: the text and
    /// the images.
    pub fn submission(
        &self,
        c: &TextEditingController,
        image: impl Fn(usize) -> String,
    ) -> UserMessage {
        self.submission_with(c, image, &[])
    }

    /// What a submit sends: the expanded text, the remote images above the draft, the images
    /// its placeholders stand for, in the order they were attached, each with the placeholder's
    /// range in that text, and the tools its bound mentions name that the
    /// `catalog` lists (`prepare_submission_text`,
    /// `prune_local_images_for_submission`, `input_submission.rs`).
    pub fn submission_with(
        &self,
        c: &TextEditingController,
        image: impl Fn(usize) -> String,
        catalog: &[Mention],
    ) -> UserMessage {
        self.relabel_images(c, image);
        let (text, placed) = self.expand(c);
        let bindings: Vec<ComposerBinding> = placed
            .iter()
            .filter_map(|(_, e)| e.binding.clone())
            .collect();
        let mut images: Vec<(u64, LocalImage)> = placed
            .into_iter()
            .filter_map(|(range, e)| {
                let path = e.image?;
                Some((
                    e.seq,
                    LocalImage {
                        path: (*path).clone(),
                        range,
                    },
                ))
            })
            .collect();
        images.sort_by_key(|(seq, _)| *seq);
        UserMessage {
            text,
            images: images.into_iter().map(|(_, image)| image).collect(),
            remote_images: self.remote_images(),
            mentions: tool_mentions(&bindings, catalog),
        }
    }

    /// The expanded text and where each element's payload stands in it.
    fn expand(
        &self,
        c: &TextEditingController,
    ) -> (String, Vec<(std::ops::Range<usize>, Element)>) {
        let elements = self.elements(c);
        let text = c.text();
        let mut out = String::with_capacity(text.len() + 1);
        if self.shell() {
            out.push('!');
        }
        let mut placed = Vec::new();
        let mut at = 0;
        for e in elements {
            out.push_str(&text[at..e.start]);
            let start = out.len();
            out.push_str(&e.payload);
            placed.push((start..out.len(), e.clone()));
            at = e.end;
        }
        out.push_str(&text[at..]);
        (out, placed)
    }

    /// Whether the draft is in shell mode.
    pub fn shell(&self) -> bool {
        self.0.borrow().shell
    }

    /// Enters or leaves shell mode; `true` when that changed it.
    pub fn set_shell(&self, on: bool) -> bool {
        let mut inner = self.0.borrow_mut();
        // a plain-text draft has no shell mode (`shell_commands_enabled`)
        if inner.plain {
            return false;
        }
        let changed = inner.shell != on;
        inner.shell = on;
        changed
    }

    /// A `!` the text starts with becomes shell mode, its prompt
    /// (`sync_bash_mode_from_text`): out of the text, the caret kept where
    /// it was in what follows. `true` when it did.
    pub fn absorb_shell_prefix(&self, c: &TextEditingController) -> bool {
        if self.shell() || self.is_plain() || !c.text().starts_with('!') {
            return false;
        }
        let selection = c.selection();
        // shell mode first: the command popup reads it when the text changes
        // (a `!/diff` paste must not open it on `/diff`)
        self.set_shell(true);
        c.replace(0, 1, "");
        c.set_selection(TextSelection::new(
            selection.base_offset.saturating_sub(1),
            selection.extent_offset.saturating_sub(1),
        ));
        true
    }

    /// The element that holds `offset` strictly inside it.
    fn inside(&self, c: &TextEditingController, offset: usize) -> Option<Element> {
        self.elements(c)
            .into_iter()
            .find(|e| e.start < offset && offset < e.end)
    }

    /// The element that ends at `offset`.
    pub fn ending_at(&self, c: &TextEditingController, offset: usize) -> Option<Element> {
        self.elements(c).into_iter().find(|e| e.end == offset)
    }

    /// The element that starts at `offset`.
    pub fn starting_at(&self, c: &TextEditingController, offset: usize) -> Option<Element> {
        self.elements(c).into_iter().find(|e| e.start == offset)
    }

    /// A caret the field or the mouse put inside an element moves to its
    /// nearer end (`clamp_pos_to_nearest_boundary`), the start on a tie.
    /// A selection that cuts an element (a double click in its label, a drag
    /// or Shift+arrows across it) grows to hold the element whole: the
    /// elements are atomic.
    pub fn snap_caret(&self, c: &TextEditingController) {
        let selection = c.selection();
        if !selection.is_collapsed() {
            let (from, to) = (selection.start(), selection.end());
            let from = self.inside(c, from).map_or(from, |e| e.start);
            let to = self.inside(c, to).map_or(to, |e| e.end);
            let grown = if selection.base_offset <= selection.extent_offset {
                TextSelection::new(from, to)
            } else {
                TextSelection::new(to, from)
            };
            if grown != selection {
                c.set_selection(grown);
            }
            return;
        }
        let caret = selection.extent_offset;
        if let Some(e) = self.inside(c, caret) {
            let to = if caret - e.start <= e.end - caret {
                e.start
            } else {
                e.end
            };
            c.set_selection(TextSelection::collapsed(to));
        }
    }

    /// Keeps `text` (and the elements in `from..to` of the draft) as the
    /// last kill.
    pub fn kill(&self, c: &TextEditingController, from: usize, to: usize) {
        let held: Vec<Element> = self
            .elements(c)
            .into_iter()
            .filter(|e| from <= e.start && e.end <= to)
            .map(|e| Element {
                start: e.start - from,
                end: e.end - from,
                ..e
            })
            .collect();
        let text = c.text()[from..to].to_string();
        let mut inner = self.0.borrow_mut();
        inner.kill = Some((text, held));
        // a kill pastes as characters; Vim's linewise yank says otherwise after
        inner.vim.register_kind = Default::default();
    }

    /// Puts the last kill back at the caret, its elements whole; `false`
    /// with nothing killed.
    pub fn yank(&self, c: &TextEditingController) -> bool {
        let Some((text, held)) = self.0.borrow().kill.clone() else {
            return false;
        };
        if text.is_empty() {
            return false;
        }
        let mut at = 0;
        for e in &held {
            if at < e.start {
                c.insert(&text[at..e.start]);
            }
            self.insert_with(c, &e.label, &e.payload, e.image.clone(), e.binding.clone());
            at = e.end;
        }
        if at < text.len() {
            c.insert(&text[at..]);
        }
        true
    }
}

impl ComposerDraft {
    /// Up (`up`) or Down at the draft's ends (`navigate_up`,
    /// `navigate_down`, behind `should_handle_navigation`): the entry it
    /// finds replaces the draft, caret at its end. A persistent entry the
    /// store does not hold yet is asked for in `fx` and the draft waits
    /// for `history_poll`. `false`: the key is the field's (the caret
    /// moves).
    pub fn history_step(
        &self,
        c: &TextEditingController,
        store: &Store,
        up: bool,
        fx: &mut Vec<Lookup>,
    ) -> bool {
        let Some((text, cursor)) = self.navigation(c) else {
            return false;
        };
        let entry = {
            let mut inner = self.0.borrow_mut();
            if !inner.prompts.should_handle_navigation(store, &text, cursor) {
                return false;
            }
            if up {
                inner.prompts.navigate_up(store, fx)
            } else {
                inner.prompts.navigate_down(store, fx)
            }
        };
        let Some(entry) = entry else {
            return false;
        };
        self.recall(c, entry);
        true
    }

    /// Whether Up and Down walk the history from here (an empty draft, or
    /// the text of the last recall with the caret at an end): no popup
    /// opens meanwhile, nothing steals the keys of the walk
    /// (`browsing_history` in `sync_popups`).
    pub fn history_browsing(&self, c: &TextEditingController, store: &Store) -> bool {
        let Some((text, cursor)) = self.navigation(c) else {
            return false;
        };
        self.0
            .borrow()
            .prompts
            .should_handle_navigation(store, &text, cursor)
    }

    /// Whether a read the history asked for is outstanding: the composer
    /// polls when the store changes.
    pub fn history_waiting(&self) -> bool {
        self.0.borrow().prompts.is_waiting()
    }

    /// The store changed: the entry Up or Down waited for goes into the
    /// draft; a search's answer is returned for `search_poll`'s owner.
    /// `true` when the draft or the search changed.
    pub fn history_poll(
        &self,
        c: &TextEditingController,
        store: &Store,
        fx: &mut Vec<Lookup>,
    ) -> bool {
        use codex_tui_core::kit::history::Response;
        let response = self.0.borrow_mut().prompts.poll(store, fx);
        match response {
            Some(Response::Found(entry)) => {
                self.recall(c, entry);
                true
            }
            Some(Response::Search(result)) => {
                self.search_result(c, result);
                true
            }
            Some(Response::Ignored) | None => false,
        }
    }

    /// What the walk sees of the draft: its text as the oracle's
    /// `current_text()` (shell mode's `!` in front) and the caret in it
    /// (`history_navigation_cursor`: Vim's Normal mode rests on the last
    /// character, which counts as the end). None while text is selected.
    fn navigation(&self, c: &TextEditingController) -> Option<(String, usize)> {
        let selection = c.selection();
        if !selection.is_collapsed() {
            return None;
        }
        let shell = self.shell();
        let text = c.text();
        let full = if shell {
            format!("!{text}")
        } else {
            text.to_string()
        };
        let caret = selection.extent_offset;
        let cursor = if shell && caret == 0 {
            0
        } else if !text.is_empty()
            && self.0.borrow().vim.vim.is_normal()
            && caret == self.vim_end(c)
        {
            full.len()
        } else {
            caret + usize::from(shell)
        };
        Some((full, cursor))
    }

    /// The caret at the draft's end, where a recalled entry or an accepted
    /// search match leaves it; Vim's Normal mode rests on the last
    /// character instead (the textarea clamps it).
    pub(super) fn rest_at_end(&self, c: &TextEditingController) {
        let end = c.text().len();
        let at = if end > 0 && self.0.borrow().vim.vim.is_normal() {
            self.vim_end(c)
        } else {
            end
        };
        c.set_selection(TextSelection::collapsed(at));
    }

    /// The caret that stands for the last item in Vim's Normal mode
    /// (`vim_normal_end_cursor`): the start of the last character or
    /// element.
    fn vim_end(&self, c: &TextEditingController) -> usize {
        let len = c.text().len();
        match self.ending_at(c, len) {
            Some(e) => e.start,
            None => c.buffer().character_start_before(len),
        }
    }

    /// The draft as a history entry: the text as it shows (`!` in front in
    /// shell mode) with its placeholders, the images they stand for and the
    /// pastes (`clear_for_ctrl_c`).
    fn kept_entry(&self, c: &TextEditingController) -> HistoryEntry {
        let skip = usize::from(self.shell());
        let mut entry = HistoryEntry::new(if skip == 1 {
            format!("!{}", c.text())
        } else {
            c.text().to_string()
        });
        for e in self.elements(c) {
            entry.text_elements.push(TextElement {
                start: e.start + skip,
                end: e.end + skip,
                placeholder: Some(e.label.to_string()),
            });
            match (&e.image, &e.binding) {
                (Some(path), _) => entry.local_image_paths.push((**path).clone()),
                (None, Some(binding)) => entry.mention_bindings.push(MentionBinding {
                    sigil: binding.sigil,
                    mention: binding.mention.clone(),
                    path: binding.path.clone(),
                }),
                (None, None) => entry
                    .pending_pastes
                    .push((e.label.to_string(), e.payload.to_string())),
            }
        }
        entry.remote_image_urls = self.remote_images();
        entry
    }

    /// The draft as the submit sends it, as a history entry: pastes
    /// expanded, the image placeholders kept (`prepare_submission_text`
    /// records the expanded text).
    fn sent_entry(&self, c: &TextEditingController) -> HistoryEntry {
        let (text, placed) = self.expand(c);
        let mut entry = HistoryEntry::new(text);
        for (range, e) in placed {
            if let Some(path) = &e.image {
                entry.text_elements.push(TextElement {
                    start: range.start,
                    end: range.end,
                    placeholder: Some(e.label.to_string()),
                });
                entry.local_image_paths.push((**path).clone());
            } else if let Some(binding) = &e.binding {
                entry.mention_bindings.push(MentionBinding {
                    sigil: binding.sigil,
                    mention: binding.mention.clone(),
                    path: binding.path.clone(),
                });
            }
        }
        entry.remote_image_urls = self.remote_images();
        entry
    }

    /// `entry` put in the draft's place (`apply_history_entry`): its text
    /// without a leading `!` (shell mode again, `imported_text_for_textarea`),
    /// its placeholders as elements with the payload or the image they
    /// stand for, the caret at the end, and the text noted as the one the
    /// walk goes on from.
    pub(super) fn recall(&self, c: &TextEditingController, entry: HistoryEntry) {
        let shell = entry.text.starts_with('!');
        let skip = usize::from(shell);
        let text = entry.text[skip..].to_string();
        let mut images = entry.local_image_paths.iter();
        let mut elements = Vec::new();
        for e in &entry.text_elements {
            let (Some(start), Some(end)) = (e.start.checked_sub(skip), e.end.checked_sub(skip))
            else {
                continue;
            };
            let Some(label) = text.get(start..end) else {
                continue;
            };
            let label: Rc<str> = Rc::from(label);
            let paste = entry.pending_pastes.iter().find(|(p, _)| **p == *label);
            let (payload, image) = if let Some((_, payload)) = paste {
                (Rc::from(payload.as_str()), None)
            } else if let Some(path) = images.next() {
                (label.clone(), Some(Rc::new(path.clone())))
            } else {
                continue;
            };
            elements.push(Element {
                start,
                end,
                label,
                payload,
                image,
                seq: 0,
                binding: None,
            });
        }
        bind_mentions(&text, &entry, skip, &mut elements);
        self.clear();
        self.set_shell(shell);
        c.set_text(&text);
        {
            let mut inner = self.0.borrow_mut();
            for e in &mut elements {
                e.seq = inner.next_seq;
                inner.next_seq += 1;
            }
            inner.elements = elements;
            inner.seen = Some(c.buffer());
            inner.remote.urls = entry.remote_image_urls.clone();
            inner.remote.selected = None;
            // a recalled entry is a new draft: Vim's undo steps were of the old one
            inner.vim.history.reset();
        }
        self.rest_at_end(c);
        let shown = if shell { format!("!{text}") } else { text };
        self.0.borrow_mut().prompts.record_recalled_text(shown);
    }

    /// The composer cleared (the thread took the submit, a command opened a
    /// screen): the line it sent is kept for Up and Ctrl+R with its
    /// pastes expanded (`record_local_submission`), the history walk is
    /// over and shell mode left. A submit the thread did not take clears
    /// nothing, and keeps all of it.
    pub fn cleared(&self, c: &TextEditingController) {
        let entry = self.sent_entry(c);
        self.clear_into_history(c, entry);
    }

    /// A command line the composer keeps because it was refused (a command
    /// while a task runs, `/review` validated by dispatch): the line is in
    /// the local recall and stays in the composer, to be corrected
    /// (`record_pending_slash_command_history`).
    pub fn recorded(&self, c: &TextEditingController) {
        let entry = self.sent_entry(c);
        self.0.borrow_mut().prompts.record_local_submission(entry);
    }

    /// A user message of a replayed transcript (`record_replayed_user_message_history`):
    /// kept for Up and Ctrl+R as a local entry, and remembered so the history
    /// file's copy of it is not offered twice. A message the recall holds
    /// already (the page seeds again from the whole list when older turns
    /// came) is not added twice.
    pub fn replayed(&self, entry: HistoryEntry) {
        let mut inner = self.0.borrow_mut();
        if !inner.prompts.local().contains(&entry) {
            inner.prompts.record_replayed_submission(entry);
        }
    }

    /// Ctrl+C on a draft (`clear_for_ctrl_c`): the draft is kept for Up and
    /// Ctrl+R whole, its placeholders and what they stand for with it.
    pub fn cancelled(&self, c: &TextEditingController) {
        let entry = self.kept_entry(c);
        self.clear_into_history(c, entry);
    }

    fn clear_into_history(&self, c: &TextEditingController, entry: HistoryEntry) {
        {
            let mut inner = self.0.borrow_mut();
            inner.shell = false;
            inner.remote = remote::Remote::default();
            inner.vim.history.reset();
            inner.prompts.reset_navigation();
            inner.prompts.record_local_submission(entry);
        }
        self.vim_reset_mode(c);
    }
}

/// The changed part of `b` against `a`: `(start, old_end, new_end)` after
/// the common head and tail, on character boundaries.
/// The mentions of `entry` as elements of `text` again
/// (`bind_mentions_from_snapshot`): a task's where the entry's text
/// elements hold its token, any other at the next place its token stands as
/// a word of its own; each after the one before.
fn bind_mentions(text: &str, entry: &HistoryEntry, skip: usize, elements: &mut Vec<Element>) {
    let mut scan_from = 0;
    for binding in &entry.mention_bindings {
        let token = format!("{}{}", binding.sigil, binding.mention);
        let range = if valid_thread_path(&binding.path).is_some() {
            entry
                .text_elements
                .iter()
                .filter_map(|e| Some(e.start.checked_sub(skip)?..e.end.checked_sub(skip)?))
                .find(|r| r.start >= scan_from && text.get(r.clone()) == Some(token.as_str()))
        } else {
            find_next_mention_token_range(text, &token, scan_from)
        };
        let Some(range) = range else { continue };
        let taken = elements
            .iter()
            .any(|e| e.start < range.end && range.start < e.end);
        if !taken {
            elements.push(Element {
                start: range.start,
                end: range.end,
                label: Rc::from(token.as_str()),
                payload: Rc::from(token.as_str()),
                image: None,
                seq: 0,
                binding: Some(ComposerBinding {
                    sigil: binding.sigil,
                    mention: binding.mention.clone(),
                    path: binding.path.clone(),
                }),
            });
        }
        scan_from = range.end;
    }
    elements.sort_by_key(|e| e.start);
}

fn changed(a: &str, b: &str) -> (usize, usize, usize) {
    let head = a
        .char_indices()
        .zip(b.chars())
        .find(|((_, x), y)| x != y)
        .map_or(a.len().min(b.len()), |((i, _), _)| i);
    let tail = a[head..]
        .chars()
        .rev()
        .zip(b[head..].chars().rev())
        .take_while(|(x, y)| x == y)
        .map(|(x, _)| x.len_utf8())
        .sum::<usize>();
    (head, a.len() - tail, b.len() - tail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_finds_the_edited_middle() {
        assert_eq!(changed("abcdef", "abXdef"), (2, 3, 3));
        assert_eq!(changed("abc", "abc!"), (3, 3, 4));
        assert_eq!(changed("abc", "bc"), (0, 1, 0));
    }

    #[test]
    fn an_element_moves_with_an_edit_before_it_and_breaks_when_cut() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        assert!(d.paste(
            &c,
            &"x".repeat(1001),
            |n| format!("[{n}]"),
            |n| format!("<{n}>")
        ));
        assert_eq!(&*c.text(), "[1001]");
        c.set_selection(TextSelection::collapsed(0));
        c.insert("hi ");
        let e = d.elements(&c);
        assert_eq!((e[0].start, e[0].end), (3, 9));
        assert_eq!(d.expanded(&c), format!("hi {}", "x".repeat(1001)));
        c.replace(5, 6, "");
        assert!(d.elements(&c).is_empty());
    }

    /// A double click in a label, a drag or Shift+arrows across its edge
    /// select the element whole: the elements are atomic.
    #[test]
    fn a_selection_that_cuts_an_element_grows_to_hold_it() {
        let c = TextEditingController::new("");
        let d = ComposerDraft::default();
        c.insert("ab ");
        assert!(d.paste(
            &c,
            &"x".repeat(1001),
            |n| format!("[{n}]"),
            |n| format!("<{n}>")
        ));
        c.insert(" cd");
        assert_eq!(&*c.text(), "ab [1001] cd");
        // inside the label: both ends go out to its edges
        c.set_selection(TextSelection::new(5, 7));
        d.snap_caret(&c);
        assert_eq!((c.selection().start(), c.selection().end()), (3, 9));
        // one end in the label, the other in the text: only the cut end moves
        c.set_selection(TextSelection::new(1, 6));
        d.snap_caret(&c);
        assert_eq!((c.selection().start(), c.selection().end()), (1, 9));
        // the direction of a drag is kept
        c.set_selection(TextSelection::new(11, 6));
        d.snap_caret(&c);
        assert_eq!(c.selection(), TextSelection::new(11, 3));
        // a selection that cuts nothing stays as it is
        c.set_selection(TextSelection::new(0, 3));
        d.snap_caret(&c);
        assert_eq!(c.selection(), TextSelection::new(0, 3));
    }
}
